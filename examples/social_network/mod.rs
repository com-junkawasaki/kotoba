//! Social Networkユースケースの実装
//!
//! このモジュールは、Kotobaを使用したソーシャルネットワークアプリケーションの
//! 典型的なユースケースを実装します。

pub mod data_generator;
pub mod queries;
pub mod analysis;

pub use data_generator::*;
pub use queries::*;
pub use analysis::*;
