#!/usr/bin/env bash
# apply-patches.sh BUILD_DIR [UPSTREAM_REF]
#
# Clones the configured upstream at the pinned ref into BUILD_DIR, then applies
# all patches/*.patch onto it using `git am`. Exits non-zero on any failure.
#
# Config precedence (first non-empty wins):
#   UPSTREAM_REF:    $2 arg > $UPSTREAM_REF env > <repo>/UPSTREAM_REF file
#   UPSTREAM_REMOTE: $UPSTREAM_REMOTE env > <repo>/UPSTREAM_REPO file > hardcoded default
#
# Env overrides:
#   UPSTREAM_REMOTE — upstream repo URL (e.g. https://github.com/user/rtk.git)
#   PATCHES_DIR     — default <repo>/patches

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 BUILD_DIR [UPSTREAM_REF]" >&2
  exit 2
fi

BUILD_DIR="$1"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UPSTREAM_REF="${2:-${UPSTREAM_REF:-$(cat "$REPO_ROOT/UPSTREAM_REF")}}"
if [[ -z "${UPSTREAM_REMOTE:-}" ]]; then
  if [[ -f "$REPO_ROOT/UPSTREAM_REPO" ]]; then
    UPSTREAM_REMOTE="$(cat "$REPO_ROOT/UPSTREAM_REPO")"
  else
    UPSTREAM_REMOTE="https://github.com/rtk-ai/rtk.git"
  fi
fi
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
