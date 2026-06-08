//! Implementations of each subcommand.

use anyhow::{bail, Context, Result};

use crate::adapter::{self, Adapter};
use crate::cli::{AddArgs, ListArgs, LoginArgs, RemoveArgs, WhichArgs};
use crate::config::{self, Kind, Profile};
use crate::invoke::{self, Invocation};

/// Placeholder substituted for the real API key when *describing* a launch (`hr which`), so the
/// secret is never read from disk or printed.
const REDACTED_KEY: &str = "<hidden>";

/// `hr <tool> [profile] [args...]` — the headline command.
///
/// The profile is optional. Bare `hr <tool>` (or anything where the first extra token looks like a
/// flag, e.g. `hr claude -p ...`) targets the reserved [`config::DEFAULT_PROFILE`] — your
/// already-installed account, launched with no isolation.
pub fn run(tokens: Vec<String>) -> Result<()> {
    let mut it = tokens.into_iter();
    let tool = it.next().context("usage: hr <tool> [profile] [args...]")?;
    let adapter = lookup(&tool)?;

    // Decide what's a profile name vs. passthrough args (see `split_profile_args`).
    let rest: Vec<String> = it.collect();
    let (profile_name, passthrough) = split_profile_args(rest);

    let reg = config::load()?;

    // A configured profile always wins — including one explicitly named `default`, which lets a
    // power user repoint the bare command at an isolated account.
    if let Some(profile) = reg.get(adapter.name, &profile_name).cloned() {
        return run_profile(adapter, &profile_name, &profile, &passthrough);
    }

    // No configured profile under that name. The reserved `default` name falls back to an ambient
    // passthrough; any other name is a genuine miss.
    if profile_name == config::DEFAULT_PROFILE {
        warn_if_experimental(adapter);
        announce_default(adapter, &reg.profile_names(adapter.name));
        let inv = invoke::resolve_default(adapter, &passthrough);
        return invoke::exec(&inv);
    }

    bail!(
        "no profile '{1}' for {0}. Add it:  hr add {0} {1}   (or just `hr {0}` for your default account)",
        adapter.name,
        profile_name
    );
}

/// Split the tokens that follow the tool name into `(profile_name, passthrough_args)`.
///
/// The first token is the profile name — *unless* it's missing or looks like a flag (`-x`), in which
/// case the reserved [`config::DEFAULT_PROFILE`] is used and every token is forwarded as args. That's
/// what lets both `hr claude` and `hr claude -p "…"` target the already-installed account, while
/// `hr claude work -p "…"` still selects the `work` profile.
fn split_profile_args(rest: Vec<String>) -> (String, Vec<String>) {
    let first_is_flag = rest.first().is_some_and(|s| s.starts_with('-'));
    if rest.is_empty() || first_is_flag {
        (config::DEFAULT_PROFILE.to_string(), rest)
    } else {
        let mut iter = rest.into_iter();
        let name = iter.next().expect("non-empty: checked above");
        (name, iter.collect())
    }
}

/// Launch a configured (OAuth/API) profile with full credential isolation.
fn run_profile(
    adapter: &Adapter,
    profile_name: &str,
    profile: &Profile,
    passthrough: &[String],
) -> Result<()> {
    let api_key = match profile.kind {
        Kind::Api => Some(config::read_secret(adapter.name, profile_name)?),
        Kind::Oauth => None,
    };
    let data_dir = config::profile_data_dir(adapter.name, profile_name)?;

    warn_if_experimental(adapter);
    let inv = invoke::resolve(
        adapter,
        profile,
        &data_dir,
        api_key.as_deref(),
        passthrough,
        None,
    );
    invoke::exec(&inv)
}

/// When launching the implicit default *and* the user has named profiles, say so — otherwise it's
/// easy to think you're on `work` when you're actually on the ambient account. Silent when there's
/// no ambiguity (no other profiles configured).
fn announce_default(adapter: &Adapter, named: &[String]) {
    if named.is_empty() {
        return;
    }
    eprintln!(
        "hr: launching {}'s default (already-installed) account — no isolation. Named profiles: {}.",
        adapter.name,
        named.join(", ")
    );
}

/// `hr add <tool> <profile> [--oauth|--api ...]`
pub fn add(args: AddArgs) -> Result<()> {
    let adapter = lookup(&args.tool)?;
    if !valid_name(&args.profile) {
        bail!(
            "invalid profile name '{}': use letters, digits, '-' or '_'",
            args.profile
        );
    }

    let wants_api =
        args.api || args.key.is_some() || args.base_url.is_some() || !args.key_env.is_empty();
    let kind = if wants_api {
        Kind::Api
    } else if args.oauth {
        Kind::Oauth
    } else {
        prompt_kind()?
    };

    match kind {
        Kind::Api => add_api(adapter, &args)?,
        Kind::Oauth => add_oauth(adapter, &args.profile)?,
    }

    if args.profile == config::DEFAULT_PROFILE {
        eprintln!(
            "hr: note: '{0}/default' overrides the implicit default — `hr {0}` now launches this \
             profile instead of your already-installed account.",
            adapter.name
        );
    }
    Ok(())
}

