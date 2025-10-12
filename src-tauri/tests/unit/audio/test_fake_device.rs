// Unit Tests for FakeAudioDevice
// TDD Red Phase: Tests that will drive the implementation

use meeting_minutes_automator_lib::audio::{AudioDevice, FakeAudioDevice};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn ut_2_1_1_fake_audio_device_initialization() {
    // Test that FakeAudioDevice can be initialized successfully
    let mut device = FakeAudioDevice::new();

    let result = device.initialize();
    assert!(
        result.is_ok(),
        "FakeAudioDevice initialization should succeed"
    );
}

#[tokio::test]
async fn ut_2_1_2_fake_audio_device_generates_dummy_data() {
    // Test that FakeAudioDevice generates 16-byte dummy data
    let mut device = FakeAudioDevice::new();
    device.initialize().expect("Initialization should succeed");

    // Start the device
    device.start().expect("Start should succeed");

    // Wait a bit to allow data generation (this test will be refined in GREEN phase)
    sleep(Duration::from_millis(150)).await;

    // For now, just verify that start() succeeds
    // Actual data verification will be added when we implement the callback mechanism

    device.stop().expect("Stop should succeed");
}

#[tokio::test]
async fn ut_2_1_3_fake_audio_device_timer_interval() {
    // Test that FakeAudioDevice generates data at 100ms intervals
    let mut device = FakeAudioDevice::new();
    device.initialize().expect("Initialization should succeed");

    device.start().expect("Start should succeed");

    // Wait for 250ms to allow at least 2 data chunks
    sleep(Duration::from_millis(250)).await;

    // This test verifies timing - implementation will use tokio::time::interval(100ms)
    // Actual interval verification will be refined in GREEN phase

    device.stop().expect("Stop should succeed");
}

#[tokio::test]
async fn ut_2_1_4_fake_audio_device_stop() {
    // Test that FakeAudioDevice stops data generation correctly
    let mut device = FakeAudioDevice::new();
    device.initialize().expect("Initialization should succeed");

    device.start().expect("Start should succeed");

    // Stop the device
    let result = device.stop();
    assert!(result.is_ok(), "Stop should succeed");

    // After stopping, start should be callable again
    let result = device.start();
    assert!(result.is_ok(), "Start should succeed after stop");

    device.stop().expect("Final stop should succeed");
}

#[test]
fn ut_2_1_5_fake_audio_device_generates_16_byte_data() {
    // Test that generate_dummy_data() returns exactly 16 bytes
    let device = FakeAudioDevice::new();

    let data = device.generate_dummy_data();
    assert_eq!(data.len(), 16, "Dummy data should be exactly 16 bytes");
}
