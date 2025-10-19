#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LOG_DIR="${PROJECT_ROOT}/logs/platform"
TIMESTAMP="$(date +"%Y%m%d-%H%M%S")"
LOG_FILE="${LOG_DIR}/${TIMESTAMP}-smoke.log"

mkdir -p "${LOG_DIR}"

exec > >(tee -a "${LOG_FILE}") 2>&1

log_step() {
  printf '\n==> %s\n' "$1"
}

log_warn() {
  printf '⚠️  %s\n' "$1"
}

run_optional() {
  if "$@"; then
    printf '   ✓ %s\n' "$*"
  else
    log_warn "Command failed (continuing): $*"
  fi
}

log_step "Platform smoke test started at ${TIMESTAMP}"
printf 'Project root: %s\n' "${PROJECT_ROOT}"
printf 'Log file: %s\n' "${LOG_FILE}"

log_step "Environment snapshot"
run_optional uname -a
run_optional sw_vers
run_optional system_profiler SPUSBDataType
run_optional python3 --version
run_optional cargo --version

log_step "Rust platform tests (ignored group)"
run_optional cargo test --manifest-path "${PROJECT_ROOT}/src-tauri/Cargo.toml" -- --ignored platform

log_step "Ring buffer / IPC quick check (optional target)"
run_optional cargo test --manifest-path "${PROJECT_ROOT}/src-tauri/Cargo.toml" ring_buffer -- --ignored --nocapture

log_step "Python sidecar import smoke test"
if [ -f "${PROJECT_ROOT}/python-stt/.venv/bin/python" ]; then
  run_optional env PYTHONPATH="${PROJECT_ROOT}/python-stt:${PYTHONPATH:-}" "${PROJECT_ROOT}/python-stt/.venv/bin/python" - <<'PY'
try:
    from stt_engine.audio_pipeline import AudioPipeline  # noqa: F401
    from stt_engine.transcription.voice_activity_detector import VoiceActivityDetector  # noqa: F401
    from stt_engine.transcription.whisper_client import WhisperClient  # noqa: F401
    print("Python sidecar modules import: OK")
except Exception as exc:  # pragma: no cover
    print(f"Python sidecar import failed: {exc!r}")
    raise
PY
else
  log_warn "Python venv not found at ${PROJECT_ROOT}/python-stt/.venv"
  printf '   ℹ️  Run: cd python-stt && python3 -m venv .venv && .venv/bin/pip install -r requirements.txt\n'
fi

log_step "Python unit tests (quick smoke)"
if [ -f "${PROJECT_ROOT}/python-stt/.venv/bin/pytest" ]; then
  run_optional "${PROJECT_ROOT}/python-stt/.venv/bin/python" -m pytest "${PROJECT_ROOT}/python-stt/tests/" -k "not test_audio_recording" --maxfail=3 -v
else
  log_warn "pytest not found, skipping Python tests"
fi

log_step "Smoke test completed"
printf '\n==> Summary\n'
printf '   Platform: %s\n' "$(uname -s)"
printf '   Logs: %s\n' "${LOG_FILE}"
printf '   Next: Review logs for failures, run full E2E test with:\n'
printf '         cargo test --test stt_e2e_test -- --ignored --nocapture\n'
