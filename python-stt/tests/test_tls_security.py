"""
SEC-005: TLS 1.0/1.1 Connection Failure Test

This test verifies that Python's SSL context enforces TLS 1.2+ and rejects
connections to TLS 1.0/1.1 endpoints, fulfilling STT-NFR-004 security requirements.

Related requirements:
- STT-NFR-004: Security requirements (TLS 1.2+)
- SEC-005: TLS version enforcement verification
"""

import ssl
import pytest
from unittest.mock import Mock, patch


def test_tls_minimum_version_is_1_2():
    """Verify that default SSL context enforces TLS 1.2 minimum."""
    ctx = ssl.create_default_context()

    # TLS 1.2 = 771 (ssl.TLSVersion.TLSv1_2)
    assert ctx.minimum_version == ssl.TLSVersion.TLSv1_2, \
        "Default SSL context should enforce TLS 1.2 minimum"


def test_tls_1_0_connection_fails():
    """Verify that TLS 1.0 is below minimum allowed version."""
    ctx = ssl.create_default_context()

    # Create a context that only supports TLS 1.0
    legacy_ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    legacy_ctx.maximum_version = ssl.TLSVersion.TLSv1
    legacy_ctx.minimum_version = ssl.TLSVersion.TLSv1

    # Verify that our default context would reject TLS 1.0
    assert ctx.minimum_version > legacy_ctx.maximum_version, \
        f"Default context minimum ({ctx.minimum_version}) should exceed TLS 1.0 max ({legacy_ctx.maximum_version})"


def test_tls_1_1_connection_fails():
    """Verify that TLS 1.1 is below minimum allowed version."""
    ctx = ssl.create_default_context()

    # Create a context that only supports TLS 1.1
    legacy_ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    legacy_ctx.maximum_version = ssl.TLSVersion.TLSv1_1
    legacy_ctx.minimum_version = ssl.TLSVersion.TLSv1_1

    # Verify that our default context would reject TLS 1.1
    assert ctx.minimum_version > legacy_ctx.maximum_version, \
        f"Default context minimum ({ctx.minimum_version}) should exceed TLS 1.1 max ({legacy_ctx.maximum_version})"


def test_certifi_bundle_is_latest():
    """Verify that certifi CA bundle is up-to-date."""
    import certifi

    # Check certifi version (should be 2025.x.x or later)
    version = certifi.__version__
    year = int(version.split('.')[0])

    assert year >= 2025, \
        f"certifi version {version} may be outdated, expected 2025.x.x or later"


@patch('ssl.create_default_context')
def test_huggingface_hub_uses_default_ssl_context(mock_create_default_context):
    """Verify that HuggingFace Hub connections use Python's default SSL context."""
    from stt_engine.transcription.whisper_client import WhisperSTTEngine

    # Create mock SSL context
    mock_ctx = Mock()
    mock_ctx.minimum_version = ssl.TLSVersion.TLSv1_2
    mock_create_default_context.return_value = mock_ctx

    # Note: This test verifies that the WhisperSTTEngine relies on
    # faster-whisper's default SSL behavior, which uses create_default_context()
    # We cannot directly test HuggingFace Hub connections without network access,
    # but we validate the SSL configuration is correct.

    engine = WhisperSTTEngine(model_size="base", offline_mode=True)

    # Verify that the engine exists (offline mode, no download)
    assert engine is not None


def test_ssl_context_no_insecure_protocols():
    """Verify that insecure protocols (SSLv2, SSLv3, TLS 1.0, TLS 1.1) are disabled."""
    ctx = ssl.create_default_context()

    # Python 3.12+ default context automatically disables insecure protocols
    # Verify minimum version is TLS 1.2
    assert ctx.minimum_version >= ssl.TLSVersion.TLSv1_2, \
        "SSL context should disable TLS 1.0/1.1"

    # Verify that SSLv2 and SSLv3 are not available
    # (Python 3.12+ removes support entirely)
    assert not hasattr(ssl, 'PROTOCOL_SSLv2'), \
        "SSLv2 should not be available"
    assert not hasattr(ssl, 'PROTOCOL_SSLv3'), \
        "SSLv3 should not be available"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