fn add_api(adapter: &Adapter, args: &AddArgs) -> Result<()> {
    // Determine which env vars the key will be exported as.
    let effective_key_env: Vec<String> = if args.key_env.is_empty() {
        adapter.api_key_env.iter().map(|s| s.to_string()).collect()
    } else {
        args.key_env.clone()
    };
    if effective_key_env.is_empty() {
        bail!(
            "{0} API profiles need an explicit key env var: pass --key-env VAR \
             (e.g. hr add {0} {1} --api --key-env ANTHROPIC_API_KEY)",
            adapter.name,
            args.profile
        );
    }
    if args.base_url.is_some() && adapter.base_url_env.is_none() {
        eprintln!(
            "hr: warning: {} has no known base-url env var; --base-url won't be applied automatically.",
            adapter.name
        );
    }

    let key = read_api_key(args, adapter)?;
    if key.trim().is_empty() {
        bail!("empty API key — nothing stored");
    }
    config::write_secret(adapter.name, &args.profile, key.trim())?;

    let profile = Profile {
        kind: Kind::Api,
        base_url: args.base_url.clone(),
        // Only persist key_env if the user overrode the adapter default.
        key_env: args.key_env.clone(),
        env: Default::default(),
    };
    let mut reg = config::load()?;
    reg.insert(adapter.name, &args.profile, profile);
    config::save(&reg)?;

    println!("Added API profile {}/{}.", adapter.name, args.profile);
    println!("Launch it:  hr {} {}", adapter.name, args.profile);
    Ok(())
}

fn add_oauth(adapter: &Adapter, profile: &str) -> Result<()> {
    let new = Profile {
        kind: Kind::Oauth,
        base_url: None,
        key_env: Vec::new(),
        env: Default::default(),
    };
    let mut reg = config::load()?;
    reg.insert(adapter.name, profile, new);
    config::save(&reg)?;

    println!("Added OAuth profile {}/{}.", adapter.name, profile);
    if adapter.login_args.is_empty() {
        println!(
            "Log in:     hr {0} {1}   (complete {0}'s normal login the first time)",
            adapter.name, profile
        );
    } else {
        println!("Log in:     hr login {} {}", adapter.name, profile);
    }
    Ok(())
}

/// `hr login <tool> <profile>` — run the tool's own auth flow inside the profile's isolated dir.
pub fn login(args: LoginArgs) -> Result<()> {
    let adapter = lookup(&args.tool)?;
    let reg = config::load()?;
    let profile = reg
        .get(adapter.name, &args.profile)
        .cloned()
        .with_context(|| {
            format!(
                "no profile '{1}' for {0}. Add it:  hr add {0} {1} --oauth",
                adapter.name, args.profile
            )
        })?;
    if profile.kind != Kind::Oauth {
        bail!(
            "'{}/{}' is an API profile — no login flow is needed.",
            adapter.name,
            args.profile
        );
    }

    let data_dir = config::profile_data_dir(adapter.name, &args.profile)?;
    let login_args: Vec<String> = adapter.login_args.iter().map(|s| s.to_string()).collect();

    warn_if_experimental(adapter);
    let inv = invoke::resolve(adapter, &profile, &data_dir, None, &[], Some(&login_args));
    invoke::exec(&inv)
}

/// `hr ls [tool]`
pub fn list(args: ListArgs) -> Result<()> {
    let reg = config::load()?;
    if reg.is_empty() {
        println!("No profiles yet.");
        println!("`hr <tool>` already launches your default (already-installed) account.");
        println!("Add isolated ones with, e.g.:  hr add claude work");
        return Ok(());
    }

    let filter = args.tool.as_deref().map(|t| {
        adapter::find(t)
            .map(|a| a.name.to_string())
            .unwrap_or_else(|| t.to_string())
    });

    for (tool, profiles) in reg.tools() {
        if let Some(want) = &filter {
            if tool != want {
                continue;
            }
        }
        let about = adapter::find(tool).map(|a| a.about).unwrap_or("");
        if about.is_empty() {
            println!("{tool}");
        } else {
            println!("{tool}  ({about})");
        }
        for (name, profile) in profiles {
            let kind = match profile.kind {
                Kind::Oauth => "oauth",
                Kind::Api => "api",
            };
            match &profile.base_url {
                Some(url) => println!("  {name:<16} {kind:<6} -> {url}"),
                None => println!("  {name:<16} {kind}"),
            }
        }
        // Unless the user redefined it, `hr <tool>` hits the ambient default — surface that here.
        if !profiles.contains_key(config::DEFAULT_PROFILE) {
            println!(
                "  {:<16} ambient — your already-installed account",
                config::DEFAULT_PROFILE
            );
        }
    }
    Ok(())
}

