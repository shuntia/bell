#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${SCHEDULE_REPO_URL:-https://github.com/nicolaschan/bell-schedules.git}"
SCHEDULE_DIR="${SCHEDULE_DIR:-schedules}"

for dep in git cargo; do
  if ! command -v "${dep}" >/dev/null 2>&1; then
    echo "Missing dependency: ${dep}"
    exit 1
  fi
done

if [[ -d "${SCHEDULE_DIR}/.git" ]]; then
  (cd "${SCHEDULE_DIR}" && git pull --ff-only)
else
  if [[ -d "${SCHEDULE_DIR}" ]] && [[ -n "$(ls -A "${SCHEDULE_DIR}")" ]]; then
    echo "Directory '${SCHEDULE_DIR}' exists but is not a git repo."
    exit 1
  fi
  git clone "${REPO_URL}" "${SCHEDULE_DIR}"
fi

mapfile -t schedule_dirs < <(
  find "${SCHEDULE_DIR}" -maxdepth 2 -mindepth 2 -type f -name schedules.bell -printf '%h\n' \
    | xargs -n1 basename \
    | sort -u
)

if [[ ${#schedule_dirs[@]} -eq 0 ]]; then
  echo "No schedules found in '${SCHEDULE_DIR}'."
  exit 1
fi

echo "Available schedules:"
for i in "${!schedule_dirs[@]}"; do
  printf "%2d) %s\n" "$((i + 1))" "${schedule_dirs[$i]}"
done

read -r -p "Select schedule (name or number): " choice

selected=""
if [[ "${choice}" =~ ^[0-9]+$ ]]; then
  idx=$((choice - 1))
  if [[ $idx -ge 0 && $idx -lt ${#schedule_dirs[@]} ]]; then
    selected="${schedule_dirs[$idx]}"
  fi
else
  selected="${choice}"
fi

if [[ -z "${selected}" || ! -d "${SCHEDULE_DIR}/${selected}" ]]; then
  echo "Unknown schedule '${choice}'."
  exit 1
fi

export SELECTED_SCHEDULE="${selected}"
export SCHEDULE_DIR="${SCHEDULE_DIR}"
cargo build --release
cp "target/release/bell" "./bell"
