use crate::error::{Error, Result};
use crate::types::{
    ExternalSupplySlack, FacilityMachineCount, OptimizationResult, OutpostValue, RecipeUsage,
    SaleValue, SolveInputs, StageSolution, ThermalBankUsage,
};
use end_model::{Catalog, FacilityId, FacilityKind, ItemId};
use good_lp::{
    Expression, Solution, SolverModel, Variable, constraint, default_solver, variable, variables,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Maximum tolerated distance from an integer value for decoded integer variables.
pub const NEAR_INT_EPS: f64 = 1e-6;
/// Relative epsilon used to derive stage-2 revenue floor from stage-1 objective.
pub const STAGE2_REVENUE_FLOOR_REL_EPS: f64 = 1e-6;

#[derive(Debug, Clone, Copy)]
enum StageObjective {
    MaxRevenue,
    MinMachines { revenue_floor_per_min: f64 },
}

#[derive(Debug, Clone)]
struct OutpostVars {
    outpost_index: usize,
    money_cap_per_hour: u32,
    sell_lines: Vec<(ItemId, u32, Variable)>,
}

#[derive(Debug, Clone)]
struct RecipeVars {
    recipe_index: usize,
    facility: FacilityId,
    x: Variable,
    y: Variable,
    throughput_per_min: f64,
    net: HashMap<ItemId, f64>,
}

#[derive(Debug, Clone)]
struct PowerVars {
    power_recipe_index: usize,
    ingredient: ItemId,
    power_w: u32,
    duration_s: u32,
    z: Variable,
}

/// Run the two-stage optimizer:
/// 1) maximize revenue, 2) minimize machines under a near-optimal revenue floor.
pub fn run_two_stage(catalog: &Catalog, inputs: &SolveInputs) -> Result<OptimizationResult> {
    if catalog.recipes.is_empty() {
        return Err(Error::InvalidInput {
            message: "catalog.recipes must not be empty".to_string(),
        });
    }

    let stage1 = solve_stage(catalog, inputs, StageObjective::MaxRevenue)?;

    let rel_eps = STAGE2_REVENUE_FLOOR_REL_EPS * stage1.revenue_per_min.max(1.0);
    let revenue_floor_per_min = (stage1.revenue_per_min - rel_eps).max(0.0);
    let stage2 = solve_stage(
        catalog,
        inputs,
        StageObjective::MinMachines {
            revenue_floor_per_min,
        },
    )?;

    Ok(OptimizationResult { stage1, stage2 })
}

fn solve_stage(
    catalog: &Catalog,
    inputs: &SolveInputs,
    objective: StageObjective,
) -> Result<StageSolution> {
    let external_supply = &inputs.aic.supply_per_min;
    let mut vars = variables!();

    let mut recipe_vars = Vec::with_capacity(catalog.recipes.len());
    for (idx, recipe) in catalog.recipes.iter().enumerate() {
        let x = vars.add(variable().min(0.0));
        let y = vars.add(variable().integer().min(0.0));

        let time_s = recipe.time_s as f64;
        if time_s <= 0.0 {
            return Err(Error::InvalidInput {
                message: format!("recipe[{idx}] has non-positive time_s {}", recipe.time_s),
            });
        }

        let mut net: HashMap<ItemId, f64> = HashMap::new();
        for stack in &recipe.ingredients {
            *net.entry(stack.item).or_insert(0.0) -= stack.count as f64;
        }
        for stack in &recipe.products {
            *net.entry(stack.item).or_insert(0.0) += stack.count as f64;
        }

        recipe_vars.push(RecipeVars {
            recipe_index: idx,
            facility: recipe.facility,
            x,
            y,
            throughput_per_min: 60.0 / time_s,
            net,
        });
    }

    let mut power_vars = Vec::with_capacity(catalog.power_recipes.len());
    for (idx, p) in catalog.power_recipes.iter().enumerate() {
        let z = vars.add(variable().integer().min(0.0));
        power_vars.push(PowerVars {
            power_recipe_index: idx,
            ingredient: p.ingredient.item,
            power_w: p.power_w,
            duration_s: p.time_s,
            z,
        });
    }

    let mut outpost_vars = Vec::with_capacity(inputs.aic.outposts.len());
    for (idx, outpost) in inputs.aic.outposts.iter().enumerate() {
        let mut sell_lines = Vec::with_capacity(outpost.prices.len());
        for (&item, &price) in &outpost.prices {
            let qty = vars.add(variable().min(0.0));
            sell_lines.push((item, price, qty));
        }
        sell_lines.sort_by_key(|(item, _, _)| *item);

        outpost_vars.push(OutpostVars {
            outpost_index: idx,
            money_cap_per_hour: outpost.money_cap_per_hour,
            sell_lines,
        });
    }

    let mut items = BTreeSet::new();
    items.extend(external_supply.keys().copied());
    for rv in &recipe_vars {
        items.extend(rv.net.keys().copied());
    }
    for ov in &outpost_vars {
        items.extend(ov.sell_lines.iter().map(|(item, _, _)| *item));
    }
    for pv in &power_vars {
        items.insert(pv.ingredient);
    }

    let mut revenue = Expression::default();
    for ov in &outpost_vars {
        for (_, price, qty) in &ov.sell_lines {
            revenue += *price as f64 * *qty;
        }
    }

    let mut total_machines_expr = Expression::default();
    for rv in &recipe_vars {
        total_machines_expr += 1.0 * rv.y;
    }

    let mut total_thermal_banks_expr = Expression::default();
    for pv in &power_vars {
        total_thermal_banks_expr += 1.0 * pv.z;
    }

    let mut power_gen = Expression::default();
    power_gen += inputs.p_core_w as f64;
    for pv in &power_vars {
        power_gen += pv.power_w as f64 * pv.z;
    }

    let mut power_use = Expression::default();
    power_use += inputs.aic.external_power_consumption_w as f64;
    for rv in &recipe_vars {
        let facility = catalog
            .facility(rv.facility)
            .ok_or_else(|| Error::Internal {
                message: format!("unknown facility id {}", rv.facility.0),
            })?;
        let machine_power_w = facility.power_w.ok_or_else(|| Error::MissingMachinePower {
            facility: facility.key.clone(),
        })?;
        if facility.kind != FacilityKind::Machine {
            return Err(Error::InvalidInput {
                message: format!(
                    "recipe[{}] references non-machine facility `{}`",
                    rv.recipe_index, facility.key
                ),
            });
        }
        power_use += machine_power_w as f64 * rv.y;
    }

    let mut balance_exprs: BTreeMap<ItemId, Expression> = BTreeMap::new();
    for item in items {
        let mut balance = Expression::default();
        balance += external_supply.get(&item).copied().unwrap_or(0) as f64;

        for rv in &recipe_vars {
            if let Some(delta) = rv.net.get(&item) {
                balance += *delta * rv.x;
            }
        }

        for ov in &outpost_vars {
            for (sell_item, _, qty) in &ov.sell_lines {
                if *sell_item == item {
                    balance -= 1.0 * *qty;
                }
            }
        }

        for pv in &power_vars {
            if pv.ingredient == item {
                let consume_per_min = 60.0 / pv.duration_s as f64;
                balance -= consume_per_min * pv.z;
            }
        }

        balance_exprs.insert(item, balance);
    }

    let mut model = match objective {
        StageObjective::MaxRevenue => vars.maximise(revenue.clone()).using(default_solver),
        StageObjective::MinMachines {
            revenue_floor_per_min,
        } => vars
            .minimise(total_machines_expr.clone() + total_thermal_banks_expr.clone())
            .using(default_solver)
            .with(constraint!(revenue.clone() >= revenue_floor_per_min)),
    };

    for ov in &outpost_vars {
        let mut outpost_value = Expression::default();
        for (_, price, qty) in &ov.sell_lines {
            outpost_value += *price as f64 * *qty;
        }
        model = model.with(constraint!(
            outpost_value <= ov.money_cap_per_hour as f64 / 60.0
        ));
    }

    model = model.with(constraint!(power_gen.clone() >= power_use.clone()));

    for rv in &recipe_vars {
        model = model.with(constraint!(rv.x <= rv.throughput_per_min * rv.y));
    }

    for expr in balance_exprs.values() {
        model = model.with(constraint!(expr.clone() >= 0.0));
    }

    let solution = model.solve().map_err(|source| Error::Solver { source })?;

    let revenue_per_min = solution.eval(&revenue);
    let total_machines = near_u32(
        || "total_machines".to_string(),
        solution.eval(&total_machines_expr),
    )?;
    let total_thermal_banks = near_u32(
        || "total_thermal_banks".to_string(),
        solution.eval(&total_thermal_banks_expr),
    )?;

    let power_gen_w = near_i64(|| "power_gen_w".to_string(), solution.eval(&power_gen))?;
    let power_use_w = near_i64(|| "power_use_w".to_string(), solution.eval(&power_use))?;
    let power_margin_w = power_gen_w - power_use_w;

    let mut outpost_values = Vec::with_capacity(outpost_vars.len());
    let mut top_sales = Vec::with_capacity(
        outpost_vars
            .iter()
            .map(|ov| ov.sell_lines.len())
            .sum::<usize>(),
    );

    for ov in &outpost_vars {
        let mut value_per_min = 0.0;
        for (item, price, qty) in &ov.sell_lines {
            let qty_value = solution.value(*qty);
            if qty_value <= 1e-9 {
                continue;
            }
            let value = *price as f64 * qty_value;
            if value <= 1e-9 {
                continue;
            }
            value_per_min += value;
            top_sales.push(SaleValue {
                outpost_index: ov.outpost_index,
                item: *item,
                value_per_min: value,
            });
        }

        let cap_per_min = ov.money_cap_per_hour as f64 / 60.0;
        let ratio = if cap_per_min > 0.0 {
            value_per_min / cap_per_min
        } else {
            0.0
        };

        outpost_values.push(OutpostValue {
            outpost_index: ov.outpost_index,
            value_per_min,
            cap_per_min,
            ratio,
        });
    }

    top_sales.sort_by(|a, b| b.value_per_min.total_cmp(&a.value_per_min));
    top_sales.truncate(10);

    let mut machines_by_facility_map: HashMap<FacilityId, u32> = HashMap::new();
    let mut recipes_used = Vec::with_capacity(recipe_vars.len());
    for rv in &recipe_vars {
        let machines = near_u32(
            || format!("recipes[{}].machines", rv.recipe_index),
            solution.value(rv.y),
        )?;
        let executions_per_min = solution.value(rv.x);
        if machines > 0 {
            *machines_by_facility_map.entry(rv.facility).or_insert(0) += machines;
            recipes_used.push(RecipeUsage {
                recipe_index: rv.recipe_index,
                machines,
                executions_per_min,
            });
        }
    }

    let mut machines_by_facility = machines_by_facility_map
        .into_iter()
        .map(|(facility, machines)| FacilityMachineCount { facility, machines })
        .collect::<Vec<_>>();
    machines_by_facility.sort_by(|a, b| b.machines.cmp(&a.machines));

    recipes_used.sort_by(|a, b| b.machines.cmp(&a.machines));
    recipes_used.truncate(20);

    let mut thermal_banks_used = Vec::with_capacity(power_vars.len());
    for pv in &power_vars {
        let banks = near_u32(
            || format!("power_recipes[{}].banks", pv.power_recipe_index),
            solution.value(pv.z),
        )?;
        if banks > 0 {
            thermal_banks_used.push(ThermalBankUsage {
                power_recipe_index: pv.power_recipe_index,
                ingredient: pv.ingredient,
                banks,
                power_w: pv.power_w,
                duration_s: pv.duration_s,
            });
        }
    }
    thermal_banks_used.sort_by(|a, b| b.banks.cmp(&a.banks));

    let mut external_supply_slack = Vec::with_capacity(external_supply.len());
    for (item, supply) in external_supply {
        if let Some(expr) = balance_exprs.get(item) {
            external_supply_slack.push(ExternalSupplySlack {
                item: *item,
                slack_per_min: solution.eval(expr),
                supply_per_min: *supply as f64,
            });
        }
    }
    external_supply_slack.sort_by(|a, b| a.slack_per_min.total_cmp(&b.slack_per_min));

    Ok(StageSolution {
        p_core_w: inputs.p_core_w,
        p_ext_w: inputs.aic.external_power_consumption_w,
        revenue_per_min,
        total_machines,
        total_thermal_banks,
        power_gen_w,
        power_use_w,
        power_margin_w,
        outpost_values,
        top_sales,
        machines_by_facility,
        recipes_used,
        thermal_banks_used,
        external_supply_slack,
    })
}

fn near_u32<F>(var_name: F, value: f64) -> Result<u32>
where
    F: FnOnce() -> String,
{
    if !value.is_finite() {
        return Err(Error::OutOfRange {
            var_name: var_name(),
            value,
        });
    }

    let nearest = value.round();
    let delta = (value - nearest).abs();
    if delta > NEAR_INT_EPS {
        return Err(Error::NotNearInt {
            var_name: var_name(),
            value,
            nearest,
            delta,
            eps: NEAR_INT_EPS,
        });
    }

    if nearest < 0.0 || nearest > u32::MAX as f64 {
        return Err(Error::OutOfRange {
            var_name: var_name(),
            value,
        });
    }

    Ok(nearest as u32)
}

fn near_i64<F>(var_name: F, value: f64) -> Result<i64>
where
    F: FnOnce() -> String,
{
    if !value.is_finite() {
        return Err(Error::OutOfRange {
            var_name: var_name(),
            value,
        });
    }

    let nearest = value.round();
    let delta = (value - nearest).abs();
    if delta > NEAR_INT_EPS {
        return Err(Error::NotNearInt {
            var_name: var_name(),
            value,
            nearest,
            delta,
            eps: NEAR_INT_EPS,
        });
    }

    if nearest < i64::MIN as f64 || nearest > i64::MAX as f64 {
        return Err(Error::OutOfRange {
            var_name: var_name(),
            value,
        });
    }

    Ok(nearest as i64)
}
