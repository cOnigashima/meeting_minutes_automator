// Test Support Module
// Shared helpers for integration tests

pub mod legacy_ipc;

// Re-export for convenience (use crate::support::LegacyIpcMessage;)
pub use legacy_ipc::LegacyIpcMessage;
