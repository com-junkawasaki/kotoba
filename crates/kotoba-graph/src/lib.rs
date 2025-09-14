//! kotoba-graph - Kotoba Graph Components

pub mod graph;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::graph::*;
}

#[cfg(test)]
mod tests {
    // Tests will be added here
}
