//! Two-stage optimization for production planning.
//!
//! Stage 1 maximizes total revenue.
//! Stage 2 minimizes machine counts under a near-optimal revenue floor.

mod error;
mod solver;
mod types;

pub use error::{Error, Result};
pub use solver::{NEAR_INT_EPS, STAGE2_REVENUE_FLOOR_REL_EPS, run_two_stage};
pub use types::{
    ExternalSupplySlack, FacilityMachineCount, OptimizationResult, OutpostIndex, OutpostValue,
    PowerRecipeIndex, RecipeIndex, RecipeUsage, SaleValue, SolveInputs, StageSolution,
    ThermalBankUsage,
};
