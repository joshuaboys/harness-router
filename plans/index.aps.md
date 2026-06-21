<!--
APS Index Template
==================
AGENT: This is NON-EXECUTABLE. Do not create tasks here.
Focus on: intent, scope, modules, risks — NOT implementation.
See: plans/aps-rules.md
-->

# harness-router — Plan

| Field   | Value       |
| ------- | ----------- |
| Status  | In Progress |
| Owner   | @joshuaboys |
| Created | 2026-06-11  |

## Problem

`harness-router` (`hr`) isolates AI-CLI accounts per profile, which works on
Linux and Windows but not on macOS for Claude Code: there OAuth credentials
live in the login Keychain rather than under `CLAUDE_CONFIG_DIR`, so two
`claude` OAuth profiles silently share and overwrite one login. Planned work
closes that platform gap without violating the project's ephemeral,
no-daemon principles.

## Success Criteria

- [ ] Two `claude` OAuth profiles stay isolated across sessions on macOS — or
      the limitation is surfaced explicitly at runtime instead of failing
      silently.
- [ ] Linux and Windows launch resolution is unchanged.
- [ ] No proxy, daemon, or global "current account" state is introduced.

## Constraints

- Pure `exec`-based launch model; no background process or persistent state.
- Build on documented Claude Code behaviour where possible; keep any
  workaround for undocumented behaviour narrow and well-tested.
- All behaviour changes covered by tests (CI across Linux/macOS/Windows).

## Modules

| Module                                                | Purpose                                     | Status      | Dependencies |
| ----------------------------------------------------- | ------------------------------------------- | ----------- | ------------ |
| [KEYCHN](./modules/01-claude-macos-keychain.aps.md)   | Claude OAuth isolation on macOS (Keychain)  | In Progress | —            |

## Risks

| Risk                                                                   | Impact                                      | Mitigation                                                                              |
| ---------------------------------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------- |
| Claude Code's macOS credential storage is undocumented and may change  | Workaround breaks on a Claude Code release  | Prefer the documented `setup-token` path (Option A); keep the fallback narrow and tested |
| Chosen strategy needs a paid subscription tier                         | Some users cannot use per-profile tokens    | Document the limit; keep Keychain-swap as a documented fallback (Option C)               |

## Open Questions

- [ ] Does `.credentials.json` take precedence over the Keychain when both
      exist on macOS? (Q-001 in [issues.md](./issues.md); probe step P4)

## Decisions

- **D-001:** Strategy for macOS Claude OAuth isolation — **Option A**
  (per-profile `setup-token` → `CLAUDE_CODE_OAUTH_TOKEN`), with Option C
  (Keychain swap) as the documented fallback. _Accepted 2026-06-14._
