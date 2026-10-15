#!/usr/bin/env bash
# Rejects agent session links in the commit messages a PR adds.
#
# These are per-conversation URLs that mean nothing to anyone reading the history
# later, and they leak a handle on a private session into a public repo. They are
# easy to paste in by accident because agents put them in the messages they draft.
set -euo pipefail

base_sha="${1:?usage: check-commit-messages.sh <base-sha> <head-sha>}"
head_sha="${2:?usage: check-commit-messages.sh <base-sha> <head-sha>}"

# The trailer and the bare URL, in either order, case-insensitively.
pattern='claude\.ai/code/session_|^[[:space:]]*Claude-Session:'

status=0
while read -r sha; do
  [ -n "${sha}" ] || continue
  if message="$(git log -1 --format=%B "${sha}")" && printf '%s' "${message}" |
    grep -inE "${pattern}" >/dev/null; then
    echo "FAIL ${sha}: commit message contains an agent session link"
    printf '%s' "${message}" | grep -inE "${pattern}" | sed 's/^/       /'
    status=1
  fi
done < <(git rev-list "${base_sha}..${head_sha}")

if [ "${status}" -eq 0 ]; then
  echo "ok   no session links in this PR's commit messages"
else
  echo
  echo "Rewrite the offending messages (git rebase -i, or git commit --amend for the tip)."
fi
exit "${status}"
