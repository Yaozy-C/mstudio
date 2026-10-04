//! DSH-style host-owned driver: event-derived session, provider adapter, tool pipeline.
//! See docs/architecture.md for the Rust integration boundary.
mod binding;
mod budget;
#[cfg(test)]
mod content_storage_tests;
pub(crate) mod context_boundary;
pub(crate) mod context_source;
pub mod delegation;
mod driver;
mod generation_tool;
#[cfg(test)]
mod generation_tool_tests;
pub(crate) mod handoff;
mod image_read;
mod mailbox;
mod metering;
mod model;
mod operation_tools;
mod outcomes;
mod registry;
mod scheduler;
pub(crate) mod schema;
mod schema_definition;
pub mod session;
mod session_events;
mod session_recovery;
pub(crate) mod session_selection;
mod stored_image;
#[cfg(test)]
mod tests;
mod tool_output;

use rig_core::{
    completion::ToolDefinition,
    message::{Message, ToolCall},
};
use serde_json::Value;
use std::future::Future;
use tokio_util::sync::CancellationToken;

pub trait Host: Sync {
    fn deadline(&self) -> Option<tokio::time::Instant> {
        None
    }
    fn token(&self) -> &CancellationToken;
    fn result_turn(&self) -> Option<&str> {
        None
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String>;
    fn definitions(&self) -> Vec<ToolDefinition>;
    fn parallel_safe(&self, call: &ToolCall) -> bool;
    /// Returns messages already committed to the session journal with delivery acknowledgements.
    fn injected(&self) -> Result<Vec<Message>, String> {
        Ok(vec![])
    }
    fn execute(&self, call: &ToolCall) -> impl Future<Output = Value> + Send;
}
pub use budget::compact_manual;
pub(crate) use budget::tokens as estimate_message;
pub use driver::run;
pub use registry::ProjectHost;
mod progress;
