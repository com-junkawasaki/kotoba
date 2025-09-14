//! kotoba-core - Kotoba Core Components

pub mod types;
pub mod ir;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::types::*;
    pub use crate::ir::*;
}

#[cfg(test)]
mod tests {
    // Tests will be added here
}
