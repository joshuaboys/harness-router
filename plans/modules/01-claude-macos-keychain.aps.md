# Claude OAuth Isolation on macOS (Keychain)

| ID     | Owner       | Status      |
| ------ | ----------- | ----------- |
| KEYCHN | @joshuaboys | In Progress |

**Last reviewed:** 2026-06-21

## Purpose

On macOS, Claude Code stores OAuth credentials in the Keychain, which
`CLAUDE_CONFIG_DIR` does not relocate, so two `claude` OAuth profiles silently
share (and overwrite) a single login. This module makes OAuth-profile
isolation for `claude` as reliable on macOS as it already is on Linux — or,
where full isolation is impossible, surfaces the limitation explicitly at
runtime instead of failing silently.

## In Scope

- macOS credential isolation for `claude` OAuth profiles
- Clear runtime behaviour when isolation cannot be guaranteed
- Documentation: README known-limitations update, CHANGELOG entry

## Out of Scope

- antigravity (`agy`) Keychain isolation — same class of problem, separate
  module if/when tackled
- API-key profiles (already work on every platform)
- Linux/Windows behaviour changes
- Any proxy, daemon, or persistent "current account" state (violates project
  principles — see `plans/project-context.md`)

## Interfaces

**Depends on:**

- `claude` adapter (`src/adapter.rs`) — current `CLAUDE_CONFIG_DIR` env-dir
  isolation and OAuth env-var clearing
- Profile → launch resolution (`src/invoke.rs`)
- Claude Code's macOS credential storage — external, undocumented surface that
  may change between releases

**Exposes:**

- Unchanged CLI grammar: `hr add claude <name> --oauth`,
  `hr login claude <name>`, `hr claude <name>`

## Risks

| Risk                                                                 | Impact                                          | Mitigation                                                       |
| -------------------------------------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------- |
| Claude Code refreshes tokens mid-session and writes back to Keychain | Snapshot-style workarounds lose refreshed creds | Spike (KEYCHN-001) must observe write-back before design is set   |
| Concurrent sessions on two profiles race on one Keychain item        | Cross-profile credential leakage                | Decide concurrency stance up front (see Open Questions)           |
| Claude Code's storage behaviour is undocumented and may change       | Workaround breaks on a Claude Code release      | Keep workaround narrow, document the assumption, test the seams   |
| Workaround requires touching the user's login Keychain               | Trust/security concern for users                | Prefer strategies that never read other apps' items; document     |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one work item defined
- [x] Owner has reviewed Open Questions and the concurrency stance (Q3)

## Work Items

> KEYCHN-001 complete (design accepted). KEYCHN-002 is **Ready** but gated on
> its entry criterion (design P3 confirmed on a real Mac) — the entry-criteria
> **probe is now In Progress** (action plan:
> [../execution/KEYCHN.actions.md](../execution/KEYCHN.actions.md)), awaiting a
> macOS run. KEYCHN-003 stays Draft until KEYCHN-002 lands.

### KEYCHN-001: Spike — characterise macOS credential storage, select strategy

- **Status:** Complete (2026-06-14). Design doc
  ([2026-06-11-claude-macos-keychain](../designs/2026-06-11-claude-macos-keychain.design.md))
  is **Accepted**: Option A (per-profile `setup-token` →
  `CLAUDE_CODE_OAUTH_TOKEN`), Option C documented as fallback. Probe script +
  tester checklist (`scripts/macos-keychain-probe.sh`, design Appendix A)
  remain as KEYCHN-002 entry-criteria — P3 must be confirmed on a real Mac
  before merge.
- **Intent:** Establish exactly how Claude Code reads and writes OAuth
  credentials on macOS, and select an isolation strategy backed by observed
  behaviour rather than assumption.
- **Expected Outcome:** A design doc at
  `plans/designs/<date>-claude-macos-keychain.design.md` that compares the
  candidate strategies, records observed token write-back and
  concurrent-session behaviour, and makes one recommendation with trade-offs.
- **Validation:** `ls plans/designs/*claude-macos-keychain.design.md`
- **Confidence:** medium
- **Non-scope:** No changes to `src/` — evidence gathering and design only.

### KEYCHN-002: Keychain-aware OAuth isolation for `claude` on macOS

- **Status:** Ready (2026-06-14). Strategy fixed by accepted design: Option A.
- **Intent:** Two `claude` OAuth profiles on the same Mac keep distinct logins
  across sessions by authenticating each with its own long-lived token rather
  than the shared Keychain.
- **Expected Outcome:** After `hr login claude work` and `hr login claude
  home`, launching either profile uses its own credentials across relaunches
  and concurrently; the per-profile token is captured without appearing in CLI
  args, shell history, or terminal scrollback; Linux and Windows launch
  resolution is byte-for-byte unchanged; behaviour is covered by tests
  (including a macOS CI job that plants a synthetic `"Claude Code-credentials"`
  item and asserts a token-profile launch leaves it untouched — design P3).
- **Validation:** `cargo fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --verbose`
- **Dependencies:** KEYCHN-001
- **Entry criteria:** Design P3 confirmed on a real Mac (probe step) — a
  `CLAUDE_CODE_OAUTH_TOKEN` session must leave the Keychain item unmodified.
- **Non-scope:** Option C (Keychain swap) — fallback only, not built unless P3
  invalidates A. No changes to Linux/Windows launch resolution.
- **Confidence:** medium
- **Files:** `src/adapter.rs`, `src/invoke.rs`, `src/commands.rs`,
  `src/config.rs` (best effort)

### KEYCHN-003: Honest failure modes and documentation

- **Intent:** When isolation cannot be guaranteed on macOS (unsupported edge,
  concurrent-session conflict), `hr` says so clearly instead of silently
  sharing credentials.
- **Expected Outcome:** The README "macOS + Claude OAuth" caveat is replaced
  with the supported behaviour and any remaining limits; CHANGELOG Unreleased
  entry added; warning/error paths covered by tests.
- **Validation:** `cargo test --verbose && ! grep -q "tracked for a future release" README.md`
- **Dependencies:** KEYCHN-002

## Open Questions

- [x] **Q1:** Does Claude Code write refreshed OAuth tokens back to the
      Keychain mid-session? — **Tentatively no** (community-reported: the
      refresh token is never used; tokens expire and force re-`/login`, which
      *does* rewrite the Keychain). Empirical confirmation = probe step P2.
- [x] **Q2:** Is there a supported path that bypasses the Keychain entirely?
      — **Yes**: `claude setup-token` → `CLAUDE_CODE_OAUTH_TOKEN` is
      documented to skip the Keychain, including interactive sessions.
      Limits: paid-tier subscription, ~1-year lifetime, inference-only scope.
      Empirical confirmation = probe step P3.
- [x] **Q3 (reframed):** The recommended strategy (per-profile setup-token)
      makes concurrent macOS sessions safe, but requires a Pro+ subscription
      and ~yearly re-login. — **Resolved (2026-06-14): accept those limits for
      v1 (Option A).** Concurrent sessions work; Keychain-swap kept as
      documented fallback only.

## Designs

- [Claude OAuth isolation on macOS](../designs/2026-06-11-claude-macos-keychain.design.md) — KEYCHN-001 output

## Execution _(optional)_

Action Plan: [../execution/KEYCHN.actions.md](../execution/KEYCHN.actions.md)
_(In Progress — entry-criteria probe P1–P4, awaiting a macOS run.)_
