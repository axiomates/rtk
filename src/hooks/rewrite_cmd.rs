//! Translates a raw shell command into its RTK-optimized equivalent.

use crate::discover::registry;
use std::io::Write;

/// Run the `rtk rewrite` command.
///
/// This Axiomate branch treats rewrite as a pure service endpoint. Axiomate has
/// its own permission resolver and does not use Claude Code hook permission
/// verdicts, so `rtk rewrite` must not read Claude settings or return advisory
/// ask/deny statuses.
///
/// | Exit | Stdout   | Meaning                                                      |
/// |------|----------|--------------------------------------------------------------|
/// | 0    | rewritten| Rewrite found.                                               |
/// | 1    | (none)   | No RTK equivalent — caller should run original command.      |
pub fn run(cmd: &str) -> anyhow::Result<()> {
    let (excluded, transparent_prefixes) = crate::core::config::Config::load()
        .map(|c| (c.hooks.exclude_commands, c.hooks.transparent_prefixes))
        .unwrap_or_default();

    match registry::rewrite_command(cmd, &excluded, &transparent_prefixes) {
        Some(rewritten) => {
            print!("{}", rewritten);
            let _ = std::io::stdout().flush();
            Ok(())
        }
        None => {
            // No RTK equivalent. Exit 1 = passthrough.
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite_command_no_prefixes(cmd: &str) -> Option<String> {
        registry::rewrite_command(cmd, &[], &[])
    }

    #[test]
    fn test_run_supported_command_succeeds() {
        assert!(rewrite_command_no_prefixes("git status").is_some());
    }

    #[test]
    fn test_run_unsupported_returns_none() {
        assert!(rewrite_command_no_prefixes("htop").is_none());
    }

    #[test]
    fn test_run_already_rtk_returns_some() {
        assert_eq!(
            rewrite_command_no_prefixes("rtk git status"),
            Some("rtk git status".into())
        );
    }

    #[test]
    fn test_axiomate_rewrite_default_permission_is_rewrite() {
        assert_eq!(
            registry::rewrite_command("git status", &[], &[]),
            Some("rtk git status".into())
        );
    }
}
