use crate::error::{Error, Result};
use crate::types::{
    ExternalSupplySlack, FacilityMachineCount, OptimizationResult, OutpostValue, RecipeUsage,
    SaleValue, SolveInputs, StageSolution, ThermalBankUsage,
};
use end_model::{Catalog, FacilityId, ItemId, OutpostId, PowerRecipeId, RecipeId};
use good_lp::{
    Expression, Solution, SolverModel, Variable, constraint, default_solver, variable, variables,
};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Maximum tolerated distance from an integer value for decoded integer variables.
pub const NEAR_INT_EPS: f64 = 1e-6;

#[derive(Debug, Clone, Copy)]
enum StageObjective {
    MaxRevenue,
    MinMachines { revenue_floor_per_min: f64 },
}

#[derive(Debug, Clone)]
struct OutpostVars {
    outpost_index: OutpostId,
    money_cap_per_hour: u32,
    sell_lines: Vec<(ItemId, u32, Variable)>,
}

#[derive(Debug, Clone)]
struct RecipeVars {
    recipe_index: RecipeId,
    facility: FacilityId,
    facility_power_w: u32,
    x: Variable,
    y: Variable,
    throughput_per_min: f64,
    net: SmallVec<[(ItemId, f64); 4]>,
}

#[derive(Debug, Clone)]
struct PowerVars {
    power_recipe_index: PowerRecipeId,
    ingredient: ItemId,
    power_w: u32,
    duration_s: u32,
    z: Variable,
}

