use end_model::{AicInputs, FacilityId, ItemId, OutpostId, PowerRecipeId, RecipeId};

/// Input bundle for optimization.
#[derive(Debug, Clone)]
pub struct SolveInputs {
    /// Core generation capacity in watts.
    pub p_core_w: u32,
    /// Scenario inputs (external power, supply, outposts).
    pub aic: AicInputs,
}

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
pub struct OutpostValue {
    pub outpost_index: OutpostId,
    /// Realized revenue on this outpost.
    pub value_per_min: f64,
    /// Theoretical cap from outpost config.
    pub cap_per_min: f64,
    /// `value_per_min / cap_per_min`, in `[0, +inf)`.
    pub ratio: f64,
}

/// One sale line contribution in the ranked top-sales list.
#[derive(Debug, Clone)]
pub struct SaleValue {
    pub outpost_index: OutpostId,
    /// Item being sold.
    pub item: ItemId,
    /// Revenue contribution.
    pub value_per_min: f64,
}

/// One sale line contribution with quantity information.
#[derive(Debug, Clone, PartialEq)]
pub struct OutpostSaleQty {
    pub outpost_index: OutpostId,
    /// Item being sold.
    pub item: ItemId,
    /// Sold quantity in units/min.
    pub qty_per_min: PosF64,
    /// Unit price used by optimization objective.
    pub price: u32,
}

/// Machine count aggregated by facility type.
#[derive(Debug, Clone)]
pub struct FacilityMachineCount {
    /// Facility id from `Catalog.facilities`.
    pub facility: FacilityId,
    /// Integer machine count.
    pub machines: u32,
}

/// Execution and machine usage for one recipe.
#[derive(Debug, Clone)]
pub struct RecipeUsage {
    pub recipe_index: RecipeId,
    /// Integer machine count assigned to the recipe.
    pub machines: u32,
    /// Recipe runs per minute.
    pub executions_per_min: f64,
}

/// Thermal bank deployment for one power recipe.
#[derive(Debug, Clone)]
pub struct ThermalBankUsage {
    pub power_recipe_index: PowerRecipeId,
    /// Fuel item consumed by the thermal bank.
    pub ingredient: ItemId,
    /// Number of thermal banks.
    pub banks: u32,
    /// Per-bank power output.
    pub power_w: u32,
    /// Per-bank cycle duration in seconds.
    pub duration_s: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SupplyNodeId(u32);

impl SupplyNodeId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DemandNodeId(u32);

impl DemandNodeId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LogisticsNodeId(u32);

impl LogisticsNodeId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MachineOrdinal(u32);

impl MachineOrdinal {
    pub(crate) fn from_1_based(value: u32) -> Self {
        Self(value)
    }

    pub fn new(value: u32) -> Option<Self> {
        (value > 0).then_some(Self(value))
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupplySite {
    ExternalSupply {
        item: ItemId,
    },
    RecipeOutput {
        recipe_index: RecipeId,
        machine: MachineOrdinal,
        item: ItemId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemandSite {
    RecipeInput {
        recipe_index: RecipeId,
        machine: MachineOrdinal,
        item: ItemId,
    },
    OutpostSale {
        outpost_index: OutpostId,
        item: ItemId,
    },
    ThermalBankFuel {
        power_recipe_index: PowerRecipeId,
        bank: MachineOrdinal,
        item: ItemId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SupplyNode {
    pub id: SupplyNodeId,
    pub site: SupplySite,
    pub capacity_per_min: PosF64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DemandNode {
    pub id: DemandNodeId,
    pub site: DemandSite,
    pub demand_per_min: PosF64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemSubproblem {
    pub item: ItemId,
    pub supplies: Vec<SupplyNode>,
    pub demands: Vec<DemandNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogisticsNodeSite {
    ExternalSupply {
        item: ItemId,
    },
    RecipeMachine {
        recipe_index: RecipeId,
        machine: MachineOrdinal,
    },
    OutpostSale {
        outpost_index: OutpostId,
        item: ItemId,
    },
    ThermalBankFuel {
        power_recipe_index: PowerRecipeId,
        bank: MachineOrdinal,
        item: ItemId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogisticsNode {
    pub id: LogisticsNodeId,
    pub site: LogisticsNodeSite,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFlowEdge {
    pub item: ItemId,
    pub from: SupplyNodeId,
    pub to: DemandNodeId,
    pub flow_per_min: PosF64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFlowPlan {
    pub item: ItemId,
    pub edges: Vec<ItemFlowEdge>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogisticsEdge {
    pub item: ItemId,
    pub from: LogisticsNodeId,
    pub to: LogisticsNodeId,
    pub flow_per_min: PosF64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LogisticsPlan {
    pub nodes: Vec<LogisticsNode>,
    pub edges: Vec<LogisticsEdge>,
}

/// Remaining slack for each externally supplied item.
#[derive(Debug, Clone)]
pub struct ExternalSupplySlack {
    /// Item id from `Catalog.items`.
    pub item: ItemId,
    /// Remaining quantity in the item balance constraint.
    pub slack_per_min: f64,
    /// Input external supply configured for this item.
    pub supply_per_min: f64,
}

/// Result of one optimization stage.
#[derive(Debug, Clone)]
pub struct StageSolution {
    /// Core generation capacity in watts.
    pub p_core_w: u32,
    /// External power consumption in watts.
    pub p_ext_w: u32,
    /// Total revenue objective value.
    pub revenue_per_min: f64,
    /// Sum of all production machines.
    pub total_machines: u32,
    /// Sum of all thermal banks.
    pub total_thermal_banks: u32,
    /// Total power generation.
    pub power_gen_w: i64,
    /// Total power usage.
    pub power_use_w: i64,
    /// `power_gen_w - power_use_w`.
    pub power_margin_w: i64,
    /// Per-outpost value realization.
    pub outpost_values: Vec<OutpostValue>,
    /// Top sales by value contribution.
    pub top_sales: Vec<SaleValue>,
    /// Full sale lines with quantities, used to reconstruct logistics demands.
    pub outpost_sales_qty: Vec<OutpostSaleQty>,
    /// Machine counts by facility.
    pub machines_by_facility: Vec<FacilityMachineCount>,
    /// Top recipes by machine count.
    pub recipes_used: Vec<RecipeUsage>,
    /// Thermal bank allocations.
    pub thermal_banks_used: Vec<ThermalBankUsage>,
    /// Slack information for externally supplied items.
    pub external_supply_slack: Vec<ExternalSupplySlack>,
}

/// Combined output for stage 1 and stage 2.
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Stage 1: max revenue.
    pub stage1: StageSolution,
    /// Stage 2: min machine counts with revenue floor.
    pub stage2: StageSolution,
    /// Machine-granularity logistics flow plan derived from stage 2.
    pub logistics: LogisticsPlan,
}
