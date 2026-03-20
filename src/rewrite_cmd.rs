use crate::discover::registry;

/// Run the `rtk rewrite` command.
///
/// Prints the RTK-rewritten command to stdout and exits 0.
/// Exits 1 (without output) if the command has no RTK equivalent.
///
/// Used by shell hooks to rewrite commands transparently:
/// ```bash
/// REWRITTEN=$(rtk rewrite "$CMD") || exit 0
/// [ "$CMD" = "$REWRITTEN" ] && exit 0  # already RTK, skip
/// ```
pub fn run(cmd: &str) -> anyhow::Result<()> {
    let config = crate::config::Config::load().unwrap_or_default();
    let excluded = config.hooks.exclude_commands.clone();

    // Check user-defined aliases first.
    // An aliased command is rewritten to `rtk <original>` so that run_fallback
    // can execute the original command and apply the configured filter.
    let trimmed = cmd.trim();
    if !trimmed.starts_with("rtk ") {
        let aliases = &config.aliases.map;
        let alias_match = aliases.keys().filter(|prefix| {
            let p = prefix.as_str();
            trimmed == p || trimmed.starts_with(&format!("{} ", p))
        }).max_by_key(|p| p.len());

        if alias_match.is_some() {
            print!("rtk {}", trimmed);
            return Ok(());
        }
    }

    match registry::rewrite_command(cmd, &excluded) {
        Some(rewritten) => {
            print!("{}", rewritten);
            Ok(())
        }
        None => {
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_supported_command_succeeds() {
        assert!(registry::rewrite_command("git status", &[]).is_some());
    }

    #[test]
    fn test_run_unsupported_returns_none() {
        assert!(registry::rewrite_command("htop", &[]).is_none());
    }

    #[test]
    fn test_run_already_rtk_returns_some() {
        assert_eq!(
            registry::rewrite_command("rtk git status", &[]),
            Some("rtk git status".into())
        );
    }
}
