// Structured Logging Module
// Walking Skeleton (MVP0) - JSON log output

use serde::Serialize;
use serde_json;
use std::time::SystemTime;

/// Log levels
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Structured log entry
#[derive(Debug, Serialize)]
pub struct LogEntry {
    level: LogLevel,
    component: String,
    event: String,
    message: Option<String>,
    timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl LogEntry {
    pub fn new(level: LogLevel, component: &str, event: &str) -> Self {
        Self {
            level,
            component: component.to_string(),
            event: event.to_string(),
            message: None,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            details: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn log(self) {
        match serde_json::to_string(&self) {
            Ok(json) => println!("{}", json),
            Err(_) => eprintln!("Failed to serialize log entry"),
        }
    }
}

/// Convenience macros for structured logging
#[macro_export]
macro_rules! log_info {
    ($component:expr, $event:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Info, $component, $event).log();
    };
    ($component:expr, $event:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Info, $component, $event)
            .with_message($msg)
            .log();
    };
}

#[macro_export]
macro_rules! log_info_details {
    ($component:expr, $event:expr, $details:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Info, $component, $event)
            .with_details($details)
            .log();
    };
    ($component:expr, $event:expr, $details:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Info, $component, $event)
            .with_message($msg)
            .with_details($details)
            .log();
    };
}

#[macro_export]
macro_rules! log_error {
    ($component:expr, $event:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Error, $component, $event)
            .with_message($msg)
            .log();
    };
}

#[macro_export]
macro_rules! log_error_details {
    ($component:expr, $event:expr, $details:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Error, $component, $event)
            .with_details($details)
            .log();
    };
    ($component:expr, $event:expr, $details:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Error, $component, $event)
            .with_message($msg)
            .with_details($details)
            .log();
    };
}

#[macro_export]
macro_rules! log_warn {
    ($component:expr, $event:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Warn, $component, $event)
            .with_message($msg)
            .log();
    };
}

#[macro_export]
macro_rules! log_warn_details {
    ($component:expr, $event:expr, $details:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Warn, $component, $event)
            .with_details($details)
            .log();
    };
    ($component:expr, $event:expr, $details:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Warn, $component, $event)
            .with_message($msg)
            .with_details($details)
            .log();
    };
}

#[macro_export]
macro_rules! log_debug {
    ($component:expr, $event:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Debug, $component, $event)
            .with_message($msg)
            .log();
    };
}

#[macro_export]
macro_rules! log_debug_details {
    ($component:expr, $event:expr, $details:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Debug, $component, $event)
            .with_details($details)
            .log();
    };
    ($component:expr, $event:expr, $details:expr, $msg:expr) => {
        $crate::logger::LogEntry::new($crate::logger::LogLevel::Debug, $component, $event)
            .with_message($msg)
            .with_details($details)
            .log();
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new(LogLevel::Info, "test_component", "test_event")
            .with_message("Test message");

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"component\":\"test_component\""));
        assert!(json.contains("\"event\":\"test_event\""));
        assert!(json.contains("\"message\":\"Test message\""));
    }

    #[test]
    fn test_log_entry_with_details() {
        let details = serde_json::json!({
            "key": "value",
            "count": 42
        });

        let entry = LogEntry::new(LogLevel::Error, "test", "error_occurred")
            .with_message("An error happened")
            .with_details(details);

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"key\":\"value\""));
        assert!(json.contains("\"count\":42"));
    }
}
