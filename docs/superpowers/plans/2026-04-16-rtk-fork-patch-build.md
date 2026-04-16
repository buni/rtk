# RTK Fork Patch-Based Build Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the `buni/rtk` fork to carry changes as reviewable `.patch` files applied at build time, with CI verifying patches apply + tests pass, and a `workflow_dispatch` release that publishes multi-platform binaries to fork GitHub Releases.

**Architecture:** Orphan `fork` branch on the fork repo holds only fork-specific files (patches, workflows, scripts, version pins). All upstream source is cloned at build time from a pinned upstream tag, then `git am`-ed with fork patches before `cargo build`. Two workflows: `ci.yml` (push/PR) and `release.yml` (workflow_dispatch).

**Tech Stack:** GitHub Actions, bash, `git am`/`git format-patch`, cargo, sed.

**Spec:** `docs/superpowers/specs/2026-04-16-rtk-fork-patch-build-design.md`

**Preconditions:**
- Working directory `/home/hristo/rtk` is the fork clone.
- `origin` points to `git@github.com:buni/rtk.git`.
- `alias-extension` branch contains the alias-support commit to be patched.
- Upstream tag `v0.31.0` is reachable (fetch tags if not).

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `patches/0001-add-alias-support-in-config.patch` | The fork's alias-support change, exported from `alias-extension` |
| `UPSTREAM_REF` | Pinned upstream tag (one line, e.g. `v0.31.0`) |
| `VERSION` | Fork version string (one line, e.g. `0.31.0-alias.1`) |
| `scripts/apply-patches.sh` | Clone upstream ref, apply patches. Used by both workflows + local dev. |
| `.github/workflows/ci.yml` | Push/PR: patch-check + test on Linux+macOS |
| `.github/workflows/release.yml` | workflow_dispatch: build 5 targets, publish GH Release on fork |
| `README.fork.md` | Explains fork layout, dev workflow, upstream bump procedure |

> **Version format note:** We use `0.31.0-alias.1` (pre-release form) rather than `0.31.0+alias.1` (build-metadata form). Some cargo tooling and downstream consumers choke on `+` in `Cargo.toml` versions. Pre-release is universally supported.

---

## Task 1: Add upstream remote and fetch tags

**Files:**
- No files modified (git config only, no commit).

- [ ] **Step 1: Check current remotes**

```bash
git remote -v
```

Expected: only `origin git@github.com:buni/rtk.git`.

- [ ] **Step 2: Add upstream remote if missing**

```bash
git remote add upstream https://github.com/rtk-ai/rtk.git 2>/dev/null || true
git remote -v
```

Expected: both `origin` (buni) and `upstream` (rtk-ai) listed.

- [ ] **Step 3: Fetch upstream tags**

```bash
git fetch upstream --tags
```

Expected: fetches tags, no errors.

- [ ] **Step 4: Verify `v0.31.0` tag resolves**

```bash
git rev-parse v0.31.0
```

Expected: prints a SHA (not an error).

No commit this task — git config only.

---

## Task 2: Generate the patch file from `alias-extension`

**Files:**
- Create (in temp, not committed yet): `/tmp/rtk-fork-patches/0001-*.patch`

- [ ] **Step 1: Confirm alias-extension is based on `v0.31.0`**

```bash
git log --oneline v0.31.0..alias-extension
```

Expected: exactly one line — `89888b7 feat: add alias support in config`. If more or fewer, STOP and ask before continuing.

- [ ] **Step 2: Export the patch to a temp dir**

```bash
mkdir -p /tmp/rtk-fork-patches
git format-patch v0.31.0..alias-extension -o /tmp/rtk-fork-patches/
ls /tmp/rtk-fork-patches/
```

Expected: one file, e.g. `0001-feat-add-alias-support-in-config.patch`.

- [ ] **Step 3: Verify patch applies cleanly to upstream tag (smoke test)**

```bash
TMPDIR=$(mktemp -d)
git clone --depth 1 --branch v0.31.0 https://github.com/rtk-ai/rtk.git "$TMPDIR/rtk-src"
cd "$TMPDIR/rtk-src"
git am /tmp/rtk-fork-patches/*.patch
git log --oneline -2
cd -
rm -rf "$TMPDIR"
```

