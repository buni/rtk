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
