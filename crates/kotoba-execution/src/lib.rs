//! kotoba-execution - Kotoba Execution Components

pub mod execution;
pub mod planner;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::execution::*;
    pub use crate::planner::*;
}

#[cfg(test)]
mod tests {
    // Tests will be added here
}
