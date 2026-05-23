pub mod classifier;
pub mod resolver;
pub mod rules;

pub use classifier::{Component, ComponentKind, classify};
pub use resolver::Resolver;
pub use rules::RuleConfig;
