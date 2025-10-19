# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x (MVP1)   | :white_check_mark: |

## Reporting a Vulnerability

Please report security vulnerabilities by creating a GitHub Issue with the `security` label.

## Known Security Issues

### SEC-001: pip 25.0 Vulnerability (GHSA-4xh5-x5gv-qwph)

**Status**: Accepted Risk (Temporary)
**Severity**: Medium (CVSS v4: 5.9, CVSS v3: 6.5)
**Current Version**: pip 25.2
**Fix Available**: pip 25.3 (Expected: 2025-Q1)

**Description**:
pip's fallback tar extraction doesn't check symbolic links point to extraction directory. A malicious sdist can overwrite files outside the target directory.

**Mitigation**:
- Upgraded to pip 25.2 (partial mitigation)
- Only install packages from trusted sources (HuggingFace Hub, PyPI official packages)
- Monitor pip 25.3 release for complete fix

**Follow-up**: Track pip 25.3 release, upgrade immediately upon availability

### SEC-004: cargo-audit Not Executed (Rust 1.85 Required)

**Status**: Blocked (Technical Constraint)
**Severity**: N/A (No Known Vulnerabilities)
**Current Rust Version**: 1.83.0
**Required**: Rust 1.85+ (Edition 2024 support)

**Description**:
cargo-audit v0.21.2 requires Rust 1.85 with Edition 2024 support. Current Rust 1.83 cannot compile cargo-audit.

**Mitigation**:
- Manual dependency review conducted
- No known critical vulnerabilities in current dependencies
- `cargo tree -d` checked for duplicate/suspicious crates

**Follow-up**: Execute cargo-audit immediately after Rust 1.85 release

## Content Security Policy (CSP)

**Directive Justification**:

- **`script-src 'self' 'wasm-unsafe-eval'`**:
  - **Required for**: Tauri + Rust/WASM compilation
  - **Justification**: Tauri applications compile Rust to WebAssembly, which requires `'wasm-unsafe-eval'` for dynamic module loading
  - **Unavoidable**: This is a Tauri framework requirement, not application-specific
  - **Mitigation**: All WASM modules are bundled with the application, not loaded from external sources

- **`style-src 'self' 'unsafe-inline'`**:
  - **Required for**: Vite/React inline styles
  - **Justification**: Vite development server and React components use inline styles for hot module replacement (HMR)
  - **Post-MVP**: Consider migrating to CSS-in-JS libraries that support CSP (e.g., styled-components with nonce)
  - **Mitigation**: All inline styles are from trusted application code

- **`connect-src 'self' ws://localhost:* http://localhost:*`**:
  - **Required for**: WebSocket communication with Chrome extension
  - **Justification**: Real-time transcription delivery to Google Meet via WebSocket (localhost-only)
  - **Security**: Localhost-only restriction prevents external WebSocket connections

## File Permissions

### Unix/Linux/macOS

**Implemented**: ✅ Complete
- **Audio files** (`audio.wav`): `0o600` (rw-------)
- **Transcripts** (`transcription.jsonl`): `0o600` (rw-------)
- **Metadata** (`session.json`): `0o600` (rw-------)

### Windows

**Implemented**: ⚠️ Partial (Default ACLs)
- **Status**: Uses Windows default ACLs (typically equivalent to `644`)
- **Todo**: Implement `SetNamedSecurityInfoW` via `winapi` crate for owner-only access
- **Tracking**: Phase 13.1.4 (Cross-platform compatibility)

## TLS Requirements

**Enforced**: TLS 1.2+ only
**Verified by**: `tests/test_tls_security.py` (6 tests)

- Minimum TLS version: 1.2 (Python `ssl.create_default_context()`)
- TLS 1.0/1.1 connections rejected (deprecated)
- Latest CA certificates via `certifi` (2025.10.5)

## Security Test Coverage

| Test Suite | Location | Status |
|------------|----------|--------|
| Python Dependencies | `pip-audit` | ⚠️ 1 Medium (SEC-001) |
| Rust Dependencies | `cargo-audit` | 🔒 Blocked (Rust 1.85) |
| TLS Verification | `tests/test_tls_security.py` | ✅ 6/6 passed |
| File Permissions | Manual verification | ✅ Unix only |

## Release Checklist

Before releasing to production:

- [ ] Verify pip 25.3 is available and upgrade
- [ ] Run cargo-audit with Rust 1.85+
- [ ] Test file permissions on Windows platform
- [ ] Review CSP policy for production deployment
- [ ] Update this document with any new findings
