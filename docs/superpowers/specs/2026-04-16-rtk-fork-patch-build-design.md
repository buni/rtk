# RTK Fork — Patch-Based Build & Release Design

**Date:** 2026-04-16
**Fork repo:** `buni/rtk`
**Upstream:** `rtk-ai/rtk`
**Status:** Approved, pending implementation plan

## Goal

Maintain a lean fork of `rtk-ai/rtk` that carries fork-specific changes (starting with the alias-support feature, commit `89888b7`) as reviewable `.patch` files rather than long-lived divergent branches. CI verifies patches apply to a pinned upstream tag and tests pass; a manual workflow builds multi-platform binaries and publishes them as GitHub Releases on the fork.

## Non-Goals

- No publishing to crates.io.
- No container images (GHCR).
- No DEB/RPM packages (can be added later; upstream `release.yml` has templates).
- No Homebrew tap / Scoop bucket / AUR auto-publish.
- No binary signing (cosign, sigstore).
- No automatic `UPSTREAM_REF` bumps or scheduled drift checks.
- No Windows in CI (only in release builds).

## Repo Layout — `fork` Branch

The fork's default branch is `fork` (not `master`). It contains **only** fork-specific files; all upstream source is fetched at build time.

```
buni/rtk (branch: fork)
├── patches/
│   └── 0001-add-alias-support-in-config.patch
├── .github/workflows/
│   ├── ci.yml           # push/PR: apply patches, build, test
│   └── release.yml      # workflow_dispatch: multi-platform build + GH Release
├── UPSTREAM_REF         # single line, e.g. `v0.31.0`
├── VERSION              # single line, e.g. `0.31.0+alias.1`
├── scripts/
│   └── apply-patches.sh
└── README.fork.md
```

The fork's `master` branch may keep mirroring upstream for convenience, but CI/release workflows only run on `fork`.

## Patch Workflow

### One-time bootstrap

```bash
git checkout alias-extension
git format-patch v0.31.0..HEAD -o patches/
# Move patches/ onto a new `fork` branch, delete `alias-extension`.
```

### `scripts/apply-patches.sh BUILD_DIR`

1. `git clone --depth 1 --branch "$(cat UPSTREAM_REF)" https://github.com/rtk-ai/rtk "$BUILD_DIR"`
2. `cd "$BUILD_DIR" && git am "$FORK_ROOT"/patches/*.patch`
3. Non-zero exit if `git am` rejects any hunk. Never `--3way`, never `--reject`.

### Editing the alias feature

1. Run `apply-patches.sh build/` locally.
2. Edit code in `build/`, commit there.
3. `cd build && git format-patch v0.31.0..HEAD -o ../patches/`.
4. Commit updated patches on the `fork` branch.

### Bumping upstream

1. Edit `UPSTREAM_REF` (e.g., `v0.31.0` → `v0.32.0`).
2. Run `apply-patches.sh` locally.
3. Clean apply → commit the new `UPSTREAM_REF`.
4. Conflicts → resolve in built tree, regenerate patch, commit both files.

## CI Workflow — `.github/workflows/ci.yml`

**Triggers:** `push` and `pull_request` on the `fork` branch.

### Job 1: `patch-check` (ubuntu-latest)

- Checkout fork repo.
- Run `scripts/apply-patches.sh build/`.
- Fails loudly if patches don't apply to the pinned upstream tag.

### Job 2: `test` — needs `patch-check`

- Matrix: `ubuntu-latest`, `macos-latest`.
- Same checkout + apply steps.
- `cd build && cargo fmt --all --check`
- `cd build && cargo clippy --all-targets -- -D warnings`
- `cd build && cargo test --all`
- Caches keyed on `hashFiles('UPSTREAM_REF', 'patches/**')`:
  - `~/.cargo/registry`
  - `~/.cargo/git`
  - `build/target`

Windows is intentionally excluded from CI — release workflow still builds Windows binaries, but keeping CI Linux+macOS holds wall time under ~3 minutes.

