# Fork Patch: Alias Support in Config

Agent-friendly description of the fork-specific feature carried by the sibling `.patch` file. If upstream drift makes the patch impossible to rebase, use this document to re-implement the feature against the new upstream.

## Feature Summary

RTK normally filters commands it recognizes (`rtk pytest`, `rtk cargo test`, etc.). This fork adds a **`[aliases]` section in `~/.config/rtk/config.toml`** so users can register arbitrary command prefixes that should be filtered through a named RTK filter, **without changing how the underlying command is invoked**.

### Why this matters

Many Python/Rust workflows wrap commands through tools (`uv run`, `poetry run`, `make`, `just`, `mise exec`). RTK's `rtk pytest` wrapper doesn't know about `uv run pytest`, so users lose filtering. Aliases close that gap without forcing users to learn a new invocation.

### User-facing example

```toml
# ~/.config/rtk/config.toml
[aliases]
"uv run pytest" = "pytest"
"uv run mypy"   = "mypy"
"make test"     = "pytest"
"make lint"     = "mypy"
"just check"    = "cargo clippy"
```

After adding this, running `uv run pytest tests/` still invokes the real command (preserving venv activation), but RTK captures stdout and filters it through `pytest_cmd::filter_pytest_output` before printing.

## Supported Filter Names

The right-hand side of an alias must be one of these names (the `apply_named_filter` match arms in `src/main.rs`):

| Name | Filter function |
|------|-----------------|
| `pytest` | `pytest_cmd::filter_pytest_output` |
| `mypy` | `mypy_cmd::filter_mypy_output` |
| `ruff check` or `ruff` | `ruff_cmd::filter_ruff_check_json` |
| `ruff format` | `ruff_cmd::filter_ruff_format` |
| `cargo test` | `cargo_cmd::filter_cargo_test` |

Unknown names fall back to printing raw stdout (no filtering, no crash).

## Matching Rules

- **Longest-prefix wins.** If both `"uv run"` and `"uv run pytest"` match, the longer alias target is used.
- **Word-boundary match.** Alias `"make"` matches `"make test"` (space after prefix) or exact `"make"`, but not `"makefile"`.
- **Pre-rewrite check.** The hook in `src/rewrite_cmd.rs` checks aliases *before* RTK's normal command-detection, so `uv run pytest` rewrites to `rtk uv run pytest` and reaches `run_fallback` in `src/main.rs`.

## Re-Implementation Guide (if the patch breaks on future upstream)

### Files touched (by role, with v0.36.0 paths)

Since upstream keeps reorganizing (v0.31.0 used a flat `src/`, v0.34.2+ moved to the modular layout below), describe each edit by role and find the current file via `grep` if paths have moved again.

| Role | v0.31.0 path | v0.34.2+ path |
|------|--------------|---------------|
| `Config` struct | `src/config.rs` | `src/core/config.rs` |
| `run_fallback` | `src/main.rs` | `src/main.rs` |
| Hook rewrite entry | `src/rewrite_cmd.rs` | `src/hooks/rewrite_cmd.rs` |
| `filter_cargo_test` | `src/cargo_cmd.rs` | `src/cmds/rust/cargo_cmd.rs` |
| `filter_pytest_output` | `src/pytest_cmd.rs` | `src/cmds/python/pytest_cmd.rs` |
| `filter_mypy_output` | `src/mypy_cmd.rs` | `src/cmds/python/mypy_cmd.rs` |
| `filter_ruff_*` | `src/ruff_cmd.rs` | `src/cmds/python/ruff_cmd.rs` |

Quick find:
```bash
grep -rln "pub struct Config " src/
grep -rln "fn run_fallback" src/
grep -rln "fn filter_pytest_output\|fn filter_cargo_test" src/
```

### Edits per file

1. **Config module (`Config` struct)** — add the `AliasesConfig` type and a field on `Config`.
2. **`src/main.rs::run_fallback`** — inject alias check before TOML filter lookup; add `find_alias_match` and `apply_named_filter` helpers near other free functions.
3. **Hook rewrite (`rewrite_cmd.rs::run`)** — load full `Config` (instead of just `hooks.exclude_commands`), check aliases, feed the result into the existing permission-verdict match.
4. **`cargo_cmd.rs`** — change `filter_cargo_test` from `fn` to `pub fn`.
5. **`pytest_cmd.rs`** — change `filter_pytest_output` from `fn` to `pub fn`.

Already-public filters in v0.36.0 (no visibility edit needed): `mypy_cmd::filter_mypy_output`, `ruff_cmd::filter_ruff_check_json`, `ruff_cmd::filter_ruff_format`.

### Module import paths (v0.36.0)

Use these when porting; if upstream reorganizes again, `grep` for the names.

- `crate::core::config::Config`
- `crate::core::utils::resolved_command`
- `crate::core::utils::exit_code_from_output`
- `crate::core::tee::tee_and_hint`
- `crate::core::tracking::record_parse_failure_silent`
- `crate::core::tracking::TimedExecution`
- `crate::cmds::python::pytest_cmd::filter_pytest_output`
- `crate::cmds::python::mypy_cmd::filter_mypy_output`
- `crate::cmds::python::ruff_cmd::filter_ruff_check_json`
- `crate::cmds::python::ruff_cmd::filter_ruff_format`
- `crate::cmds::rust::cargo_cmd::filter_cargo_test`

### Step 1 — `src/config.rs`

Add this struct:

```rust
/// Command aliases: map a command prefix to an existing RTK filter name.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AliasesConfig {
    #[serde(flatten)]
    pub map: std::collections::HashMap<String, String>,
}
```

And add to `Config`:

```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub aliases: AliasesConfig,
}
```

### Step 2 — `src/main.rs`

Add two free functions near the bottom:

