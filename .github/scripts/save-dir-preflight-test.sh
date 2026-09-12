#!/usr/bin/env bash
# Reproduces #1515 against a built image: a save volume the container's uid cannot write.
#
# Valheim keeps running when it cannot save — it logs `Error saving world!` every save
# interval and carries on in memory, so the world on disk and every backup taken from it
# stay stale until a restart loads that stale world. `odin start` refuses to launch into
# that state, and this checks it still does.
#
# Usage: ./.github/scripts/save-dir-preflight-test.sh <image>
set -euo pipefail

IMAGE="${1:-valheim-docker:ci}"
SAVES="$(mktemp -d)"
WORLD_DIR="$SAVES/worlds_local/Dedicated"
mkdir -p "$WORLD_DIR"
trap 'chmod -R u+w "$SAVES" 2>/dev/null || true; rm -rf "$SAVES"' EXIT

# The container runs as 1000:1000 below, so the host side has to be owned by a uid the
# check can fail against. Refuse to run as root, where mode bits are ignored and every
# case would pass.
if [ "$(id -u)" -eq 0 ]; then
  echo "::error::run this as a non-root user; root ignores directory modes"
  exit 1
fi

# `odin start` loads its config before checking anything, and `odin configure` insists on a
# server executable, so stub one. It is never launched in the failing case, and in the
# passing case it exits immediately.
odin_start() {
  docker run --rm --user 1000:1000 \
    -e SAVE_LOCATION=/saves -e WORLD=Dedicated \
    -v "$SAVES:/saves" --entrypoint bash "$IMAGE" -c '
      printf "#!/bin/sh\nexit 0\n" > /home/steam/valheim/valheim_server.x86_64
      chmod +x /home/steam/valheim/valheim_server.x86_64
      odin configure --password "Str0ngPassw0rd" --name preflight-probe >/dev/null 2>&1 \
        || { echo "odin configure failed"; exit 9; }
      odin start 2>&1
      exit ${PIPESTATUS[0]}'
}

echo "==> An unwritable world directory must stop the server from starting"
chmod 555 "$WORLD_DIR"
set +e
output="$(odin_start)"
rc=$?
set -e
echo "$output" | tail -5
if [ "$rc" -eq 0 ]; then
  echo "::error::odin start succeeded with an unwritable save directory (regression of #1515)"
  exit 1
fi
if ! grep -q "Save directory is not writable" <<<"$output"; then
  echo "::error::odin start failed, but not with the save-directory message — check what broke"
  exit 1
fi
grep -q "worlds_local/Dedicated" <<<"$output" || {
  echo "::error::the error does not name the directory at fault, which is what makes it fixable"
  exit 1
}
echo "    refused to start, naming the directory: ok"

echo "==> A writable world directory must let the server start"
chmod 755 "$WORLD_DIR"
set +e
output="$(odin_start)"
rc=$?
set -e
if [ "$rc" -ne 0 ] || grep -q "Save directory is not writable" <<<"$output"; then
  echo "$output" | tail -5
  echo "::error::the check rejected a writable save directory"
  exit 1
fi
echo "    started: ok"

echo "==> The write probes must not leave files behind"
leftovers="$(find "$SAVES" -name '.write_test_*' -print)"
if [ -n "$leftovers" ]; then
  echo "::error::probe files left in the save volume: $leftovers"
  exit 1
fi
echo "    save volume is clean: ok"

echo "All save-directory preflight checks passed."
