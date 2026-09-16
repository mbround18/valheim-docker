#!/usr/bin/env bash
#
# Tests for the TYPE dispatcher in src/scripts/start_valheim.sh.
#
# start_valheim.sh runs a server when sourced, so the dispatcher is lifted out of it
# verbatim and run against stubs. That keeps the test honest: it executes the shipped
# case statement rather than a copy of it.
#
# Run with: ./src/scripts/tests/start_valheim_test.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

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

assert_not_contains() {
  local haystack="$1" needle="$2" name="$3"
  assertions=$((assertions + 1))
  if [[ "$haystack" != *"$needle"* ]]; then
    printf 'ok   %s\n' "$name"
  else
    printf 'FAIL %s\n       %q unexpectedly contains %q\n' "$name" "$haystack" "$needle"
    failures=$((failures + 1))
  fi
}

# The dispatcher, exactly as shipped.
sed -n '/^case "\${TYPE}" in$/,/^esac$/p' "${SCRIPTS_DIR}/start_valheim.sh" >"${WORK_DIR}/dispatch.sh"
if [ ! -s "${WORK_DIR}/dispatch.sh" ]; then
  echo "FAIL could not extract the TYPE dispatcher from start_valheim.sh" >&2
  exit 1
fi

cat >"${WORK_DIR}/run.sh" <<'RUNNER'
source "${SCRIPTS_DIR}/utils.sh"
# Stub everything the dispatcher reaches for, so nothing is installed or logged via odin.
log() { printf 'LOG %s\n' "$*" >&2; }
install_bepinex() { printf 'INSTALL_BEPINEX\n' >&2; }

source "${WORK_DIR}/dispatch.sh"

# Printed from a child process, so this only shows a MODS the dispatcher exported.
bash -c 'printf "%s" "${MODS-}"'
RUNNER

# Runs the dispatcher. Sets `stdout` to the exported MODS, `stderr` to the stub output and
# `status` to the exit code.
dispatch() {
  local type="$1" mods="${2-}" game_location="${3:-${WORK_DIR}/game}"
  mkdir -p "${game_location}"
  stderr_file="${WORK_DIR}/stderr"
  stdout="$(
    SCRIPTS_DIR="${SCRIPTS_DIR}" WORK_DIR="${WORK_DIR}" \
      TYPE="${type,,}" MODS="${mods}" GAME_LOCATION="${game_location}" \
      bash "${WORK_DIR}/run.sh" 2>"${stderr_file}"
  )"
  status=$?
  stderr="$(cat "${stderr_file}")"
}

vp_url="https://github.com/Grantapher/ValheimPlus/releases/latest/download/ValheimPlus.dll"

# --- TYPE=ValheimPlus ------------------------------------------------------

dispatch "ValheimPlus"
assert_eq "0" "$status" "ValheimPlus is a known type"
assert_eq "${vp_url}" "$stdout" "ValheimPlus exports MODS with the plugin URL"
assert_contains "$stderr" "INSTALL_BEPINEX" "ValheimPlus installs BepInEx first"

dispatch "valheimplus"
assert_eq "${vp_url}" "$stdout" "the lowercased spelling works too"

dispatch "valheim_plus"
assert_eq "${vp_url}" "$stdout" "the underscored spelling works too"

dispatch "ValheimPlus" "Author-SomeMod-1.0.0"
assert_eq "Author-SomeMod-1.0.0 ${vp_url}" "$stdout" "ValheimPlus is appended to existing MODS"

dispatch "ValheimPlus" "https://example.com/ValheimPlus.dll"
assert_eq "https://example.com/ValheimPlus.dll" "$stdout" "a hand picked ValheimPlus entry is kept"
assert_contains "$stderr" "MODS already names ValheimPlus" "and the reason is logged"

# A server that already has BepInEx on its volume must not reinstall it every boot.
installed="${WORK_DIR}/installed"
mkdir -p "${installed}/BepInEx/core"
touch "${installed}/BepInEx/core/BepInEx.dll"
dispatch "ValheimPlus" "" "${installed}"
assert_not_contains "$stderr" "INSTALL_BEPINEX" "BepInEx is not reinstalled when already present"
assert_eq "${vp_url}" "$stdout" "ValheimPlus is still added when BepInEx is already installed"

VALHEIM_PLUS_RELEASE=0.10.1.2 dispatch "ValheimPlus"
assert_eq "https://github.com/Grantapher/ValheimPlus/releases/download/0.10.1.2/ValheimPlus.dll" \
  "$stdout" "VALHEIM_PLUS_RELEASE reaches the dispatcher"

# The helpers live in utils.sh, which start_valheim.sh only sources when present. Without
# them the dispatcher has to say so rather than die on an undefined function.
cat >"${WORK_DIR}/run_without_utils.sh" <<'RUNNER'
log() { printf 'LOG %s\n' "$*" >&2; }
install_bepinex() { printf 'INSTALL_BEPINEX\n' >&2; }
source "${WORK_DIR}/dispatch.sh"
bash -c 'printf "%s" "${MODS-}"'
RUNNER

stderr_file="${WORK_DIR}/stderr"
stdout="$(
  WORK_DIR="${WORK_DIR}" TYPE="valheimplus" MODS="" GAME_LOCATION="${WORK_DIR}/game" \
    bash "${WORK_DIR}/run_without_utils.sh" 2>"${stderr_file}"
)"
status=$?
stderr="$(cat "${stderr_file}")"
assert_eq "1" "$status" "a missing utils.sh fails the dispatcher"
assert_contains "$stderr" "needs /home/steam/scripts/utils.sh" "and says what is missing"

# --- the types that already existed ----------------------------------------

dispatch "BepInEx" "Author-SomeMod-1.0.0"
assert_eq "0" "$status" "BepInEx is still a known type"
assert_eq "Author-SomeMod-1.0.0" "$stdout" "BepInEx leaves MODS alone"
assert_contains "$stderr" "INSTALL_BEPINEX" "BepInEx installs BepInEx"

dispatch "Vanilla" "Author-SomeMod-1.0.0"
assert_eq "0" "$status" "Vanilla is still a known type"
assert_not_contains "$stderr" "INSTALL_BEPINEX" "Vanilla installs nothing"
assert_contains "$stderr" "Mods will NOT be installed" "Vanilla still warns about ignored mods"

dispatch "Nonsense"
assert_eq "1" "$status" "an unknown type still fails the container"
assert_contains "$stderr" "Unknown type: nonsense" "and says which type it did not know"

printf '\n%d assertion(s), %d failure(s)\n' "$assertions" "$failures"
[ "$failures" -eq 0 ]
