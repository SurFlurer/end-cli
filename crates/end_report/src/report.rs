use crate::ansi::Ansi;
use crate::format::{
    facility_display_name, format_recipe_label, item_display_name, outpost_display_name, t,
};
use crate::{Error, Lang, Result};
use end_model::{AicInputs, Catalog, ItemId, OutpostId};
use end_opt::{LogisticsEdge, LogisticsNode, LogisticsNodeSite, OptimizationResult};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
struct ReportSaleValue {
    outpost_index: OutpostId,
    item: ItemId,
    value_per_min: f64,
}

/// Render a human-readable optimization report from solved results.
pub fn build_report(
    lang: Lang,
    catalog: &Catalog,
    inputs: &AicInputs,
    result: &OptimizationResult,
) -> Result<String> {
    let stage1 = &result.stage1;
    let stage2 = &result.stage2;
    let a = Ansi::from_env();

    let mut out = String::new();

    out.push_str(&format!("{}\n", a.h(t(lang, "结论", "Conclusion"))));
    out.push_str(&format!(
        "{}\n",
        a.good(&match lang {
            Lang::Zh => format!(
                "结论：在当前外部供给与 P^ext={}W 下，最优收入约 {:.2}/min（{:.0}/h），对应产线规模：生产机器 {} 台 + 热容池 {} 台；电力余量 {}W。",
                stage2.p_ext_w,
                stage2.revenue_per_min,
                stage2.revenue_per_min * 60.0,
                stage2.total_machines,
                stage2.total_thermal_banks,
                stage2.power_margin_w,
            ),
            Lang::En => format!(
                "Conclusion: with the current external supply and P^ext={}W, optimal revenue is about {:.2}/min ({:.0}/h). Line size: {} production machines + {} thermal banks; power margin {}W.",
                stage2.p_ext_w,
                stage2.revenue_per_min,
                stage2.revenue_per_min * 60.0,
                stage2.total_machines,
                stage2.total_thermal_banks,
                stage2.power_margin_w,
            ),
        })
    ));

    let delta = (stage1.revenue_per_min - stage2.revenue_per_min).abs();
    if delta > 1e-6 {
        out.push_str(&format!(
            "{}\n",
            a.warn(&match lang {
                Lang::Zh => format!(
                    "提示：阶段2收入与阶段1最优有微小差异（|Δ|={:.6}），可能受数值精度影响。",
                    delta
                ),
                Lang::En => format!(
                    "Note: stage2 revenue differs slightly from stage1 optimum (|Δ|={delta:.6}), likely due to numeric precision."
                ),
            })
        ));
    }

    out.push('\n');
    out.push_str(&format!("{}\n", a.h(t(lang, "交易", "Trading"))));
    for ov in &stage2.outpost_values {
        let outpost = inputs
            .outpost(ov.outpost_index)
            .ok_or(Error::MissingOutpost(ov.outpost_index))?;
        let name = outpost_display_name(lang, outpost);
        let at_cap = ov.cap_per_min > 0.0 && ov.ratio >= 0.9999;
        let tag = if at_cap {
            a.good(t(lang, "触顶", "Capped"))
        } else {
            a.warn(t(lang, "未触顶", "Not capped"))
        };

        out.push_str(&format!(
            "- {}: {}\n",
            name,
            match lang {
                Lang::Zh => format!(
                    "{:.2}/min（上限 {:.2}/min，{:.0}%） {}",
                    ov.value_per_min,
                    ov.cap_per_min,
                    ov.ratio * 100.0,
                    tag
                ),
                Lang::En => format!(
                    "{:.2}/min (cap {:.2}/min, {:.0}%) {}",
                    ov.value_per_min,
                    ov.cap_per_min,
                    ov.ratio * 100.0,
                    tag
                ),
            }
        ));
    }

    let top_sales = top_sales_by_value(&stage2.outpost_sales_qty);
    if !top_sales.is_empty() {
        out.push_str(&format!(
            "{}\n",
            a.dim(t(
                lang,
                "销售排行（按收入贡献）:",
                "Top sales (by revenue):"
            ))
        ));
        for sale in top_sales {
            let outpost = inputs
                .outpost(sale.outpost_index)
                .ok_or(Error::MissingOutpost(sale.outpost_index))?;
            let item = item_display_name(lang, catalog, sale.item)?;
            out.push_str(&format!(
                "- {} @ {}: {:.2}/min\n",
                item,
                outpost_display_name(lang, outpost),
                sale.value_per_min
            ));
        }
    }

    out.push('\n');
    out.push_str(&format!("{}\n", a.h(t(lang, "电力", "Power"))));
    let p_tag = if stage2.power_margin_w < 1 {
        a.warn(t(lang, "紧张", "Tight"))
    } else {
        a.good(t(lang, "充足", "OK"))
    };
    out.push_str(&format!(
        "- {}\n",
        match lang {
            Lang::Zh => format!(
                "发电 {}W = P^core {}W + 热容池发电；用电 {}W = P^ext {}W + 生产机器耗电；余量 {}W {}",
                stage2.power_gen_w,
                stage2.p_core_w,
                stage2.power_use_w,
                stage2.p_ext_w,
                stage2.power_margin_w,
                p_tag
            ),
            Lang::En => format!(
                "Generation {}W = P^core {}W + thermal banks; usage {}W = P^ext {}W + production machines; margin {}W {}",
                stage2.power_gen_w,
                stage2.p_core_w,
                stage2.power_use_w,
                stage2.p_ext_w,
                stage2.power_margin_w,
                p_tag
            ),
        }
    ));

    if !stage2.thermal_banks_used.is_empty() {
        out.push_str(&format!(
            "{}\n",
            a.dim(t(lang, "热容池配置:", "Thermal bank setup:"))
        ));
        for tb in &stage2.thermal_banks_used {
            let item = item_display_name(lang, catalog, tb.ingredient)?;
            let consume_per_min = 60.0 / tb.duration_s as f64;
            out.push_str(&format!(
                "- {} x{}: {}\n",
                item,
                tb.banks.get(),
                match lang {
                    Lang::Zh => format!(
                        "每台 {}W，耗时 {}s（消耗 {:.3}/min）",
                        tb.power_w, tb.duration_s, consume_per_min
                    ),
                    Lang::En => format!(
                        "{}W each, {}s (consumes {:.3}/min)",
                        tb.power_w, tb.duration_s, consume_per_min
                    ),
                }
            ));
        }
    }

    out.push('\n');
    out.push_str(&format!("{}\n", a.h(t(lang, "产线", "Production"))));
    out.push_str(&format!(
        "- {}\n",
        match lang {
            Lang::Zh => format!("生产机器总数 {}（按设施汇总）", stage2.total_machines),
            Lang::En => format!(
                "Total production machines: {} (by facility)",
                stage2.total_machines
            ),
        }
    ));

    for f in stage2.machines_by_facility.iter() {
        let facility = facility_display_name(lang, catalog, f.facility)?;
        out.push_str(&format!("- {}: {}\n", facility, f.machines));
    }
    if stage2.machines_by_facility.len() > 12 {
        out.push_str(&format!(
            "{}\n",
            a.dim(&match lang {
                Lang::Zh => format!(
                    "（其余 {} 种设施略）",
                    stage2.machines_by_facility.len() - 12
                ),
                Lang::En => format!(
                    "(omitted {} more facilities)",
                    stage2.machines_by_facility.len() - 12
                ),
            })
        ));
    }

    if !stage2.recipes_used.is_empty() {
        out.push_str(&format!(
            "{}\n",
            a.dim(t(
                lang,
                "配方排行（按机器数）:",
                "Top recipes (by machine count):"
            ))
        ));
        for r in stage2.recipes_used.iter() {
            let recipe = catalog
                .recipe(r.recipe_index)
                .ok_or(Error::MissingRecipe(r.recipe_index))?;
            let label = format_recipe_label(
                lang,
                catalog,
                recipe.facility,
                &recipe.ingredients,
                &recipe.products,
                recipe.time_s,
            )?;
            out.push_str(&format!(
                "- {} | {} {} | {:.3} {}\n",
                label,
                t(lang, "机器", "machines"),
                r.machines.get(),
                r.executions_per_min,
                t(lang, "次/min", "runs/min")
            ));
        }
    }

    out.push('\n');
    out.push_str(&format!("{}\n", a.h(t(lang, "物流", "Logistics"))));
    render_logistics(lang, catalog, inputs, result, &mut out)?;

    out.push('\n');
    out.push_str(&format!(
        "{}\n",
        a.h(t(lang, "瓶颈与改进", "Bottlenecks & Tips"))
    ));

    let mut any = false;
    for ov in &stage2.outpost_values {
        if ov.ratio > 0.98 {
            any = true;
            let outpost = inputs
                .outpost(ov.outpost_index)
                .ok_or(Error::MissingOutpost(ov.outpost_index))?;
            out.push_str(&format!(
                "- {}\n",
                match lang {
                    Lang::Zh => format!(
                        "{}交易额触顶，做得好！",
                        outpost_display_name(lang, outpost)
                    ),
                    Lang::En => format!(
                        "{} is capped, great job!",
                        outpost_display_name(lang, outpost)
                    ),
                }
            ));
        }
    }

    if stage2.power_margin_w < 1 {
        any = true;
        out.push_str(&format!(
            "- {}\n",
            t(
                lang,
                "电力接近满载：想扩产需要更多热容池燃料。",
                "Power is near full load: scaling up needs more thermal-bank fuel.",
            )
        ));
    }

    for s in stage2.external_supply_slack.iter().take(3) {
        if s.slack_per_min < 1e-3 {
            any = true;
            let item = item_display_name(lang, catalog, s.item)?;
            out.push_str(&format!(
                "- {}\n",
                match lang {
                    Lang::Zh => format!(
                        "外部供给吃满：{} 基本用尽（{:.0}/min），提升采集/供给会直接提高上限。",
                        item, s.supply_per_min
                    ),
                    Lang::En => format!(
                        "External supply is saturated: {} is basically exhausted ({:.0}/min). Increasing supply raises the cap directly.",
                        item, s.supply_per_min
                    ),
                }
            ));
        }
    }

    out.push_str(&format!(
        "{}\n",
        a.dim(t(lang, "外部供给剩余:", "External supply slack:"))
    ));
    for s in &stage2.external_supply_slack {
        let used = (s.supply_per_min - s.slack_per_min).max(0.0);
        let pct = if s.supply_per_min > 0.0 {
            used / s.supply_per_min * 100.0
        } else {
            0.0
        };
        let item = item_display_name(lang, catalog, s.item)?;
        out.push_str(&format!(
            "- {}: {}\n",
            item,
            match lang {
                Lang::Zh => format!(
                    "用 {:.1}/min / 供给 {:.1}/min（剩 {:.1}/min，{:.0}% 使用）",
                    used, s.supply_per_min, s.slack_per_min, pct
                ),
                Lang::En => format!(
                    "used {:.1}/min / supply {:.1}/min (slack {:.1}/min, {:.0}% used)",
                    used, s.supply_per_min, s.slack_per_min, pct
                ),
            }
        ));
    }

    if !any {
        out.push_str(&format!(
            "- {}\n",
            t(
                lang,
                "暂无明显单一瓶颈：可优先尝试提高高价商品产量，或检查是否有更优的售卖组合。",
                "No single obvious bottleneck: try increasing high-price outputs first, or check for a better sales mix.",
            )
        ));
    }

    Ok(out)
}

