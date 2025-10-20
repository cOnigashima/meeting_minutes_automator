// Task 10.4: Device Disconnect/Reconnect E2E Test
// Requirement: STT-REQ-004.9, STT-REQ-004.10, STT-REQ-004.11

use meeting_minutes_automator_lib::audio::FakeAudioDevice;
use meeting_minutes_automator_lib::audio_device_adapter::{AudioDeviceAdapter, AudioDeviceEvent};
use std::sync::mpsc;
use std::time::Duration;

/// Test Scenario 1: Device disconnect detection (STT-REQ-004.9)
/// GIVEN: FakeAudioDevice is recording
/// WHEN: simulate_disconnect() is called
/// THEN: DeviceGone event is sent and recording stops
#[test]
fn test_device_disconnect_detection() {
    // Arrange
    let mut device = FakeAudioDevice::new();
    let (event_tx, event_rx) = mpsc::channel();
    device.set_event_sender(event_tx);

    // Start recording with a dummy device ID
    device
        .start_recording_with_callback("fake-device-0", Box::new(|_data| {}))
        .expect("Failed to start recording");
    assert!(device.is_recording(), "Device should be recording");

    // Act: Simulate disconnect
    device
        .simulate_disconnect()
        .expect("Failed to simulate disconnect");

    // Assert
    assert!(!device.is_recording(), "Device should stop recording");

    // Verify DeviceGone event was sent
    let event = event_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("Should receive DeviceGone event");

    match event {
        AudioDeviceEvent::DeviceGone { device_id } => {
            assert_eq!(device_id, "fake-device-0", "Device ID should match");
        }
        _ => panic!("Expected DeviceGone event, got {:?}", event),
    }
}

/// Test Scenario 2: User notification on disconnect (STT-REQ-004.10)
/// GIVEN: Device disconnect is detected
/// WHEN: DeviceGone event is received
/// THEN: Event contains correct device_id for user notification
#[test]
fn test_user_notification_on_disconnect() {
    // Arrange
    let mut device = FakeAudioDevice::new();
    let (event_tx, event_rx) = mpsc::channel();
    device.set_event_sender(event_tx);

    let test_device_id = "test-microphone-123";
    device
        .start_recording_with_callback(test_device_id, Box::new(|_data| {}))
        .expect("Failed to start recording");

    // Act
    device
        .simulate_disconnect()
        .expect("Failed to simulate disconnect");

    // Assert: Verify event contains device_id for notification
    let event = event_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("Should receive event");

    if let AudioDeviceEvent::DeviceGone { device_id } = event {
        assert_eq!(
            device_id, test_device_id,
            "Event should contain device_id for user notification"
        );
    } else {
        panic!("Expected DeviceGone event");
    }
}

/// Test Scenario 3: Automatic reconnection (STT-REQ-004.11)
/// GIVEN: Device disconnect was detected
/// WHEN: simulate_reconnect() is called (simulates successful reconnect)
/// THEN: Recording resumes
/// NOTE: Full auto-reconnect logic (max 3 attempts, 5s intervals) will be tested
/// in integration tests with the full recorder state machine.
#[test]
fn test_automatic_reconnection_simulation() {
    // Arrange
    let mut device = FakeAudioDevice::new();
    let (event_tx, _event_rx) = mpsc::channel();
    device.set_event_sender(event_tx);

    device
        .start_recording_with_callback("fake-device-0", Box::new(|_data| {}))
        .expect("Failed to start recording");

    // Simulate disconnect
    device
        .simulate_disconnect()
        .expect("Failed to simulate disconnect");
    assert!(!device.is_recording(), "Device should be stopped");

    // Act: Simulate successful reconnect
    device
        .simulate_reconnect()
        .expect("Failed to simulate reconnect");

    // Assert
    assert!(
        device.is_recording(),
        "Device should resume recording after reconnect"
    );
}

/// Test Scenario 4: Multiple disconnect/reconnect cycles
/// GIVEN: Device is recording
/// WHEN: Multiple disconnect/reconnect cycles occur
/// THEN: Device handles all cycles correctly
#[test]
fn test_multiple_disconnect_reconnect_cycles() {
    // Arrange
    let mut device = FakeAudioDevice::new();
    let (event_tx, event_rx) = mpsc::channel();
    device.set_event_sender(event_tx);

    device
        .start_recording_with_callback("fake-device-0", Box::new(|_data| {}))
        .expect("Failed to start recording");

    // Act & Assert: 3 disconnect/reconnect cycles
    for _cycle in 0..3 {
        // Disconnect
        device
            .simulate_disconnect()
            .expect("Failed to simulate disconnect");
        assert!(!device.is_recording(), "Device should stop");

        // Verify event
        let event = event_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("Should receive DeviceGone event");
        assert!(
            matches!(event, AudioDeviceEvent::DeviceGone { .. }),
            "Should receive DeviceGone event"
        );

        // Reconnect
        device
            .simulate_reconnect()
            .expect("Failed to simulate reconnect");
        assert!(device.is_recording(), "Device should resume recording");
    }
}

/// Test Scenario 5: Stream error event (non-fatal error)
/// GIVEN: Device is recording
/// WHEN: simulate_stream_error() is called
/// THEN: StreamError event is sent but recording continues
#[test]
fn test_stream_error_non_fatal() {
    // Arrange
    let mut device = FakeAudioDevice::new();
    let (event_tx, event_rx) = mpsc::channel();
    device.set_event_sender(event_tx);

    device
        .start_recording_with_callback("fake-device-0", Box::new(|_data| {}))
        .expect("Failed to start recording");

    // Act
    device
        .simulate_stream_error("Buffer underrun")
        .expect("Failed to simulate stream error");

    // Assert
    assert!(
        device.is_recording(),
        "Device should continue recording after non-fatal error"
    );

    let event = event_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("Should receive StreamError event");

    match event {
        AudioDeviceEvent::StreamError(msg) => {
            assert_eq!(msg, "Buffer underrun");
        }
        _ => panic!("Expected StreamError event, got {:?}", event),
    }
}

/// Test Scenario 6: No event sender configured
/// GIVEN: FakeAudioDevice without event sender
/// WHEN: simulate_disconnect() is called
/// THEN: Recording stops but no event is sent (no panic)
#[test]
fn test_disconnect_without_event_sender() {
    // Arrange
    let mut device = FakeAudioDevice::new();
    // Note: No set_event_sender() call

    device
        .start_recording_with_callback("fake-device-0", Box::new(|_data| {}))
        .expect("Failed to start recording");

    // Act
    let result = device.simulate_disconnect();

    // Assert: Should not panic
    assert!(result.is_ok(), "Should handle disconnect without panic");
    assert!(!device.is_recording(), "Device should stop recording");
}
