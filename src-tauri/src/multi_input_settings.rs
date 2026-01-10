//! Multi-Input Settings Persistence
//!
//! Saves and loads multi-input audio configuration to/from disk.
//!
//! Requirements: STTMIX-REQ-001.2, STTMIX-REQ-005.1
//! Design: meeting-minutes-stt-multi-input/design.md

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::multi_input_manager::InputRole;

// ============================================================================
// Degradation Policy
// ============================================================================

/// Policy for handling input device failures
///
/// Requirement: STTMIX-REQ-006
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DegradationPolicy {
    /// Continue recording with remaining inputs when one fails
    #[default]
    ContinueWithRemaining,
    /// Stop recording when any input fails
    StopOnAnyFailure,
}

// ============================================================================
// Settings Struct
// ============================================================================

/// Multi-input configuration settings
///
/// Persisted to `settings/multi_input.json` in app data directory.
///
/// Requirements: STTMIX-REQ-001.2, STTMIX-REQ-005.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiInputSettings {
    /// Selected device IDs (max 2)
    #[serde(default)]
    pub selected_device_ids: Vec<String>,

    /// Input role for each device (Microphone/Loopback)
    #[serde(default)]
    pub input_roles: HashMap<String, InputRole>,

    /// Gain value for each device (in dB, typically -6.0)
    #[serde(default)]
    pub gains: HashMap<String, f32>,

    /// Mute state for each device
    #[serde(default)]
    pub mute_states: HashMap<String, bool>,

    /// Multi-input mode enabled
    #[serde(default)]
    pub multi_input_enabled: bool,

    /// Degradation policy on input failure
    #[serde(default)]
    pub degradation_policy: DegradationPolicy,

    /// Settings version for future migrations
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl Default for MultiInputSettings {
    fn default() -> Self {
        Self {
            selected_device_ids: Vec::new(),
            input_roles: HashMap::new(),
            gains: HashMap::new(),
            mute_states: HashMap::new(),
            multi_input_enabled: false,
            degradation_policy: DegradationPolicy::default(),
            version: 1,
        }
    }
}

impl MultiInputSettings {
    /// Create new settings with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a device with role and default gain
    pub fn add_device(&mut self, device_id: &str, role: InputRole) {
        if !self.selected_device_ids.contains(&device_id.to_string()) {
            self.selected_device_ids.push(device_id.to_string());
        }
        self.input_roles.insert(device_id.to_string(), role);
        // Default gain: -6dB (0.5 linear)
        self.gains.entry(device_id.to_string()).or_insert(-6.0);
        self.mute_states.entry(device_id.to_string()).or_insert(false);
    }

    /// Remove a device
    pub fn remove_device(&mut self, device_id: &str) {
        self.selected_device_ids.retain(|id| id != device_id);
        self.input_roles.remove(device_id);
        self.gains.remove(device_id);
        self.mute_states.remove(device_id);
    }

    /// Set gain for a device (in dB)
    pub fn set_gain(&mut self, device_id: &str, gain_db: f32) {
        self.gains.insert(device_id.to_string(), gain_db);
    }

    /// Set mute state for a device
    pub fn set_mute(&mut self, device_id: &str, muted: bool) {
        self.mute_states.insert(device_id.to_string(), muted);
    }

    /// Get gain for a device (in dB), defaults to -6.0
    pub fn get_gain(&self, device_id: &str) -> f32 {
        self.gains.get(device_id).copied().unwrap_or(-6.0)
    }

    /// Get mute state for a device
    pub fn is_muted(&self, device_id: &str) -> bool {
        self.mute_states.get(device_id).copied().unwrap_or(false)
    }

    /// Check if settings are valid (max 2 devices)
    pub fn is_valid(&self) -> bool {
        self.selected_device_ids.len() <= 2
    }
}

// ============================================================================
// Persistence
// ============================================================================

const SETTINGS_FILENAME: &str = "multi_input.json";
const SETTINGS_SUBDIR: &str = "settings";

/// Get the settings file path
fn get_settings_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join(SETTINGS_SUBDIR).join(SETTINGS_FILENAME)
}

/// Save multi-input settings to disk
///
/// Creates the settings directory if it doesn't exist.
///
/// Requirement: STTMIX-REQ-001.2
pub fn save_settings(app_data_dir: &PathBuf, settings: &MultiInputSettings) -> Result<()> {
    let settings_dir = app_data_dir.join(SETTINGS_SUBDIR);

    // Create settings directory if needed
    if !settings_dir.exists() {
        std::fs::create_dir_all(&settings_dir)
            .with_context(|| format!("Failed to create settings directory: {:?}", settings_dir))?;
    }

    let settings_path = get_settings_path(app_data_dir);
    let json = serde_json::to_string_pretty(settings)
        .context("Failed to serialize multi-input settings")?;

    std::fs::write(&settings_path, json)
        .with_context(|| format!("Failed to write settings file: {:?}", settings_path))?;

    eprintln!("💾 Saved multi-input settings to {:?}", settings_path);
    Ok(())
}