fn top_sales_by_value(lines: &[end_opt::OutpostSaleQty]) -> Vec<ReportSaleValue> {
    let mut sales = lines
        .iter()
        .map(|line| ReportSaleValue {
            outpost_index: line.outpost_index,
            item: line.item,
            value_per_min: line.qty_per_min.get() * line.price as f64,
        })
        .collect::<Vec<_>>();
    sales.sort_by(|a, b| b.value_per_min.total_cmp(&a.value_per_min));
    sales
}

fn render_logistics(
    lang: Lang,
    catalog: &Catalog,
    inputs: &AicInputs,
    result: &OptimizationResult,
    out: &mut String,
) -> Result<()> {
    let node_by_id = result
        .logistics
        .nodes
        .iter()
        .map(|node| (node.id, node))
        .collect::<BTreeMap<_, _>>();
    let mut edges_by_item = BTreeMap::<end_model::ItemId, Vec<&LogisticsEdge>>::new();
    for edge in &result.logistics.edges {
        edges_by_item.entry(edge.item).or_default().push(edge);
    }

    let total_edges = result.logistics.edges.len();
    let total_flow = result
        .logistics
        .edges
        .iter()
        .map(|edge| edge.flow_per_min.get())
        .sum::<f64>();

    out.push_str(&format!(
        "- {}\n",
        match lang {
            Lang::Zh => format!(
                "总物流连接 {} 条，覆盖 {} 种物品，总流量 {:.3}/min。",
                total_edges,
                edges_by_item.len(),
                total_flow
            ),
            Lang::En => format!(
                "{} logistics edges across {} items, total flow {:.3}/min.",
                total_edges,
                edges_by_item.len(),
                total_flow
            ),
        }
    ));

    if edges_by_item.is_empty() {
        out.push_str(&format!(
            "{}\n",
            t(
                lang,
                "- 当前场景没有需要分配的物流边。",
                "- No logistics edges are required in this scenario."
            )
        ));
        return Ok(());
    }

    let edge_preview_limit = 6usize;
    for (item, edges) in edges_by_item {
        let item_name = item_display_name(lang, catalog, item)?;
        let total_item_flow = edges
            .iter()
            .map(|edge| edge.flow_per_min.get())
            .sum::<f64>();
        let node_count = edges
            .iter()
            .flat_map(|edge| [edge.from.as_u32(), edge.to.as_u32()])
            .collect::<BTreeSet<_>>()
            .len();
        let grouped_edges =
            group_logistics_edges(lang, catalog, inputs, &node_by_id, item, &edges)?;

        out.push_str(&format!(
            "- {}: {}\n",
            item_name,
            match lang {
                Lang::Zh => format!(
                    "节点 {}，连接 {}（合并后 {} 组），流量 {:.3}/min",
                    node_count,
                    edges.len(),
                    grouped_edges.len(),
                    total_item_flow,
                ),
                Lang::En => format!(
                    "nodes {}, edges {} (merged into {} groups), flow {:.3}/min",
                    node_count,
                    edges.len(),
                    grouped_edges.len(),
                    total_item_flow,
                ),
            }
        ));

        for group in grouped_edges.iter().take(edge_preview_limit) {
            let from = with_machine_ordinals(&group.from.base, &group.from_ordinals);
            let to = with_machine_ordinals(&group.to.base, &group.to_ordinals);
            let detail = if group.edge_count == 1 {
                format!("{:.3}/min", group.flow_per_min)
            } else {
                format!(
                    "{:.3}/min x{} = {:.3}/min",
                    group.flow_per_min, group.edge_count, group.total_flow_per_min
                )
            };
            out.push_str(&format!("  - {} -> {}: {}\n", from, to, detail));
        }

        if grouped_edges.len() > edge_preview_limit {
            out.push_str(&format!(
                "  - {}\n",
                match lang {
                    Lang::Zh => format!(
                        "其余 {} 组连接略。",
                        grouped_edges.len() - edge_preview_limit
                    ),
                    Lang::En => format!(
                        "{} more merged groups omitted.",
                        grouped_edges.len() - edge_preview_limit
                    ),
                }
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct RenderedEndpoint {
    base: String,
    machine_ordinal: Option<u32>,
}

#[derive(Debug, Clone)]
struct GroupedLogisticsEdge {
    from: RenderedEndpoint,
    to: RenderedEndpoint,
    from_ordinals: BTreeSet<u32>,
    to_ordinals: BTreeSet<u32>,
    flow_per_min: f64,
    total_flow_per_min: f64,
    edge_count: usize,
}

fn group_logistics_edges(
    lang: Lang,
    catalog: &Catalog,
    inputs: &AicInputs,
    node_by_id: &BTreeMap<end_opt::LogisticsNodeId, &LogisticsNode>,
    item: end_model::ItemId,
    item_edges: &[&LogisticsEdge],
) -> Result<Vec<GroupedLogisticsEdge>> {
    let mut grouped = BTreeMap::<(String, String, i64), GroupedLogisticsEdge>::new();

    for edge in item_edges {
        let from_node = node_by_id
            .get(&edge.from)
            .copied()
            .ok_or(Error::MissingLogisticsNode {
                item,
                node: edge.from,
            })?;
        let to_node = node_by_id
            .get(&edge.to)
            .copied()
            .ok_or(Error::MissingLogisticsNode {
                item,
                node: edge.to,
            })?;
        let from = describe_logistics_site(lang, inputs, catalog, &from_node.site)?;
        let to = describe_logistics_site(lang, inputs, catalog, &to_node.site)?;
        let flow_per_min = edge.flow_per_min.get();
        let key = (
            from.base.clone(),
            to.base.clone(),
            quantized_flow(flow_per_min),
        );

        let group = grouped.entry(key).or_insert_with(|| GroupedLogisticsEdge {
            from: from.clone(),
            to: to.clone(),
            from_ordinals: BTreeSet::new(),
            to_ordinals: BTreeSet::new(),
            flow_per_min,
            total_flow_per_min: 0.0,
            edge_count: 0,
        });

        if let Some(ordinal) = from.machine_ordinal {
            group.from_ordinals.insert(ordinal);
        }
        if let Some(ordinal) = to.machine_ordinal {
            group.to_ordinals.insert(ordinal);
        }
        group.edge_count += 1;
        group.total_flow_per_min += flow_per_min;
    }

    let mut groups = grouped.into_values().collect::<Vec<_>>();
    groups.sort_by(|lhs, rhs| {
        rhs.total_flow_per_min
            .total_cmp(&lhs.total_flow_per_min)
            .then_with(|| rhs.edge_count.cmp(&lhs.edge_count))
            .then_with(|| lhs.from.base.cmp(&rhs.from.base))
            .then_with(|| lhs.to.base.cmp(&rhs.to.base))
    });
    Ok(groups)
}

fn describe_logistics_site(
    lang: Lang,
    inputs: &AicInputs,
    catalog: &Catalog,
    site: &LogisticsNodeSite,
) -> Result<RenderedEndpoint> {
    let rendered = match site {
        LogisticsNodeSite::ExternalSupply { item } => {
            let item = item_display_name(lang, catalog, *item)?;
            match lang {
                Lang::Zh => RenderedEndpoint {
                    base: format!("外部供给({item})"),
                    machine_ordinal: None,
                },
                Lang::En => RenderedEndpoint {
                    base: format!("External supply ({item})"),
                    machine_ordinal: None,
                },
            }
        }
        LogisticsNodeSite::RecipeGroup { recipe_index } => {
            let recipe = catalog
                .recipe(*recipe_index)
                .ok_or(Error::MissingRecipe(*recipe_index))?;
            let facility = facility_display_name(lang, catalog, recipe.facility)?;
            match lang {
                Lang::Zh => RenderedEndpoint {
                    base: format!("{} r{}", facility, recipe_index.as_u32()),
                    machine_ordinal: None,
                },
                Lang::En => RenderedEndpoint {
                    base: format!("{} r{}", facility, recipe_index.as_u32()),
                    machine_ordinal: None,
                },
            }
        }
        LogisticsNodeSite::OutpostSale {
            outpost_index,
            item,
        } => {
            let outpost = inputs
                .outpost(*outpost_index)
                .ok_or(Error::MissingOutpost(*outpost_index))?;
            let item = item_display_name(lang, catalog, *item)?;
            match lang {
                Lang::Zh => RenderedEndpoint {
                    base: format!("{} 出售({item})", outpost_display_name(lang, outpost)),
                    machine_ordinal: None,
                },
                Lang::En => RenderedEndpoint {
                    base: format!("{} sale ({item})", outpost_display_name(lang, outpost)),
                    machine_ordinal: None,
                },
            }
        }
        LogisticsNodeSite::ThermalBankGroup {
            power_recipe_index, ..
        } => match lang {
            Lang::Zh => RenderedEndpoint {
                base: format!("热容池组 p{}", power_recipe_index.as_u32()),
                machine_ordinal: None,
            },
            Lang::En => RenderedEndpoint {
                base: format!("Thermal bank group p{}", power_recipe_index.as_u32()),
                machine_ordinal: None,
            },
        },
    };
    Ok(rendered)
}

fn quantized_flow(flow_per_min: f64) -> i64 {
    (flow_per_min * 1_000_000_000.0).round() as i64
}

fn with_machine_ordinals(base: &str, ordinals: &BTreeSet<u32>) -> String {
    if ordinals.is_empty() {
        return base.to_string();
    }
    format!("{base} #{}", format_machine_ranges(ordinals))
}

fn format_machine_ranges(ordinals: &BTreeSet<u32>) -> String {
    let mut parts = Vec::new();
    let mut start = None::<u32>;
    let mut prev = 0u32;

    for &ordinal in ordinals {
        match start {
            None => {
                start = Some(ordinal);
                prev = ordinal;
            }
            Some(range_start) if ordinal == prev + 1 => {
                prev = ordinal;
                start = Some(range_start);
            }
            Some(range_start) => {
                parts.push(render_machine_range(range_start, prev));
                start = Some(ordinal);
                prev = ordinal;
            }
        }
    }
    if let Some(range_start) = start {
        parts.push(render_machine_range(range_start, prev));
    }
    parts.join(",")
}

fn render_machine_range(start: u32, end: u32) -> String {
    if start == end {
        return start.to_string();
    }
    format!("{start}-{end}")
}
