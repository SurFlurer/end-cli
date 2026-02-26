use crate::{FacilityId, ItemId, OutpostId, PowerRecipeId, RecipeId};
use generativity::Id;
use std::num::NonZeroU32;

/// Finite, strictly positive floating-point value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PosF64(f64);

impl PosF64 {
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

/// Value utilization of one outpost.
#[derive(Debug, Clone)]
pub struct OutpostValue<'sid> {
    pub outpost_index: OutpostId<'sid>,
    /// Realized revenue on this outpost.
    pub value_per_min: f64,
    /// Theoretical cap from outpost config.
    pub cap_per_min: f64,
    /// `value_per_min / cap_per_min`, in `[0, +inf)`.
    pub ratio: f64,
}

/// One sale line contribution with quantity information.
#[derive(Debug, Clone, PartialEq)]
pub struct OutpostSaleQty<'cid, 'sid> {
    pub outpost_index: OutpostId<'sid>,
    /// Item being sold.
    pub item: ItemId<'cid>,
    /// Sold quantity in units/min.
    pub qty_per_min: PosF64,
    /// Unit price used by optimization objective.
    pub price: u32,
}

/// Machine count aggregated by facility type.
#[derive(Debug, Clone)]
pub struct FacilityMachineCount<'id> {
    /// Facility id from `Catalog.facilities`.
    pub facility: FacilityId<'id>,
    /// Integer machine count.
    pub machines: u32,
}

/// Execution and machine usage for one recipe.
#[derive(Debug, Clone)]
pub struct RecipeUsage<'id> {
    pub recipe_index: RecipeId<'id>,
    /// Integer machine count assigned to the recipe.
    pub machines: NonZeroU32,
    /// Recipe runs per minute.
    pub executions_per_min: f64,
}

/// Thermal bank deployment for one power recipe.
#[derive(Debug, Clone)]
pub struct ThermalBankUsage<'id> {
    pub power_recipe_index: PowerRecipeId<'id>,
    /// Fuel item consumed by the thermal bank.
    pub ingredient: ItemId<'id>,
    /// Number of thermal banks.
    pub banks: NonZeroU32,
    /// Per-bank power output.
    pub power_w: u32,
    /// Per-bank cycle duration in seconds.
    pub duration_s: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LogisticsNodeId<'rid> {
    raw: u32,
    brand: Id<'rid>,
}

impl<'rid> LogisticsNodeId<'rid> {
    pub fn from_index(index: usize, brand: Id<'rid>) -> Self {
        Self {
            raw: index as u32,
            brand,
        }
    }

    pub fn as_u32(self) -> u32 {
        self.raw
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogisticsNodeSite<'cid, 'sid> {
    ExternalSupply {
        item: ItemId<'cid>,
    },
    ExternalConsumption {
        item: ItemId<'cid>,
    },
    RecipeGroup {
        recipe_index: RecipeId<'cid>,
    },
    OutpostSale {
        outpost_index: OutpostId<'sid>,
        item: ItemId<'cid>,
    },
    ThermalBankGroup {
        power_recipe_index: PowerRecipeId<'cid>,
        item: ItemId<'cid>,
    },
    /// Stockpile leftover items into warehouse.
    ///
    /// This node represents per-item remaining quantities after fulfilling all *real* demands.
    /// In particular, virtual sales (used by stage-2 objectives) should be interpreted as
    /// potential value rather than a physical sink, so those quantities end up here.
    WarehouseStockpile {
        item: ItemId<'cid>,
    },
}

/// Per-item stockpile quantity in units/min.
#[derive(Debug, Clone)]
pub struct ItemStockpile<'cid> {
    pub item: ItemId<'cid>,
    pub qty_per_min: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogisticsNode<'cid, 'sid, 'rid> {
    pub id: LogisticsNodeId<'rid>,
    pub site: LogisticsNodeSite<'cid, 'sid>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogisticsEdge<'cid, 'rid> {
    pub item: ItemId<'cid>,
    pub from: LogisticsNodeId<'rid>,
    pub to: LogisticsNodeId<'rid>,
    pub flow_per_min: PosF64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LogisticsPlan<'cid, 'sid, 'rid> {
    pub nodes: Box<[LogisticsNode<'cid, 'sid, 'rid>]>,
    pub edges: Box<[LogisticsEdge<'cid, 'rid>]>,
}

/// Remaining slack for each externally supplied item.
#[derive(Debug, Clone)]
pub struct ExternalSupplySlack<'cid> {
    /// Item id from `Catalog.items`.
    pub item: ItemId<'cid>,
    /// Remaining quantity in the item balance constraint.
    pub slack_per_min: f64,
    /// Input external supply configured for this item.
    pub supply_per_min: f64,
}

/// Result of one optimization stage.
#[derive(Debug, Clone)]
pub struct StageSolution<'cid, 'sid> {
    /// Total revenue objective value.
    pub revenue_per_min: f64,
    /// Per-outpost value realization.
    pub outpost_values: Box<[OutpostValue<'sid>]>,
    /// Full sale lines with quantities and unit prices.
    /// Used to reconstruct logistics demands and derive top-sales summaries.
    pub outpost_sales_qty: Box<[OutpostSaleQty<'cid, 'sid>]>,
    /// Machine counts by facility.
    pub machines_by_facility: Box<[FacilityMachineCount<'cid>]>,
    /// Top recipes by machine count.
    pub recipes_used: Box<[RecipeUsage<'cid>]>,
    /// Thermal bank allocations.
    pub thermal_banks_used: Box<[ThermalBankUsage<'cid>]>,
    /// Slack information for externally supplied items.
    pub external_supply_slack: Box<[ExternalSupplySlack<'cid>]>,
    /// Per-item stockpile quantities (units/min).
    pub item_stockpile: Box<[ItemStockpile<'cid>]>,
    /// Core generation capacity in watts.
    pub p_core_w: u32,
    /// External power consumption in watts.
    pub p_ext_w: u32,
    /// Sum of all production machines.
    pub total_machines: u32,
    /// Sum of all thermal banks.
    pub total_thermal_banks: u32,
    /// Total power generation.
    pub power_gen_w: u32,
    /// Total power usage.
    pub power_use_w: u32,
    /// `power_gen_w - power_use_w`.
    pub power_margin_w: u32,
    /// Stage-level virtual money slack value in per-minute revenue.
    pub money_slack_per_min: f64,
}

/// Combined output for stage 1 and stage 2.
#[derive(Debug, Clone)]
pub struct OptimizationResult<'cid, 'sid, 'rid> {
    /// Stage 1: max revenue.
    pub stage1: StageSolution<'cid, 'sid>,
    /// Stage 2: min machine counts with revenue floor.
    pub stage2: StageSolution<'cid, 'sid>,
    /// Machine-granularity logistics flow plan derived from stage 2.
    pub logistics: LogisticsPlan<'cid, 'sid, 'rid>,
}
