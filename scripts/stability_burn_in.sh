#!/usr/bin/env bash

# Stability burn-in orchestrator
# Runs the stt_burn_in harness for an extended duration and captures logs

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/stability_burn_in.sh [--duration SECS] [--python PATH] [--session-label LABEL]

Options:
  --duration SECS      Total run duration in seconds (default: 7200 = 2 hours)
  --python PATH        Override Python executable passed to stt_burn_in
  --session-label LABEL  Optional suffix for the log directory (default: auto timestamp)

The script creates logs under logs/platform/stability-<timestamp> and executes:

  cargo run --bin stt_burn_in -- --duration-secs <SECS> --log-file <dir>/burnin.log [--python PATH]

Run from the repository root. Use a separate terminal for `npm run tauri dev`
to observe the UI while the burn-in harness exercises the pipeline.
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

DURATION_SECS=7200
PYTHON_PATH=""
SESSION_LABEL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --duration)
      shift
      DURATION_SECS="${1:?--duration requires a value}"
      ;;
    --python)
      shift
      PYTHON_PATH="${1:?--python requires a value}"
      ;;
    --session-label)
      shift
      SESSION_LABEL="${1:?--session-label requires a value}"
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
  shift || break
done

if ! [[ "${DURATION_SECS}" =~ ^[0-9]+$ && "${DURATION_SECS}" -gt 0 ]]; then
  echo "Duration must be a positive integer (seconds)" >&2
  exit 1
fi

TIMESTAMP="$(date +"%Y%m%d-%H%M%S")"
SESSION_SUFFIX="${SESSION_LABEL:+-${SESSION_LABEL}}"
SESSION_DIR="${PROJECT_ROOT}/logs/platform/stability-${TIMESTAMP}${SESSION_SUFFIX}"

mkdir -p "${SESSION_DIR}"

LOG_FILE="${SESSION_DIR}/burnin.log"
SUMMARY_FILE="${SESSION_DIR}/summary.txt"

if [[ -z "${APP_PYTHON:-}" && -z "${PYTHON_PATH}" ]]; then
  DEFAULT_PYTHON="${PROJECT_ROOT}/python-stt/.venv/bin/python"
  if [[ -x "${DEFAULT_PYTHON}" ]]; then
    export APP_PYTHON="${DEFAULT_PYTHON}"
  fi
fi

{
  echo "Stability burn-in session"
  echo "Started at: ${TIMESTAMP}"
  echo "Duration (sec): ${DURATION_SECS}"
  if [[ -n "${PYTHON_PATH}" ]]; then
    echo "Python override: ${PYTHON_PATH}"
  elif [[ -n "${APP_PYTHON:-}" ]]; then
    echo "Python via APP_PYTHON: ${APP_PYTHON}"
  fi
  echo "Log file: ${LOG_FILE}"
} | tee "${SUMMARY_FILE}"

CARGO_MANIFEST="${PROJECT_ROOT}/src-tauri/Cargo.toml"

if [[ ! -f "${CARGO_MANIFEST}" ]]; then
  echo "Unable to locate Cargo manifest at ${CARGO_MANIFEST}" >&2
  exit 1
fi

CMD=(cargo run --manifest-path "${CARGO_MANIFEST}" --bin stt_burn_in -- --duration-secs "${DURATION_SECS}" --log-file "${LOG_FILE}")
if [[ -n "${PYTHON_PATH}" ]]; then
  CMD+=(--python "${PYTHON_PATH}")
fi

echo
echo "Executing: ${CMD[*]}"
echo

"${CMD[@]}"

echo
echo "Burn-in completed."
echo "Artifacts saved in: ${SESSION_DIR}"
