//! Built-in tool adapters.
//!
//! An [`Adapter`] is the small amount of per-tool knowledge harness-router needs to launch a CLI
//! with an isolated account: which binary to run, and how to point that tool at a profile-specific
//! set of credentials. Isolation is expressed in one of two ways:
//!
//! * **env-var dirs** — set an environment variable to a per-profile directory (e.g. Claude's
//!   `CLAUDE_CONFIG_DIR`, Codex's `CODEX_HOME`, Copilot's `COPILOT_HOME`, opencode's `XDG_*`). A
//!   tool that hardcodes a `$HOME`-relative path with no relocation var (e.g. Antigravity's
//!   `~/.gemini`) is isolated by redirecting `HOME` itself.
//! * **arg dirs** — pass a CLI flag pointing at a per-profile directory (e.g. a VS Code-derived
//!   IDE's `--user-data-dir`).
//!
//! Adding support for a new tool is just adding an entry to [`ADAPTERS`].

/// How harness-router launches a given CLI and isolates its credentials.
#[derive(Debug)]
pub struct Adapter {
    /// Canonical tool name (the registry key), e.g. `"claude"`.
    pub name: &'static str,
    /// Executable to launch (looked up on `PATH`).
    pub binary: &'static str,
    /// Alternate names accepted on the command line.
    pub aliases: &'static [&'static str],
    /// One-line description shown by `hr tools` / `hr ls`.
    pub about: &'static str,
    /// OAuth/dir isolation via environment variables: `(ENV_VAR, subdir-under-profile-data-dir)`.
    pub env_dirs: &'static [(&'static str, &'static str)],
    /// OAuth/dir isolation via a CLI flag: `(flag, subdir-under-profile-data-dir)`.
    pub arg_dirs: &'static [(&'static str, &'static str)],
    /// Default env var(s) that should carry the API key for an API profile.
    pub api_key_env: &'static [&'static str],
    /// Env var that carries a custom base URL for an API profile, if the tool supports one.
    pub base_url_env: Option<&'static str>,
    /// Args appended when running `hr login <tool> <profile>` (the tool's own login flow).
    pub login_args: &'static [&'static str],
    /// Env vars cleared when launching an OAuth profile, so a stray *global* API key in the
    /// environment doesn't silently override the profile's logged-in account.
    pub unset_for_oauth: &'static [&'static str],
    /// Experimental adapters print a warning: their isolation hasn't been verified on real installs.
    pub experimental: bool,
}

impl Adapter {
    /// Human-readable summary of how this adapter isolates accounts (for `hr tools`).
    pub fn isolation_summary(&self) -> String {
        let mut parts = Vec::new();
        for (var, _) in self.env_dirs {
            parts.push(format!("${var}"));
        }
        for (flag, _) in self.arg_dirs {
            parts.push(format!("{flag} <dir>"));
        }
        if parts.is_empty() {
            parts.push("API key only (no OAuth isolation)".to_string());
        }
        parts.join(", ")
    }
}

/// All built-in adapters. Order here is purely cosmetic (drives `hr tools` output).
pub const ADAPTERS: &[Adapter] = &[
    Adapter {
        name: "claude",
        binary: "claude",
        aliases: &["cc"],
        about: "Claude Code (Anthropic)",
        env_dirs: &[("CLAUDE_CONFIG_DIR", "claude")],
        arg_dirs: &[],
        api_key_env: &["ANTHROPIC_API_KEY"],
        base_url_env: Some("ANTHROPIC_BASE_URL"),
        login_args: &["auth", "login"],
        // API key / tokens take precedence over OAuth in Claude Code, so clear them for OAuth profiles.
        unset_for_oauth: &[
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
        ],
        experimental: false,
    },
    Adapter {
        name: "codex",
        binary: "codex",
        aliases: &[],
        about: "OpenAI Codex CLI",
        // CODEX_HOME relocates everything: auth.json, config.toml, sessions and logs.
        env_dirs: &[("CODEX_HOME", "codex")],
        arg_dirs: &[],
        api_key_env: &["OPENAI_API_KEY"],
        base_url_env: None,
        login_args: &["login"],
        unset_for_oauth: &["OPENAI_API_KEY"],
        experimental: false,
    },
    Adapter {
        name: "opencode",
        binary: "opencode",
        aliases: &["oc"],
        about: "opencode (terminal AI coding agent)",
        // opencode follows XDG: auth lives at $XDG_DATA_HOME/opencode, config at $XDG_CONFIG_HOME/opencode.
        env_dirs: &[
            ("XDG_DATA_HOME", "xdg-data"),
            ("XDG_CONFIG_HOME", "xdg-config"),
        ],
        arg_dirs: &[],
        // Provider-specific; API profiles must declare the env var with --key-env.
        api_key_env: &[],
        base_url_env: None,
        login_args: &["auth", "login"],
        unset_for_oauth: &[],
        experimental: false,
    },
    Adapter {
        name: "grok",
        binary: "grok",
        aliases: &[],
        about: "Grok CLI (xAI) — API-key based",
        env_dirs: &[],
        arg_dirs: &[],
        api_key_env: &["XAI_API_KEY", "GROK_API_KEY"],
        base_url_env: None,
        login_args: &[],
        unset_for_oauth: &[],
        experimental: false,
    },
    Adapter {
        name: "copilot",
        binary: "copilot",
        aliases: &["gh-copilot"],
        about: "GitHub Copilot CLI",
        // COPILOT_HOME relocates the entire ~/.copilot tree: auth token, config, sessions and logs.
        env_dirs: &[("COPILOT_HOME", "copilot")],
        arg_dirs: &[],
        // Copilot's "key" is a GitHub token; COPILOT_GITHUB_TOKEN has the highest precedence.
        api_key_env: &["COPILOT_GITHUB_TOKEN"],
        base_url_env: None,
        // Login is the in-TUI `/login` slash command on first run — no standalone subcommand.
        login_args: &[],
        // Copilot prefers a token from the environment over its stored login, so clear any stray
        // global tokens for OAuth profiles or they'd silently override the profile's account.
        unset_for_oauth: &["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"],
        experimental: false,
    },
    Adapter {
        name: "antigravity",
        binary: "agy",
        aliases: &["agy", "ag"],
        about: "Google Antigravity CLI (agy) — Gemini CLI's successor",
        // agy is a terminal agent (not a VS Code IDE) and hardcodes everything under ~/.gemini with
        // no relocation env var, so the only way to isolate accounts is to redirect HOME itself.
        env_dirs: &[("HOME", "home")],
        arg_dirs: &[],
        // agy dropped Gemini CLI's GEMINI_API_KEY support — OAuth (Google sign-in) is the only path.
        api_key_env: &[],
        base_url_env: None,
        // Login is the browser-based Google sign-in triggered automatically on first run.
        login_args: &[],
        unset_for_oauth: &[],
        // HOME-redirect isolates the Linux token file under ~/.gemini, but not the macOS Keychain
        // entry ("Antigravity Safe Storage"); and the fake HOME hides ~/.gitconfig, ~/.ssh, etc.
        experimental: true,
    },
];

/// Find a built-in adapter by canonical name or alias.
pub fn find(name: &str) -> Option<&'static Adapter> {
    ADAPTERS
        .iter()
        .find(|a| a.name == name || a.aliases.contains(&name))
}

/// Comma-separated list of built-in tool names (for error messages).
pub fn names() -> String {
    ADAPTERS
        .iter()
        .map(|a| a.name)
        .collect::<Vec<_>>()
        .join(", ")
}
