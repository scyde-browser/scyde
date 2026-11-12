#!/usr/bin/env bash
# Publishes CI screenshots to the branch `ci-shots-<platform>` (one orphan commit, force-pushed), so
# they can be reviewed with `git fetch` where artifact downloads are not reachable.
# Approved by the owner on 2026-11-12 (the job needs `contents: write`).
#   tools/ci/push-shots.sh <platform> <file-or-dir>...
set -euo pipefail
platform=$1
shift
tmp=$(mktemp -d)
for src in "$@"; do
  if [ -e "$src" ]; then cp -R "$src" "$tmp/"; fi
done
cd "$tmp"
if [ -z "$(ls -A .)" ]; then echo "no screenshots to publish"; exit 0; fi
git init -q
git checkout -q --orphan shots
git add -A
git -c user.name="github-actions[bot]" -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
  commit -qm "ci: ${platform} screenshots for ${GITHUB_SHA:0:7}"
git push -qf "https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" "HEAD:refs/heads/ci-shots-${platform}"
echo "published to ci-shots-${platform}"
