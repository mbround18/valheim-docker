#!/usr/bin/env bash
# Checks that a built valheim image is usable by the uids people actually run it as,
# without downloading the game.
#
#   1000:1000 - the documented `user:` / `runAsUser` value, and the image's own steam user
#   111:1000  - steam's uid before #1507; existing `runAsUser: 111` setups rely on it
#
# For each uid:
#   - it must have a passwd entry. The Valheim server segfaults at startup (in PlayFab's
#     logger) when its uid has none, so an arbitrary uid cannot run the server at all.
#   - it must be able to write every directory odin's install preflight checks
#     (`preflight_write_checks` in src/odin/server/install.rs) plus $HOME.
# Sudo is blocked with no-new-privileges, as it is under `allowPrivilegeEscalation: false`,
# so the entrypoint's chown fallback cannot paper over a bad image.
#
# Usage: .github/scripts/check-image-permissions.sh <image>
set -euo pipefail

IMAGE="${1:?usage: $0 <image>}"
USERS=("1000:1000" "111:1000")

steam_uid="$(docker run --rm --entrypoint id "${IMAGE}" -u steam)"
if [ "${steam_uid}" != "1000" ]; then
  echo "FAIL: steam should be uid 1000, image has ${steam_uid}"
  exit 1
fi
echo "ok   steam is uid 1000"

failures=0
for user in "${USERS[@]}"; do
  if output="$(docker run --rm --user "${user}" --security-opt no-new-privileges \
    --entrypoint bash "${IMAGE}" -c '
      status=0
      if ! getent passwd "$(id -u)" >/dev/null; then
        echo "  no passwd entry for uid $(id -u); the Valheim server segfaults without one"
        status=1
      fi
      for p in "$HOME" /home/steam/Steam /home/steam/.steam /home/steam/.local/share/Steam \
               /home/steam/steamcmd /home/steam/.local/share/Steam/steamcmd /tmp; do
        [ -d "$p" ] || continue
        if touch "$p/.permcheck" 2>/dev/null; then
          rm -f "$p/.permcheck"
        else
          echo "  not writable: $p ($(stat -c "%U:%G %a" "$p"))"
          status=1
        fi
      done
      exit $status' 2>&1)"; then
    echo "ok   --user ${user} has a passwd entry and can write every preflight path"
  else
    echo "FAIL --user ${user}"
    echo "${output}"
    failures=$((failures + 1))
  fi
done

[ "${failures}" -eq 0 ]
