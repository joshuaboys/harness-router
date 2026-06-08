//! Profile registry and secure secret storage.
//!
//! Non-secret metadata lives in a single inspectable TOML file:
//! `$XDG_CONFIG_HOME/harness-router/config.toml` (e.g. `~/.config/harness-router/config.toml`).
//!
//! Per-profile state — the isolated OAuth config dirs, and the API key for API profiles — lives
//! under `$XDG_DATA_HOME/harness-router/profiles/<tool>/<profile>/`. API keys are written to an
//! `api_key` file with `0600` permissions and never stored in the registry.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Whether a profile authenticates via an isolated OAuth login or a stored API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Oauth,
    Api,
}

/// A single named account for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub kind: Kind,
    /// Custom endpoint (e.g. GLM/OpenRouter) for API profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Override which env var name(s) the API key is exported as (defaults to the adapter's).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_env: Vec<String>,
    /// Extra, non-secret environment variables to set when launching this profile.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

/// The whole registry: `tool name -> (profile name -> profile)`.
///
/// Serialized transparently so the on-disk TOML is just nested tables, e.g.
/// `[claude.home]` / `kind = "oauth"`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Registry(pub BTreeMap<String, BTreeMap<String, Profile>>);

impl Registry {
    pub fn tools(&self) -> &BTreeMap<String, BTreeMap<String, Profile>> {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, tool: &str, profile: &str) -> Option<&Profile> {
        self.0.get(tool).and_then(|m| m.get(profile))
    }

    pub fn profile_names(&self, tool: &str) -> Vec<String> {
        self.0
            .get(tool)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn insert(&mut self, tool: &str, profile: &str, p: Profile) {
        self.0
            .entry(tool.to_string())
            .or_default()
            .insert(profile.to_string(), p);
    }

    pub fn remove(&mut self, tool: &str, profile: &str) -> Option<Profile> {
        let map = self.0.get_mut(tool)?;
        let removed = map.remove(profile);
        if map.is_empty() {
            self.0.remove(tool);
        }
        removed
    }
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine your config directory")?;
    Ok(base.join("harness-router").join("config.toml"))
}

pub fn data_root() -> Result<PathBuf> {
    let base = dirs::data_dir().context("could not determine your data directory")?;
    Ok(base.join("harness-router"))
}

pub fn profile_data_dir(tool: &str, profile: &str) -> Result<PathBuf> {
    Ok(data_root()?.join("profiles").join(tool).join(profile))
}

pub fn secret_path(tool: &str, profile: &str) -> Result<PathBuf> {
    Ok(profile_data_dir(tool, profile)?.join("api_key"))
}

pub fn load() -> Result<Registry> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Registry::default());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

pub fn save(reg: &Registry) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        create_dir_secure(parent)?;
    }
    let text = toml::to_string_pretty(reg).context("serializing config")?;
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn read_secret(tool: &str, profile: &str) -> Result<String> {
    let path = secret_path(tool, profile)?;
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "reading the API key for {tool}/{profile} at {} (re-add it with `hr add {tool} {profile} --api`)",
            path.display()
        )
    })?;
    Ok(text.trim().to_string())
}

pub fn write_secret(tool: &str, profile: &str, key: &str) -> Result<()> {
    let dir = profile_data_dir(tool, profile)?;
    create_dir_secure(&dir)?;
    let path = dir.join("api_key");

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts
        .open(&path)
        .with_context(|| format!("writing {}", path.display()))?;
    use std::io::Write;
    file.write_all(key.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Create a directory (and parents) restricted to the current user where the platform supports it.
pub fn create_dir_secure(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}
