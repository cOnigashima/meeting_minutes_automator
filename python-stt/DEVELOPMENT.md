# Development Guide - Python STT Sidecar

## Quick Start for AI Coding Agents

### Environment Detection

```bash
# Step 1: Find Python virtual environment
ls -la .venv/bin/python          # Primary location (THIS PROJECT)
ls -la venv/bin/python           # Alternative (not used here)

# Step 2: Verify Python version
.venv/bin/python --version       # Should be 3.12.x

# Step 3: Check pytest availability
.venv/bin/python -m pytest --version
```

### Running Tests

```bash
# Run all tests
.venv/bin/python -m pytest tests/ -v

# Run specific test file
.venv/bin/python -m pytest tests/test_audio_pipeline.py -v

# Run with async support
.venv/bin/python -m pytest tests/ -v --asyncio-mode=auto

# Run with coverage
.venv/bin/python -m pytest tests/ --cov=stt_engine --cov-report=term
```

### Test Discovery

```bash
# List all test files
find tests/ -name "test_*.py" -type f

# Note: Tests are flat (no tests/unit/ subdirectory)
# Actual structure:
#   tests/
#   ├── test_audio_pipeline.py
#   ├── test_voice_activity_detector.py
#   └── ...
```

## Environment Setup (for Humans)

### Initial Setup

```bash
cd python-stt

# Create virtual environment
python3 -m venv .venv

# Activate (macOS/Linux)
source .venv/bin/activate

# Install dependencies
pip install -r requirements-dev.txt
```

### Daily Workflow

```bash
# Activate venv
source .venv/bin/activate

# Run tests
pytest tests/ -v

# Deactivate when done
deactivate
```

## Common Issues

### Issue 1: "No module named pytest"

**Symptom**: Running `python3 -m pytest` fails
**Cause**: Using system Python instead of venv
**Solution**: Use `.venv/bin/python -m pytest`

### Issue 2: "tests/unit/test_*.py not found"

**Symptom**: Test path doesn't exist
**Cause**: Tests are in `tests/` not `tests/unit/`
**Solution**: Use `tests/test_*.py` pattern

### Issue 3: README says "venv" but ".venv" exists

**Symptom**: Documentation mismatch
**Cause**: Project structure changed
**Solution**: Always use `.venv/` (actual implementation)

## AI Agent Recovery Strategy

When encountering test execution failures:

1. **Check venv location**: `.venv/bin/python` or `venv/bin/python`?
2. **List test files**: `find tests/ -name "*.py"`
3. **Try direct execution**: `.venv/bin/python -m pytest tests/`
4. **Check dependencies**: `.venv/bin/pip list | grep pytest`
5. **Read this file**: Project-specific quirks documented here

## Project Structure

```
python-stt/
├── .venv/                  # Virtual environment (ACTUAL LOCATION)
├── stt_engine/             # Main package
│   ├── audio_pipeline.py
│   ├── transcription/
│   │   └── voice_activity_detector.py
│   └── ...
├── tests/                  # Tests (flat structure)
│   ├── test_audio_pipeline.py
│   └── ...
├── main.py                 # Entry point
├── requirements.txt        # Production dependencies
├── requirements-dev.txt    # Dev dependencies (includes pytest)
├── README.md               # User documentation
└── DEVELOPMENT.md          # This file (AI agent guide)
```

## Testing P0 Fixes

### Example: Testing VAD AttributeError Fix

```bash
# Interactive test
.venv/bin/python -c "
from stt_engine.audio_pipeline import AudioPipeline
from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector

vad = VoiceActivityDetector()
pipeline = AudioPipeline(vad=vad)

# This should NOT raise AttributeError
result = pipeline.is_in_speech()
print(f'is_in_speech: {result}')
"
```

## Key Differences from README

| README.md | Actual Implementation | Use This |
|-----------|----------------------|----------|
| `venv/` | `.venv/` | **`.venv/`** |
| `source venv/bin/activate` | `source .venv/bin/activate` | **`.venv`** |
| Generic structure | Flat `tests/` dir | **Flat** |

## CI/CD Notes

This project structure is optimized for:
- GitHub Actions: `.venv/bin/python -m pytest`
- Pre-commit hooks: Uses `.venv/bin/python`
- Local development: Activate `.venv` for IDE integration

## Version History

- 2025-10-14: Created DEVELOPMENT.md for AI agent guidance
- 2025-10-14: Documented `.venv` vs `venv` discrepancy
