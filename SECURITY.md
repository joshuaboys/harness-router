# Security Policy

`harness-router` handles account-selection paths and API keys for local AI coding CLIs. Please report suspected credential leaks, account-isolation failures, or unsafe secret handling privately before opening a public issue.

## Supported Versions

Security fixes target the latest released version and the current `main` branch.

## Reporting a Vulnerability

Report vulnerabilities through GitHub's private vulnerability reporting for this repository:

<https://github.com/joshuaboys/harness-router/security/advisories/new>

If that is unavailable, open a minimal public issue asking for a private contact path. Do not include secrets, exploit payloads, or reproduction details in the public issue.

Please include:

- The affected `hr` version or commit.
- Your operating system and target CLI (`claude`, `codex`, `opencode`, etc.).
- A concise description of the impact.
- Reproduction steps that avoid real credentials where possible.
- Whether any token, API key, or OAuth credential may have been exposed.

## Scope

Security-relevant reports include:

- API keys written somewhere other than the per-profile `api_key` file.
- Secrets stored in the registry at `~/.config/harness-router/config.toml`.
- Profile isolation failures that can launch a tool with the wrong account.
- File permissions that expose profile data to other local users.
- Command-line behavior that unexpectedly leaks secrets through arguments, logs, or errors.

Out of scope:

- Vulnerabilities in the target CLIs themselves.
- Compromise of a user's local machine, shell history, terminal, or filesystem permissions outside `hr`.
- Experimental adapter limitations already documented in the README, unless they create a new secret leak.

## Handling Secrets

Do not paste real API keys, OAuth tokens, registry files, or profile directories into issues or pull requests. Use redacted examples and temporary test credentials.
