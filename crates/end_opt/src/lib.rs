//! Two-stage optimization for production planning.
//!
//! Stage 1 maximizes total revenue.
//! Stage 2 minimizes machine counts under a near-optimal revenue floor.

mod error;
mod logistics;
mod solver;
mod types;

pub use error::{Error, Result};
pub use logistics::{
    LOGISTICS_EPS, build_item_subproblems, build_logistics_plan, expand_recipe_machine_rates,
    solve_item_best_fit,
};
pub use solver::{NEAR_INT_EPS, run_two_stage};
pub use types::{
    DemandNode, DemandNodeId, DemandSite, ExternalSupplySlack, FacilityMachineCount, ItemFlowEdge,
    ItemFlowPlan, ItemSubproblem, LogisticsEdge, LogisticsNode, LogisticsNodeId,
    LogisticsNodeSite, LogisticsPlan, MachineOrdinal, OptimizationResult, OutpostSaleQty,
    OutpostValue, PosF64, RecipeUsage, SaleValue, SolveInputs, StageSolution, SupplyNode,
    SupplyNodeId, SupplySite, ThermalBankUsage,
};
