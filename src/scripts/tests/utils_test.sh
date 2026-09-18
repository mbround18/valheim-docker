#!/usr/bin/env bash
#
# Tests for the pure helper functions in src/scripts/utils.sh.
#
# utils.sh only defines functions, so sourcing it here is side effect free.
#
# Run with: ./src/scripts/tests/utils_test.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=../utils.sh
source "${SCRIPT_DIR}/../utils.sh"
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

assert_true() {
  local name="$2"
  assertions=$((assertions + 1))
  if eval "$1"; then
    printf 'ok   %s\n' "$name"
  else
    printf 'FAIL %s\n       expected %q to succeed\n' "$name" "$1"
    failures=$((failures + 1))
  fi
}

assert_false() {
  local name="$2"
  assertions=$((assertions + 1))
  if eval "$1"; then
    printf 'FAIL %s\n       expected %q to fail\n' "$name" "$1"
    failures=$((failures + 1))
  else
    printf 'ok   %s\n' "$name"
  fi
}

clear_valheim_plus_env() {
  unset VALHEIM_PLUS_DOWNLOAD_URL VALHEIM_PLUS_RELEASE VALHEIM_PLUS_REPOSITORY
}

# --- valheim_plus_download_url ---------------------------------------------

clear_valheim_plus_env
assert_eq "https://github.com/Grantapher/ValheimPlus/releases/latest/download/ValheimPlus.dll" \
  "$(valheim_plus_download_url)" \
  "valheim_plus_download_url defaults to the latest Grantapher release"

assert_eq "https://github.com/Grantapher/ValheimPlus/releases/download/0.9.17.1/ValheimPlus.dll" \
  "$(VALHEIM_PLUS_RELEASE=0.9.17.1 valheim_plus_download_url)" \
  "VALHEIM_PLUS_RELEASE pins a release"

assert_eq "https://github.com/Grantapher/ValheimPlus/releases/latest/download/ValheimPlus.dll" \
  "$(VALHEIM_PLUS_RELEASE=Latest valheim_plus_download_url)" \
  "VALHEIM_PLUS_RELEASE=latest is case insensitive"

assert_eq "https://github.com/someone/fork/releases/latest/download/ValheimPlus.dll" \
  "$(VALHEIM_PLUS_REPOSITORY=someone/fork valheim_plus_download_url)" \
  "VALHEIM_PLUS_REPOSITORY points at another fork"

assert_eq "https://example.com/mirror/ValheimPlus.dll" \
  "$(VALHEIM_PLUS_DOWNLOAD_URL=https://example.com/mirror/ValheimPlus.dll VALHEIM_PLUS_RELEASE=0.9.17.1 valheim_plus_download_url)" \
  "VALHEIM_PLUS_DOWNLOAD_URL wins over every other setting"

# The URL has to be one odin recognises as ValheimPlus, since that is what triggers the
# valheim_plus.cfg download. Mirrored by is_valheim_plus_dll_url in
# src/odin/mods/valheim_plus.rs.
clear_valheim_plus_env
assert_true '[[ "$(valheim_plus_download_url)" == *"/ValheimPlus.dll" ]]' \
  "the default URL ends in ValheimPlus.dll so odin fetches the config for it"

# --- normalize_type ----------------------------------------------------------

assert_eq "vanilla" "$(normalize_type "Vanilla")" "lowercases"
assert_eq "bepinex" "$(normalize_type "BepInEx")" "lowercases every type"
assert_eq "vanilla" "$(normalize_type "")" "empty is vanilla"
assert_eq "vanilla" "$(normalize_type '""')" "an empty double-quoted value is vanilla"
assert_eq "vanilla" "$(normalize_type "''")" "an empty single-quoted value is vanilla"
assert_eq "vanilla" "$(normalize_type '"Vanilla"')" "surrounding double quotes are stripped"
assert_eq "valheimplus" "$(normalize_type "'ValheimPlus'")" "surrounding single quotes are stripped"
assert_eq "vanilla" "$(normalize_type '  "Vanilla"  ')" "whitespace around the quotes is stripped"
assert_eq "vanilla" "$(normalize_type "  vanilla  ")" "whitespace alone is stripped"
assert_eq '"vanilla' "$(normalize_type '"Vanilla')" "an unmatched quote is left alone"
assert_eq '"' "$(normalize_type '"')" "a lone quote is left alone"

# --- mods_include_valheim_plus ---------------------------------------------

assert_false 'mods_include_valheim_plus ""' \
  "an empty mod list does not include ValheimPlus"

assert_false 'mods_include_valheim_plus "ts:denikson-BepInExPack_Valheim-5.4.2202 Author-SomeMod-1.0.0"' \
  "an unrelated mod list does not include ValheimPlus"

assert_true 'mods_include_valheim_plus "https://github.com/Grantapher/ValheimPlus/releases/download/0.9.17.1/ValheimPlus.dll"' \
  "a release URL counts as ValheimPlus"

assert_true 'mods_include_valheim_plus "ts:Grantapher-ValheimPlus-0.9.17.1"' \
  "a package name counts as ValheimPlus"

assert_true 'mods_include_valheim_plus "https://example.com/valheim_plus.dll"' \
  "the underscored spelling counts as ValheimPlus"

# --- mods_with_valheim_plus ------------------------------------------------

url="https://github.com/Grantapher/ValheimPlus/releases/latest/download/ValheimPlus.dll"

assert_eq "${url}" \
  "$(mods_with_valheim_plus "" "${url}")" \
  "an empty mod list becomes just ValheimPlus"

assert_eq "${url}" \
  "$(mods_with_valheim_plus "  , " "${url}")" \
  "a list of only separators becomes just ValheimPlus"

assert_eq "Author-SomeMod-1.0.0 ${url}" \
  "$(mods_with_valheim_plus "Author-SomeMod-1.0.0" "${url}")" \
  "ValheimPlus is appended to an existing list"

assert_eq "Author-SomeMod-1.0.0,Author-Other-2.0.0 ${url}" \
  "$(mods_with_valheim_plus "Author-SomeMod-1.0.0,Author-Other-2.0.0" "${url}")" \
  "a comma separated list keeps its commas and gains ValheimPlus"

assert_eq "ts:Grantapher-ValheimPlus-0.9.17.1" \
  "$(mods_with_valheim_plus "ts:Grantapher-ValheimPlus-0.9.17.1" "${url}")" \
  "a hand picked ValheimPlus entry is left alone"

assert_eq "Author-SomeMod-1.0.0 https://example.com/ValheimPlus.dll" \
  "$(mods_with_valheim_plus "Author-SomeMod-1.0.0 https://example.com/ValheimPlus.dll" "${url}")" \
  "ValheimPlus is not added twice when it is already in a longer list"

printf '\n%d assertion(s), %d failure(s)\n' "$assertions" "$failures"
[ "$failures" -eq 0 ]
