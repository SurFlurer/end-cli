use end_io::{default_aic_toml, load_aic_from_str, load_catalog};
use end_model::{AicInputs, Catalog, FacilityId, ItemId, OutpostId, RecipeId};
use end_opt::{LogisticsNodeSite, OptimizationResult, run_two_stage};
use end_report::{Lang, build_report};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("catalog load failed: {0}")]
    Catalog(#[source] end_io::Error),

    #[error("default aic generation failed: {0}")]
    DefaultAic(#[source] end_io::Error),

    #[error("aic parse failed: {0}")]
    Aic(#[source] end_io::Error),

    #[error("optimization failed: {0}")]
    Optimize(#[source] end_opt::Error),

    #[error("report build failed: {0}")]
    Report(#[source] end_report::Error),

    #[error("missing item id {0:?}")]
    MissingItem(ItemId),

    #[error("missing facility id {0:?}")]
    MissingFacility(FacilityId),

    #[error("missing recipe id {0:?}")]
    MissingRecipe(RecipeId),

    #[error("missing outpost id {0:?}")]
    MissingOutpost(OutpostId),

    #[error("missing logistics node {node:?} for item {item:?}")]
    MissingLogisticsNode {
        item: ItemId,
        node: end_opt::LogisticsNodeId,
    },

    #[error("unknown lang `{value}` (expected `zh` or `en`)")]
    UnknownLang { value: String },

    #[error("null pointer for argument `{name}`")]
    NullPointer { name: &'static str },

    #[error("argument `{name}` is not valid UTF-8")]
    InvalidUtf8 { name: &'static str },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
struct ComputedSaleValue {
    outpost_index: OutpostId,
    item: ItemId,
    value_per_min: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub default_aic_toml: String,
    pub catalog: CatalogDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDto {
    pub items: Vec<CatalogItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItemDto {
    pub key: String,
    pub en: String,
    pub zh: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolvePayload {
    pub report_text: String,
    pub summary: SummaryDto,
    pub logistics_graph: LogisticsGraphDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryDto {
    pub lang: &'static str,
    pub stage1_revenue_per_min: f64,
    pub stage2_revenue_per_min: f64,
    pub stage2_revenue_per_hour: f64,
    pub total_machines: u32,
    pub total_thermal_banks: u32,
    pub power_gen_w: i64,
    pub power_use_w: i64,
    pub power_margin_w: i64,
    pub outposts: Vec<OutpostValueDto>,
    pub top_sales: Vec<SaleValueDto>,
    pub facilities: Vec<FacilityUsageDto>,
    pub external_supply_slack: Vec<ExternalSupplySlackDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutpostValueDto {
    pub key: String,
    pub name: String,
    pub value_per_min: f64,
    pub cap_per_min: f64,
    pub ratio: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleValueDto {
    pub outpost_key: String,
    pub outpost_name: String,
    pub item_key: String,
    pub item_name: String,
    pub value_per_min: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilityUsageDto {
    pub key: String,
    pub name: String,
    pub machines: u32,
    /// 每台机器的耗电（瓦）
    pub power_w: u32,
    /// 该类机器的总耗电（瓦）
    pub total_power_w: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSupplySlackDto {
    pub item_key: String,
    pub item_name: String,
    pub supply_per_min: f64,
    pub slack_per_min: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogisticsGraphDto {
    pub items: Vec<LogisticsItemSummaryDto>,
    pub nodes: Vec<LogisticsNodeDto>,
    pub edges: Vec<LogisticsEdgeDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogisticsItemSummaryDto {
    pub item_key: String,
    pub item_name: String,
    pub edge_count: usize,
    pub node_count: usize,
    pub total_flow_per_min: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogisticsNodeDto {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogisticsEdgeDto {
    pub id: String,
    pub item_key: String,
    pub item_name: String,
    pub source: String,
    pub target: String,
    pub flow_per_min: f64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ApiEnvelope<T> {
    Ok { data: T },
    Err { error: ApiErrorDto },
}

#[derive(Debug, Serialize)]
struct ApiErrorDto {
    message: String,
}

pub fn bootstrap(lang: Lang) -> Result<BootstrapPayload> {
    let catalog = load_catalog(None).map_err(Error::Catalog)?;
    let default_aic_toml = default_aic_toml(&catalog).map_err(Error::DefaultAic)?;

    let mut items = catalog
        .items()
        .iter()
        .map(|item| CatalogItemDto {
            key: item.key.as_str().to_string(),
            en: item.en.as_str().to_string(),
            zh: item.zh.as_str().to_string(),
        })
        .collect::<Vec<_>>();
    items.sort_by(|lhs, rhs| lhs.key.cmp(&rhs.key));

    // `lang` is currently not used by bootstrap payload fields, but keeping it in signature
    // makes the frontend contract symmetric with solve API and future localization extensions.
    let _ = lang;

    Ok(BootstrapPayload {
        default_aic_toml,
        catalog: CatalogDto { items },
    })
}

pub fn solve_from_aic_toml(lang: Lang, aic_toml: &str) -> Result<SolvePayload> {
    let catalog = load_catalog(None).map_err(Error::Catalog)?;
    let aic = load_aic_from_str(aic_toml, &catalog).map_err(Error::Aic)?;

    let solved = run_two_stage(&catalog, &aic).map_err(Error::Optimize)?;
    let report_text = build_report(lang, &catalog, &aic, &solved).map_err(Error::Report)?;

    Ok(SolvePayload {
        report_text,
        summary: build_summary(lang, &catalog, &aic, &solved)?,
        logistics_graph: build_logistics_graph(lang, &catalog, &aic, &solved)?,
    })
}

fn build_summary(
    lang: Lang,
    catalog: &Catalog,
    inputs: &AicInputs,
    solved: &OptimizationResult,
) -> Result<SummaryDto> {
    let stage1 = &solved.stage1;
    let stage2 = &solved.stage2;

    let outposts = stage2
        .outpost_values
        .iter()
        .map(|value| {
            let outpost = inputs
                .outpost(value.outpost_index)
                .ok_or(Error::MissingOutpost(value.outpost_index))?;
            Ok::<_, Error>(OutpostValueDto {
                key: outpost.key.as_str().to_string(),
                name: outpost_name(lang, outpost).to_string(),
                value_per_min: value.value_per_min,
                cap_per_min: value.cap_per_min,
                ratio: value.ratio,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let top_sales = top_sales_by_value(&stage2.outpost_sales_qty)
        .into_iter()
        .map(|sale| {
            let outpost = inputs
                .outpost(sale.outpost_index)
                .ok_or(Error::MissingOutpost(sale.outpost_index))?;
            Ok::<_, Error>(SaleValueDto {
                outpost_key: outpost.key.as_str().to_string(),
                outpost_name: outpost_name(lang, outpost).to_string(),
                item_key: item_key(catalog, sale.item)?.to_string(),
                item_name: item_name(lang, catalog, sale.item)?.to_string(),
                value_per_min: sale.value_per_min,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let facilities = stage2
        .machines_by_facility
        .iter()
        .map(|usage| {
            let facility_def = catalog
                .facility(usage.facility)
                .ok_or(Error::MissingFacility(usage.facility))?;
            Ok::<_, Error>(FacilityUsageDto {
                key: facility_key(catalog, usage.facility)?.to_string(),
                name: facility_name(lang, catalog, usage.facility)?.to_string(),
                machines: usage.machines,
                power_w: facility_def.power_w.get(),
                total_power_w: facility_def.power_w.get() * usage.machines,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let external_supply_slack = stage2
        .external_supply_slack
        .iter()
        .map(|row| {
            Ok::<_, Error>(ExternalSupplySlackDto {
                item_key: item_key(catalog, row.item)?.to_string(),
                item_name: item_name(lang, catalog, row.item)?.to_string(),
                supply_per_min: row.supply_per_min,
                slack_per_min: row.slack_per_min,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(SummaryDto {
        lang: lang_tag(lang),
        stage1_revenue_per_min: stage1.revenue_per_min,
        stage2_revenue_per_min: stage2.revenue_per_min,
        stage2_revenue_per_hour: stage2.revenue_per_min * 60.0,
        total_machines: stage2.total_machines,
        total_thermal_banks: stage2.total_thermal_banks,
        power_gen_w: stage2.power_gen_w,
        power_use_w: stage2.power_use_w,
        power_margin_w: stage2.power_margin_w,
        outposts,
        top_sales,
        facilities,
        external_supply_slack,
    })
}

fn top_sales_by_value(lines: &[end_opt::OutpostSaleQty]) -> Vec<ComputedSaleValue> {
    let mut sales = lines
        .iter()
        .map(|line| ComputedSaleValue {
            outpost_index: line.outpost_index,
            item: line.item,
            value_per_min: line.qty_per_min.get() * line.price as f64,
        })
        .collect::<Vec<_>>();
    sales.sort_by(|a, b| b.value_per_min.total_cmp(&a.value_per_min));
    sales
}

fn build_logistics_graph(
    lang: Lang,
    catalog: &Catalog,
    inputs: &AicInputs,
    solved: &OptimizationResult,
) -> Result<LogisticsGraphDto> {
    // 构建配方机器数查找表
    let recipe_machines: std::collections::BTreeMap<end_model::RecipeId, u32> = solved
        .stage2
        .recipes_used
        .iter()
        .map(|r| (r.recipe_index, r.machines.get()))
        .collect();
    // 构建热容池数量查找表
    let thermal_banks: std::collections::BTreeMap<end_model::PowerRecipeId, u32> = solved
        .stage2
        .thermal_banks_used
        .iter()
        .map(|t| (t.power_recipe_index, t.banks.get()))
        .collect();

    let node_by_id = solved
        .logistics
        .nodes
        .iter()
        .map(|node| (node.id, node))
        .collect::<BTreeMap<_, _>>();

    let mut nodes = solved
        .logistics
        .nodes
        .iter()
        .map(|node| {
            let (kind, label) = describe_logistics_site(
                lang,
                catalog,
                inputs,
                &node.site,
                &recipe_machines,
                &thermal_banks,
            )?;
            Ok::<_, Error>(LogisticsNodeDto {
                id: logistics_node_id(node.id),
                kind,
                label,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    nodes.sort_by(|lhs, rhs| lhs.id.cmp(&rhs.id));

    let mut edges = Vec::<LogisticsEdgeDto>::new();
    let mut item_summary = BTreeMap::<ItemId, ItemGraphStats>::new();

    for edge in &solved.logistics.edges {
        let item = edge.item;
        let item_key = item_key(catalog, item)?.to_string();
        let item_name = item_name(lang, catalog, item)?.to_string();

        if !node_by_id.contains_key(&edge.from) {
            return Err(Error::MissingLogisticsNode {
                item,
                node: edge.from,
            });
        }
        if !node_by_id.contains_key(&edge.to) {
            return Err(Error::MissingLogisticsNode {
                item,
                node: edge.to,
            });
        }

        edges.push(LogisticsEdgeDto {
            id: format!("{}:{}:{}", item_key, edge.from.as_u32(), edge.to.as_u32()),
            item_key,
            item_name,
            source: logistics_node_id(edge.from),
            target: logistics_node_id(edge.to),
            flow_per_min: edge.flow_per_min.get(),
        });

        let entry = item_summary.entry(item).or_default();
        entry.edge_count += 1;
        entry.node_ids.insert(edge.from.as_u32());
        entry.node_ids.insert(edge.to.as_u32());
        entry.total_flow_per_min += edge.flow_per_min.get();
    }

    let mut items = item_summary
        .into_iter()
        .map(|(item, stats)| {
            Ok::<_, Error>(LogisticsItemSummaryDto {
                item_key: item_key(catalog, item)?.to_string(),
                item_name: item_name(lang, catalog, item)?.to_string(),
                edge_count: stats.edge_count,
                node_count: stats.node_ids.len(),
                total_flow_per_min: stats.total_flow_per_min,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    items.sort_by(|lhs, rhs| lhs.item_key.cmp(&rhs.item_key));

    Ok(LogisticsGraphDto {
        items,
        nodes,
        edges,
    })
}

#[derive(Debug, Default)]
struct ItemGraphStats {
    edge_count: usize,
    node_ids: BTreeSet<u32>,
    total_flow_per_min: f64,
}

fn describe_logistics_site(
    lang: Lang,
    catalog: &Catalog,
    inputs: &AicInputs,
    site: &LogisticsNodeSite,
    recipe_machines: &std::collections::BTreeMap<end_model::RecipeId, u32>,
    thermal_banks: &std::collections::BTreeMap<end_model::PowerRecipeId, u32>,
) -> Result<(String, String)> {
    match site {
        LogisticsNodeSite::ExternalSupply { item } => Ok((
            "external_supply".to_string(),
            match lang {
                Lang::Zh => format!("外部供给 ({})", item_name(lang, catalog, *item)?),
                Lang::En => format!("External supply ({})", item_name(lang, catalog, *item)?),
            },
        )),
        LogisticsNodeSite::RecipeGroup { recipe_index } => {
            let recipe = catalog
                .recipe(*recipe_index)
                .ok_or(Error::MissingRecipe(*recipe_index))?;
            let facility = facility_name(lang, catalog, recipe.facility)?;
            let machines = recipe_machines.get(recipe_index).copied().unwrap_or(1);
            Ok((
                "recipe_group".to_string(),
                match lang {
                    Lang::Zh => format!("{} x{} (r{})", facility, machines, recipe_index.as_u32()),
                    Lang::En => format!("{} x{} (r{})", facility, machines, recipe_index.as_u32()),
                },
            ))
        }
        LogisticsNodeSite::OutpostSale {
            outpost_index,
            item,
        } => {
            let outpost = inputs
                .outpost(*outpost_index)
                .ok_or(Error::MissingOutpost(*outpost_index))?;
            Ok((
                "outpost_sale".to_string(),
                match lang {
                    Lang::Zh => format!(
                        "{} 出售 ({})",
                        outpost_name(lang, outpost),
                        item_name(lang, catalog, *item)?
                    ),
                    Lang::En => format!(
                        "{} sale ({})",
                        outpost_name(lang, outpost),
                        item_name(lang, catalog, *item)?
                    ),
                },
            ))
        }
        LogisticsNodeSite::ThermalBankGroup {
            power_recipe_index,
            item: _,
        } => {
            let banks = thermal_banks.get(power_recipe_index).copied().unwrap_or(1);
            Ok((
                "thermal_bank_group".to_string(),
                match lang {
                    Lang::Zh => format!("热容池组 x{} (p{})", banks, power_recipe_index.as_u32()),
                    Lang::En => format!(
                        "Thermal bank group x{} (p{})",
                        banks,
                        power_recipe_index.as_u32()
                    ),
                },
            ))
        }
    }
}

fn outpost_name(lang: Lang, outpost: &end_model::OutpostInput) -> &str {
    match lang {
        Lang::Zh => outpost.zh.as_deref().unwrap_or(outpost.key.as_str()),
        Lang::En => outpost.en.as_deref().unwrap_or(outpost.key.as_str()),
    }
}

fn item_key(catalog: &Catalog, item: ItemId) -> Result<&str> {
    catalog
        .item(item)
        .map(|v| v.key.as_str())
        .ok_or(Error::MissingItem(item))
}

fn item_name(lang: Lang, catalog: &Catalog, item: ItemId) -> Result<&str> {
    catalog
        .item(item)
        .map(|v| match lang {
            Lang::Zh => v.zh.as_str(),
            Lang::En => v.en.as_str(),
        })
        .ok_or(Error::MissingItem(item))
}

fn facility_key(catalog: &Catalog, facility: FacilityId) -> Result<&str> {
    catalog
        .facility(facility)
        .map(|v| v.key.as_str())
        .ok_or(Error::MissingFacility(facility))
}

fn facility_name(lang: Lang, catalog: &Catalog, facility: FacilityId) -> Result<&str> {
    catalog
        .facility(facility)
        .map(|v| match lang {
            Lang::Zh => v.zh.as_str(),
            Lang::En => v.en.as_str(),
        })
        .ok_or(Error::MissingFacility(facility))
}

fn lang_tag(lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => "zh",
        Lang::En => "en",
    }
}

fn parse_lang(tag: &str) -> Result<Lang> {
    match tag.trim().to_ascii_lowercase().as_str() {
        "zh" => Ok(Lang::Zh),
        "en" => Ok(Lang::En),
        value => Err(Error::UnknownLang {
            value: value.to_string(),
        }),
    }
}

fn logistics_node_id(node: end_opt::LogisticsNodeId) -> String {
    format!("n{}", node.as_u32())
}

fn envelope_json<T: Serialize>(result: Result<T>) -> String {
    let envelope = match result {
        Ok(data) => ApiEnvelope::Ok { data },
        Err(err) => ApiEnvelope::Err {
            error: ApiErrorDto {
                message: err.to_string(),
            },
        },
    };

    match serde_json::to_string(&envelope) {
        Ok(json) => json,
        Err(_) => "{\"status\":\"err\",\"error\":{\"message\":\"failed to serialize response\"}}"
            .to_string(),
    }
}

/// FFI slice representation.
#[repr(C)]
pub struct Slice {
    pub ptr: *const u8,
    pub len: usize,
    pub cap: usize,
}

impl Slice {
    /// Create a Slice from a string.
    /// Reuses the string allocation and transfers ownership to the FFI caller.
    fn from_string(s: String) -> *mut Slice {
        let mut bytes = std::mem::ManuallyDrop::new(s.into_bytes());
        let ptr = bytes.as_mut_ptr() as *const u8;
        let len = bytes.len();
        let cap = bytes.capacity();
        Box::into_raw(Box::new(Slice { ptr, len, cap }))
    }

    /// Read the slice as a string slice.
    ///
    /// # Safety
    /// `ptr` must be valid for `len` bytes.
    unsafe fn as_str(&self) -> Result<&str> {
        if self.ptr.is_null() {
            return Err(Error::NullPointer { name: "slice" });
        }
        let slice = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
        std::str::from_utf8(slice).map_err(|_| Error::InvalidUtf8 { name: "slice" })
    }
}

#[unsafe(no_mangle)]
/// Free a Slice allocated by Rust.
///
/// # Safety
/// `s` must be a pointer previously returned by `end_web_bootstrap` or
/// `end_web_solve_from_aic_toml`, and must be freed exactly once.
pub unsafe extern "C" fn end_web_free_slice(s: *mut Slice) {
    if s.is_null() {
        return;
    }
    // SAFETY: caller ensures s is valid and freed exactly once
    let s_ref = unsafe { Box::from_raw(s) };
    if s_ref.cap == 0 || s_ref.ptr.is_null() {
        return;
    }
    // SAFETY: from_string constructed (ptr, len, cap) from a Vec<u8>.
    unsafe {
        _ = Vec::from_raw_parts(s_ref.ptr as *mut u8, s_ref.len, s_ref.cap);
    }
}

#[unsafe(no_mangle)]
/// Build bootstrap payload JSON string (`catalog` + default `aic.toml`).
///
/// # Safety
/// `lang` must be a valid pointer to a Slice.
/// Returns a pointer to a Slice on success, or null on failure.
pub unsafe extern "C" fn end_web_bootstrap(lang: *const Slice) -> *mut Slice {
    if lang.is_null() {
        return Slice::from_string(envelope_json::<BootstrapPayload>(Err(Error::NullPointer {
            name: "lang",
        })));
    }
    let result = {
        let lang = match unsafe { (*lang).as_str() } {
            Ok(s) => s,
            Err(e) => return Slice::from_string(envelope_json::<BootstrapPayload>(Err(e))),
        };
        match parse_lang(lang) {
            Ok(lang) => bootstrap(lang),
            Err(err) => Err(err),
        }
    };
    Slice::from_string(envelope_json(result))
}

#[unsafe(no_mangle)]
/// Run optimization from `aic.toml` text and return JSON result.
///
/// # Safety
/// `lang` and `aic_toml` must be valid pointers to Slice.
/// Returns a pointer to a Slice on success, or null on failure.
pub unsafe extern "C" fn end_web_solve_from_aic_toml(
    lang: *const Slice,
    aic_toml: *const Slice,
) -> *mut Slice {
    if lang.is_null() {
        return Slice::from_string(envelope_json::<SolvePayload>(Err(Error::NullPointer {
            name: "lang",
        })));
    }
    if aic_toml.is_null() {
        return Slice::from_string(envelope_json::<SolvePayload>(Err(Error::NullPointer {
            name: "aic_toml",
        })));
    }
    let result = {
        let lang = match unsafe { (*lang).as_str() } {
            Ok(s) => s,
            Err(e) => return Slice::from_string(envelope_json::<SolvePayload>(Err(e))),
        };
        let aic_toml = match unsafe { (*aic_toml).as_str() } {
            Ok(s) => s,
            Err(e) => return Slice::from_string(envelope_json::<SolvePayload>(Err(e))),
        };

        match parse_lang(lang) {
            Ok(lang) => solve_from_aic_toml(lang, aic_toml),
            Err(err) => Err(err),
        }
    };
    Slice::from_string(envelope_json(result))
}

#[cfg(test)]
mod tests {
    use super::{bootstrap, solve_from_aic_toml};
    use end_io::{default_aic_toml, load_catalog};
    use end_report::Lang;

    #[test]
    fn bootstrap_returns_catalog_and_default_aic() {
        let payload = bootstrap(Lang::Zh).expect("bootstrap should succeed");
        assert!(
            payload
                .default_aic_toml
                .contains("external_power_consumption_w"),
            "default aic should include external power field"
        );
        assert!(
            !payload.catalog.items.is_empty(),
            "catalog items should not be empty"
        );
    }

    #[test]
    fn solve_runs_with_builtin_default_aic() {
        let catalog = load_catalog(None).expect("builtin catalog should load");
        let aic_toml = default_aic_toml(&catalog).expect("default aic should build");

        let payload = solve_from_aic_toml(Lang::Zh, &aic_toml).expect("solve should succeed");
        assert!(
            payload.summary.stage2_revenue_per_min >= 0.0,
            "revenue should be non-negative"
        );
        assert!(
            !payload.logistics_graph.nodes.is_empty(),
            "default scenario should include logistics nodes"
        );
    }

    #[test]
    fn solve_rejects_invalid_toml() {
        let err = solve_from_aic_toml(Lang::Zh, "external_power_consumption_w = 'oops'")
            .expect_err("invalid toml should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("aic parse failed"),
            "error message should mention aic parse"
        );
    }
}
