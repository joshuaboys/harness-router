# Changelog

All notable changes to `harness-router` are documented here.

## Unreleased

## 0.2.2 - 2026-06-09

### Added

- Windows support (`x86_64`). `hr` now builds, tests, and ships a prebuilt binary on Windows. The
  launch spawns the target CLI as a child and forwards its exit code (no true `exec`), and the
  Unix permission hardening is skipped — see the README caveats.
- Prebuilt `hr` binaries (with SHA-256 checksums) attached to every GitHub Release for Linux
  (`x86_64`, `aarch64`), macOS (Intel, Apple Silicon), and Windows (`x86_64`).
  `cargo binstall harness-router` now installs without compiling from source.

### Changed

- Upgraded `dirs` 5 → 6 and `toml` 0.8 → 1.
- CI/release workflows now run on `actions/checkout@v5` (Node 24 runtime).

### Fixed

- Release workflow now creates the GitHub Release before uploading binaries, so the prebuilt
  artifacts actually attach (the v0.2.1 binary jobs failed with "release not found").

## 0.2.1 - 2026-06-09

Published to crates.io; the prebuilt-binary upload failed (no GitHub Release existed to attach to),
so no release assets were produced for this version. Fixed in 0.2.2.

## 0.2.0 - 2026-06-09

### Added

- Built-in adapter for the GitHub Copilot CLI (`copilot`), isolating accounts via `COPILOT_HOME`
  and clearing stray `COPILOT_GITHUB_TOKEN`/`GH_TOKEN`/`GITHUB_TOKEN` for OAuth profiles.

### Fixed

- Antigravity adapter now targets the real `agy` binary (was `antigravity`) and adds the `agy` alias.
  `agy` is a terminal agent, not a VS Code IDE: the invalid `--user-data-dir` isolation is replaced
  with a per-profile `HOME` redirect (the only option, as `agy` hardcodes `~/.gemini` with no
  relocation env var). Remains experimental and Linux-only — see the README caveats.

## 0.1.0 - 2026-06-09

Initial public release.

### Added

- `hr <tool> [profile] [args...]` launcher for switching AI coding CLI accounts per process.
- Built-in adapters for Claude Code, Codex, opencode, grok, and experimental Google Antigravity support.
- OAuth profiles with isolated config/state directories where the target CLI supports them.
- API-key profiles with optional custom base URLs for Anthropic-/OpenAI-compatible endpoints.
- `hr add`, `hr login`, `hr ls`, `hr rm`, `hr tools`, and `hr which` commands.
- Default-account passthrough via `hr <tool>` without requiring profile setup.
- Secret handling that keeps API keys out of the registry and stores them in per-profile files.
- CI coverage for formatting, clippy, builds, and tests across Linux and macOS.

### Known Limitations

- Claude OAuth profile isolation is reliable on Linux; macOS Claude OAuth still depends on Keychain behavior and is documented as a caveat.
- Antigravity support is experimental and has not yet been verified on a real install.
