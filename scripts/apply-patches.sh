#!/usr/bin/env bash
# apply-patches.sh BUILD_DIR [UPSTREAM_REF]
#
# Clones upstream rtk-ai/rtk at the pinned tag into BUILD_DIR, then applies
# all patches/*.patch onto it using `git am`. Exits non-zero on any failure.
#
# Env overrides:
#   UPSTREAM_REMOTE — default https://github.com/rtk-ai/rtk.git
#   PATCHES_DIR     — default <repo>/patches

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 BUILD_DIR [UPSTREAM_REF]" >&2
  exit 2
fi

BUILD_DIR="$1"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM_REF="${2:-$(cat "$REPO_ROOT/UPSTREAM_REF")}"
UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-https://github.com/rtk-ai/rtk.git}"
PATCHES_DIR="${PATCHES_DIR:-$REPO_ROOT/patches}"

if [[ -e "$BUILD_DIR" ]]; then
  echo "error: BUILD_DIR '$BUILD_DIR' already exists; remove it first" >&2
  exit 1
fi

if [[ ! -d "$PATCHES_DIR" ]] || ! compgen -G "$PATCHES_DIR/*.patch" >/dev/null; then
  echo "error: no patches found in '$PATCHES_DIR'" >&2
  exit 1
fi

echo "==> cloning $UPSTREAM_REMOTE @ $UPSTREAM_REF -> $BUILD_DIR"
git clone --depth 1 --branch "$UPSTREAM_REF" "$UPSTREAM_REMOTE" "$BUILD_DIR"

cd "$BUILD_DIR"

# `git am` needs a configured identity to record the apply; harmless in CI.
git config user.email "fork-build@buni.local"
git config user.name "fork-build"

echo "==> applying patches from $PATCHES_DIR"
git am "$PATCHES_DIR"/*.patch

echo "==> done; HEAD is now:"
git log --oneline -5
