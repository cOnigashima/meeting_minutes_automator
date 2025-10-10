// Audio Device Adapter Unit Tests
// MVP1 - Task 2.5: Device Disconnection Detection and Auto-Reconnect

use meeting_minutes_automator_lib::audio_device_adapter::{
    AudioDeviceEvent, AudioEventSender, AudioEventReceiver, AudioDeviceAdapter,
};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn test_audio_device_event_enum() {
    // Test event creation
    let stream_error = AudioDeviceEvent::StreamError("test error".to_string());
    let stalled = AudioDeviceEvent::Stalled { elapsed_ms: 1500 };
    let device_gone = AudioDeviceEvent::DeviceGone {
        device_id: "test-device".to_string(),
    };

    // Verify event types
    match stream_error {
        AudioDeviceEvent::StreamError(msg) => assert_eq!(msg, "test error"),
        _ => panic!("Wrong event type"),
    }

    match stalled {
        AudioDeviceEvent::Stalled { elapsed_ms } => assert_eq!(elapsed_ms, 1500),
        _ => panic!("Wrong event type"),
    }

    match device_gone {
        AudioDeviceEvent::DeviceGone { device_id } => assert_eq!(device_id, "test-device"),
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_event_channel_send_receive() {
    // Create channel
    let (tx, rx): (AudioEventSender, AudioEventReceiver) = mpsc::channel();

    // Send events
    tx.send(AudioDeviceEvent::StreamError("error1".to_string()))
        .expect("Failed to send StreamError");
    tx.send(AudioDeviceEvent::Stalled { elapsed_ms: 2000 })
        .expect("Failed to send Stalled");
    tx.send(AudioDeviceEvent::DeviceGone {
        device_id: "device1".to_string(),
    })
    .expect("Failed to send DeviceGone");

    // Receive and verify events
    let event1 = rx.recv_timeout(Duration::from_millis(100))
        .expect("Failed to receive event1");
    match event1 {
        AudioDeviceEvent::StreamError(msg) => assert_eq!(msg, "error1"),
        _ => panic!("Wrong event type"),
    }

    let event2 = rx.recv_timeout(Duration::from_millis(100))
        .expect("Failed to receive event2");
    match event2 {
        AudioDeviceEvent::Stalled { elapsed_ms } => assert_eq!(elapsed_ms, 2000),
        _ => panic!("Wrong event type"),
    }

    let event3 = rx.recv_timeout(Duration::from_millis(100))
        .expect("Failed to receive event3");
    match event3 {
        AudioDeviceEvent::DeviceGone { device_id } => assert_eq!(device_id, "device1"),
        _ => panic!("Wrong event type"),
    }

    // Verify channel is empty
    assert!(rx.recv_timeout(Duration::from_millis(10)).is_err());
}

#[test]
fn test_event_channel_clone() {
    // Test that sender can be cloned
    let (tx, rx): (AudioEventSender, AudioEventReceiver) = mpsc::channel();

    let tx_clone = tx.clone();

    // Send from original
    tx.send(AudioDeviceEvent::StreamError("from_original".to_string()))
        .expect("Failed to send from original");

    // Send from clone
    tx_clone
        .send(AudioDeviceEvent::StreamError("from_clone".to_string()))
        .expect("Failed to send from clone");

    // Receive both
    let event1 = rx.recv_timeout(Duration::from_millis(100))
        .expect("Failed to receive event1");
    let event2 = rx.recv_timeout(Duration::from_millis(100))
        .expect("Failed to receive event2");

    // Verify both were received (order not guaranteed)
    let messages: Vec<String> = vec![event1, event2]
        .into_iter()
        .map(|e| match e {
            AudioDeviceEvent::StreamError(msg) => msg,
            _ => panic!("Wrong event type"),
        })
        .collect();

    assert!(messages.contains(&"from_original".to_string()));
    assert!(messages.contains(&"from_clone".to_string()));
}

#[test]
#[cfg(target_os = "macos")]
fn test_core_audio_adapter_event_sender() {
    use meeting_minutes_automator_lib::audio_device_adapter::CoreAudioAdapter;

    let mut adapter = CoreAudioAdapter::new();
    let (tx, _rx) = mpsc::channel();

    // Test set_event_sender
    adapter.set_event_sender(tx);

    // Verify adapter was created successfully
    assert!(!adapter.is_recording());
}

#[test]
#[cfg(target_os = "macos")]
fn test_core_audio_adapter_permission_check() {
    use meeting_minutes_automator_lib::audio_device_adapter::CoreAudioAdapter;

    let adapter = CoreAudioAdapter::new();

    // Check permission - this should either succeed (if permission granted)
    // or fail with appropriate error message
    match adapter.check_permission() {
        Ok(_) => {
            // Permission granted
            println!("Microphone permission: granted");
        }
        Err(e) => {
            // Permission denied - verify error message
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("マイクアクセスが拒否されました") ||
                error_msg.contains("システム設定から許可してください"),
                "Expected permission error message, got: {}",
                error_msg
            );
        }
    }
}

#[test]
#[cfg(target_os = "windows")]
fn test_wasapi_adapter_event_sender() {
    use meeting_minutes_automator_lib::audio_device_adapter::WasapiAdapter;

    let mut adapter = WasapiAdapter::new();
    let (tx, _rx) = mpsc::channel();

    // Test set_event_sender
    adapter.set_event_sender(tx);

    // Verify adapter was created successfully
    assert!(!adapter.is_recording());
}

#[test]
#[cfg(target_os = "windows")]
fn test_wasapi_adapter_permission_check() {
    use meeting_minutes_automator_lib::audio_device_adapter::WasapiAdapter;

    let adapter = WasapiAdapter::new();

    match adapter.check_permission() {
        Ok(_) => {
            println!("Microphone permission: granted");
        }
        Err(e) => {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("マイクアクセスが拒否されました") ||
                error_msg.contains("システム設定から許可してください"),
                "Expected permission error message, got: {}",
                error_msg
            );
        }
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_alsa_adapter_event_sender() {
    use meeting_minutes_automator_lib::audio_device_adapter::AlsaAdapter;

    let mut adapter = AlsaAdapter::new();
    let (tx, _rx) = mpsc::channel();

    // Test set_event_sender
    adapter.set_event_sender(tx);

    // Verify adapter was created successfully
    assert!(!adapter.is_recording());
}

#[test]
#[cfg(target_os = "linux")]
fn test_alsa_adapter_permission_check() {
    use meeting_minutes_automator_lib::audio_device_adapter::AlsaAdapter;

    let adapter = AlsaAdapter::new();

    match adapter.check_permission() {
        Ok(_) => {
            println!("Microphone permission: granted");
        }
        Err(e) => {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("マイクアクセスが拒否されました") ||
                error_msg.contains("システム設定から許可してください"),
                "Expected permission error message, got: {}",
                error_msg
            );
        }
    }
}
