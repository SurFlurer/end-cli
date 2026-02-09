use crate::ansi::Ansi;
use crate::format::{
    facility_display_name, format_recipe_label, item_display_name, outpost_display_name, t,
};
use crate::{Error, Lang, Result};
use end_model::{AicInputs, Catalog};
use end_opt::OptimizationResult;

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
            .outposts
            .get(ov.outpost_index.as_usize())
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

    if !stage2.top_sales.is_empty() {
        out.push_str(&format!(
            "{}\n",
            a.dim(t(
                lang,
                "销售排行（按收入贡献）:",
                "Top sales (by revenue):"
            ))
        ));
        for sale in &stage2.top_sales {
            let outpost = inputs
                .outposts
                .get(sale.outpost_index.as_usize())
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
                tb.banks,
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

    for f in stage2.machines_by_facility.iter().take(12) {
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
        for r in stage2.recipes_used.iter().take(12) {
            let recipe = catalog
                .recipe(r.recipe_index.as_usize())
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
                r.machines,
                r.executions_per_min,
                t(lang, "次/min", "runs/min")
            ));
        }
    }

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
                .outposts
                .get(ov.outpost_index.as_usize())
                .ok_or(Error::MissingOutpost(ov.outpost_index))?;
            out.push_str(&format!(
                "- {}\n",
                match lang {
                    Lang::Zh => format!(
                        "{} 交易额已触顶：继续增产也卖不出去，优先改卖单价更高的商品或换/加据点。",
                        outpost_display_name(lang, outpost)
                    ),
                    Lang::En => format!(
                        "{} is capped: producing more won't sell; prioritize higher-price products or switch/add outposts.",
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
                "电力接近满载：想扩产需要更多热容池燃料（或降低 P^ext/机器数）。",
                "Power is near full load: scaling up needs more thermal-bank fuel (or fewer machines / lower P^ext).",
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
