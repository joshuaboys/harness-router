# Contributing

Thanks for helping improve `harness-router`. Keep changes small, direct, and easy to verify.

## Development Setup

Install stable Rust, then run the tool from a clone:

```console
cargo run -- --help
cargo run -- tools
```

The binary name is `hr` when installed:

```console
cargo install --path .
hr --help
```

## Before Opening a Pull Request

Run the same checks as CI:

```console
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --verbose
```

If you changed launch behavior for a supported tool, also test the relevant flow manually with disposable profiles or test credentials.

## Project Shape

- `src/cli.rs` defines the command-line interface.
- `src/commands.rs` implements subcommands.
- `src/adapter.rs` contains built-in tool adapters.
- `src/invoke.rs` resolves a profile into the concrete process launch.
- `src/config.rs` manages the registry and per-profile secret storage.

Adding a built-in tool should usually be a small `Adapter` entry plus tests for the resolved invocation.

## Contribution Guidelines

- Preserve the no-proxy, no-daemon design unless the change is explicitly scoped otherwise.
- Keep secrets out of the registry and out of command-line arguments where possible.
- Prefer minimal changes over broad rewrites.
- Add or update tests for behavior changes.
- Document user-visible caveats in `README.md`.
- Avoid committing real profile data, API keys, OAuth tokens, or local config files.

## Releasing

Releases publish to [crates.io](https://crates.io/crates/harness-router) automatically from CI when a
`v*` tag is pushed:

1. Bump `version` in `Cargo.toml`, refresh `Cargo.lock` (`cargo build`), and move the `Unreleased`
   CHANGELOG section to a dated `## <version> - <date>` heading.
2. Merge that to `main`.
3. Tag the merge commit and push it:

   ```console
   git tag -a v0.2.0 -m "v0.2.0"
   git push origin v0.2.0
   ```

The `Release` workflow verifies the tag matches `Cargo.toml`, then runs `cargo publish` using the
`CARGO_REGISTRY_TOKEN` repository secret (a crates.io API token with publish scope).

## Reporting Bugs

Please include:

- The `hr` version or commit.
- Your operating system.
- The target CLI and profile kind (`oauth` or `api`).
- The command you ran, with secrets redacted.
- Expected and actual behavior.

For security issues, use the private reporting process in `SECURITY.md` instead of a public issue.
