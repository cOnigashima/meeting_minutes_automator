#!/bin/bash
# SEC-001: Python dependency vulnerability scanning
# Usage: ./scripts/security_scan.sh

set -e

cd "$(dirname "$0")/.."

echo "=== Python Dependency Vulnerability Scan ==="
echo "Running pip-audit..."

# Activate virtual environment
source .venv/bin/activate

# Run pip-audit with known vulnerability exclusions
# GHSA-4xh5-x5gv-qwph: pip tarfile symbolic link escape (fixed in pip 25.3, planned release)
# Mitigation: Install only from trusted sources (PyPI, requirements.txt pinned versions)
pip-audit --ignore-vuln GHSA-4xh5-x5gv-qwph

echo ""
echo "✅ Security scan complete. No unmitigated vulnerabilities found."
