# rtk — buni fork

This is a fork of [rtk-ai/rtk](https://github.com/rtk-ai/rtk). It carries additional fork-specific features (starting with alias support) as `git am`-applicable patches rather than a divergent source tree.

## Layout

- `UPSTREAM_REPO` — upstream repo URL (default `https://github.com/rtk-ai/rtk.git`; set to any fork URL to rebase on a different upstream)
- `UPSTREAM_REF` — upstream tag this fork targets (e.g. `v0.31.0`)
- `VERSION` — fork version string (e.g. `0.31.0-alias.1`)
- `patches/` — fork-specific `.patch` files, applied in lexical order
- `PATCH.md` — agent-friendly description of the fork feature; used to re-implement if the patch fails to rebase
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
2. Optionally edit `UPSTREAM_REPO` to track a different fork (e.g. `https://github.com/other/rtk.git`).
3. `./scripts/apply-patches.sh build`.
4. If clean → commit `UPSTREAM_REF` (and `UPSTREAM_REPO` if changed).
5. If conflict → resolve in `build/`, regenerate patch as above, commit both.

## Building against a different upstream (ad-hoc)

Without editing files, override via env:

```bash
UPSTREAM_REMOTE=https://github.com/other/rtk.git \
  ./scripts/apply-patches.sh build v0.32.0
```

The release workflow exposes the same knob as a `upstream_repo` input in its `workflow_dispatch` form.

## Cutting a release

1. Bump `VERSION` (e.g. `0.31.0-alias.1` → `0.31.0-alias.2`).
2. Commit and push to `fork`.
3. GitHub Actions → Release workflow → Run workflow.
4. Release appears at <https://github.com/buni/rtk/releases>.
