//! kotoba-rewrite - Kotoba Rewrite Components

pub mod rewrite;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::rewrite::*;
}

#[cfg(test)]
mod tests {
    // Tests will be added here
}
