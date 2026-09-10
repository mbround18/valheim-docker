#!/usr/bin/env bash
#
# Tests for the pure helper functions in src/scripts/entrypoint.sh.
#
# entrypoint.sh guards its main flow behind a BASH_SOURCE check, so sourcing it
# here defines the functions without launching a server.
#
# Run with: ./src/scripts/tests/entrypoint_test.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=../entrypoint.sh
source "${SCRIPT_DIR}/../entrypoint.sh"
set +e

failures=0
assertions=0

assert_eq() {
  local expected="$1" actual="$2" name="$3"
  assertions=$((assertions + 1))
  if [ "$expected" = "$actual" ]; then
    printf 'ok   %s\n' "$name"
  else
    printf 'FAIL %s\n       expected: %q\n       actual:   %q\n' "$name" "$expected" "$actual"
    failures=$((failures + 1))
  fi
}

assert_contains() {
  local haystack="$1" needle="$2" name="$3"
  assertions=$((assertions + 1))
  if [[ "$haystack" == *"$needle"* ]]; then
    printf 'ok   %s\n' "$name"
  else
    printf 'FAIL %s\n       %q does not contain %q\n' "$name" "$haystack" "$needle"
    failures=$((failures + 1))
  fi
}

# --- parse_total_memory_mb -------------------------------------------------

assert_eq "32022" \
  "$(printf 'MemTotal:       32791208 kB\nMemFree:  1000 kB\n' | parse_total_memory_mb)" \
  "parse_total_memory_mb reads MemTotal in kB"

assert_eq "1024" \
  "$(printf 'MemTotal:        1048576 kB\n' | parse_total_memory_mb)" \
  "parse_total_memory_mb converts exactly 1GiB"

assert_eq "" \
  "$(printf 'SwapTotal:  0 kB\n' | parse_total_memory_mb)" \
  "parse_total_memory_mb yields empty without a MemTotal line"

assert_eq "" \
  "$(printf '' | parse_total_memory_mb)" \
  "parse_total_memory_mb yields empty on empty input"

# Regression: the previous implementation piped `free -h` through `tr -d 'G'`,
# which leaves the "i" of "31Gi" behind and fed "31i" to bc.
assert_eq "" \
  "$(printf 'MemTotal: 31Gi\n' | parse_total_memory_mb | tr -dc 'a-z')" \
  "parse_total_memory_mb never emits a unit suffix"

# --- format_memory_gb ------------------------------------------------------

assert_eq "31.3" "$(format_memory_gb 32022)" "format_memory_gb renders one decimal"
assert_eq "1.0" "$(format_memory_gb 1024)" "format_memory_gb renders whole GB"
assert_eq "0.5" "$(format_memory_gb 512)" "format_memory_gb renders sub-GB"

# --- check_memory ----------------------------------------------------------
# Stub log() and total_memory_mb() to capture what check_memory reports.

log() { printf '%s\n' "$*"; }

total_memory_mb() { printf '32022'; }
assert_contains "$(check_memory)" "Total memory: 31.3 GB" "check_memory reports total on a large host"

total_memory_mb() { printf '2048'; }
assert_contains "$(check_memory)" "Total memory: 2.0 GB" "check_memory accepts exactly 2GB"

total_memory_mb() { printf '1024'; }
assert_contains "$(check_memory)" "less than 2GB of RAM" "check_memory warns below 2GB"

total_memory_mb() { printf ''; }
assert_contains "$(check_memory)" "Unable to determine total system memory" \
  "check_memory degrades gracefully when memory is unknown"

# Guard against the old failure mode: a non-numeric reading must not produce a
# shell error, it must take the "unknown" branch.
total_memory_mb() { printf '31i'; }
output="$(check_memory 2>&1)"
assert_contains "$output" "Unable to determine total system memory" \
  "check_memory handles a non-numeric reading"
assert_eq "" "$(printf '%s' "$output" | grep -c 'syntax error' | tr -d '0')" \
  "check_memory emits no arithmetic syntax error"

# --- summary ---------------------------------------------------------------

printf '\n%d assertions, %d failures\n' "$assertions" "$failures"
[ "$failures" -eq 0 ]
