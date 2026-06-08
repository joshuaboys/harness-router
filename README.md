# harness-router (`hr`)

Add multiple OAuth/API accounts for your AI coding CLIs and switch between them with one command:

```console
$ hr claude               # your default, already-installed account (no setup)
$ hr claude work          # launch Claude Code on your "work" account
$ hr claude home          # …or your "home" account, in another terminal, at the same time
$ hr codex personal
$ hr opencode oss
```

No proxy. No daemon. No dashboard. Switching accounts for these tools is really just _"point the
tool at the right credentials, then launch it"_ — so that's all `hr` does. It sets the right
environment for the profile you named and `exec`s the real CLI, with any extra arguments passed
straight through.

This is the deliberately-small alternative to heavier "profile manager + proxy" tools.

## How it works

Every supported CLI resolves its credentials from a directory or environment variable that `hr` can
redirect per-launch. A **profile** is just one of two things:

- **OAuth profile** — an isolated config directory. Log in once into it (`hr login claude work`),
  and it's reused forever. Two profiles never share login state, so you can run them side by side.
- **API profile** — a set of environment variables (an API key, optionally a custom base URL). No
  login step. This also covers any Anthropic-/OpenAI-compatible endpoint (GLM, OpenRouter, Ollama,
  Kimi, …) by pointing the base URL at it.

The account that's **already installed** needs no profile at all: a bare `hr <tool>` (equivalently
`hr <tool> default`) launches the tool with your existing login and *no* isolation — so you get one
consistent front-door for every account, the default included. A leading flag is treated the same
way, so `hr claude -p "…"` just runs your default account with those args. Register a profile named
`default` to repoint the bare command at an isolated account instead. Not sure which account a
command would land on? `hr which <tool> [profile]` prints exactly that — binary, env and isolated
dirs — without launching anything (the API key is redacted).

Switching is **ephemeral**: `hr claude work` affects only the process it launches. There is no
global "current account" to get out of sync — the account is chosen fresh, per command.

## Install

```console
# from a clone
cargo install --path .

# the binary is `hr`
hr --help
```

Requires the target CLIs (`claude`, `codex`, `opencode`, …) to be installed and on your `PATH`.

## Quick start

```console
# 1. Register a profile (you'll be asked OAuth vs API, or pass --oauth / --api)
hr add claude home --oauth
hr add claude work --oauth

# 2. Log in to each (runs the tool's own login inside that profile's isolated config)
hr login claude home
hr login claude work
hr login codex home --device-auth            # flags after the profile pass to the login flow

# 3. Use them — or skip all of the above and use your existing login:
hr claude                                    # your default, already-installed account
hr claude home
hr claude work -p "summarise CHANGES.md"     # extra args go straight to claude

# API / custom-endpoint profiles need no login:
hr add claude glm --api --base-url https://open.bigmodel.cn/api/anthropic
hr claude glm
```

## Commands

| Command                            | What it does                                                                        |
| ---------------------------------- | ----------------------------------------------------------------------------------- |
| `hr <tool> <profile> [args…]`      | Launch `<tool>` on `<profile>`, forwarding `args…`.                                 |
| `hr add <tool> <profile>`          | Register a profile. `--oauth` or `--api` (with `--key`, `--base-url`, `--key-env`). |
| `hr login <tool> <profile> [args…]` | Run the tool's own login flow inside an OAuth profile's isolated dir; `args…` forward to it (e.g. `--device-auth`). |
| `hr ls [tool]`                     | List configured tools and profiles.                                                 |
| `hr rm <tool> <profile> [--purge]` | Remove a profile (`--purge` also deletes its stored credentials).                   |
| `hr tools`                         | Show the built-in tool adapters and how each isolates accounts.                     |

`--key -` reads the API key from stdin (so it never lands in your shell history):

```console
echo "$MY_KEY" | hr add grok work --api --key -
```

## Supported tools

| Tool                                                     | Account isolation                                                                                                                                                                |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **claude** (Claude Code)                                 | `CLAUDE_CONFIG_DIR` per profile; API profiles set `ANTHROPIC_API_KEY` (+ `ANTHROPIC_BASE_URL`). OAuth profiles clear stray `ANTHROPIC_API_KEY`/`*_AUTH_TOKEN` so the login wins. |
| **codex** (OpenAI Codex CLI)                             | `CODEX_HOME` per profile (relocates auth, config, sessions and logs).                                                                                                            |
| **opencode**                                             | `XDG_DATA_HOME` + `XDG_CONFIG_HOME` per profile. API profiles require `--key-env` (provider-specific).                                                                           |
| **grok** (xAI)                                           | API-key based: `XAI_API_KEY` / `GROK_API_KEY`.                                                                                                                                   |
| **antigravity** (Google Antigravity, Gemini's successor) | **Experimental.** Isolates via a `--user-data-dir` launch arg; not yet verified on a real install.                                                                               |

Adding a tool is a single entry in [`src/adapter.rs`](src/adapter.rs).

### Generic / custom endpoints

Any Anthropic-compatible endpoint works as a `claude` API profile; any OpenAI-compatible endpoint
works as a `codex`/`opencode` API profile. Examples:

```console
hr add claude glm  --api --base-url https://open.bigmodel.cn/api/anthropic
hr add opencode or --api --key-env OPENAI_API_KEY        # OpenRouter etc.
```

## Where things live

- **Registry** (non-secret, inspectable TOML): `~/.config/harness-router/config.toml`
- **Per-profile state**: `~/.local/share/harness-router/profiles/<tool>/<profile>/`
  - isolated OAuth config dirs, and the API key in an `api_key` file (`0600`).

Secrets are never written to the registry.

## Caveats

- **macOS + Claude OAuth.** Claude Code stores OAuth credentials in the macOS Keychain, which
  `CLAUDE_CONFIG_DIR` does not relocate. OAuth-profile isolation for `claude` is therefore reliable
  on Linux; on macOS, _API_ profiles work everywhere, but separating two OAuth logins needs a
  Keychain-aware workaround (tracked for a future release). Codex/opencode are unaffected.
- **Antigravity** is experimental — see the table above.

## Roadmap

- Verify and harden the Antigravity adapter on real installs.
- macOS Keychain handling for Claude OAuth profiles.
- User-defined custom tools (arbitrary binary + env mapping) from the registry.
- Optional interactive picker (`hr` with no args) as a thin TUI layer.
- An optional local API router for the "everything behind one endpoint" use case.

## License

MIT — see [LICENSE](LICENSE).