/// Load multi-input settings from disk
///
/// Returns default settings if file doesn't exist.
///
/// Requirement: STTMIX-REQ-001.2
pub fn load_settings(app_data_dir: &PathBuf) -> Result<MultiInputSettings> {
    let settings_path = get_settings_path(app_data_dir);

    if !settings_path.exists() {
        eprintln!("📁 No multi-input settings found, using defaults");
        return Ok(MultiInputSettings::default());
    }

    let json = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("Failed to read settings file: {:?}", settings_path))?;

    let settings: MultiInputSettings =
        serde_json::from_str(&json).context("Failed to parse multi-input settings")?;

    eprintln!(
        "📁 Loaded multi-input settings: {} devices, enabled={}",
        settings.selected_device_ids.len(),
        settings.multi_input_enabled
    );

    Ok(settings)
}

/// Check if selected devices are still available
///
/// Returns list of device IDs that are no longer available.
///
/// Requirement: STTMIX-REQ-001.2
pub fn validate_devices(
    settings: &MultiInputSettings,
    available_device_ids: &[String],
) -> Vec<String> {
    settings
        .selected_device_ids
        .iter()
        .filter(|id| !available_device_ids.contains(id))
        .cloned()
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_settings_default() {
        let settings = MultiInputSettings::default();
        assert!(settings.selected_device_ids.is_empty());
        assert!(!settings.multi_input_enabled);
        assert_eq!(settings.degradation_policy, DegradationPolicy::ContinueWithRemaining);
        assert_eq!(settings.version, 1);
    }

    #[test]
    fn test_add_remove_device() {
        let mut settings = MultiInputSettings::new();

        settings.add_device("mic-1", InputRole::Microphone);
        assert_eq!(settings.selected_device_ids.len(), 1);
        assert_eq!(settings.input_roles.get("mic-1"), Some(&InputRole::Microphone));
        assert_eq!(settings.get_gain("mic-1"), -6.0);
        assert!(!settings.is_muted("mic-1"));

        settings.add_device("loopback-1", InputRole::Loopback);
        assert_eq!(settings.selected_device_ids.len(), 2);

        settings.remove_device("mic-1");
        assert_eq!(settings.selected_device_ids.len(), 1);
        assert!(settings.input_roles.get("mic-1").is_none());
    }

    #[test]
    fn test_gain_and_mute() {
        let mut settings = MultiInputSettings::new();
        settings.add_device("mic-1", InputRole::Microphone);

        settings.set_gain("mic-1", -12.0);
        assert_eq!(settings.get_gain("mic-1"), -12.0);

        settings.set_mute("mic-1", true);
        assert!(settings.is_muted("mic-1"));
    }

    #[test]
    fn test_is_valid() {
        let mut settings = MultiInputSettings::new();
        assert!(settings.is_valid());

        settings.add_device("mic-1", InputRole::Microphone);
        settings.add_device("loopback-1", InputRole::Loopback);
        assert!(settings.is_valid());

        // Force add third device (bypassing add_device logic)
        settings.selected_device_ids.push("mic-2".to_string());
        assert!(!settings.is_valid());
    }

    #[test]
    fn test_save_load_settings() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        let mut settings = MultiInputSettings::new();
        settings.add_device("mic-1", InputRole::Microphone);
        settings.add_device("loopback-1", InputRole::Loopback);
        settings.set_gain("mic-1", -3.0);
        settings.set_mute("loopback-1", true);
        settings.multi_input_enabled = true;
        settings.degradation_policy = DegradationPolicy::StopOnAnyFailure;

        // Save
        save_settings(&app_data_dir, &settings).unwrap();

        // Load
        let loaded = load_settings(&app_data_dir).unwrap();
        assert_eq!(loaded.selected_device_ids.len(), 2);
        assert!(loaded.selected_device_ids.contains(&"mic-1".to_string()));
        assert!(loaded.selected_device_ids.contains(&"loopback-1".to_string()));
        assert_eq!(loaded.get_gain("mic-1"), -3.0);
        assert!(loaded.is_muted("loopback-1"));
        assert!(loaded.multi_input_enabled);
        assert_eq!(loaded.degradation_policy, DegradationPolicy::StopOnAnyFailure);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        let settings = load_settings(&app_data_dir).unwrap();
        assert!(settings.selected_device_ids.is_empty());
    }

    #[test]
    fn test_validate_devices() {
        let mut settings = MultiInputSettings::new();
        settings.add_device("mic-1", InputRole::Microphone);
        settings.add_device("loopback-1", InputRole::Loopback);

        let available = vec!["mic-1".to_string(), "other-device".to_string()];
        let unavailable = validate_devices(&settings, &available);

        assert_eq!(unavailable.len(), 1);
        assert!(unavailable.contains(&"loopback-1".to_string()));
    }

    #[test]
    fn test_serialization_format() {
        let mut settings = MultiInputSettings::new();
        settings.add_device("mic-1", InputRole::Microphone);
        settings.degradation_policy = DegradationPolicy::StopOnAnyFailure;

        let json = serde_json::to_string_pretty(&settings).unwrap();

        // Check snake_case serialization
        assert!(json.contains("\"stop_on_any_failure\""));
        assert!(json.contains("\"Microphone\""));
    }
}
