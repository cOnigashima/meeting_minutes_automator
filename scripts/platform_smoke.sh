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
run_optional env PYTHONPATH="${PROJECT_ROOT}/python-stt:${PYTHONPATH:-}" python3 - <<'PY'
try:
    from stt_engine.audio_pipeline import AudioPipeline  # noqa: F401
    from transcription.voice_activity_detector import VoiceActivityDetector  # noqa: F401
    print("Python sidecar modules import: OK")
except Exception as exc:  # pragma: no cover
    print(f"Python sidecar import failed: {exc!r}")
    raise
PY

log_step "Smoke test completed"
printf 'Logs written to %s\n' "${LOG_FILE}"
