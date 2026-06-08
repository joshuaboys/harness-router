//! Turning a (tool, profile) pair into a concrete process launch.
//!
//! [`resolve`] is deliberately pure — it takes everything it needs as parameters and returns an
//! [`Invocation`] describing exactly what to run, which env vars to set/unset, and which directories
//! must exist first. That makes the interesting logic trivially unit-testable. [`exec`] is the thin,
//! side-effecting wrapper that creates the dirs and replaces the current process with the tool.

use std::path::Path;

use anyhow::Result;

use crate::adapter::Adapter;
use crate::config::{Kind, Profile};

/// A fully-resolved description of how to launch a tool for a given profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub env_set: Vec<(String, String)>,
    pub env_unset: Vec<String>,
    /// Directories that must exist before launch (the isolated profile dirs).
    pub ensure_dirs: Vec<String>,
}

/// Resolve a launch. Pure: no filesystem or environment access.
///
/// * `data_dir` is this profile's data directory; isolated dirs are created beneath it.
/// * `api_key` is supplied for API profiles only.
/// * `passthrough` are the user's extra args (forwarded to the tool).
/// * `login_args`, when `Some`, replace `passthrough` (used by `hr login`).
pub fn resolve(
    adapter: &Adapter,
    profile: &Profile,
    data_dir: &Path,
    api_key: Option<&str>,
    passthrough: &[String],
    login_args: Option<&[String]>,
) -> Invocation {
    let mut env_set: Vec<(String, String)> = Vec::new();
    let mut env_unset: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    let mut ensure_dirs: Vec<String> = Vec::new();

    // Directory isolation always applies when the adapter supports it — even for API profiles, so a
    // profile's session/config state never bleeds into your default login.
    for (var, subdir) in adapter.env_dirs {
        let dir = data_dir.join(subdir);
        let dir = dir.to_string_lossy().into_owned();
        env_set.push(((*var).to_string(), dir.clone()));
        ensure_dirs.push(dir);
    }
    for (flag, subdir) in adapter.arg_dirs {
        let dir = data_dir.join(subdir);
        let dir = dir.to_string_lossy().into_owned();
        args.push((*flag).to_string());
        args.push(dir.clone());
        ensure_dirs.push(dir);
    }

    match profile.kind {
        Kind::Oauth => {
            // Make sure a stray global API key can't override the profile's logged-in account.
            for var in adapter.unset_for_oauth {
                env_unset.push((*var).to_string());
            }
        }
        Kind::Api => {
            if let Some(key) = api_key {
                let names: Vec<String> = if profile.key_env.is_empty() {
                    adapter.api_key_env.iter().map(|s| s.to_string()).collect()
                } else {
                    profile.key_env.clone()
                };
                for name in names {
                    env_set.push((name, key.to_string()));
                }
            }
            if let Some(base) = &profile.base_url {
                if let Some(var) = adapter.base_url_env {
                    env_set.push((var.to_string(), base.clone()));
                }
            }
        }
    }

    // User-defined extras win over everything above.
    for (key, value) in &profile.env {
        env_set.push((key.clone(), value.clone()));
    }

    match login_args {
        Some(login) => args.extend(login.iter().cloned()),
        None => args.extend(passthrough.iter().cloned()),
    }

    Invocation {
        program: adapter.binary.to_string(),
        args,
        env_set,
        env_unset,
        ensure_dirs,
    }
}

