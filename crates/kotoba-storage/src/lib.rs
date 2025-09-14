//! kotoba-storage - Kotoba Storage Components

pub mod storage;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::storage::*;
}

#[cfg(test)]
mod tests {
    // Tests will be added here
}
