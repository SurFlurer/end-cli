use crate::error::{Error, Result};
use crate::types::{
    ExternalSupplySlack, FacilityMachineCount, OptimizationResult, OutpostIndex, OutpostValue,
    PowerRecipeIndex, RecipeIndex, RecipeUsage, SaleValue, SolveInputs, StageSolution,
    ThermalBankUsage,
};
use end_model::{Catalog, FacilityId, FacilityKind, ItemId};
use good_lp::{
    Expression, Solution, SolverModel, Variable, constraint, default_solver, variable, variables,
};
use smallvec::SmallVec;
use std::collections::HashMap;

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
    outpost_index: OutpostIndex,
    money_cap_per_hour: u32,
    sell_lines: Vec<(ItemId, u32, Variable)>,
}

#[derive(Debug, Clone)]
struct RecipeVars {
    recipe_index: RecipeIndex,
    facility: FacilityId,
    x: Variable,
    y: Variable,
    throughput_per_min: f64,
    net: SmallVec<[(ItemId, f64); 4]>,
}

#[derive(Debug, Clone)]
struct PowerVars {
    power_recipe_index: PowerRecipeIndex,
    ingredient: ItemId,
    power_w: u32,
    duration_s: u32,
    z: Variable,
}

/// Run the two-stage optimizer:
/// 1) maximize revenue, 2) minimize machines under a near-optimal revenue floor.
pub fn run_two_stage(catalog: &Catalog, inputs: &SolveInputs) -> Result<OptimizationResult> {
    if catalog.recipes().is_empty() {
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
    let item_count = catalog.items().len();
    let mut external_supply = vec![0_u32; item_count];
    let mut external_supply_items = Vec::with_capacity(inputs.aic.supply_per_min.len());
    let mut active_items = vec![false; item_count];
    for (item, supply) in inputs.aic.supply_per_min.iter() {
        let item_index = item_index(item, item_count)?;
        external_supply[item_index] = supply;
        external_supply_items.push((item, supply));
        active_items[item_index] = true;
    }

    let mut vars = variables!();

    let mut recipe_vars = Vec::with_capacity(catalog.recipes().len());
    for (idx, recipe) in catalog.recipes().iter().enumerate() {
        let recipe_index = to_recipe_index(idx)?;
        let x = vars.add(variable().min(0.0));
        let y = vars.add(variable().integer().min(0.0));

        let time_s = recipe.time_s as f64;
        if time_s <= 0.0 {
            return Err(Error::InvalidInput {
                message: format!("recipe[{idx}] has non-positive time_s {}", recipe.time_s),
            });
        }

        let mut net: SmallVec<[(ItemId, f64); 4]> =
            SmallVec::with_capacity(recipe.ingredients.len() + recipe.products.len());
        for stack in &recipe.ingredients {
            add_recipe_net_delta(&mut net, stack.item, -(stack.count as f64));
        }
        for stack in &recipe.products {
            add_recipe_net_delta(&mut net, stack.item, stack.count as f64);
        }
        net.retain(|(_, delta)| delta.abs() > 1e-12);
        net.sort_by_key(|(item, _)| *item);
        for (item, _) in &net {
            active_items[item_index(*item, item_count)?] = true;
        }

        recipe_vars.push(RecipeVars {
            recipe_index,
            facility: recipe.facility,
            x,
            y,
            throughput_per_min: 60.0 / time_s,
            net,
        });
    }

    let mut power_vars = Vec::with_capacity(catalog.power_recipes().len());
    for (idx, p) in catalog.power_recipes().iter().enumerate() {
        let power_recipe_index = to_power_recipe_index(idx)?;
        let z = vars.add(variable().integer().min(0.0));
        power_vars.push(PowerVars {
            power_recipe_index,
            ingredient: p.ingredient.item,
            power_w: p.power_w,
            duration_s: p.time_s,
            z,
        });
    }

    let mut outpost_vars = Vec::with_capacity(inputs.aic.outposts.len());
    for (idx, outpost) in inputs.aic.outposts.iter().enumerate() {
        let outpost_index = to_outpost_index(idx)?;
        let mut sell_lines = Vec::with_capacity(outpost.prices.len());
        for (item, price) in outpost.prices.iter() {
            active_items[item_index(item, item_count)?] = true;
            let qty = vars.add(variable().min(0.0));
            sell_lines.push((item, price, qty));
        }
        sell_lines.sort_by_key(|(item, _, _)| *item);

        outpost_vars.push(OutpostVars {
            outpost_index,
            money_cap_per_hour: outpost.money_cap_per_hour,
            sell_lines,
        });
    }

    for pv in &power_vars {
        active_items[item_index(pv.ingredient, item_count)?] = true;
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
                message: format!("unknown facility id {}", rv.facility.as_u32()),
            })?;
        let machine_power_w = facility.power_w.ok_or_else(|| Error::MissingMachinePower {
            facility: facility.key.clone(),
        })?;
        if facility.kind != FacilityKind::Machine {
            return Err(Error::InvalidInput {
                message: format!(
                    "recipe[{}] references non-machine facility `{}`",
                    rv.recipe_index.as_u32(),
                    facility.key
                ),
            });
        }
        power_use += machine_power_w.get() as f64 * rv.y;
    }

    let mut balance_exprs: Vec<Option<Expression>> = vec![None; item_count];
    for (item_idx, is_active) in active_items.into_iter().enumerate() {
        if !is_active {
            continue;
        }
        let mut balance = Expression::default();
        balance += external_supply[item_idx] as f64;

        for rv in &recipe_vars {
            for (item, delta) in &rv.net {
                if item_index(*item, item_count)? == item_idx {
                    balance += *delta * rv.x;
                    break;
                }
            }
        }

        for ov in &outpost_vars {
            for (sell_item, _, qty) in &ov.sell_lines {
                if item_index(*sell_item, item_count)? == item_idx {
                    balance -= 1.0 * *qty;
                }
            }
        }

        for pv in &power_vars {
            if item_index(pv.ingredient, item_count)? == item_idx {
                let consume_per_min = 60.0 / pv.duration_s as f64;
                balance -= consume_per_min * pv.z;
            }
        }

        balance_exprs[item_idx] = Some(balance);
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

    for expr in balance_exprs.iter().flatten() {
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
            || format!("recipes[{}].machines", rv.recipe_index.as_u32()),
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
            || format!("power_recipes[{}].banks", pv.power_recipe_index.as_u32()),
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

    let mut external_supply_slack = Vec::with_capacity(external_supply_items.len());
    for (item, supply) in external_supply_items {
        if let Some(expr) = &balance_exprs[item_index(item, item_count)?] {
            external_supply_slack.push(ExternalSupplySlack {
                item,
                slack_per_min: solution.eval(expr),
                supply_per_min: supply as f64,
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

fn add_recipe_net_delta(net: &mut SmallVec<[(ItemId, f64); 4]>, item: ItemId, delta: f64) {
    if let Some((_, acc)) = net.iter_mut().find(|(net_item, _)| *net_item == item) {
        *acc += delta;
        return;
    }
    net.push((item, delta));
}

fn item_index(item: ItemId, item_count: usize) -> Result<usize> {
    let idx = item.as_u32() as usize;
    if idx >= item_count {
        return Err(Error::InvalidInput {
            message: format!(
                "item id {} is out of bounds for catalog with {} items",
                item.as_u32(),
                item_count
            ),
        });
    }
    Ok(idx)
}

fn to_outpost_index(index: usize) -> Result<OutpostIndex> {
    OutpostIndex::from_usize(index).ok_or_else(|| Error::InvalidInput {
        message: format!("outpost index {index} exceeds u32::MAX"),
    })
}

fn to_recipe_index(index: usize) -> Result<RecipeIndex> {
    RecipeIndex::from_usize(index).ok_or_else(|| Error::InvalidInput {
        message: format!("recipe index {index} exceeds u32::MAX"),
    })
}

fn to_power_recipe_index(index: usize) -> Result<PowerRecipeIndex> {
    PowerRecipeIndex::from_usize(index).ok_or_else(|| Error::InvalidInput {
        message: format!("power recipe index {index} exceeds u32::MAX"),
    })
}