/// Create the isolated dirs, then replace the current process with the tool.
///
/// On Unix this is a true `execvp`, so the tool inherits our TTY/signals directly and no wrapper
/// process lingers. On success it never returns.
pub fn exec(inv: &Invocation) -> Result<()> {
    for dir in &inv.ensure_dirs {
        crate::config::create_dir_secure(Path::new(dir))?;
    }

    let mut cmd = std::process::Command::new(&inv.program);
    cmd.args(&inv.args);
    for key in &inv.env_unset {
        cmd.env_remove(key);
    }
    for (key, value) in &inv.env_set {
        cmd.env(key, value);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Only returns on failure to launch.
        let err = cmd.exec();
        Err(anyhow::Error::from(err).context(format!(
            "failed to launch '{}' — is it installed and on your PATH?",
            inv.program
        )))
    }
    #[cfg(not(unix))]
    {
        use anyhow::Context;
        let status = cmd
            .status()
            .with_context(|| format!("failed to launch '{}'", inv.program))?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter;
    use std::collections::BTreeMap;

    fn prof(kind: Kind) -> Profile {
        Profile {
            kind,
            base_url: None,
            key_env: Vec::new(),
            env: BTreeMap::new(),
        }
    }

    fn has_env(inv: &Invocation, key: &str, value: &str) -> bool {
        inv.env_set.iter().any(|(k, v)| k == key && v == value)
    }

    #[test]
    fn claude_oauth_isolates_config_dir_and_unsets_keys() {
        let ad = adapter::find("claude").unwrap();
        let inv = resolve(
            ad,
            &prof(Kind::Oauth),
            Path::new("/data/claude/home"),
            None,
            &["--continue".to_string()],
            None,
        );
        assert_eq!(inv.program, "claude");
        assert!(has_env(
            &inv,
            "CLAUDE_CONFIG_DIR",
            "/data/claude/home/claude"
        ));
        assert!(inv.env_unset.iter().any(|k| k == "ANTHROPIC_API_KEY"));
        assert_eq!(inv.args, vec!["--continue".to_string()]);
        assert!(inv
            .ensure_dirs
            .iter()
            .any(|d| d == "/data/claude/home/claude"));
    }

    #[test]
    fn claude_api_sets_key_base_url_and_still_isolates_dir() {
        let ad = adapter::find("claude").unwrap();
        let mut p = prof(Kind::Api);
        p.base_url = Some("https://glm.example/anthropic".to_string());
        let inv = resolve(ad, &p, Path::new("/d"), Some("sk-test"), &[], None);
        assert!(has_env(&inv, "ANTHROPIC_API_KEY", "sk-test"));
        assert!(has_env(
            &inv,
            "ANTHROPIC_BASE_URL",
            "https://glm.example/anthropic"
        ));
        // Dir isolation still applies for API profiles.
        assert!(inv.env_set.iter().any(|(k, _)| k == "CLAUDE_CONFIG_DIR"));
        // API profiles must not clear the very key they depend on.
        assert!(inv.env_unset.is_empty());
    }

    #[test]
    fn antigravity_isolates_via_user_data_dir_arg() {
        let ad = adapter::find("ag").unwrap(); // alias resolves
        let inv = resolve(ad, &prof(Kind::Oauth), Path::new("/d"), None, &[], None);
        let joined = inv.args.join(" ");
        assert!(joined.contains("--user-data-dir"));
        assert!(joined.contains("/d/antigravity"));
        assert!(inv.ensure_dirs.iter().any(|d| d == "/d/antigravity"));
    }

    #[test]
    fn opencode_isolates_both_xdg_dirs() {
        let ad = adapter::find("opencode").unwrap();
        let inv = resolve(ad, &prof(Kind::Oauth), Path::new("/d"), None, &[], None);
        assert!(inv.env_set.iter().any(|(k, _)| k == "XDG_DATA_HOME"));
        assert!(inv.env_set.iter().any(|(k, _)| k == "XDG_CONFIG_HOME"));
    }

    #[test]
    fn explicit_key_env_overrides_adapter_default() {
        let ad = adapter::find("opencode").unwrap();
        let mut p = prof(Kind::Api);
        p.key_env = vec![
            "ANTHROPIC_API_KEY".to_string(),
            "OPENAI_API_KEY".to_string(),
        ];
        let inv = resolve(ad, &p, Path::new("/d"), Some("k"), &[], None);
        assert!(has_env(&inv, "ANTHROPIC_API_KEY", "k"));
        assert!(has_env(&inv, "OPENAI_API_KEY", "k"));
    }

    #[test]
    fn profile_env_is_applied_last() {
        let ad = adapter::find("grok").unwrap();
        let mut p = prof(Kind::Api);
        p.env
            .insert("XAI_API_KEY".to_string(), "override".to_string());
        let inv = resolve(ad, &p, Path::new("/d"), Some("from-secret"), &[], None);
        // adapter default sets XAI_API_KEY=from-secret first, profile env appends override last.
        let last = inv
            .env_set
            .iter()
            .rev()
            .find(|(k, _)| k == "XAI_API_KEY")
            .unwrap();
        assert_eq!(last.1, "override");
    }

    #[test]
    fn login_args_replace_passthrough() {
        let ad = adapter::find("codex").unwrap();
        let login = vec!["login".to_string()];
        let inv = resolve(
            ad,
            &prof(Kind::Oauth),
            Path::new("/d"),
            None,
            &["ignored".to_string()],
            Some(&login),
        );
        assert_eq!(inv.args, vec!["login".to_string()]);
        assert!(has_env(&inv, "CODEX_HOME", "/d/codex"));
    }
}