```rust
/// Find the longest alias prefix that matches `cmd`.
fn find_alias_match(
    cmd: &str,
    aliases: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for (prefix, target) in aliases {
        let matches = cmd == prefix.as_str() || cmd.starts_with(&format!("{} ", prefix));
        if matches {
            let len = prefix.len();
            if best.as_ref().map_or(true, |(bl, _)| len > *bl) {
                best = Some((len, target.clone()));
            }
        }
    }
    best.map(|(_, t)| t)
}

/// Apply a named RTK filter function to captured stdout.
fn apply_named_filter(name: &str, stdout: &str) -> Option<String> {
    match name {
        "pytest" => Some(pytest_cmd::filter_pytest_output(stdout)),
        "mypy" => Some(mypy_cmd::filter_mypy_output(stdout)),
        "ruff check" | "ruff" => Some(ruff_cmd::filter_ruff_check_json(stdout)),
        "ruff format" => Some(ruff_cmd::filter_ruff_format(stdout)),
        "cargo test" => Some(cargo_cmd::filter_cargo_test(stdout)),
        _ => None,
    }
}
```

In `run_fallback`, *after* computing `lookup_cmd` and *before* the TOML filter lookup, insert:

```rust
let alias_filter_name = {
    let aliases = config::Config::load()
        .map(|c| c.aliases.map)
        .unwrap_or_default();
    find_alias_match(&lookup_cmd, &aliases)
};

if let Some(ref filter_name) = alias_filter_name {
    let result = utils::resolved_command(&args[0])
        .args(&args[1..])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output();

    match result {
        Ok(output) => {
            let stdout_raw = String::from_utf8_lossy(&output.stdout);
            let tee_hint = if !output.status.success() {
                tee::tee_and_hint(&stdout_raw, &raw_command, output.status.code().unwrap_or(1))
            } else {
                None
            };
            let filtered = apply_named_filter(filter_name, &stdout_raw)
                .unwrap_or_else(|| stdout_raw.to_string());
            println!("{}", filtered);
            if let Some(hint) = tee_hint {
                println!("{}", hint);
            }
            timer.track(
                &raw_command,
                &format!("rtk:alias({}) {}", filter_name, raw_command),
                &stdout_raw,
                &filtered,
            );
            tracking::record_parse_failure_silent(&raw_command, &error_message, true);
            if !output.status.success() {
                std::process::exit(output.status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            tracking::record_parse_failure_silent(&raw_command, &error_message, false);
            eprintln!("[rtk: {}]", e);
            std::process::exit(127);
        }
    }
    return Ok(());
}
```

Names that must exist in upstream for this block to compile: `utils::resolved_command`, `tee::tee_and_hint`, `tracking::record_parse_failure_silent`, `timer.track`. If any have been renamed, adapt the calls.

### Step 3 — Hook rewrite (v0.36.0: `src/hooks/rewrite_cmd.rs`)

**Important — security:** modern upstream's `rewrite_cmd::run` evaluates a `PermissionVerdict` (Allow/Deny/Ask) before the rewrite. Aliased commands must go through the same verdict match as registry rewrites so they never bypass permission checks.

Replace the existing config load:

```rust
let excluded = crate::core::config::Config::load()
    .map(|c| c.hooks.exclude_commands)
    .unwrap_or_default();
```

with:

```rust
let config = crate::core::config::Config::load().unwrap_or_default();
let excluded = config.hooks.exclude_commands.clone();
```

Then, AFTER the Deny exit (so aliases inherit Deny behavior too) and BEFORE the `match registry::rewrite_command(...)` block, compute an alias rewrite option and feed it into the existing match:

```rust
let alias_rewritten: Option<String> = {
    let trimmed = cmd.trim();
    if !trimmed.starts_with("rtk ") {
        let any_match = config.aliases.map.keys().any(|prefix| {
            let p = prefix.as_str();
            trimmed == p || trimmed.starts_with(&format!("{} ", p))
        });
        if any_match {
            Some(format!("rtk {}", trimmed))
        } else {
            None
        }
    } else {
        None
    }
};

let resolved = alias_rewritten.or_else(|| registry::rewrite_command(cmd, &excluded));

match resolved {
    // ... existing Some(rewritten) => match verdict { ... } unchanged ...
}
```

This routes aliased commands through the same Allow/Ask/Default exit-code logic as registry rewrites — they never bypass security.

### Step 4 — Visibility changes

Change each of these from `fn` to `pub fn`:

- `src/cargo_cmd.rs`: `fn filter_cargo_test(output: &str) -> String`
- `src/pytest_cmd.rs`: `fn filter_pytest_output(output: &str) -> String`
- Any other filter function referenced by `apply_named_filter` that isn't already `pub`.

### Step 5 — Verify

After re-implementing:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Then manually test:

```bash
echo '[aliases]
"uv run pytest" = "pytest"' > /tmp/test-config.toml
RTK_CONFIG=/tmp/test-config.toml cargo run -- uv run pytest tests/
```

Should invoke `uv run pytest tests/` and print pytest output filtered through RTK's pytest filter.

### Regenerate the patch

Once working on fresh upstream:

```bash
git format-patch <new-upstream-tag>..HEAD -o patches/
rm patches/0001-feat-add-alias-support-in-config.patch  # the old one
# commit the new patch and updated UPSTREAM_REF
```

## Intentional Non-Features

These were considered and excluded:

- **Alias → raw shell command.** Only filter-name targets are supported. Aliasing `"ll"` → `"ls -la"` is out of scope; users should use shell aliases for that.
- **Per-project aliases.** Only `~/.config/rtk/config.toml` is consulted. Project-local `.rtk/config.toml` is not merged in this version.
- **Regex prefixes.** Alias keys are literal space-separated prefixes, not regex. Keep it simple.
