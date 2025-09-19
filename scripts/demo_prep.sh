#!/usr/bin/env bash
set -euo pipefail

# Prepare demo data for VHS tapes without recording the setup steps.
# Usage: scripts/demo_prep.sh [all|diff-file-date|diff-date]

MODE=${1:-all}

here() { cd "$(dirname "$0")/.."; }
have_dt() { [[ -x ./package/latest/dt ]]; }

ensure_dt() {
  here
  if ! have_dt; then
    echo "[prep] Packaging dt..."
    ./package.sh >/dev/null
  fi
}

clean_query() {
  # Clean records matching a query string (auto-confirm)
  local q="$1"
  printf "YES\n" | ./package/latest/dt clean search "$q" >/dev/null || true
}

prep_diff_file_date() {
  echo "[prep] Preparing file-based diff demo..."
  ensure_dt
  local FILE=/tmp/dt_demo_vhs/file.txt
  rm -rf /tmp/dt_demo_vhs
  mkdir -p /tmp/dt_demo_vhs
  : > "$FILE"
  # Remove only previous records for this command
  clean_query "cat $FILE"
  # First snapshot
  date '+%F %T' > "$FILE"; echo alpha >> "$FILE"
  ./package/latest/dt run cat "$FILE" >/dev/null
  # Second snapshot with visible changes
  sleep 1; echo --- >> "$FILE"; date '+%F %T' >> "$FILE"; echo beta >> "$FILE"
  ./package/latest/dt run cat "$FILE" >/dev/null
}

prep_diff_date() {
  echo "[prep] Preparing simple date diff demo..."
  ensure_dt
  clean_query "date"
  # Create only the initial baseline run so code 'a' exists.
  ./package/latest/dt run date >/dev/null
}

case "$MODE" in
  diff-file-date) prep_diff_file_date ;;
  diff-date) prep_diff_date ;;
  all) prep_diff_file_date; prep_diff_date ;;
  *) echo "Unknown mode: $MODE" >&2; exit 2 ;;
esac

echo "[prep] Done."