Expected: `git am` prints `Applying: feat: add alias support in config` with no errors. `git log` shows the alias commit on top of the upstream tag.

No commit this task — patch lives in `/tmp` until Task 4.

---

## Task 3: Create orphan `fork` branch with empty working tree

**Files:**
- None yet (branch creation + clearing index).

- [ ] **Step 1: Stash any unrelated untracked work**

```bash
git status --short
```

Expected output — only `?? docs/superpowers/` (the spec + plan we're writing). If anything else, STOP and ask.

```bash
# Move untracked spec/plan out of the way so orphan checkout doesn't trip on them
mv docs/superpowers /tmp/rtk-fork-docs-backup
```

- [ ] **Step 2: Create orphan branch**

```bash
git checkout --orphan fork
```

Expected: `Switched to a new branch 'fork'`.

- [ ] **Step 3: Clear the index and working tree**

```bash
git rm -rf --cached . >/dev/null
git clean -fdx
git status
```

Expected: `No commits yet` and empty working tree (`nothing to commit`).

- [ ] **Step 4: Restore spec and plan**

```bash
mkdir -p docs/superpowers
mv /tmp/rtk-fork-docs-backup/specs docs/superpowers/
mv /tmp/rtk-fork-docs-backup/plans docs/superpowers/
rmdir /tmp/rtk-fork-docs-backup
ls docs/superpowers/
```

Expected: `specs/` and `plans/` both present.

No commit this task — branch is empty, next tasks populate it.

---

## Task 4: Install the patch file

**Files:**
- Create: `patches/0001-feat-add-alias-support-in-config.patch` (copied from Task 2)

- [ ] **Step 1: Copy patch into fork branch**

```bash
mkdir -p patches
cp /tmp/rtk-fork-patches/*.patch patches/
ls patches/
```

Expected: one `.patch` file.

- [ ] **Step 2: Verify patch is well-formed**

```bash
head -5 patches/*.patch
```

Expected: starts with `From <sha>`, `From: Hristo`, `Date: ...`, `Subject: [PATCH] feat: add alias support in config`.

No commit yet — commit after all files are in place (Task 10).

---

## Task 5: Write `UPSTREAM_REF` and `VERSION`

**Files:**
- Create: `UPSTREAM_REF`
- Create: `VERSION`

- [ ] **Step 1: Write `UPSTREAM_REF`**

Create `/home/hristo/rtk/UPSTREAM_REF` with exactly this content (one line, no trailing blank line beyond the single newline):

```
v0.31.0
```

- [ ] **Step 2: Write `VERSION`**

Create `/home/hristo/rtk/VERSION` with exactly this content:

```
0.31.0-alias.1
```

- [ ] **Step 3: Verify**

```bash
cat UPSTREAM_REF
cat VERSION
```

Expected: exactly `v0.31.0` and `0.31.0-alias.1`, each one line.

---

## Task 6: Write `scripts/apply-patches.sh`

**Files:**
- Create: `scripts/apply-patches.sh`

- [ ] **Step 1: Create script**

Create `/home/hristo/rtk/scripts/apply-patches.sh`:

```bash
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
```

- [ ] **Step 2: Make it executable**

```bash
chmod +x scripts/apply-patches.sh
ls -l scripts/apply-patches.sh
```

Expected: shows `-rwxr-xr-x`.

- [ ] **Step 3: Smoke test locally**

```bash
rm -rf /tmp/rtk-build-test
scripts/apply-patches.sh /tmp/rtk-build-test
```

Expected: clones upstream, applies patch, prints `git log` showing the alias commit on top of the upstream tag. Exit code 0.

- [ ] **Step 4: Verify idempotency-fails-safely behavior**

```bash
scripts/apply-patches.sh /tmp/rtk-build-test || echo "Exit: $?"
```

Expected: exits non-zero with `BUILD_DIR ... already exists` — this is the desired guard.

- [ ] **Step 5: Clean up**

```bash
rm -rf /tmp/rtk-build-test
```

---

## Task 7: Write `.github/workflows/ci.yml`

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create workflow**

Create `/home/hristo/rtk/.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [fork]
  pull_request:
    branches: [fork]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  patch-check:
    name: Patches apply cleanly
    runs-on: ubuntu-latest
    steps:
      - name: Checkout fork
        uses: actions/checkout@v4

      - name: Apply patches against pinned upstream
        run: |
          scripts/apply-patches.sh build

      - name: Show resulting HEAD
        run: |
          cd build && git log --oneline -5

  test:
    name: Test on ${{ matrix.os }}
    needs: patch-check
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - name: Checkout fork
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo registry + git
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-${{ hashFiles('UPSTREAM_REF', 'patches/**') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Apply patches
        run: scripts/apply-patches.sh build

      - name: Cache build target
        uses: actions/cache@v4
        with:
          path: build/target
          key: ${{ runner.os }}-target-${{ hashFiles('UPSTREAM_REF', 'patches/**') }}
          restore-keys: |
            ${{ runner.os }}-target-

      - name: cargo fmt --check
        working-directory: build
        run: cargo fmt --all --check

      - name: cargo clippy
        working-directory: build
        run: cargo clippy --all-targets -- -D warnings

      - name: cargo test
        working-directory: build
        run: cargo test --all
```

- [ ] **Step 2: Sanity-lint the YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "YAML OK"
```

Expected: `YAML OK`.

No commit this task.

---

## Task 8: Write `.github/workflows/release.yml`

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create workflow**

Create `/home/hristo/rtk/.github/workflows/release.yml`:

```yaml
name: Release

on:
  workflow_dispatch:
    inputs:
      upstream_ref:
        description: "Upstream tag to build from (default: contents of UPSTREAM_REF)"
        required: false
        default: ""
      version:
        description: "Fork version (default: contents of VERSION)"
        required: false
        default: ""
      prerelease:
        description: "Mark release as prerelease"
        type: boolean
        default: false

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  resolve-inputs:
    name: Resolve inputs
    runs-on: ubuntu-latest
    outputs:
      upstream_ref: ${{ steps.resolve.outputs.upstream_ref }}
      version: ${{ steps.resolve.outputs.version }}
    steps:
      - uses: actions/checkout@v4
      - id: resolve
        run: |
          UPSTREAM_REF="${{ inputs.upstream_ref }}"
          VERSION="${{ inputs.version }}"
          [ -z "$UPSTREAM_REF" ] && UPSTREAM_REF="$(cat UPSTREAM_REF)"
          [ -z "$VERSION" ] && VERSION="$(cat VERSION)"
          echo "upstream_ref=$UPSTREAM_REF" >> "$GITHUB_OUTPUT"
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
          echo "Resolved: upstream=$UPSTREAM_REF version=$VERSION"

  build:
    name: Build ${{ matrix.target }}
    needs: resolve-inputs
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-apple-darwin
            os: macos-latest
            archive: tar.gz
          - target: aarch64-apple-darwin
            os: macos-latest
            archive: tar.gz
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            archive: tar.gz
            musl: true
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
            cross: true
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            archive: zip
    steps:
      - name: Checkout fork
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install musl tools
        if: matrix.musl
        run: sudo apt-get update && sudo apt-get install -y musl-tools

      - name: Install cross
        if: matrix.cross
        run: cargo install cross --locked

      - name: Apply patches
        shell: bash
        run: scripts/apply-patches.sh build "${{ needs.resolve-inputs.outputs.upstream_ref }}"

      - name: Set Cargo.toml version to fork version
        shell: bash
        working-directory: build
        run: |
          VERSION='${{ needs.resolve-inputs.outputs.version }}'
          # Replace only the top-level [package] version line (first `version = "..."`).
          # Portable sed: handle macOS BSD sed requiring '' after -i.
          if sed --version >/dev/null 2>&1; then
            sed -i "0,/^version = .*/s//version = \"$VERSION\"/" Cargo.toml
          else
            sed -i '' "1,/^version = .*/s//version = \"$VERSION\"/" Cargo.toml
          fi
          grep '^version = ' Cargo.toml | head -1

      - name: Build (native)
        if: "!matrix.cross"
        working-directory: build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Build (cross)
        if: matrix.cross
        working-directory: build
        run: cross build --release --target ${{ matrix.target }}

      - name: Strip (Unix)
        if: runner.os != 'Windows' && !matrix.cross
        working-directory: build
        run: strip target/${{ matrix.target }}/release/rtk

      - name: Package (Unix)
        if: runner.os != 'Windows'
        shell: bash
        working-directory: build
        run: |
          VERSION='${{ needs.resolve-inputs.outputs.version }}'
          NAME="rtk-${VERSION}-${{ matrix.target }}"
          mkdir -p "../dist"
          cp target/${{ matrix.target }}/release/rtk "$NAME"
          tar czf "../dist/${NAME}.tar.gz" "$NAME"
          rm "$NAME"
          cd ../dist
          shasum -a 256 "${NAME}.tar.gz" > "${NAME}.tar.gz.sha256"

      - name: Package (Windows)
        if: runner.os == 'Windows'
        shell: bash
        working-directory: build
        run: |
          VERSION='${{ needs.resolve-inputs.outputs.version }}'
          NAME="rtk-${VERSION}-${{ matrix.target }}"
          mkdir -p "../dist"
          cp target/${{ matrix.target }}/release/rtk.exe "${NAME}.exe"
          7z a "../dist/${NAME}.zip" "${NAME}.exe"
          rm "${NAME}.exe"
          cd ../dist
          sha256sum "${NAME}.zip" > "${NAME}.zip.sha256"

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: rtk-${{ matrix.target }}
          path: dist/*
          if-no-files-found: error

  release:
    name: Publish GitHub Release
    needs: [resolve-inputs, build]
    runs-on: ubuntu-latest
    steps:
      - name: Checkout fork
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: List artifacts
        run: ls -la dist/

      - name: Build release notes
        id: notes
        run: |
          VERSION='${{ needs.resolve-inputs.outputs.version }}'
          UPSTREAM='${{ needs.resolve-inputs.outputs.upstream_ref }}'
          FORK_SHA='${{ github.sha }}'
          {
            echo "**Fork version:** \`$VERSION\`"
            echo ""
            echo "**Upstream base:** \`$UPSTREAM\` ([rtk-ai/rtk@$UPSTREAM](https://github.com/rtk-ai/rtk/releases/tag/$UPSTREAM))"
            echo ""
            echo "**Fork commit:** \`$FORK_SHA\`"
            echo ""
            echo "**Applied patches:**"
            for p in patches/*.patch; do
              SUBJ=$(grep -m1 '^Subject:' "$p" | sed 's/^Subject: \[PATCH\] //')
              echo "- $SUBJ (\`$(basename "$p")\`)"
            done
          } > notes.md
          cat notes.md

      - name: Create tag
        run: |
          VERSION='${{ needs.resolve-inputs.outputs.version }}'
          TAG="v$VERSION"
          git config user.email "fork-release@buni.local"
          git config user.name "fork-release"
          if git rev-parse "$TAG" >/dev/null 2>&1; then
            echo "tag $TAG already exists; refusing to overwrite" >&2
            exit 1
          fi
          git tag -a "$TAG" -m "Fork release $VERSION"
          git push origin "$TAG"

      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          VERSION='${{ needs.resolve-inputs.outputs.version }}'
          PRE=''
          if [ '${{ inputs.prerelease }}' = 'true' ]; then PRE='--prerelease'; fi
          gh release create "v$VERSION" \
            --title "rtk-fork v$VERSION" \
            --notes-file notes.md \
            $PRE \
            dist/*
```

- [ ] **Step 2: Sanity-lint the YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "YAML OK"
```

Expected: `YAML OK`.

No commit this task.

---

## Task 9: Write `README.fork.md`

**Files:**
- Create: `README.fork.md`

- [ ] **Step 1: Create the file**

Create `/home/hristo/rtk/README.fork.md`:

```markdown
# rtk — buni fork

This is a fork of [rtk-ai/rtk](https://github.com/rtk-ai/rtk). It carries additional fork-specific features (starting with alias support) as `git am`-applicable patches rather than a divergent source tree.

## Layout

- `UPSTREAM_REF` — upstream tag this fork targets (e.g. `v0.31.0`)
- `VERSION` — fork version string (e.g. `0.31.0-alias.1`)
- `patches/` — fork-specific `.patch` files, applied in lexical order
- `scripts/apply-patches.sh` — clones upstream, applies patches
- `.github/workflows/ci.yml` — CI: patch-apply check + tests
- `.github/workflows/release.yml` — workflow_dispatch: publishes GitHub Release binaries

## Building locally

```bash
./scripts/apply-patches.sh build
cd build && cargo build --release
./target/release/rtk --version
```

## Editing a fork feature

```bash
./scripts/apply-patches.sh build
# edit files in build/, commit locally
cd build && git commit -am "tweak alias handling"
# regenerate patch
git format-patch v0.31.0..HEAD -o ../patches/
cd ..
# commit updated patch on fork branch
git add patches/ && git commit -m "update alias patch"
```

## Bumping upstream

1. Edit `UPSTREAM_REF` (e.g. `v0.31.0` → `v0.32.0`).
2. `./scripts/apply-patches.sh build`.
3. If clean → commit `UPSTREAM_REF`.
4. If conflict → resolve in `build/`, regenerate patch as above, commit both.

## Cutting a release

1. Bump `VERSION` (e.g. `0.31.0-alias.1` → `0.31.0-alias.2`).
2. Commit and push to `fork`.
3. GitHub Actions → Release workflow → Run workflow.
4. Release appears at <https://github.com/buni/rtk/releases>.
```

- [ ] **Step 2: Verify**

```bash
head -5 README.fork.md
```

Expected: starts with `# rtk — buni fork`.

---

## Task 10: Initial commit on `fork` branch

**Files:**
- Commit all files staged above except the unrelated `docs/superpowers/` spec+plan.

- [ ] **Step 1: Review what will be committed**

```bash
git status --short
```

Expected: untracked files —
```
?? .github/workflows/ci.yml
?? .github/workflows/release.yml
?? README.fork.md
?? UPSTREAM_REF
?? VERSION
?? docs/superpowers/
?? patches/
?? scripts/apply-patches.sh
```

- [ ] **Step 2: Stage fork infrastructure (not the spec/plan docs)**

```bash
git add \
  UPSTREAM_REF \
  VERSION \
  patches/ \
  scripts/apply-patches.sh \
  .github/workflows/ci.yml \
  .github/workflows/release.yml \
  README.fork.md
git status --short
```

Expected: the above files staged (`A`), `docs/superpowers/` still untracked (`??`).

- [ ] **Step 3: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: initialize fork branch with patch-based build

Orphan branch holding only fork-specific files. Upstream source is cloned
at build time from UPSTREAM_REF and patches/*.patch are applied before
cargo build runs. See README.fork.md for the workflow.

- UPSTREAM_REF: v0.31.0
- VERSION: 0.31.0-alias.1
- patches/: alias-support feature
- scripts/apply-patches.sh: upstream clone + git am
- .github/workflows/ci.yml: patch-check + test on Linux/macOS
- .github/workflows/release.yml: workflow_dispatch multi-platform release
EOF
)"
```

Expected: commit succeeds. Pre-commit hooks (if any) run on the tiny changeset; should pass since no Rust code is being committed.

- [ ] **Step 4: Separately commit the spec + plan**

```bash
git add docs/superpowers/specs/2026-04-16-rtk-fork-patch-build-design.md
git add docs/superpowers/plans/2026-04-16-rtk-fork-patch-build.md
git commit -m "docs: fork build design and implementation plan"
git log --oneline
```

Expected: two commits on `fork` branch.

---

## Task 11: Push `fork` branch and verify CI

**Files:**
- None (remote push only).

- [ ] **Step 1: Push branch**

```bash
git push -u origin fork
```

Expected: branch pushed, tracking set. If the remote already has a `fork` branch with different content, STOP and ask the user — do not force-push.

- [ ] **Step 2: Watch CI**

```bash
gh run watch -R buni/rtk --workflow=CI || gh run list -R buni/rtk --workflow=CI --limit 1
```

Expected: CI run appears. `patch-check` job succeeds. `test` matrix (ubuntu + macos) succeeds. If it fails, read logs, fix on local `fork` branch, push, repeat.

- [ ] **Step 3: Confirm final status**

```bash
gh run list -R buni/rtk --workflow=CI --limit 1
```

Expected: status `completed`, conclusion `success`.

No commit this task (unless fixing CI required one).

---

## Task 12: Test the release workflow

**Files:**
- None (remote workflow dispatch).

- [ ] **Step 1: Dispatch the release workflow**

```bash
gh workflow run Release -R buni/rtk --ref fork
```

Expected: `✓ Created workflow_dispatch event`.

- [ ] **Step 2: Watch the run**

```bash
gh run list -R buni/rtk --workflow=Release --limit 1
RUN_ID=$(gh run list -R buni/rtk --workflow=Release --limit 1 --json databaseId -q '.[0].databaseId')
gh run watch -R buni/rtk "$RUN_ID"
```

Expected: resolve-inputs → 5-target build matrix → release job. Total ~8-15 min.

- [ ] **Step 3: Verify the release**

```bash
gh release view "v$(cat VERSION)" -R buni/rtk
```

Expected: release page shows all 5 archives + `.sha256` files, body lists upstream ref, fork SHA, applied patches.

- [ ] **Step 4: Smoke-test one archive**

```bash
TMP=$(mktemp -d)
cd "$TMP"
gh release download "v$(cd /home/hristo/rtk && cat VERSION)" -R buni/rtk -p '*x86_64-unknown-linux-musl*'
tar xzf rtk-*.tar.gz
./rtk-*/rtk --version
cd - && rm -rf "$TMP"
```

Expected: `rtk <VERSION>` (with the fork-specific suffix). Confirms the Cargo.toml override worked end-to-end.

---

## Task 13: Change default branch on GitHub and retire `alias-extension`

**Files:**
- None (GitHub settings + local cleanup).

- [ ] **Step 1: Set `fork` as default branch**

```bash
gh repo edit buni/rtk --default-branch fork
```

Expected: default branch is `fork`.

- [ ] **Step 2: Delete local alias-extension branch**

```bash
git branch -d alias-extension
```

Expected: `Deleted branch alias-extension`. If git refuses (unmerged), the branch content is already in `patches/` so:

```bash
git branch -D alias-extension
```

- [ ] **Step 3: Delete remote alias-extension branch**

```bash
git push origin --delete alias-extension
```

Expected: deletion succeeds.

- [ ] **Step 4: Confirm final state**

```bash
git branch -a
gh repo view buni/rtk --json defaultBranchRef -q .defaultBranchRef.name
```

Expected: local branches show `fork` as current, `master` still present (mirrors upstream), no `alias-extension`. Default branch on GitHub is `fork`.

---

## Post-Implementation Verification

Run all of these from the freshly-checked-out `fork` branch:

- [ ] `scripts/apply-patches.sh /tmp/rtk-verify` succeeds, HEAD shows alias commit
- [ ] `rm -rf /tmp/rtk-verify && cd /tmp && git clone -b fork git@github.com:buni/rtk.git rtk-fork && cd rtk-fork && ./scripts/apply-patches.sh build` succeeds on a clean clone
- [ ] Latest CI run on `fork` is green
- [ ] Latest Release run produced 5 archives + checksums
- [ ] `rtk --version` from a downloaded binary prints the fork version string
- [ ] GitHub default branch is `fork`
- [ ] `alias-extension` branch is gone locally and remotely

---

## Follow-ups (not in this plan)

- Decide whether to build DEB/RPM packages (upstream `release.yml` has templates that could be merged back in).
- Decide whether to publish `cargo binstall`-compatible metadata.
- Consider whether to add a scheduled "upstream drift check" workflow that rebases patches against upstream master nightly and opens an issue on failure.