/// `hr which <tool> [profile]` — describe which account a launch would use, without launching.
pub fn which(args: WhichArgs) -> Result<()> {
    let adapter = lookup(&args.tool)?;
    let profile_name = args
        .profile
        .unwrap_or_else(|| config::DEFAULT_PROFILE.to_string());
    let reg = config::load()?;

    // A configured profile resolves exactly as `run` would — but with a redacted key.
    if let Some(profile) = reg.get(adapter.name, &profile_name).cloned() {
        let api_key = match profile.kind {
            Kind::Api => Some(REDACTED_KEY),
            Kind::Oauth => None,
        };
        let data_dir = config::profile_data_dir(adapter.name, &profile_name)?;
        let inv = invoke::resolve(adapter, &profile, &data_dir, api_key, &[], None);
        let kind = match profile.kind {
            Kind::Oauth => "oauth",
            Kind::Api => "api",
        };
        print!(
            "{}",
            format_plan(&format!("{}/{}", adapter.name, profile_name), kind, &inv)
        );
        if profile.kind == Kind::Api && !config::secret_path(adapter.name, &profile_name)?.exists()
        {
            println!(
                "  note:    no stored API key — re-add it with `hr add {} {} --api`",
                adapter.name, profile_name
            );
        }
        return Ok(());
    }

    // The reserved `default` name describes an ambient (no-isolation) launch.
    if profile_name == config::DEFAULT_PROFILE {
        let inv = invoke::resolve_default(adapter, &[]);
        print!(
            "{}",
            format_plan(
                &format!("{}/default", adapter.name),
                "ambient — your already-installed account",
                &inv,
            )
        );
        return Ok(());
    }

    bail!(
        "no profile '{1}' for {0}. Add it:  hr add {0} {1}   (or `hr which {0}` for your default account)",
        adapter.name,
        profile_name
    );
}

/// Render a launch plan as human-readable text. Pure, so it's unit-testable and the `which` command
/// stays a thin wrapper. The API key is expected to be already redacted by the caller.
fn format_plan(label: &str, kind: &str, inv: &Invocation) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "{label}  ({kind})");
    let _ = writeln!(out, "  {:<8} {}", "binary:", inv.program);

    if inv.env_set.is_empty() && inv.env_unset.is_empty() && inv.args.is_empty() {
        let _ = writeln!(
            out,
            "  (no isolation — launches with your current environment)"
        );
        return out;
    }
    for (i, (k, v)) in inv.env_set.iter().enumerate() {
        let lead = if i == 0 { "env:" } else { "" };
        let _ = writeln!(out, "  {lead:<8} {k}={v}");
    }
    if !inv.env_unset.is_empty() {
        let _ = writeln!(out, "  {:<8} {}", "unset:", inv.env_unset.join(", "));
    }
    if !inv.args.is_empty() {
        let _ = writeln!(out, "  {:<8} {}", "args:", inv.args.join(" "));
    }
    out
}

/// `hr tools`
pub fn tools() -> Result<()> {
    println!("Built-in tool adapters:\n");
    for ad in adapter::ADAPTERS {
        let tag = if ad.experimental {
            "  [experimental]"
        } else {
            ""
        };
        println!("  {:<13}{}{}", ad.name, ad.about, tag);
        println!("  {:<13}isolation: {}", "", ad.isolation_summary());
        if !ad.aliases.is_empty() {
            println!("  {:<13}aliases:   {}", "", ad.aliases.join(", "));
        }
        println!();
    }
    println!("Every tool also has an implicit `default` profile:");
    println!("  hr <tool>            launch your already-installed account (no isolation)");
    println!("  hr <tool> default    the same, named explicitly");
    println!(
        "Add an isolated account with `hr add <tool> <name>`; naming one `default` overrides this."
    );
    Ok(())
}

/// `hr rm <tool> <profile> [--purge]`
pub fn remove(args: RemoveArgs) -> Result<()> {
    // Accept aliases, but fall back to the raw name so you can always clean up stale entries.
    let key = adapter::find(&args.tool)
        .map(|a| a.name.to_string())
        .unwrap_or_else(|| args.tool.clone());

    let mut reg = config::load()?;
    if reg.remove(&key, &args.profile).is_none() {
        bail!("no profile '{}' for '{}'", args.profile, key);
    }
    config::save(&reg)?;
    println!("Removed {}/{} from the registry.", key, args.profile);

    let dir = config::profile_data_dir(&key, &args.profile)?;
    if args.purge {
        if dir.exists() {
            std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
            println!("Purged {}.", dir.display());
        }
    } else if dir.exists() {
        println!(
            "Stored credentials/data remain at {} (use --purge to delete).",
            dir.display()
        );
    }
    Ok(())
}