/// Run the two-stage optimizer:
/// 1) maximize revenue, 2) minimize machines under a near-optimal revenue floor.
pub fn run_two_stage(catalog: &Catalog, inputs: &SolveInputs) -> Result<OptimizationResult> {
    // TODO this should be parse instead of validate, but it is not possible to encode checked info into types
    // without branded types. so we leave this as future work.
    // Once this done we can eliminate panic in item_balance indexing, or even the check at all.
    validate_aic_item_ids(inputs, catalog.items().len())?;

    let stage1 = solve_stage(catalog, inputs, StageObjective::MaxRevenue)?;

    let rel_eps = NEAR_INT_EPS * stage1.revenue_per_min.max(1.0);
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
    let mut vars = variables!();

    let recipe_vars = catalog
        .recipes_with_id_and_facility()
        .map(|(recipe_index, recipe, facility)| {
            let x = vars.add(variable().min(0.0));
            let y = vars.add(variable().integer().min(0.0));
            let time_s = recipe.time_s as f64;

            let net = recipe
                .ingredients
                .iter()
                .map(|stack| (stack.item, -(stack.count as f64)))
                .chain(
                    recipe
                        .products
                        .iter()
                        .map(|stack| (stack.item, stack.count as f64)),
                )
                .fold(
                    SmallVec::with_capacity(recipe.ingredients.len() + recipe.products.len()),
                    |mut net, (item, delta)| {
                        add_recipe_net_delta(&mut net, item, delta);
                        net
                    },
                );

            RecipeVars {
                recipe_index,
                facility: recipe.facility,
                facility_power_w: facility.power_w.get(),
                x,
                y,
                throughput_per_min: 60.0 / time_s,
                net,
            }
        })
        .collect::<Vec<_>>();

    let power_vars = catalog
        .power_recipes_with_id()
        .map(|(id, p)| PowerVars {
            power_recipe_index: id,
            ingredient: p.ingredient.item,
            power_w: p.power_w,
            duration_s: p.time_s,
            z: vars.add(variable().integer().min(0.0)),
        })
        .collect::<Vec<_>>();

    let outpost_vars = inputs
        .aic
        .outposts_with_id()
        .map(|(id, outpost)| {
            let sell_lines = outpost
                .prices
                .iter()
                .map(|(item, price)| {
                    let qty = vars.add(variable().min(0.0));
                    (item, price, qty)
                })
                .collect::<Vec<_>>();

            OutpostVars {
                outpost_index: id,
                money_cap_per_hour: outpost.money_cap_per_hour,
                sell_lines,
            }
        })
        .collect::<Vec<_>>();

    let revenue: Expression = outpost_vars
        .iter()
        .flat_map(|ov| ov.sell_lines.iter())
        .map(|(_, price, qty)| *qty * *price)
        .sum();

    let total_machines: Expression = recipe_vars.iter().map(|rv| rv.y).sum();
    let total_thermal_banks: Expression = power_vars.iter().map(|pv| pv.z).sum();

    let power_gen = inputs.p_core_w as f64
        + power_vars
            .iter()
            .map(|pv| pv.z * pv.power_w)
            .sum::<Expression>();

    let power_use = Expression::from(inputs.aic.external_power_consumption_w() as f64)
        + recipe_vars
            .iter()
            .map(|rv| rv.y * rv.facility_power_w)
            .sum::<Expression>();

    // Build per-item balances by dispatching each contribution to its item bucket.
    // Seed from external supplies first.
    let item_count = catalog.items().len();
    let item_balance = inputs.aic.supply_per_min().iter().fold(
        vec![Expression::from(0.0); item_count],
        |mut item_balance, (item, supply)| {
            item_balance[item.index()] = Expression::from(supply.get() as f64);
            item_balance
        },
    );

    let item_balance = recipe_vars
        .iter()
        .fold(item_balance, |mut item_balance, rv| {
            rv.net.iter().for_each(|(item, delta)| {
                item_balance[item.index()] += *delta * rv.x;
            });
            item_balance
        });

    let item_balance = outpost_vars
        .iter()
        .fold(item_balance, |mut item_balance, ov| {
            ov.sell_lines.iter().for_each(|(item, _, qty)| {
                item_balance[item.index()] -= *qty;
            });
            item_balance
        });

    let item_balance = power_vars
        .iter()
        .fold(item_balance, |mut item_balance, pv| {
            let consume_per_min = 60.0 / pv.duration_s as f64;
            item_balance[pv.ingredient.index()] -= consume_per_min * pv.z;
            item_balance
        });

    let mut model = match objective {
        StageObjective::MaxRevenue => vars.maximise(revenue.clone()).using(default_solver),
        StageObjective::MinMachines {
            revenue_floor_per_min,
        } => vars
            .minimise(total_machines.clone() + total_thermal_banks.clone())
            .using(default_solver)
            .with(constraint!(revenue.clone() >= revenue_floor_per_min)),
    };

    model = outpost_vars.iter().fold(model, |model, ov| {
        let outpost_value: Expression = ov
            .sell_lines
            .iter()
            .map(|(_, price, qty)| *qty * *price)
            .sum();
        model.with(constraint!(
            outpost_value <= ov.money_cap_per_hour as f64 / 60.0
        ))
    });

    model = model.with(constraint!(&power_gen >= power_use.clone()));

    model = recipe_vars.iter().fold(model, |model, rv| {
        model.with(constraint!(rv.x <= rv.throughput_per_min * rv.y))
    });

    model = item_balance.iter().fold(model, |model, expr| {
        model.with(constraint!(expr.clone() >= 0.0))
    });

    let solution = model.solve().map_err(|source| Error::Solver { source })?;

    let revenue_per_min = solution.eval(&revenue);
    let total_machines = near_u32(
        || "total_machines".to_string(),
        solution.eval(&total_machines),
    )?;
    let total_thermal_banks = near_u32(
        || "total_thermal_banks".to_string(),
        solution.eval(&total_thermal_banks),
    )?;

    let power_gen_w = near_i64(|| "power_gen_w".to_string(), solution.eval(&power_gen))?;
    let power_use_w = near_i64(|| "power_use_w".to_string(), solution.eval(&power_use))?;
    let power_margin_w = power_gen_w - power_use_w;

    let mut sales = Vec::with_capacity(
        outpost_vars
            .iter()
            .map(|ov| ov.sell_lines.len())
            .sum::<usize>(),
    );
    let outpost_values = outpost_vars
        .iter()
        .map(|ov| {
            let outpost_sales = ov
                .sell_lines
                .iter()
                .filter_map(|(item, price, qty)| {
                    let qty_value = solution.value(*qty);
                    if qty_value <= 1e-9 {
                        return None;
                    }
                    let value = *price as f64 * qty_value;
                    Some(SaleValue {
                        outpost_index: ov.outpost_index,
                        item: *item,
                        value_per_min: value,
                    })
                })
                .collect::<Vec<_>>();
            let value_per_min = outpost_sales
                .iter()
                .map(|sale| sale.value_per_min)
                .sum::<f64>();
            sales.extend(outpost_sales);

            let cap_per_min = ov.money_cap_per_hour as f64 / 60.0;
            let ratio = value_per_min / cap_per_min;

            OutpostValue {
                outpost_index: ov.outpost_index,
                value_per_min,
                cap_per_min,
                ratio,
            }
        })
        .collect::<Vec<_>>();

    sales.sort_by(|a, b| b.value_per_min.total_cmp(&a.value_per_min));

    let (machines_by_facility_map, mut recipes_used) = recipe_vars.iter().try_fold(
        (
            HashMap::<FacilityId, u32>::new(),
            Vec::with_capacity(recipe_vars.len()),
        ),
        |(mut machines_by_facility_map, mut recipes_used), rv| -> Result<_> {
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
            Ok((machines_by_facility_map, recipes_used))
        },
    )?;

    let mut machines_by_facility = machines_by_facility_map
        .into_iter()
        .map(|(facility, machines)| FacilityMachineCount { facility, machines })
        .collect::<Vec<_>>();
    machines_by_facility.sort_by(|a, b| b.machines.cmp(&a.machines));

    recipes_used.sort_by(|a, b| b.machines.cmp(&a.machines));

    let mut thermal_banks_used = power_vars
        .iter()
        .map(|pv| -> Result<Option<ThermalBankUsage>> {
            let banks = near_u32(
                || format!("power_recipes[{}].banks", pv.power_recipe_index.as_u32()),
                solution.value(pv.z),
            )?;
            Ok((banks > 0).then_some(ThermalBankUsage {
                power_recipe_index: pv.power_recipe_index,
                ingredient: pv.ingredient,
                banks,
                power_w: pv.power_w,
                duration_s: pv.duration_s,
            }))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    thermal_banks_used.sort_by(|a, b| b.banks.cmp(&a.banks));

    let mut external_supply_slack = inputs
        .aic
        .supply_per_min()
        .iter()
        .map(|(item, supply)| {
            let expr = &item_balance[item.index()];
            ExternalSupplySlack {
                item,
                slack_per_min: solution.eval(expr),
                supply_per_min: supply.get() as f64,
            }
        })
        .collect::<Vec<_>>();
    external_supply_slack.sort_by(|a, b| a.slack_per_min.total_cmp(&b.slack_per_min));

    Ok(StageSolution {
        p_core_w: inputs.p_core_w,
        p_ext_w: inputs.aic.external_power_consumption_w(),
        revenue_per_min,
        total_machines,
        total_thermal_banks,
        power_gen_w,
        power_use_w,
        power_margin_w,
        outpost_values,
        top_sales: sales,
        machines_by_facility,
        recipes_used,
        thermal_banks_used,
        external_supply_slack,
    })
}

fn validate_aic_item_ids(inputs: &SolveInputs, item_count: usize) -> Result<()> {
    let invalid_item: Option<(Option<OutpostId>, ItemId)> = inputs
        .aic
        .supply_per_min()
        .iter()
        .map(|(item, _)| (None, item))
        .chain(
            inputs
                .aic
                .outposts_with_id()
                .flat_map(|(outpost_index, outpost)| {
                    outpost
                        .prices
                        .iter()
                        .map(move |(item, _)| (Some(outpost_index), item))
                }),
        )
        .find(|(_, item)| item.index() >= item_count);

    match invalid_item {
        Some((None, item)) => Err(Error::InvalidInput {
            message: format!(
                "supply_per_min contains item id {} out of bounds for catalog with {} items",
                item.as_u32(),
                item_count
            ),
        }),
        Some((Some(outpost_index), item)) => Err(Error::InvalidInput {
            message: format!(
                "outposts[{}].prices contains item id {} out of bounds for catalog with {} items",
                outpost_index.as_u32(),
                item.as_u32(),
                item_count
            ),
        }),
        None => Ok(()),
    }
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
