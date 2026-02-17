use crate::error::{Error, Result};
use crate::types::{
    DemandNode, DemandNodeId, DemandSite, ItemFlowEdge, ItemFlowPlan, ItemSubproblem,
    LogisticsEdge, LogisticsNode, LogisticsNodeId, LogisticsNodeSite, LogisticsPlan,
    MachineOrdinal, PosF64, StageSolution, SupplyNode, SupplyNodeId, SupplySite,
};
use end_model::{AicInputs, Catalog, ItemId, Recipe};
use std::collections::BTreeMap;

/// Floating-point tolerance used by logistics expansion and assignment.
pub const LOGISTICS_EPS: f64 = 1e-9;

#[derive(Debug, Default)]
struct ItemAccumulator {
    supplies: Vec<(SupplySite, PosF64)>,
    demands: Vec<(DemandSite, PosF64)>,
}

/// Split one recipe run across machine instances using the stable formula from the logistics plan.
pub fn expand_recipe_machine_rates(
    executions_per_min: f64,
    machines: u32,
    throughput_per_machine_per_min: f64,
) -> Vec<(MachineOrdinal, f64)> {
    if machines == 0 || executions_per_min <= LOGISTICS_EPS {
        return Vec::new();
    }

    (0..machines)
        .filter_map(|index| {
            let machine = MachineOrdinal::from_1_based(index + 1);
            let consumed_capacity = index as f64 * throughput_per_machine_per_min;
            let remaining = executions_per_min - consumed_capacity;
            let rho = remaining
                .max(0.0)
                .min(throughput_per_machine_per_min.max(0.0));
            (rho > LOGISTICS_EPS).then_some((machine, rho))
        })
        .collect()
}

