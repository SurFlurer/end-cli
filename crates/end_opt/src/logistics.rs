use crate::consts::LOGISTICS_EPS;
use crate::error::{Error, Result};
use crate::types::{
    DemandNode, DemandNodeId, DemandSite, ItemFlowEdge, ItemFlowPlan, ItemSubproblem,
    ItemSubproblemBuildError, SupplyNode, SupplyNodeId, SupplySite,
};
use end_model::{
    AicInputs, Catalog, ItemId, LogisticsEdge, LogisticsNode, LogisticsNodeId, LogisticsNodeSite,
    LogisticsPlan, PosF64, Recipe, StageSolution,
};
use generativity::{Guard, Id};
use std::collections::{BTreeMap, btree_map::Entry};

#[derive(Debug, Default)]
struct ItemAccumulator<'cid, 'sid> {
    supplies: Vec<(SupplySite<'cid>, PosF64)>,
    demands: Vec<(DemandSite<'cid, 'sid>, PosF64)>,
}

fn recipe_machine_capacity_epsilon(machine_capacity: f64) -> f64 {
    machine_capacity.abs().max(1.0) * LOGISTICS_EPS
}

fn exceeds_recipe_machine_capacity(executions_per_min: f64, machine_capacity: f64) -> bool {
    executions_per_min > machine_capacity + recipe_machine_capacity_epsilon(machine_capacity)
}

/// Build per-item flow subproblems from stage-2 closure data.
pub fn build_item_subproblems<'cid, 'sid, 'rid>(
    catalog: &Catalog<'cid>,
    inputs: &AicInputs<'cid, 'sid>,
    stage: &StageSolution<'cid, 'sid>,
    rid: Id<'rid>,
) -> Result<Vec<ItemSubproblem<'cid, 'sid, 'rid>>> {
    let mut per_item = BTreeMap::<ItemId<'cid>, ItemAccumulator<'cid, 'sid>>::new();

    let mut external_supply = inputs.supply_per_min().iter().collect::<Vec<_>>();
    external_supply.sort_by_key(|(item, _)| item.as_u32());
    for (item, supply) in external_supply {
        push_supply(
            &mut per_item,
            item,
            SupplySite::ExternalSupply { item },
            supply.get(),
            "external_supply",
        )?;
    }

    let mut external_consumption = inputs
        .external_consumption_per_min()
        .iter()
        .collect::<Vec<_>>();
    external_consumption.sort_by_key(|(item, _)| item.as_u32());
    for (item, consumption) in external_consumption {
        push_demand(
            &mut per_item,
            item,
            DemandSite::ExternalConsumption { item },
            consumption.get(),
            "external_consumption",
        )?;
    }

    let mut recipe_usage = stage.recipes_used.clone();
    recipe_usage.sort_by_key(|run| run.recipe_index.as_u32());

    for run in recipe_usage {
        if run.executions_per_min <= LOGISTICS_EPS {
            continue;
        }

        let recipe = catalog.recipe(run.recipe_index);

        let throughput_per_machine_per_min = 60.0 / recipe.time_s as f64;
        let machine_capacity = throughput_per_machine_per_min * run.machines.get() as f64;
        if exceeds_recipe_machine_capacity(run.executions_per_min, machine_capacity) {
            return Err(Error::InvalidInput {
                message: format!(
                    "recipe {} has executions_per_min {} exceeding machine capacity {}",
                    run.recipe_index.as_u32(),
                    run.executions_per_min,
                    machine_capacity,
                )
                .into_boxed_str(),
            });
        }
        let executions_per_min = run.executions_per_min.min(machine_capacity);

        let net = recipe_net_deltas(recipe);
        for (item, delta) in &net {
            let flow = *delta * executions_per_min;
            if flow > LOGISTICS_EPS {
                push_supply(
                    &mut per_item,
                    *item,
                    SupplySite::RecipeOutput {
                        recipe_index: run.recipe_index,
                        item: *item,
                    },
                    flow,
                    "recipe_output",
                )?;
            } else if flow < -LOGISTICS_EPS {
                push_demand(
                    &mut per_item,
                    *item,
                    DemandSite::RecipeInput {
                        recipe_index: run.recipe_index,
                        item: *item,
                    },
                    -flow,
                    "recipe_input",
                )?;
            }
        }
    }

    let mut outpost_sales_qty = stage.outpost_sales_qty.clone();
    outpost_sales_qty.sort_by_key(|sale| (sale.item.as_u32(), sale.outpost_index.as_u32()));
    for sale in outpost_sales_qty {
        push_demand(
            &mut per_item,
            sale.item,
            DemandSite::OutpostSale {
                outpost_index: sale.outpost_index,
                item: sale.item,
            },
            sale.qty_per_min.get(),
            "outpost_sale",
        )?;
    }

    let mut thermal_banks = stage.thermal_banks_used.clone();
    thermal_banks.sort_by_key(|run| run.power_recipe_index.as_u32());
    for run in thermal_banks {
        let demand_per_min = (60.0 / run.duration_s as f64) * run.banks.get() as f64;
        push_demand(
            &mut per_item,
            run.ingredient,
            DemandSite::ThermalBankFuel {
                power_recipe_index: run.power_recipe_index,
                item: run.ingredient,
            },
            demand_per_min,
            "thermal_bank_fuel",
        )?;
    }

    // Add a sink demand for per-item stockpiles (units/min).
    // This ensures the logistics graph explicitly shows leftover items being stored.
    let mut item_stockpile = stage.item_stockpile.iter().collect::<Vec<_>>();
    item_stockpile.sort_by_key(|row| row.item.as_u32());
    for row in item_stockpile {
        push_demand(
            &mut per_item,
            row.item,
            DemandSite::WarehouseStockpile,
            row.qty_per_min,
            "warehouse_stockpile",
        )?;
    }

    let subproblems = per_item
        .into_iter()
        .filter(|(_, bucket)| !bucket.demands.is_empty())
        .map(|(item, bucket)| {
            let supplies = bucket
                .supplies
                .into_iter()
                .enumerate()
                .map(|(index, (site, capacity_per_min))| SupplyNode {
                    id: SupplyNodeId::from_index(index, rid),
                    site,
                    capacity_per_min,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let demands = bucket
                .demands
                .into_iter()
                .enumerate()
                .map(|(index, (site, demand_per_min))| DemandNode {
                    id: DemandNodeId::from_index(index, rid),
                    site,
                    demand_per_min,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();

            ItemSubproblem::new(item, supplies, demands, LOGISTICS_EPS).map_err(|build_error| {
                match build_error {
                    ItemSubproblemBuildError::EmptyDemands { item } => Error::InvalidInput {
                        message: format!(
                            "item {} subproblem requires at least one demand node",
                            item
                        )
                        .into_boxed_str(),
                    },
                    ItemSubproblemBuildError::Infeasible {
                        item,
                        total_supply_per_min,
                        total_demand_per_min,
                    } => Error::LogisticsInfeasible {
                        item,
                        total_supply_per_min,
                        total_demand_per_min,
                    },
                }
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(subproblems)
}

/// Solve one item flow assignment with deterministic Best-Fit.
pub fn solve_item_best_fit<'cid, 'sid, 'rid>(
    subproblem: &ItemSubproblem<'cid, 'sid, 'rid>,
) -> Result<ItemFlowPlan<'cid, 'rid>> {
    let mut remaining_supply = subproblem
        .supplies()
        .iter()
        .map(|s| SupplyState {
            id: s.id,
            remaining: s.capacity_per_min.get(),
        })
        .collect::<Vec<_>>();

    let mut demand_order = subproblem
        .demands()
        .iter()
        .map(|demand| (demand.id, demand.demand_per_min.get()))
        .collect::<Vec<_>>();
    demand_order.sort_by(|(lhs_id, lhs_demand), (rhs_id, rhs_demand)| {
        rhs_demand
            .total_cmp(lhs_demand)
            .then_with(|| lhs_id.cmp(rhs_id))
    });

    let mut edge_flow = BTreeMap::<(SupplyNodeId<'rid>, DemandNodeId<'rid>), f64>::new();

    for (demand_id, demand_per_min) in demand_order {
        let mut remaining_demand = demand_per_min;

        while remaining_demand > LOGISTICS_EPS {
            let supply_index =
                if let Some(index) = find_best_fit_supply(&remaining_supply, remaining_demand) {
                    index
                } else if let Some(index) = find_largest_non_empty_supply(&remaining_supply) {
                    index
                } else {
                    let residual_supply = remaining_supply.iter().map(|s| s.remaining).sum::<f64>();
                    return Err(Error::LogisticsInfeasible {
                        item: subproblem.item().as_u32(),
                        total_supply_per_min: residual_supply,
                        total_demand_per_min: remaining_demand,
                    });
                };

            let supply = remaining_supply
                .get_mut(supply_index)
                .ok_or(Error::InvalidInput {
                    message: format!(
                        "selected supply index {} out of bounds for item {}",
                        supply_index,
                        subproblem.item().as_u32()
                    )
                    .into_boxed_str(),
                })?;
            let supply_id = supply.id;
            let available = supply.remaining;
            let flow = available.min(remaining_demand);
            if flow <= LOGISTICS_EPS {
                let residual_supply = remaining_supply.iter().map(|s| s.remaining).sum::<f64>();
                return Err(Error::LogisticsInfeasible {
                    item: subproblem.item().as_u32(),
                    total_supply_per_min: residual_supply,
                    total_demand_per_min: remaining_demand,
                });
            }

            *edge_flow.entry((supply_id, demand_id)).or_insert(0.0) += flow;

            supply.remaining -= flow;
            if supply.remaining <= LOGISTICS_EPS {
                supply.remaining = 0.0;
            }

            remaining_demand -= flow;
            if remaining_demand <= LOGISTICS_EPS {
                remaining_demand = 0.0;
            }
        }
    }

    let edges = edge_flow
        .into_iter()
        .filter_map(|((from, to), flow_per_min)| {
            if flow_per_min <= LOGISTICS_EPS {
                return None;
            }
            Some((from, to, flow_per_min))
        })
        .map(|(from, to, flow_per_min)| {
            let flow_per_min = PosF64::new(flow_per_min).ok_or(Error::InvalidPositiveFlow {
                context: format!(
                    "flow edge item={} from={} to={}",
                    subproblem.item().as_u32(),
                    from.as_u32(),
                    to.as_u32()
                )
                .into_boxed_str(),
                value: flow_per_min,
            })?;
            Ok(ItemFlowEdge {
                item: subproblem.item(),
                from,
                to,
                flow_per_min,
            })
        })
        .collect::<Result<Vec<_>>>()?
        .into_boxed_slice();

    Ok(ItemFlowPlan {
        item: subproblem.item(),
        edges,
    })
}

/// Build the full logistics plan by solving Best-Fit subproblems per item.
pub fn build_logistics_plan<'cid, 'sid, 'rid>(
    catalog: &Catalog<'cid>,
    inputs: &AicInputs<'cid, 'sid>,
    stage: &StageSolution<'cid, 'sid>,
    guard: Guard<'rid>,
) -> Result<LogisticsPlan<'cid, 'sid, 'rid>> {
    let rid = guard.into();
    let subproblems = build_item_subproblems(catalog, inputs, stage, rid)?;
    let per_item = subproblems
        .iter()
        .map(solve_item_best_fit)
        .collect::<Result<Vec<_>>>()?;

    let mut node_index = BTreeMap::<LogisticsNodeKey<'cid, 'sid>, LogisticsNodeId<'rid>>::new();
    let mut nodes = Vec::<LogisticsNode<'cid, 'sid, 'rid>>::new();
    let mut edge_flow =
        BTreeMap::<(ItemId<'cid>, LogisticsNodeId<'rid>, LogisticsNodeId<'rid>), f64>::new();

    for (subproblem, item_plan) in subproblems.iter().zip(&per_item) {
        let item_u32 = subproblem.item().as_u32();
        let mut supply_nodes = DenseNodeMap::new(subproblem.supplies().len());
        let mut demand_nodes = DenseNodeMap::new(subproblem.demands().len());

        for supply in subproblem.supplies() {
            let key = supply_site_key(&supply.site);
            let node_id = allocate_logistics_node(&mut node_index, &mut nodes, key, rid);
            supply_nodes.insert(
                supply.id.as_u32(),
                node_id,
                &format!(
                    "build_logistics_plan item={} supply_nodes.insert supply_id={}",
                    item_u32,
                    supply.id.as_u32()
                ),
            )?;
        }

        for demand in subproblem.demands() {
            let key = demand_site_key(subproblem.item(), &demand.site);
            let node_id = allocate_logistics_node(&mut node_index, &mut nodes, key, rid);
            demand_nodes.insert(
                demand.id.as_u32(),
                node_id,
                &format!(
                    "build_logistics_plan item={} demand_nodes.insert demand_id={}",
                    item_u32,
                    demand.id.as_u32()
                ),
            )?;
        }

        for edge in &item_plan.edges {
            let from = supply_nodes.get(
                edge.from.as_u32(),
                &format!(
                    "build_logistics_plan item={} supply_nodes.get from_supply_id={}",
                    item_u32,
                    edge.from.as_u32()
                ),
            )?;
            let to = demand_nodes.get(
                edge.to.as_u32(),
                &format!(
                    "build_logistics_plan item={} demand_nodes.get to_demand_id={}",
                    item_u32,
                    edge.to.as_u32()
                ),
            )?;
            *edge_flow.entry((edge.item, from, to)).or_insert(0.0) += edge.flow_per_min.get();
        }
    }

    let edges = edge_flow
        .into_iter()
        .map(|((item, from, to), flow_per_min)| {
            let flow_per_min = PosF64::new(flow_per_min).ok_or(Error::InvalidPositiveFlow {
                context: format!(
                    "graph edge item={} from={} to={}",
                    item.as_u32(),
                    from.as_u32(),
                    to.as_u32()
                )
                .into_boxed_str(),
                value: flow_per_min,
            })?;
            Ok(LogisticsEdge {
                item,
                from,
                to,
                flow_per_min,
            })
        })
        .collect::<Result<Vec<_>>>()?
        .into_boxed_slice();

    Ok(LogisticsPlan {
        nodes: nodes.into_boxed_slice(),
        edges,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LogisticsNodeKey<'cid, 'sid> {
    ExternalSupply {
        item: ItemId<'cid>,
    },
    ExternalConsumption {
        item: ItemId<'cid>,
    },
    RecipeGroup {
        recipe_index: end_model::RecipeId<'cid>,
    },
    OutpostSale {
        outpost_index: end_model::OutpostId<'sid>,
        item: ItemId<'cid>,
    },
    ThermalBankGroup {
        power_recipe_index: end_model::PowerRecipeId<'cid>,
        item: ItemId<'cid>,
    },
    WarehouseStockpile {
        item: ItemId<'cid>,
    },
}

fn allocate_logistics_node<'cid, 'sid, 'rid>(
    node_index: &mut BTreeMap<LogisticsNodeKey<'cid, 'sid>, LogisticsNodeId<'rid>>,
    nodes: &mut Vec<LogisticsNode<'cid, 'sid, 'rid>>,
    key: LogisticsNodeKey<'cid, 'sid>,
    rid: Id<'rid>,
) -> LogisticsNodeId<'rid> {
    match node_index.entry(key) {
        Entry::Occupied(entry) => *entry.get(),
        Entry::Vacant(entry) => {
            let id = LogisticsNodeId::from_index(nodes.len(), rid);
            let site = key_to_site(*entry.key());
            entry.insert(id);
            nodes.push(LogisticsNode { id, site });
            id
        }
    }
}

fn supply_site_key<'cid, 'sid>(site: &SupplySite<'cid>) -> LogisticsNodeKey<'cid, 'sid> {
    match *site {
        SupplySite::ExternalSupply { item } => LogisticsNodeKey::ExternalSupply { item },
        SupplySite::RecipeOutput {
            recipe_index,
            item: _,
        } => LogisticsNodeKey::RecipeGroup { recipe_index },
    }
}

fn demand_site_key<'cid, 'sid>(
    item: ItemId<'cid>,
    site: &DemandSite<'cid, 'sid>,
) -> LogisticsNodeKey<'cid, 'sid> {
    match *site {
        DemandSite::RecipeInput {
            recipe_index,
            item: _,
        } => LogisticsNodeKey::RecipeGroup { recipe_index },
        DemandSite::ExternalConsumption { item } => LogisticsNodeKey::ExternalConsumption { item },
        DemandSite::OutpostSale {
            outpost_index,
            item,
        } => LogisticsNodeKey::OutpostSale {
            outpost_index,
            item,
        },
        DemandSite::ThermalBankFuel {
            power_recipe_index,
            item,
        } => LogisticsNodeKey::ThermalBankGroup {
            power_recipe_index,
            item,
        },
        DemandSite::WarehouseStockpile => LogisticsNodeKey::WarehouseStockpile { item },
    }
}

fn key_to_site<'cid, 'sid>(key: LogisticsNodeKey<'cid, 'sid>) -> LogisticsNodeSite<'cid, 'sid> {
    match key {
        LogisticsNodeKey::ExternalSupply { item } => LogisticsNodeSite::ExternalSupply { item },
        LogisticsNodeKey::ExternalConsumption { item } => {
            LogisticsNodeSite::ExternalConsumption { item }
        }
        LogisticsNodeKey::RecipeGroup { recipe_index } => {
            LogisticsNodeSite::RecipeGroup { recipe_index }
        }
        LogisticsNodeKey::OutpostSale {
            outpost_index,
            item,
        } => LogisticsNodeSite::OutpostSale {
            outpost_index,
            item,
        },
        LogisticsNodeKey::ThermalBankGroup {
            power_recipe_index,
            item,
        } => LogisticsNodeSite::ThermalBankGroup {
            power_recipe_index,
            item,
        },
        LogisticsNodeKey::WarehouseStockpile { item } => {
            LogisticsNodeSite::WarehouseStockpile { item }
        }
    }
}

fn recipe_net_deltas<'cid>(recipe: &Recipe<'cid>) -> Vec<(ItemId<'cid>, f64)> {
    let mut net = BTreeMap::<ItemId<'cid>, f64>::new();
    for ingredient in &recipe.ingredients {
        *net.entry(ingredient.item).or_insert(0.0) -= ingredient.count.get() as f64;
    }
    for product in &recipe.products {
        *net.entry(product.item).or_insert(0.0) += product.count.get() as f64;
    }
    net.into_iter().collect()
}

fn push_supply<'cid, 'sid>(
    per_item: &mut BTreeMap<ItemId<'cid>, ItemAccumulator<'cid, 'sid>>,
    item: ItemId<'cid>,
    site: SupplySite<'cid>,
    qty_per_min: f64,
    context: &'static str,
) -> Result<()> {
    let Some(qty_per_min) = pos_with_eps(qty_per_min, context)? else {
        return Ok(());
    };
    per_item
        .entry(item)
        .or_default()
        .supplies
        .push((site, qty_per_min));
    Ok(())
}

fn push_demand<'cid, 'sid>(
    per_item: &mut BTreeMap<ItemId<'cid>, ItemAccumulator<'cid, 'sid>>,
    item: ItemId<'cid>,
    site: DemandSite<'cid, 'sid>,
    qty_per_min: f64,
    context: &'static str,
) -> Result<()> {
    let Some(qty_per_min) = pos_with_eps(qty_per_min, context)? else {
        return Ok(());
    };
    per_item
        .entry(item)
        .or_default()
        .demands
        .push((site, qty_per_min));
    Ok(())
}

fn pos_with_eps(value: f64, context: &'static str) -> Result<Option<PosF64>> {
    if value <= LOGISTICS_EPS {
        return Ok(None);
    }
    let pos = PosF64::new(value).ok_or(Error::InvalidPositiveFlow {
        context: context.to_string().into_boxed_str(),
        value,
    })?;
    Ok(Some(pos))
}

#[derive(Debug, Clone)]
struct DenseNodeMap<'rid> {
    values: Vec<Option<LogisticsNodeId<'rid>>>,
}

impl<'rid> DenseNodeMap<'rid> {
    fn new(len: usize) -> Self {
        Self {
            values: vec![None; len],
        }
    }

    fn insert(
        &mut self,
        dense_index: u32,
        node_id: LogisticsNodeId<'rid>,
        context: &str,
    ) -> Result<()> {
        let index = dense_index as usize;
        let Some(slot) = self.values.get_mut(index) else {
            return Err(Error::InvalidInput {
                message: format!(
                    "{}: dense_index={} len={} (out of bounds)",
                    context,
                    dense_index,
                    self.values.len(),
                )
                .into_boxed_str(),
            });
        };
        *slot = Some(node_id);
        Ok(())
    }

    fn get(&self, dense_index: u32, context: &str) -> Result<LogisticsNodeId<'rid>> {
        let index = dense_index as usize;
        let Some(value) = self.values.get(index) else {
            return Err(Error::InvalidInput {
                message: format!(
                    "{}: dense_index={} len={} (out of bounds)",
                    context,
                    dense_index,
                    self.values.len(),
                )
                .into_boxed_str(),
            });
        };
        value.ok_or(Error::InvalidInput {
            message: format!(
                "{}: dense_index={} len={} (not initialized)",
                context,
                dense_index,
                self.values.len(),
            )
            .into_boxed_str(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct SupplyState<'rid> {
    id: SupplyNodeId<'rid>,
    remaining: f64,
}

fn find_best_fit_supply<'rid>(
    remaining_supply: &[SupplyState<'rid>],
    demand: f64,
) -> Option<usize> {
    remaining_supply
        .iter()
        .enumerate()
        .filter(|(_, supply)| supply.remaining + LOGISTICS_EPS >= demand)
        .min_by(|(_, lhs_supply), (_, rhs_supply)| {
            lhs_supply
                .remaining
                .total_cmp(&rhs_supply.remaining)
                .then_with(|| lhs_supply.id.cmp(&rhs_supply.id))
        })
        .map(|(index, _)| index)
}

fn find_largest_non_empty_supply<'rid>(remaining_supply: &[SupplyState<'rid>]) -> Option<usize> {
    remaining_supply
        .iter()
        .enumerate()
        .filter(|(_, supply)| supply.remaining > LOGISTICS_EPS)
        .max_by(|(_, lhs_supply), (_, rhs_supply)| {
            lhs_supply
                .remaining
                .total_cmp(&rhs_supply.remaining)
                .then_with(|| rhs_supply.id.cmp(&lhs_supply.id))
        })
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{build_logistics_plan, solve_item_best_fit};
    use crate::LOGISTICS_EPS;
    use crate::run_two_stage;
    use crate::types::{
        DemandNode, DemandNodeId, DemandSite, ItemSubproblem, SupplyNode, SupplyNodeId, SupplySite,
    };
    use end_model::{
        AicInputs, Catalog, DisplayName, FacilityDef, ItemDef, Key, LogisticsNodeSite,
        OutpostInput, PosF64, PowerConfig, Stack, ThermalBankDef,
    };
    use generativity::make_guard;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

    fn key(value: &str) -> Key {
        value.try_into().expect("valid key")
    }

    fn name(value: &str) -> DisplayName {
        value.try_into().expect("valid display name")
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).expect("non-zero")
    }

    #[test]
    fn recipe_machine_capacity_tolerates_solver_scale_noise() {
        let machine_capacity = 540.0;
        let executions_per_min = 540.0000000011406;

        assert!(
            !super::exceeds_recipe_machine_capacity(executions_per_min, machine_capacity),
            "tiny fp noise should not trigger invalid input"
        );
    }

    #[test]
    fn recipe_machine_capacity_rejects_real_overflow() {
        let machine_capacity = 540.0;
        let executions_per_min = 540.000001;

        assert!(
            super::exceeds_recipe_machine_capacity(executions_per_min, machine_capacity),
            "meaningful overflow must still fail validation"
        );
    }

    #[test]
    fn best_fit_fully_satisfies_each_demand() {
        make_guard!(guard);
        let (_, item) = sample_catalog(guard);
        make_guard!(sid_guard);
        let mut aic_builder = AicInputs::builder(
            sid_guard,
            PowerConfig::default(),
            Default::default(),
            Default::default(),
        );
        let outpost_index = aic_builder
            .add_outpost(OutpostInput {
                key: key("camp"),
                en: Some(name("camp")),
                zh: Some(name("camp")),
                money_cap_per_hour: 1,
                prices: vec![(item, 1)].into(),
            })
            .expect("valid aic outpost");
        make_guard!(rid_guard);
        let rid = rid_guard.into();
        let subproblem = ItemSubproblem::new(
            item,
            vec![
                SupplyNode {
                    id: SupplyNodeId::from_index(0, rid),
                    site: SupplySite::ExternalSupply { item },
                    capacity_per_min: PosF64::new(8.0).expect("positive"),
                },
                SupplyNode {
                    id: SupplyNodeId::from_index(1, rid),
                    site: SupplySite::ExternalSupply { item },
                    capacity_per_min: PosF64::new(5.0).expect("positive"),
                },
            ]
            .into_boxed_slice(),
            vec![
                DemandNode {
                    id: DemandNodeId::from_index(0, rid),
                    site: DemandSite::OutpostSale {
                        outpost_index,
                        item,
                    },
                    demand_per_min: PosF64::new(6.0).expect("positive"),
                },
                DemandNode {
                    id: DemandNodeId::from_index(1, rid),
                    site: DemandSite::OutpostSale {
                        outpost_index,
                        item,
                    },
                    demand_per_min: PosF64::new(3.0).expect("positive"),
                },
                DemandNode {
                    id: DemandNodeId::from_index(2, rid),
                    site: DemandSite::OutpostSale {
                        outpost_index,
                        item,
                    },
                    demand_per_min: PosF64::new(4.0).expect("positive"),
                },
            ]
            .into_boxed_slice(),
            LOGISTICS_EPS,
        )
        .expect("feasible subproblem");

        let flow_plan = solve_item_best_fit(&subproblem).expect("best-fit should be feasible");

        let mut incoming = BTreeMap::<u32, f64>::new();
        let mut outgoing = BTreeMap::<u32, f64>::new();
        for edge in flow_plan.edges {
            *incoming.entry(edge.to.as_u32()).or_insert(0.0) += edge.flow_per_min.get();
            *outgoing.entry(edge.from.as_u32()).or_insert(0.0) += edge.flow_per_min.get();
        }

        for demand in subproblem.demands() {
            let got = incoming.get(&demand.id.as_u32()).copied().unwrap_or(0.0);
            assert!(
                (got - demand.demand_per_min.get()).abs() <= LOGISTICS_EPS,
                "demand node {} should be fully satisfied",
                demand.id.as_u32()
            );
        }

        for supply in subproblem.supplies() {
            let used = outgoing.get(&supply.id.as_u32()).copied().unwrap_or(0.0);
            assert!(
                used <= supply.capacity_per_min.get() + LOGISTICS_EPS,
                "supply node {} should not exceed capacity",
                supply.id.as_u32()
            );
        }
    }

    #[test]
    fn logistics_plan_is_deterministic_for_same_stage_solution() {
        make_guard!(guard);
        let mut b = Catalog::builder(guard);
        let ore = b
            .add_item(ItemDef {
                key: key("Ore"),
                en: name("Ore"),
                zh: name("Ore_zh"),
                is_fluid: false,
            })
            .expect("add ore");
        let ingot = b
            .add_item(ItemDef {
                key: key("Ingot"),
                en: name("Ingot"),
                zh: name("Ingot_zh"),
                is_fluid: false,
            })
            .expect("add ingot");
        let gear = b
            .add_item(ItemDef {
                key: key("Gear"),
                en: name("Gear"),
                zh: name("Gear_zh"),
                is_fluid: false,
            })
            .expect("add gear");

        let smelter = b
            .add_facility(FacilityDef {
                key: key("Smelter"),
                power_w: nz(10),
                en: name("Smelter"),
                zh: name("Smelter_zh"),
                regions: end_model::FacilityRegions::All,
            })
            .expect("add smelter");
        let assembler = b
            .add_facility(FacilityDef {
                key: key("Assembler"),
                power_w: nz(20),
                en: name("Assembler"),
                zh: name("Assembler_zh"),
                regions: end_model::FacilityRegions::All,
            })
            .expect("add assembler");
        let mut b = b
            .add_thermal_bank(ThermalBankDef {
                key: key("Thermal Bank"),
                en: name("Thermal Bank"),
                zh: name("Thermal_Bank_zh"),
            })
            .expect("add thermal bank");

        let smelt_recipe = b
            .push_recipe(
                smelter,
                nz(60),
                vec![Stack {
                    item: ore,
                    count: nz(1),
                }]
                .into(),
                vec![Stack {
                    item: ingot,
                    count: nz(1),
                }]
                .into(),
            )
            .expect("push smelting recipe");
        b.push_recipe(
            assembler,
            nz(60),
            vec![Stack {
                item: ingot,
                count: nz(2),
            }]
            .into(),
            vec![Stack {
                item: gear,
                count: nz(1),
            }]
            .into(),
        )
        .expect("push gear recipe");

        let catalog = b.build();

        make_guard!(aic_guard);
        let mut aic_builder = AicInputs::builder(
            aic_guard,
            PowerConfig::default(),
            vec![(ore, PosF64::new(20.0).expect("positive"))].into(),
            Default::default(),
        );
        aic_builder
            .add_outpost(OutpostInput {
                key: key("Camp"),
                en: Some(name("Camp")),
                zh: Some(name("Camp_zh")),
                money_cap_per_hour: 10_000,
                prices: vec![(gear, 30), (ingot, 5)].into(),
            })
            .expect("valid aic outpost");
        let aic = aic_builder.build();

        make_guard!(result_guard);
        let solved = run_two_stage(&catalog, &aic, result_guard).expect("solve scenario");

        make_guard!(plan_guard);
        let plan_a = build_logistics_plan(&catalog, &aic, &solved.stage2, plan_guard)
            .expect("logistics plan should build");
        let canonical_edges = |plan: &end_model::LogisticsPlan<'_, '_, '_>| {
            plan.edges
                .iter()
                .map(|edge| {
                    (
                        edge.item.as_u32(),
                        edge.from.as_u32(),
                        edge.to.as_u32(),
                        edge.flow_per_min.get(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            canonical_edges(&plan_a),
            canonical_edges(&solved.logistics),
            "same inputs should produce identical flow edges"
        );

        let unique_edges = plan_a
            .edges
            .iter()
            .map(|edge| (edge.item.as_u32(), edge.from.as_u32(), edge.to.as_u32()))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            unique_edges.len(),
            plan_a.edges.len(),
            "same item edge pair should have been merged into one record"
        );

        let unique_nodes = plan_a
            .nodes
            .iter()
            .map(|node| node.id.as_u32())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            unique_nodes.len(),
            plan_a.nodes.len(),
            "node ids should be unique"
        );

        let smelter_node = plan_a
            .nodes
            .iter()
            .find_map(|node| match node.site {
                LogisticsNodeSite::RecipeGroup { recipe_index } if recipe_index == smelt_recipe => {
                    Some(node.id)
                }
                _ => None,
            })
            .expect("smelter recipe group node should exist");
        let has_outgoing = plan_a.edges.iter().any(|edge| edge.from == smelter_node);
        let has_incoming = plan_a.edges.iter().any(|edge| edge.to == smelter_node);
        assert!(
            has_outgoing && has_incoming,
            "same machine node should carry both input and output edges across items"
        );
    }

    #[test]
    fn logistics_plan_contains_external_consumption_demands() {
        make_guard!(guard);
        let (catalog, item) = sample_catalog(guard);
        make_guard!(aic_guard);
        let mut aic_builder = AicInputs::builder(
            aic_guard,
            PowerConfig::default(),
            vec![(item, PosF64::new(10.0).expect("positive"))].into(),
            vec![(item, PosF64::new(4.0).expect("positive"))].into(),
        );
        aic_builder
            .add_outpost(OutpostInput {
                key: key("sink"),
                en: Some(name("sink")),
                zh: Some(name("sink")),
                money_cap_per_hour: 0,
                prices: vec![(item, 1)].into(),
            })
            .expect("valid aic outpost");
        let aic = aic_builder.build();

        make_guard!(result_guard);
        let solved = run_two_stage(&catalog, &aic, result_guard).expect("solve scenario");
        let external_consumption_node =
            solved
                .logistics
                .nodes
                .iter()
                .find_map(|node| match node.site {
                    LogisticsNodeSite::ExternalConsumption { item: node_item }
                        if node_item == item =>
                    {
                        Some(node.id)
                    }
                    _ => None,
                });
        let Some(external_consumption_node) = external_consumption_node else {
            panic!("external consumption node should exist");
        };

        let has_incoming = solved
            .logistics
            .edges
            .iter()
            .any(|edge| edge.to == external_consumption_node && edge.item == item);
        assert!(
            has_incoming,
            "external consumption node should have incoming item flow"
        );
    }

    #[test]
    fn logistics_plan_splits_warehouse_stockpile_nodes_per_item() {
        make_guard!(guard);
        let mut b = Catalog::builder(guard);
        let ore = b
            .add_item(ItemDef {
                key: key("Ore"),
                en: name("Ore"),
                zh: name("Ore_zh"),
                is_fluid: false,
            })
            .expect("add ore");
        let ingot = b
            .add_item(ItemDef {
                key: key("Ingot"),
                en: name("Ingot"),
                zh: name("Ingot_zh"),
                is_fluid: false,
            })
            .expect("add ingot");
        let b = b
            .add_thermal_bank(ThermalBankDef {
                key: key("Thermal Bank"),
                en: name("Thermal Bank"),
                zh: name("Thermal_Bank_zh"),
            })
            .expect("add thermal bank");
        let catalog = b.build();

        make_guard!(aic_guard);
        let mut aic_builder = AicInputs::builder(
            aic_guard,
            PowerConfig::default(),
            vec![
                (ore, PosF64::new(10.0).expect("positive")),
                (ingot, PosF64::new(7.0).expect("positive")),
            ]
            .into(),
            Default::default(),
        );
        aic_builder
            .add_outpost(OutpostInput {
                key: key("sink"),
                en: Some(name("sink")),
                zh: Some(name("sink")),
                money_cap_per_hour: 1,
                prices: vec![(ore, 1)].into(),
            })
            .expect("valid aic outpost");
        let aic = aic_builder.build();

        make_guard!(result_guard);
        let solved = run_two_stage(&catalog, &aic, result_guard).expect("solve scenario");

        let mut warehouse_by_item = BTreeMap::new();
        for node in &solved.logistics.nodes {
            if let LogisticsNodeSite::WarehouseStockpile { item } = node.site {
                warehouse_by_item.insert(item, node.id);
            }
        }
        assert_eq!(
            warehouse_by_item.len(),
            2,
            "each stocked item should have its own warehouse node"
        );
        assert_ne!(
            warehouse_by_item.get(&ore),
            warehouse_by_item.get(&ingot),
            "different items must not share one warehouse node"
        );

        let has_ore_incoming = solved.logistics.edges.iter().any(|edge| {
            edge.item == ore
                && warehouse_by_item
                    .get(&ore)
                    .is_some_and(|warehouse_node| edge.to == *warehouse_node)
        });
        let has_ingot_incoming = solved.logistics.edges.iter().any(|edge| {
            edge.item == ingot
                && warehouse_by_item
                    .get(&ingot)
                    .is_some_and(|warehouse_node| edge.to == *warehouse_node)
        });
        assert!(
            has_ore_incoming,
            "ore flow should end at ore warehouse node"
        );
        assert!(
            has_ingot_incoming,
            "ingot flow should end at ingot warehouse node"
        );
    }

    fn sample_catalog<'id>(
        guard: generativity::Guard<'id>,
    ) -> (Catalog<'id>, end_model::ItemId<'id>) {
        let mut b = Catalog::builder(guard);
        let item = b
            .add_item(ItemDef {
                key: key("x"),
                en: name("x"),
                zh: name("x"),
                is_fluid: false,
            })
            .expect("add item");
        let b = b
            .add_thermal_bank(ThermalBankDef {
                key: key("Thermal Bank"),
                en: name("Thermal Bank"),
                zh: name("Thermal_Bank_zh"),
            })
            .expect("add thermal bank");
        (b.build(), item)
    }
}
