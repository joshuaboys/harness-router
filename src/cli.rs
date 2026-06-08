//! Command-line surface (clap).
//!
//! The headline ergonomic — `hr <tool> <profile> [args...]` — is captured as an
//! `external_subcommand`, so unknown leading words like `claude` are routed to [`Command::Run`]
//! while real subcommands (`add`, `ls`, ...) keep working.

use clap::{Args, Parser, Subcommand};

const AFTER_HELP: &str = "\
EXAMPLES:
  hr claude                          launch your default (already-installed) account
  hr add claude home                 register an OAuth profile (then log in)
  hr login claude home               run Claude's login into the 'home' profile
  hr login codex home --device-auth  forward flags to the tool's login flow
  hr claude home                     launch Claude on the 'home' account
  hr claude work -p \"summarise\"       extra args pass straight through
  hr add claude glm --api \\
      --base-url https://open.bigmodel.cn/api/anthropic
  hr add opencode work --api --key-env ANTHROPIC_API_KEY
  hr ls                              list profiles
  hr which claude work               show which account a launch would use
  hr rm claude home --purge          remove a profile and delete its stored data
  hr tools                           show built-in tool adapters";

#[derive(Parser)]
#[command(
    name = "hr",
    version,
    about = "Switch between multiple OAuth/API accounts for AI coding CLIs",
    arg_required_else_help = true,
    after_help = AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Register a new profile for a tool
    Add(AddArgs),
    /// Remove a profile
    #[command(visible_alias = "rm")]
    Remove(RemoveArgs),
    /// List configured tools and profiles
    #[command(visible_alias = "ls")]
    List(ListArgs),
    /// Show which account a launch would use (binary, env, dirs) — without launching
    Which(WhichArgs),
    /// Run a tool's own login/auth flow for an OAuth profile
    Login(LoginArgs),
    /// Show the built-in tool adapters and how each isolates accounts
    Tools,
    /// `hr <tool> [profile] [args...]` — launch a tool. Omit the profile (or use `default`) to use
    /// your already-installed account with no isolation.
    #[command(external_subcommand)]
    Run(Vec<String>),
}

#[derive(Args)]
pub struct AddArgs {
    /// Tool name (e.g. claude, codex, opencode, grok, antigravity)
    pub tool: String,
    /// Profile name (letters, digits, '-' or '_')
    pub profile: String,
    /// Create an OAuth profile (an isolated login)
    #[arg(long, conflicts_with = "api")]
    pub oauth: bool,
    /// Create an API-key profile
    #[arg(long)]
    pub api: bool,
    /// API key. Use '-' to read it from stdin. If omitted you'll be prompted. Implies --api.
    #[arg(long)]
    pub key: Option<String>,
    /// Custom base URL for the endpoint (e.g. GLM/OpenRouter). Implies --api.
    #[arg(long)]
    pub base_url: Option<String>,
    /// Env var name(s) the key should be exported as (overrides the adapter default). Implies --api.
    #[arg(long = "key-env")]
    pub key_env: Vec<String>,
}

#[derive(Args)]
pub struct RemoveArgs {
    pub tool: String,
    pub profile: String,
    /// Also delete the profile's stored credentials/data directory
    #[arg(long)]
    pub purge: bool,
}

#[derive(Args)]
pub struct ListArgs {
    /// Only show profiles for this tool
    pub tool: Option<String>,
}

#[derive(Args)]
pub struct WhichArgs {
    pub tool: String,
    /// Profile name; omit (or use `default`) for your already-installed account
    pub profile: Option<String>,
}

#[derive(Args)]
pub struct LoginArgs {
    pub tool: String,
    pub profile: String,
    /// Extra args forwarded to the tool's own login flow (e.g. `--device-auth` for a headless box).
    /// Hyphen-flags are captured directly; `--` also works: `hr login codex home -- --device-auth`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub extra: Vec<String>,
}