## Release Workflow — `.github/workflows/release.yml`

**Trigger:** `workflow_dispatch` only.

### Inputs

| Name | Default | Purpose |
|------|---------|---------|
| `upstream_ref` | `$(cat UPSTREAM_REF)` | Override pinned tag for this build only |
| `version` | `$(cat VERSION)` | Fork version string (used in tag, release title, `Cargo.toml`) |
| `prerelease` | `false` | GitHub Release prerelease flag |

### Job 1: `build`

Matrix of 5 targets (same as upstream):
- `x86_64-apple-darwin` (macos-latest)
- `aarch64-apple-darwin` (macos-latest)
- `x86_64-unknown-linux-musl` (ubuntu-latest, musl toolchain)
- `aarch64-unknown-linux-gnu` (ubuntu-latest, `cross`)
- `x86_64-pc-windows-msvc` (windows-latest)

Steps per target:
1. Checkout fork.
2. Clone upstream at `inputs.upstream_ref` into `build/`.
3. `scripts/apply-patches.sh build/`.
4. Override `Cargo.toml` version in `build/Cargo.toml` to `inputs.version` (so `rtk --version` prints the fork version).
5. `cargo build --release --target <target>` inside `build/`.
6. Strip binary (Unix: `strip`; Windows: natively stripped by release profile).
7. Archive: `tar.gz` for Unix, `zip` for Windows.
8. Compute `sha256` of each archive.
9. Upload archive + `.sha256` as workflow artifacts.

### Job 2: `release` — needs `build`

- Download all artifacts.
- Create git tag `v${{ inputs.version }}` on the `fork` branch HEAD, push to fork.
- `gh release create` on `buni/rtk` (not upstream).
- Release body auto-generated:
  - Upstream base: `inputs.upstream_ref`
  - Fork commit: `${{ github.sha }}`
  - Applied patches: list of files under `patches/`
- Attach all archives + `.sha256` files.
- Honor `inputs.prerelease`.

### Permissions

- `contents: write` on the fork repo only.
- No external registry credentials.

## Error Handling

- **Patch apply failure (CI or Release):** job fails, no partial build published. User must resolve locally and push fixed patches.
- **Test failure in CI:** blocks PR merge. No override.
- **Build failure for one target in release:** fails whole workflow; no partial release published. Resolve, re-dispatch.
- **Tag already exists:** workflow fails before publishing (prevents accidental overwrite). Bump `VERSION` and retry.

## Versioning Scheme

- `VERSION` file holds `<upstream>+<fork-suffix>`, e.g., `0.31.0+alias.1`.
- Semver interprets `+alias.1` as build metadata — tooling treats it as equal to `0.31.0` but human-readable as the fork variant.
- Bump `alias.1` → `alias.2` when patches change without upstream bump.
- When `UPSTREAM_REF` bumps, reset suffix: `0.32.0+alias.1`.

## Open Questions (Deferred)

- Whether to add DEB/RPM package builds (yes/no decision at first release).
- Whether to add `cargo binstall` compatibility metadata to release artifacts.
- Whether to auto-open a PR on `UPSTREAM_REF` bump that updates patches (would require a bot, out of scope for v1).

## Acceptance Criteria

1. `fork` branch exists with `patches/`, `UPSTREAM_REF`, `VERSION`, `scripts/apply-patches.sh`, and both workflows.
2. `alias-extension` branch content exists as `patches/0001-*.patch`; working copy of fork branch has no `src/`.
3. Opening a PR against `fork` triggers `ci.yml`: patches apply, tests pass on Linux + macOS.
4. Running `release.yml` via workflow_dispatch produces a GitHub Release on `buni/rtk` with 5 platform archives + checksums, tagged `v<VERSION>`.
5. Downloaded binary prints `rtk <VERSION>` from `rtk --version`.
6. Bumping `UPSTREAM_REF` + rerunning CI exercises the upstream-bump workflow (either clean apply or surfaced conflict).