fn lookup(tool: &str) -> Result<&'static Adapter> {
    adapter::find(tool).with_context(|| {
        format!(
            "unknown tool '{}'. Built-ins: {}. See `hr tools`.",
            tool,
            adapter::names()
        )
    })
}

fn warn_if_experimental(adapter: &Adapter) {
    if adapter.experimental {
        eprintln!(
            "hr: note: '{}' support is experimental — verify account isolation works before relying on it.",
            adapter.name
        );
    }
}

fn read_api_key(args: &AddArgs, adapter: &Adapter) -> Result<String> {
    match args.key.as_deref() {
        Some("-") => {
            use std::io::Read;
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .context("reading API key from stdin")?;
            Ok(s)
        }
        Some(k) => Ok(k.to_string()),
        None => {
            rpassword::prompt_password(format!("API key for {}/{}: ", adapter.name, args.profile))
                .context("reading API key")
        }
    }
}

fn prompt_kind() -> Result<Kind> {
    use std::io::Write;
    print!("Profile type — [o]auth login or [a]pi key? [o] ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .context("reading input")?;
    match line.trim().to_ascii_lowercase().as_str() {
        "" | "o" | "oauth" => Ok(Kind::Oauth),
        "a" | "api" => Ok(Kind::Api),
        other => bail!("unrecognized choice '{other}' (expected 'o' or 'a')"),
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::Path;

    /// Helper: parse the post-tool tokens the way `run` does.
    fn split(tokens: &[&str]) -> (String, Vec<String>) {
        split_profile_args(tokens.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn bare_tool_targets_default_with_no_args() {
        let (profile, args) = split(&[]);
        assert_eq!(profile, config::DEFAULT_PROFILE);
        assert!(args.is_empty());
    }

    #[test]
    fn first_word_is_the_profile() {
        let (profile, args) = split(&["work"]);
        assert_eq!(profile, "work");
        assert!(args.is_empty());
    }

    #[test]
    fn named_profile_keeps_its_passthrough_args() {
        let (profile, args) = split(&["work", "-p", "hi"]);
        assert_eq!(profile, "work");
        assert_eq!(args, vec!["-p", "hi"]);
    }

    #[test]
    fn leading_flag_routes_to_default_and_forwards_every_token() {
        // `hr claude -p hi` — the flag must not be mistaken for a profile name.
        let (profile, args) = split(&["-p", "hi"]);
        assert_eq!(profile, config::DEFAULT_PROFILE);
        assert_eq!(args, vec!["-p", "hi"]);
    }

    #[test]
    fn explicit_default_keyword_still_forwards_args() {
        let (profile, args) = split(&["default", "-p", "hi"]);
        assert_eq!(profile, config::DEFAULT_PROFILE);
        assert_eq!(args, vec!["-p", "hi"]);
    }

    #[test]
    fn double_dash_separator_counts_as_a_flag() {
        // `--` (like `--help`) starts with '-', so it's passthrough for the default account.
        let (profile, args) = split(&["--", "raw"]);
        assert_eq!(profile, config::DEFAULT_PROFILE);
        assert_eq!(args, vec!["--", "raw"]);
    }

    #[test]
    fn which_describes_ambient_default_as_no_isolation() {
        let ad = adapter::find("claude").unwrap();
        let inv = invoke::resolve_default(ad, &[]);
        let out = format_plan("claude/default", "ambient", &inv);
        assert!(out.contains("claude/default  (ambient)"));
        assert!(out.contains("binary:  claude"));
        assert!(out.contains("no isolation"));
    }

    #[test]
    fn which_redacts_the_api_key_but_shows_everything_else() {
        let ad = adapter::find("claude").unwrap();
        let profile = Profile {
            kind: Kind::Api,
            base_url: Some("https://glm.example/anthropic".to_string()),
            key_env: Vec::new(),
            env: BTreeMap::new(),
        };
        // `which` resolves with the placeholder — the real secret is never in play.
        let inv = invoke::resolve(ad, &profile, Path::new("/d"), Some(REDACTED_KEY), &[], None);
        let out = format_plan("claude/glm", "api", &inv);

        assert!(out.contains(&format!("ANTHROPIC_API_KEY={REDACTED_KEY}")));
        assert!(!out.contains("sk-")); // no real-looking key ever rendered
        assert!(out.contains("ANTHROPIC_BASE_URL=https://glm.example/anthropic"));
        assert!(out.contains("CLAUDE_CONFIG_DIR=")); // dir isolation still surfaced
    }
}
