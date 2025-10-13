# Python Environment Setup for Meeting Minutes Automator

## Virtual Environment Location

**IMPORTANT**: This project uses `.venv/` NOT `venv/`

```bash
# Correct path
python-stt/.venv/bin/python

# Wrong path (documented in README but doesn't exist)
python-stt/venv/bin/python
```

## Running Tests

### Method 1: Using venv directly (RECOMMENDED)
```bash
cd python-stt
.venv/bin/python -m pytest tests/ -v
```

### Method 2: Activating venv (if preferred)
```bash
cd python-stt
source .venv/bin/activate
pytest tests/ -v
```

## Test Directory Structure

```
python-stt/tests/
├── test_audio_pipeline.py          # ✅ AudioPipeline tests (10 tests)
├── test_voice_activity_detector.py # ✅ VAD tests
├── test_audio_integration.py       # ✅ Integration tests
├── test_ipc_handler.py             # ✅ IPC tests
└── ...

# NOTE: No tests/unit/ subdirectory exists
# All tests are directly under tests/
```

## Common Pitfalls

1. **Using system python3**: Will fail with "No module named pytest"
   - Solution: Use `.venv/bin/python` instead

2. **Wrong venv path**: README says `venv/` but actual is `.venv/`
   - Solution: Update README or use `.venv/`

3. **Assuming tests/unit/ structure**: Tests are flat in `tests/`
   - Solution: Use `find tests/ -name "*.py"` to discover

## Python Version

- Project uses: Python 3.12 (.venv)
- System has: Python 3.13 (Homebrew)
- Always prefer: .venv/bin/python

## Quick Check Commands

```bash
# Verify venv exists
ls -la .venv/bin/python

# Check Python version
.venv/bin/python --version

# List all tests
find tests/ -name "test_*.py"

# Run single test file
.venv/bin/python -m pytest tests/test_audio_pipeline.py -v
```
