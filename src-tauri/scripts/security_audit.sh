#!/bin/bash
# SEC-004: Rust dependency vulnerability scanning
# Usage: ./scripts/security_audit.sh

set -e

cd "$(dirname "$0")/.."

echo "=== Rust Dependency Vulnerability Scan ==="
echo "Rust version: $(rustc --version)"
echo ""

# Check if cargo-audit is installed
if ! command -v cargo-audit &> /dev/null; then
    echo "❌ ERROR: cargo-audit not installed"
    echo ""
    echo "SEC-004 requires cargo-audit to scan RustSec advisory database."
    echo "The fallback 'cargo tree --duplicates' only checks duplicate dependencies,"
    echo "it does NOT detect actual vulnerabilities (openssl, chrono, etc.)."
    echo ""
    echo "Installation:"
    echo "  cargo install cargo-audit"
    echo ""
    echo "Note: cargo-audit v0.21.2+ requires Rust 1.85+ (edition2024)"
    echo "Current toolchain: $(rustc --version)"
    echo ""
    echo "Options:"
    echo "  1. Wait for Rust 1.85 stable release (expected 2025-02)"
    echo "  2. Install older cargo-audit version compatible with current toolchain"
    echo "  3. Manually check RustSec database: https://rustsec.org/advisories/"
    echo ""
    echo "Until cargo-audit is installed, SEC-004 is INCOMPLETE."
    exit 1
fi

echo "Running cargo-audit..."
cargo audit

echo ""
echo "✅ Security scan complete. No vulnerabilities found."
