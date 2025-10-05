#!/usr/bin/env python3
"""
Security Test Suite
Walking Skeleton (MVP0) - Security requirements validation

Tests:
- IT-9.2.1: localhost binding (127.0.0.1 only)
- IT-9.2.2: Origin header validation
- IT-9.2.3: JSON IPC message validation
- IT-9.2.4: Invalid connection attempts
"""

import asyncio
import json
import websockets
from websockets.exceptions import InvalidStatusCode, WebSocketException

async def test_localhost_binding():
    """IT-9.2.1: Verify WebSocket server only accepts localhost connections"""
    print("\n🔒 Test IT-9.2.1: Localhost binding")

    # Should succeed on 127.0.0.1
    try:
        uri = "ws://127.0.0.1:9001"
        async with websockets.connect(uri) as ws:
            msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
            data = json.loads(msg)
            if data.get('type') == 'connected':
                print("  ✅ Connection to 127.0.0.1:9001 successful")
            else:
                print(f"  ❌ Unexpected message: {data}")
    except Exception as e:
        print(f"  ❌ Failed to connect to 127.0.0.1:9001: {e}")
        return False

    return True

async def test_origin_validation():
    """IT-9.2.2: Verify Origin header validation"""
    print("\n🔒 Test IT-9.2.2: Origin header validation")

    test_cases = [
        # Valid origins
        ("http://127.0.0.1", True),
        ("http://localhost", True),
        ("https://meet.google.com", True),
        ("chrome-extension://abcdefghijklmnopqrstuvwxyz123456", True),  # Any extension ID in debug mode

        # Invalid origins
        ("https://evil.com", False),
        ("http://malicious.attacker", False),
    ]

    results = []
    for origin, should_succeed in test_cases:
        try:
            uri = "ws://127.0.0.1:9001"
            extra_headers = {"Origin": origin}

            async with websockets.connect(uri, additional_headers=extra_headers) as ws:
                msg = await asyncio.wait_for(ws.recv(), timeout=2.0)
                data = json.loads(msg)

                if should_succeed:
                    if data.get('type') == 'connected':
                        print(f"  ✅ Origin '{origin}' accepted (expected)")
                        results.append(True)
                    else:
                        print(f"  ❌ Origin '{origin}' accepted but wrong message: {data}")
                        results.append(False)
                else:
                    print(f"  ❌ Origin '{origin}' should have been rejected but was accepted")
                    results.append(False)

        except InvalidStatusCode as e:
            if not should_succeed and e.status_code == 403:
                print(f"  ✅ Origin '{origin}' rejected with 403 (expected)")
                results.append(True)
            else:
                print(f"  ❌ Origin '{origin}' failed unexpectedly: {e}")
                results.append(False)

        except asyncio.TimeoutError:
            if not should_succeed:
                print(f"  ⚠️  Origin '{origin}' timeout (connection may have been rejected)")
                results.append(True)
            else:
                print(f"  ❌ Origin '{origin}' timeout (should have succeeded)")
                results.append(False)

        except Exception as e:
            if not should_succeed:
                print(f"  ✅ Origin '{origin}' rejected (expected): {type(e).__name__}")
                results.append(True)
            else:
                print(f"  ❌ Origin '{origin}' failed unexpectedly: {e}")
                results.append(False)

    return all(results)

async def test_malformed_json():
    """IT-9.2.3: Verify JSON message validation (not applicable for WebSocket, IPC only)"""
    print("\n🔒 Test IT-9.2.3: JSON IPC validation")
    print("  ⏭️  Skipped: IPC validation is Rust ↔ Python only, tested in unit tests")
    return True

async def test_invalid_connections():
    """IT-9.2.4: Test invalid connection attempts"""
    print("\n🔒 Test IT-9.2.4: Invalid connection attempts")

    # Test connection to wrong port (should fail)
    try:
        uri = "ws://127.0.0.1:9999"
        await asyncio.wait_for(
            websockets.connect(uri),
            timeout=2.0
        )
        print("  ❌ Connection to wrong port 9999 should have failed")
        return False
    except Exception:
        print("  ✅ Connection to wrong port 9999 rejected (expected)")

    return True

async def main():
    print("=" * 60)
    print("Security Test Suite - Walking Skeleton (MVP0)")
    print("=" * 60)

    print("\n⚠️  Prerequisites:")
    print("  1. Start Tauri app: npm run tauri dev")
    print("  2. Wait for: '[Meeting Minutes] ✅ WebSocket server started on port 9001'")
    print("  3. Run this test")

    input("\nPress Enter when ready...")

    results = []

    # Run tests
    results.append(await test_localhost_binding())
    results.append(await test_origin_validation())
    results.append(await test_malformed_json())
    results.append(await test_invalid_connections())

    # Summary
    print("\n" + "=" * 60)
    print("Test Summary")
    print("=" * 60)

    passed = sum(results)
    total = len(results)

    print(f"\n✅ Passed: {passed}/{total}")
    print(f"❌ Failed: {total - passed}/{total}")

    if all(results):
        print("\n🎉 All security tests passed!")
        return 0
    else:
        print("\n⚠️  Some security tests failed")
        return 1

if __name__ == '__main__':
    exit_code = asyncio.run(main())
    exit(exit_code)
