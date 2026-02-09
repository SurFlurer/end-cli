use end_model::{AicInputs, FacilityId, ItemId};

/// Index into `AicInputs.outposts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutpostIndex(u32);

impl OutpostIndex {
    /// Returns the underlying index value.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the index as `usize` for direct slice indexing.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_usize(index: usize) -> Option<Self> {
        u32::try_from(index).ok().map(Self)
    }
}

/// Index into `Catalog.recipes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecipeIndex(u32);

impl RecipeIndex {
    /// Returns the underlying index value.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the index as `usize` for direct slice indexing.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_usize(index: usize) -> Option<Self> {
        u32::try_from(index).ok().map(Self)
    }
}

/// Index into `Catalog.power_recipes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PowerRecipeIndex(u32);

impl PowerRecipeIndex {
    /// Returns the underlying index value.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the index as `usize` for direct slice indexing.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_usize(index: usize) -> Option<Self> {
        u32::try_from(index).ok().map(Self)
    }
}

/// Input bundle for optimization.
#[derive(Debug, Clone)]
pub struct SolveInputs {
    /// Core/base generation capacity in watts.
    pub p_core_w: u32,
    /// Scenario inputs (external power, supply, outposts).
    pub aic: AicInputs,
}

/// Value utilization of one outpost.
#[derive(Debug, Clone)]
pub struct OutpostValue {
    pub outpost_index: OutpostIndex,
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
    pub outpost_index: OutpostIndex,
    /// Item being sold.
    pub item: ItemId,
    /// Revenue contribution.
    pub value_per_min: f64,
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
    pub recipe_index: RecipeIndex,
    /// Integer machine count assigned to the recipe.
    pub machines: u32,
    /// Recipe runs per minute.
    pub executions_per_min: f64,
}

/// Thermal bank deployment for one power recipe.
#[derive(Debug, Clone)]
pub struct ThermalBankUsage {
    pub power_recipe_index: PowerRecipeIndex,
    /// Fuel item consumed by the thermal bank.
    pub ingredient: ItemId,
    /// Number of thermal banks.
    pub banks: u32,
    /// Per-bank power output.
    pub power_w: u32,
    /// Per-bank cycle duration in seconds.
    pub duration_s: u32,
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
    /// Core/base generation capacity in watts.
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
    /// Top 10 sales by value contribution.
    pub top_sales: Vec<SaleValue>,
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
}
