//! Command parsing and normalization (Core-5).

pub mod parser;
pub mod types;

pub use parser::parse;
pub use types::ParsedCommand;