/// Build machine-granularity per-item flow subproblems from stage-2 closure data.
pub fn build_item_subproblems(
    catalog: &Catalog,
    inputs: &AicInputs,
    stage: &StageSolution,
) -> Result<Vec<ItemSubproblem>> {
    let mut per_item = BTreeMap::<ItemId, ItemAccumulator>::new();

    let mut external_supply = inputs.supply_per_min().iter().collect::<Vec<_>>();
    external_supply.sort_by_key(|(item, _)| item.as_u32());
    for (item, supply) in external_supply {
        push_supply(
            &mut per_item,
            item,
            SupplySite::ExternalSupply { item },
            supply.get() as f64,
            "external_supply",
        )?;
    }

    let mut recipe_usage = stage.recipes_used.clone();
    recipe_usage.sort_by_key(|run| run.recipe_index.as_u32());

    for run in recipe_usage {
        if run.executions_per_min <= LOGISTICS_EPS || run.machines == 0 {
            continue;
        }

        let recipe = catalog
            .recipe(run.recipe_index)
            .ok_or(Error::MissingRecipe {
                recipe_index: run.recipe_index,
            })?;

        let throughput_per_machine_per_min = 60.0 / recipe.time_s as f64;
        let machine_capacity = throughput_per_machine_per_min * run.machines as f64;
        if run.executions_per_min > machine_capacity + LOGISTICS_EPS {
            return Err(Error::InvalidInput {
                message: format!(
                    "recipe {} has executions_per_min {} exceeding machine capacity {}",
                    run.recipe_index.as_u32(),
                    run.executions_per_min,
                    machine_capacity,
                ),
            });
        }

        let net = recipe_net_deltas(recipe);
        let machine_rates = expand_recipe_machine_rates(
            run.executions_per_min,
            run.machines,
            throughput_per_machine_per_min,
        );

        for (machine, rho) in machine_rates {
            for (item, delta) in &net {
                let flow = *delta * rho;
                if flow > LOGISTICS_EPS {
                    push_supply(
                        &mut per_item,
                        *item,
                        SupplySite::RecipeOutput {
                            recipe_index: run.recipe_index,
                            machine,
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
                            machine,
                            item: *item,
                        },
                        -flow,
                        "recipe_input",
                    )?;
                }
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
        if run.banks == 0 {
            continue;
        }

        let demand_per_bank_per_min = 60.0 / run.duration_s as f64;
        for bank in 1..=run.banks {
            let bank = MachineOrdinal::from_1_based(bank);
            push_demand(
                &mut per_item,
                run.ingredient,
                DemandSite::ThermalBankFuel {
                    power_recipe_index: run.power_recipe_index,
                    bank,
                    item: run.ingredient,
                },
                demand_per_bank_per_min,
                "thermal_bank_fuel",
            )?;
        }
    }

    let subproblems = per_item
        .into_iter()
        .filter_map(|(item, bucket)| {
            if bucket.demands.is_empty() {
                return None;
            }

            let supplies = bucket
                .supplies
                .into_iter()
                .enumerate()
                .map(|(index, (site, capacity_per_min))| SupplyNode {
                    id: SupplyNodeId::from_index(index),
                    site,
                    capacity_per_min,
                })
                .collect::<Vec<_>>();
            let demands = bucket
                .demands
                .into_iter()
                .enumerate()
                .map(|(index, (site, demand_per_min))| DemandNode {
                    id: DemandNodeId::from_index(index),
                    site,
                    demand_per_min,
                })
                .collect::<Vec<_>>();

            Some(ItemSubproblem {
                item,
                supplies,
                demands,
            })
        })
        .collect();

    Ok(subproblems)
}

/// Solve one item flow assignment with deterministic Best-Fit.
pub fn solve_item_best_fit(subproblem: &ItemSubproblem) -> Result<ItemFlowPlan> {
    let total_supply = subproblem
        .supplies
        .iter()
        .map(|s| s.capacity_per_min.get())
        .sum::<f64>();
    let total_demand = subproblem
        .demands
        .iter()
        .map(|d| d.demand_per_min.get())
        .sum::<f64>();

    if total_demand <= LOGISTICS_EPS {
        return Ok(ItemFlowPlan {
            item: subproblem.item,
            edges: Vec::new(),
        });
    }

    if total_supply + LOGISTICS_EPS < total_demand {
        return Err(Error::LogisticsInfeasible {
            item: subproblem.item,
            total_supply_per_min: total_supply,
            total_demand_per_min: total_demand,
        });
    }

    let mut remaining_supply = subproblem
        .supplies
        .iter()
        .map(|s| SupplyState {
            id: s.id,
            remaining: s.capacity_per_min.get(),
        })
        .collect::<Vec<_>>();

    let mut demand_order = subproblem
        .demands
        .iter()
        .map(|demand| (demand.id, demand.demand_per_min.get()))
        .collect::<Vec<_>>();
    demand_order.sort_by(|(lhs_id, lhs_demand), (rhs_id, rhs_demand)| {
        rhs_demand
            .total_cmp(lhs_demand)
            .then_with(|| lhs_id.cmp(rhs_id))
    });

    let mut edge_flow = BTreeMap::<(SupplyNodeId, DemandNodeId), f64>::new();

    for (demand_id, demand_per_min) in demand_order {
        let mut remaining_demand = demand_per_min;

        while remaining_demand > LOGISTICS_EPS {
            let best_fit_supply = find_best_fit_supply(&remaining_supply, remaining_demand);

            let supply_index = if let Some(index) = best_fit_supply {
                index
            } else if let Some(index) = find_largest_non_empty_supply(&remaining_supply) {
                index
            } else {
                let residual_supply = remaining_supply.iter().map(|s| s.remaining).sum::<f64>();
                return Err(Error::LogisticsInfeasible {
                    item: subproblem.item,
                    total_supply_per_min: residual_supply,
                    total_demand_per_min: remaining_demand,
                });
            };

            let Some(supply) = remaining_supply.get_mut(supply_index) else {
                return Err(Error::InvalidInput {
                    message: format!(
                        "selected supply index {} out of bounds for item {}",
                        supply_index,
                        subproblem.item.as_u32()
                    ),
                });
            };
            let supply_id = supply.id;
            let available = supply.remaining;
            let flow = available.min(remaining_demand);
            if flow <= LOGISTICS_EPS {
                let residual_supply = remaining_supply.iter().map(|s| s.remaining).sum::<f64>();
                return Err(Error::LogisticsInfeasible {
                    item: subproblem.item,
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
                    subproblem.item.as_u32(),
                    from.as_u32(),
                    to.as_u32()
                ),
                value: flow_per_min,
            })?;
            Ok(ItemFlowEdge {
                item: subproblem.item,
                from,
                to,
                flow_per_min,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ItemFlowPlan {
        item: subproblem.item,
        edges,
    })
}

/// Build the full logistics plan by solving Best-Fit subproblems per item.
pub fn build_logistics_plan(
    catalog: &Catalog,
    inputs: &AicInputs,
    stage: &StageSolution,
) -> Result<LogisticsPlan> {
    let subproblems = build_item_subproblems(catalog, inputs, stage)?;
    let per_item = subproblems
        .iter()
        .map(solve_item_best_fit)
        .collect::<Result<Vec<_>>>()?;

    let mut node_index = BTreeMap::<LogisticsNodeKey, LogisticsNodeId>::new();
    let mut nodes = Vec::<LogisticsNode>::new();
    let mut edge_flow = BTreeMap::<(ItemId, LogisticsNodeId, LogisticsNodeId), f64>::new();

    for (subproblem, item_plan) in subproblems.iter().zip(&per_item) {
        let mut supply_nodes = BTreeMap::<SupplyNodeId, LogisticsNodeId>::new();
        let mut demand_nodes = BTreeMap::<DemandNodeId, LogisticsNodeId>::new();

        for supply in &subproblem.supplies {
            let key = supply_site_key(&supply.site);
            let node_id = allocate_logistics_node(&mut node_index, &mut nodes, key);
            supply_nodes.insert(supply.id, node_id);
        }

        for demand in &subproblem.demands {
            let key = demand_site_key(&demand.site);
            let node_id = allocate_logistics_node(&mut node_index, &mut nodes, key);
            demand_nodes.insert(demand.id, node_id);
        }

        for edge in &item_plan.edges {
            let Some(from) = supply_nodes.get(&edge.from).copied() else {
                return Err(Error::InvalidInput {
                    message: format!(
                        "missing logistics source node mapping for item {} supply {}",
                        edge.item.as_u32(),
                        edge.from.as_u32()
                    ),
                });
            };
            let Some(to) = demand_nodes.get(&edge.to).copied() else {
                return Err(Error::InvalidInput {
                    message: format!(
                        "missing logistics target node mapping for item {} demand {}",
                        edge.item.as_u32(),
                        edge.to.as_u32()
                    ),
                });
            };
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
                ),
                value: flow_per_min,
            })?;
            Ok(LogisticsEdge {
                item,
                from,
                to,
                flow_per_min,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(LogisticsPlan { nodes, edges })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LogisticsNodeKey {
    ExternalSupply {
        item: ItemId,
    },
    RecipeMachine {
        recipe_index: end_model::RecipeId,
        machine: MachineOrdinal,
    },
    OutpostSale {
        outpost_index: end_model::OutpostId,
        item: ItemId,
    },
    ThermalBankFuel {
        power_recipe_index: end_model::PowerRecipeId,
        bank: MachineOrdinal,
        item: ItemId,
    },
}

fn allocate_logistics_node(
    node_index: &mut BTreeMap<LogisticsNodeKey, LogisticsNodeId>,
    nodes: &mut Vec<LogisticsNode>,
    key: LogisticsNodeKey,
) -> LogisticsNodeId {
    if let Some(id) = node_index.get(&key).copied() {
        return id;
    }

    let id = LogisticsNodeId::from_index(nodes.len());
    let site = key_to_site(key);
    node_index.insert(key, id);
    nodes.push(LogisticsNode { id, site });
    id
}

fn supply_site_key(site: &SupplySite) -> LogisticsNodeKey {
    match *site {
        SupplySite::ExternalSupply { item } => LogisticsNodeKey::ExternalSupply { item },
        SupplySite::RecipeOutput {
            recipe_index,
            machine,
            item: _,
        } => LogisticsNodeKey::RecipeMachine {
            recipe_index,
            machine,
        },
    }
}

fn demand_site_key(site: &DemandSite) -> LogisticsNodeKey {
    match *site {
        DemandSite::RecipeInput {
            recipe_index,
            machine,
            item: _,
        } => LogisticsNodeKey::RecipeMachine {
            recipe_index,
            machine,
        },
        DemandSite::OutpostSale {
            outpost_index,
            item,
        } => LogisticsNodeKey::OutpostSale { outpost_index, item },
        DemandSite::ThermalBankFuel {
            power_recipe_index,
            bank,
            item,
        } => LogisticsNodeKey::ThermalBankFuel {
            power_recipe_index,
            bank,
            item,
        },
    }
}

fn key_to_site(key: LogisticsNodeKey) -> LogisticsNodeSite {
    match key {
        LogisticsNodeKey::ExternalSupply { item } => LogisticsNodeSite::ExternalSupply { item },
        LogisticsNodeKey::RecipeMachine {
            recipe_index,
            machine,
        } => LogisticsNodeSite::RecipeMachine {
            recipe_index,
            machine,
        },
        LogisticsNodeKey::OutpostSale { outpost_index, item } => {
            LogisticsNodeSite::OutpostSale { outpost_index, item }
        }
        LogisticsNodeKey::ThermalBankFuel {
            power_recipe_index,
            bank,
            item,
        } => LogisticsNodeSite::ThermalBankFuel {
            power_recipe_index,
            bank,
            item,
        },
    }
}

fn recipe_net_deltas(recipe: &Recipe) -> Vec<(ItemId, f64)> {
    let mut net = BTreeMap::<ItemId, f64>::new();
    for ingredient in &recipe.ingredients {
        *net.entry(ingredient.item).or_insert(0.0) -= ingredient.count as f64;
    }
    for product in &recipe.products {
        *net.entry(product.item).or_insert(0.0) += product.count as f64;
    }
    net.into_iter().collect()
}

fn push_supply(
    per_item: &mut BTreeMap<ItemId, ItemAccumulator>,
    item: ItemId,
    site: SupplySite,
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

fn push_demand(
    per_item: &mut BTreeMap<ItemId, ItemAccumulator>,
    item: ItemId,
    site: DemandSite,
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
        context: context.to_string(),
        value,
    })?;
    Ok(Some(pos))
}

#[derive(Debug, Clone, Copy)]
struct SupplyState {
    id: SupplyNodeId,
    remaining: f64,
}

fn find_best_fit_supply(remaining_supply: &[SupplyState], demand: f64) -> Option<usize> {
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

fn find_largest_non_empty_supply(remaining_supply: &[SupplyState]) -> Option<usize> {
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
    use super::{
        LOGISTICS_EPS, build_logistics_plan, expand_recipe_machine_rates, solve_item_best_fit,
    };
    use crate::types::{
        DemandNode, DemandNodeId, DemandSite, ItemSubproblem, LogisticsNodeSite, PosF64,
        SupplyNode, SupplyNodeId, SupplySite,
    };
    use crate::{SolveInputs, run_two_stage};
    use end_model::{
        AicInputs, Catalog, DisplayName, FacilityDef, ItemDef, Key, OutpostInput, Stack,
        ThermalBankDef,
    };
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
    fn machine_rate_split_preserves_total_execution() {
        let per_machine_cap = 3.0;
        let executions = 7.25;
        let machine_rates = expand_recipe_machine_rates(executions, 4, per_machine_cap);
        let total = machine_rates.iter().map(|(_, rho)| rho).sum::<f64>();
        assert!(
            (total - executions).abs() <= 1e-9,
            "expanded execution should preserve total flow"
        );
        assert!(
            machine_rates
                .iter()
                .all(|(_, rho)| *rho <= per_machine_cap + 1e-9),
            "each machine rate must be bounded by per-machine throughput"
        );
    }

    #[test]
    fn best_fit_fully_satisfies_each_demand() {
        let item = sample_item();
        let subproblem = ItemSubproblem {
            item,
            supplies: vec![
                SupplyNode {
                    id: SupplyNodeId::from_index(0),
                    site: SupplySite::ExternalSupply { item },
                    capacity_per_min: PosF64::new(8.0).expect("positive"),
                },
                SupplyNode {
                    id: SupplyNodeId::from_index(1),
                    site: SupplySite::ExternalSupply { item },
                    capacity_per_min: PosF64::new(5.0).expect("positive"),
                },
            ],
            demands: vec![
                DemandNode {
                    id: DemandNodeId::from_index(0),
                    site: DemandSite::OutpostSale {
                        outpost_index: sample_outpost_id(),
                        item,
                    },
                    demand_per_min: PosF64::new(6.0).expect("positive"),
                },
                DemandNode {
                    id: DemandNodeId::from_index(1),
                    site: DemandSite::OutpostSale {
                        outpost_index: sample_outpost_id(),
                        item,
                    },
                    demand_per_min: PosF64::new(3.0).expect("positive"),
                },
                DemandNode {
                    id: DemandNodeId::from_index(2),
                    site: DemandSite::OutpostSale {
                        outpost_index: sample_outpost_id(),
                        item,
                    },
                    demand_per_min: PosF64::new(4.0).expect("positive"),
                },
            ],
        };

        let flow_plan = solve_item_best_fit(&subproblem).expect("best-fit should be feasible");

        let mut incoming = BTreeMap::<u32, f64>::new();
        let mut outgoing = BTreeMap::<u32, f64>::new();
        for edge in flow_plan.edges {
            *incoming.entry(edge.to.as_u32()).or_insert(0.0) += edge.flow_per_min.get();
            *outgoing.entry(edge.from.as_u32()).or_insert(0.0) += edge.flow_per_min.get();
        }

        for demand in &subproblem.demands {
            let got = incoming.get(&demand.id.as_u32()).copied().unwrap_or(0.0);
            assert!(
                (got - demand.demand_per_min.get()).abs() <= LOGISTICS_EPS,
                "demand node {} should be fully satisfied",
                demand.id.as_u32()
            );
        }

        for supply in &subproblem.supplies {
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
        let mut b = Catalog::builder();
        let ore = b
            .add_item(ItemDef {
                key: key("Ore"),
                en: name("Ore"),
                zh: name("Ore_zh"),
            })
            .expect("add ore");
        let ingot = b
            .add_item(ItemDef {
                key: key("Ingot"),
                en: name("Ingot"),
                zh: name("Ingot_zh"),
            })
            .expect("add ingot");
        let gear = b
            .add_item(ItemDef {
                key: key("Gear"),
                en: name("Gear"),
                zh: name("Gear_zh"),
            })
            .expect("add gear");

        let smelter = b
            .add_facility(FacilityDef {
                key: key("Smelter"),
                power_w: nz(10),
                en: name("Smelter"),
                zh: name("Smelter_zh"),
            })
            .expect("add smelter");
        let assembler = b
            .add_facility(FacilityDef {
                key: key("Assembler"),
                power_w: nz(20),
                en: name("Assembler"),
                zh: name("Assembler_zh"),
            })
            .expect("add assembler");
        b.add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal_Bank_zh"),
        })
        .expect("add thermal bank");

        let smelt_recipe = b
            .push_recipe(
            smelter,
            60,
            vec![Stack {
                item: ore,
                count: 1,
            }],
            vec![Stack {
                item: ingot,
                count: 1,
            }],
        )
        .expect("push smelting recipe");
        b.push_recipe(
            assembler,
            60,
            vec![Stack {
                item: ingot,
                count: 2,
            }],
            vec![Stack {
                item: gear,
                count: 1,
            }],
        )
        .expect("push gear recipe");

        let catalog = b.build().expect("build catalog");

        let aic = AicInputs::new(
            0,
            vec![(ore, nz(20))].into(),
            vec![OutpostInput {
                key: key("Camp"),
                en: Some(name("Camp")),
                zh: Some(name("Camp_zh")),
                money_cap_per_hour: 10_000,
                prices: vec![(gear, 30), (ingot, 5)].into(),
            }],
        )
        .expect("valid aic");

        let solved = run_two_stage(
            &catalog,
            &SolveInputs {
                p_core_w: 500,
                aic: aic.clone(),
            },
        )
        .expect("solve scenario");

        let plan_a = build_logistics_plan(&catalog, &aic, &solved.stage2)
            .expect("logistics plan should build");
        let plan_b = build_logistics_plan(&catalog, &aic, &solved.stage2)
            .expect("logistics plan should be reproducible");

        assert_eq!(
            plan_a, plan_b,
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
                LogisticsNodeSite::RecipeMachine {
                    recipe_index,
                    machine: _,
                } if recipe_index == smelt_recipe => Some(node.id),
                _ => None,
            })
            .expect("smelter machine node should exist");
        let has_outgoing = plan_a.edges.iter().any(|edge| edge.from == smelter_node);
        let has_incoming = plan_a.edges.iter().any(|edge| edge.to == smelter_node);
        assert!(
            has_outgoing && has_incoming,
            "same machine node should carry both input and output edges across items"
        );
    }

    fn sample_item() -> end_model::ItemId {
        let mut b = Catalog::builder();
        b.add_item(ItemDef {
            key: key("x"),
            en: name("x"),
            zh: name("x"),
        })
        .expect("add item")
    }

    fn sample_outpost_id() -> end_model::OutpostId {
        let aic = AicInputs::new(
            0,
            Default::default(),
            vec![OutpostInput {
                key: key("camp"),
                en: Some(name("camp")),
                zh: Some(name("camp")),
                money_cap_per_hour: 1,
                prices: Default::default(),
            }],
        )
        .expect("valid aic");
        aic.outposts_with_id().next().expect("one outpost").0
    }
}
