# Project Context

> User-owned file. APS will never overwrite this. Populate it manually or let
> your AI agent fill it in on first run.

## Overview

`harness-router` (binary: `hr`) is a small Rust CLI for switching between
multiple OAuth/API accounts for AI coding CLIs — Claude Code, Codex, opencode,
grok, GitHub Copilot, Antigravity. A profile is either an isolated config
directory (OAuth) or a set of environment variables (API key + optional base
URL). `hr` sets the right environment for the named profile and `exec`s the
real CLI — no proxy, no daemon, no global "current account" state. Switching is
ephemeral and per-process. Published to crates.io as `harness-router`
(v0.2.2), with prebuilt binaries on GitHub Releases for Linux, macOS, and
Windows.

## Team

- Josh Boys (@joshuaboys) — author, maintainer, release owner.
- External contributors via GitHub PRs (see `CONTRIBUTING.md`).

## Tech Stack

- Rust (edition 2021), stable toolchain.
- Key dependencies: `clap` (derive CLI), `serde` + `toml` (registry/config),
  `dirs` (platform paths), `anyhow` (errors), `rpassword` (secret prompts).
- Release profile optimised for binary size (`strip`, `lto`, `opt-level = "z"`).
- CI: GitHub Actions (`ci.yml` for fmt/clippy/test across platforms incl.
  Windows; `release.yml` publishes to crates.io and uploads binaries on `v*`
  tags).

## Conventions

- Source layout: `src/cli.rs` (CLI definition), `src/commands.rs`
  (subcommands), `src/adapter.rs` (built-in tool adapters), `src/invoke.rs`
  (profile → process launch resolution), `src/config.rs` (registry +
  per-profile secret storage).
- Pre-PR checks mirror CI: `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --verbose`.
- Conventional commits (`feat`, `fix`, `docs`, `ci`, `chore`, …).
- Behavior changes require tests; user-visible caveats go in `README.md`;
  changes are logged in `CHANGELOG.md` (Unreleased section, dated on release).
- Adding a built-in tool = small `Adapter` entry plus tests for the resolved
  invocation.
- Releases: bump `Cargo.toml` version, refresh `Cargo.lock`, date the
  CHANGELOG section, merge to `main`, push a `v*` tag — CI does the rest.

## Active Decisions

- **No proxy, no daemon, no dashboard.** The tool only points the target CLI
  at the right credentials and launches it. Preserve this unless a change is
  explicitly scoped otherwise.
- **Ephemeral switching.** Account choice affects only the launched process;
  there is deliberately no persistent "current account".
- **Bare `hr <tool>` = the already-installed default account, no isolation.**
  A profile named `default` can repoint it at an isolated account.
- **Secrets stay out of the registry and CLI arguments** where possible;
  never commit real profile data, API keys, OAuth tokens, or local config.
- **Prefer minimal changes over broad rewrites.**
