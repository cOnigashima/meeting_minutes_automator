#!/usr/bin/env python3
"""
Check for forbidden audio recording imports in Python code.
This enforces ADR-001: Recording Responsibility Unification.

Usage:
  python scripts/check_forbidden_imports.py <file1.py> <file2.py> ...

Exit codes:
  0: No forbidden imports found
  1: Forbidden imports detected
"""

import sys
import re
from pathlib import Path

FORBIDDEN_IMPORTS = [
    "sounddevice",
    "pyaudio",
    "portaudio",
    "soundcard",
    "PySndHdr",
]

def check_file(filepath: str) -> int:
    """
    Check a single Python file for forbidden imports.

    Args:
        filepath: Path to the Python file

    Returns:
        0 if no forbidden imports found, 1 otherwise
    """
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        print(f"⚠️  Failed to read {filepath}: {e}")
        return 0  # Skip unreadable files

    for forbidden in FORBIDDEN_IMPORTS:
        # Check import statements
        if re.search(rf'^import {forbidden}', content, re.MULTILINE):
            print(f"❌ Forbidden import detected: 'import {forbidden}' in {filepath}")
            print(f"📖 Recording responsibility is exclusively handled by Rust AudioDeviceAdapter.")
            print(f"📄 See: .kiro/specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md")
            return 1

        if re.search(rf'^from {forbidden}', content, re.MULTILINE):
            print(f"❌ Forbidden import detected: 'from {forbidden}' in {filepath}")
            print(f"📖 Recording responsibility is exclusively handled by Rust AudioDeviceAdapter.")
            print(f"📄 See: .kiro/specs/meeting-minutes-stt/adrs/ADR-001-recording-responsibility.md")
            return 1

    return 0


def main():
    if len(sys.argv) < 2:
        print("Usage: python scripts/check_forbidden_imports.py <file1.py> <file2.py> ...")
        sys.exit(0)

    exit_code = 0
    for filepath in sys.argv[1:]:
        if filepath.endswith('.py'):
            exit_code |= check_file(filepath)

    if exit_code == 0:
        print("✅ No forbidden imports detected")

    sys.exit(exit_code)


if __name__ == "__main__":
    main()
