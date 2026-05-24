//! Output formatting for doctor subcommand

mod features;
mod human;
mod json;

pub use features::output_features;
pub use human::{output_text, TextOptions};
pub use json::output_json;
