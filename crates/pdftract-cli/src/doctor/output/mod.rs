//! Output formatting for doctor subcommand

mod human;
mod json;
mod features;

pub use human::{output_text, TextOptions};
pub use json::output_json;
pub use features::output_features;
