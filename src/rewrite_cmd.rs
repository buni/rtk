use crate::discover::registry;
use crate::permissions::{check_command, PermissionVerdict};
use std::io::Write;

/// Run the `rtk rewrite` command.
///
/// Prints the RTK-rewritten command to stdout and exits with a code that tells
/// the caller how to handle permissions:
///
/// | Exit | Stdout   | Meaning                                                      |
/// |------|----------|--------------------------------------------------------------|
/// | 0    | rewritten| Rewrite allowed — hook may auto-allow the rewritten command. |
/// | 1    | (none)   | No RTK equivalent — hook passes through unchanged.           |
/// | 2    | (none)   | Deny rule matched — hook defers to Claude Code native deny.  |
/// | 3    | rewritten| Ask rule matched — hook rewrites but lets Claude Code prompt.|
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

    // SECURITY: check deny/ask BEFORE rewrite so non-RTK commands are also covered.
    let verdict = check_command(cmd);

    if verdict == PermissionVerdict::Deny {
        std::process::exit(2);
    }

    match registry::rewrite_command(cmd, &excluded) {
        Some(rewritten) => match verdict {
            PermissionVerdict::Allow => {
                print!("{}", rewritten);
                let _ = std::io::stdout().flush();
                Ok(())
            }
            PermissionVerdict::Ask => {
                print!("{}", rewritten);
                let _ = std::io::stdout().flush();
                std::process::exit(3);
            }
            PermissionVerdict::Deny => unreachable!(),
        },
        None => {
            // No RTK equivalent. Exit 1 = passthrough.
            // Claude Code independently evaluates its own ask rules on the original cmd.
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
