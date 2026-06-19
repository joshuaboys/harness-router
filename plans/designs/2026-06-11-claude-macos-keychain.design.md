# Claude OAuth Isolation on macOS (Keychain)

| Field    | Value           |
| -------- | --------------- |
| Status   | Accepted        |
| Owner    | @joshuaboys     |
| Created  | 2026-06-11      |
| Decided  | 2026-06-14      |
| Decision | Option A (per-profile `setup-token` → `CLAUDE_CODE_OAUTH_TOKEN`); Option C documented as fallback for accounts that cannot mint a setup-token |
| Module   | [KEYCHN](../modules/01-claude-macos-keychain.aps.md) |
| Work item | KEYCHN-001     |

## Problem

`hr` isolates `claude` OAuth profiles by pointing `CLAUDE_CONFIG_DIR` at a
per-profile directory. On Linux and Windows that relocates the credential file
(`<config-dir>/.credentials.json`), so profiles are fully isolated. On macOS,
Claude Code stores OAuth credentials in the login Keychain instead, and
`CLAUDE_CONFIG_DIR` does not relocate Keychain items — so every "isolated"
OAuth profile on a Mac silently shares (and overwrites) one login.

Constraints from `plans/project-context.md`: no proxy, no daemon, no
persistent "current account" state; switching stays ephemeral and per-process;
secrets stay out of the registry and CLI arguments.

An added constraint discovered here: the maintainer has no Mac, so empirical
verification must be runnable by a remote volunteer (script + checklist) and
by macOS CI runners.

## Evidence (desk research, 2026-06-11)

Gathered from Anthropic docs and claude-code GitHub issues. Confidence tags:
**[docs]** confirmed by official docs, **[community]** community-reported,
**[unconfirmed]** needs the probe (Appendix A).

1. **Keychain item.** macOS OAuth credentials live in a generic-password item,
   service `"Claude Code-credentials"`, account = OS username. **[community]**
   A reported bug (anthropics/claude-code#9403) describes a service-name
   mismatch between write and read paths in some versions — any strategy that
   manipulates the item by name inherits this fragility. **[community]**
2. **`CLAUDE_CONFIG_DIR` scope.** Relocates credentials on Linux/Windows
   (`<config-dir>/.credentials.json`, mode 0600); on macOS it relocates config
   files only, never the Keychain item. **[docs]**
3. **No supported file-storage override on macOS.** `/login` always writes the
   Keychain. However, if a `.credentials.json` already exists in the config
   dir, Claude Code reads it when the Keychain is unavailable (SSH/locked
   keychain reports). Whether the file is consulted when the Keychain is
   *available*, and which source wins, is **[unconfirmed]** — probe step P4.
4. **`claude setup-token` / `CLAUDE_CODE_OAUTH_TOKEN`.** Produces a long-lived
   (~1 year) OAuth token; requires a Pro/Max/Team/Enterprise subscription;
   supported for interactive sessions, not only CI; when set, Claude Code
   skips the Keychain entirely (no read, no write). Precedence: below
   `ANTHROPIC_API_KEY`/`ANTHROPIC_AUTH_TOKEN`, above stored OAuth. Scope is
   inference-only (no Remote Control sessions). **[docs]**
5. **No mid-session refresh write-back.** Access tokens live ~1–8 h; the
   stored refresh token is reportedly never used (anthropics/claude-code
   #31095, #12447) — sessions hit 401 and require re-`/login`, which *does*
   rewrite the Keychain. So snapshot-style strategies are stale only after a
   re-login, not silently mid-session. **[community]** — probe steps P2/P3.
6. **No official multi-account support** in Claude Code as of June 2026 (open
   feature requests #44687, #20549). `apiKeyHelper` provides API-key auth
   only, not OAuth account switching. **[docs/community]**

## Design

### Options considered

**A. Per-profile long-lived token (`claude setup-token` → `CLAUDE_CODE_OAUTH_TOKEN`)**

`hr login claude <name>` on macOS runs the setup-token flow and stores the
resulting token in `hr`'s existing per-profile secret storage; launch sets
`CLAUDE_CODE_OAUTH_TOKEN` for that process only.

- Pros: documented behaviour (no Keychain read/write at all); ephemeral and
  per-process — exactly `hr`'s model; concurrent sessions on different
  profiles are safe; no manipulation of other apps' Keychain items; identical
  mechanism could optionally serve Linux/Windows too; keeps the `exec` launch
  model.
- Cons: requires a paid subscription tier (setup-token constraint); token
  lifetime ~1 year, then re-login; inference-only scope; one interactive
  browser flow per profile at setup. Note: the adapter currently *clears*
  `CLAUDE_CODE_OAUTH_TOKEN` for OAuth profiles — this inverts that for
  macOS-token profiles.

**B. Seed per-profile `.credentials.json` in the profile's config dir**

Capture credentials once and write the Linux-style file into each profile dir.

- Pros: would give exact Linux parity; no launch-time changes.
- Cons: hinges on **[unconfirmed]** precedence (probe P4); `/login` and
  re-login still write the shared Keychain, so drift is silent; handles raw
  refresh/access tokens in files Claude Code didn't put there; relies on an
  undocumented fallback that may change without notice.

**C. Keychain swap around the session**

Copy the profile's stored credentials into the live `"Claude Code-credentials"`
item before launch; capture back after exit.

- Pros: works with plain `/login`; no subscription-tier constraint.
- Cons: requires abandoning `exec` for claude-on-macOS (spawn + wait to get a
  post-session hook); concurrent sessions are unsafe → needs locking and a
  sequential-only rule; touches the live item other Claude installs use;
  inherits the #9403 service-name fragility; most moving parts of any option.

**D. Per-profile keychains (`security create-keychain` + search list)**

- Rejected: the keychain search list is per-user global state, not
  per-process; mutating it violates `hr`'s ephemeral, no-global-state
  principles and affects every other app's credential lookups.

### Recommendation

**Option A primary, Option C as the documented fallback for accounts that
cannot use setup-token.** A is the only option built entirely on documented
behaviour, it preserves every project principle (ephemeral, per-process, no
global state, `exec` launch), and it is concurrent-safe. Its subscription-tier
and token-scope limits are real but acceptable for v1 if clearly documented;
KEYCHN-003 covers surfacing them at runtime.

Option B is attractive but currently rests on undocumented fallback behaviour;
probe P4 decides whether it's promoted to a serious contender. Option C is
specified here only far enough to know it's viable if A's constraints prove
blocking; it implies sequential-only sessions on macOS.

**Decision (2026-06-14): Option A accepted by @joshuaboys.** The
subscription-tier requirement, ~1-year token lifetime, and inference-only
scope are accepted for v1, in exchange for documented behaviour, concurrent
session safety, and a preserved `exec` launch model. Q3's concurrency stance
is therefore satisfied (concurrency works under A). Option C remains specified
as the documented fallback for accounts that cannot mint a setup-token; it is
not built in v1 unless tester results (P3) invalidate A.

### What must be verified before KEYCHN-002 starts

| # | Claim to verify | Why it gates the design |
| - | --------------- | ----------------------- |
| P1 | Keychain item service/account names as reported | C's mechanics; probe baseline |
| P2 | No mid-session Keychain write-back (refresh unused) | C's capture-on-exit correctness |
| P3 | `CLAUDE_CODE_OAUTH_TOKEN` session touches the Keychain not at all | A's core guarantee |
| P4 | File-vs-Keychain precedence when both exist | Whether B is viable at all |

Verification vehicles: the probe script (`scripts/macos-keychain-probe.sh`)
run by a macOS volunteer per Appendix A, plus a macOS CI job (KEYCHN-002) that
plants a synthetic `"Claude Code-credentials"` item with `security
add-generic-password` and asserts `hr`'s behaviour around it.

## Open Items

- [x] Owner decision: accept Option A's subscription-tier limitation for v1?
      — **Yes** (2026-06-14).
- [ ] Recruit a macOS tester (issue draft: Appendix B) and collect probe
      output. With A chosen, **P3** (token bypass — does a
      `CLAUDE_CODE_OAUTH_TOKEN` session leave the Keychain untouched?) is the
      load-bearing check; P1/P2/P4 now de-risk the C fallback only.
- [ ] Confirm whether `hr login claude <name>` can drive `claude setup-token`
      non-interactively enough to capture the token without it touching the
      terminal scrollback (intent: secrets never in CLI args or plain output).
      Carried into KEYCHN-002 as a design constraint.

## Appendix A — macOS tester checklist

Requirements: a Mac with Claude Code installed and a Claude subscription
(Pro or above for step 5). The probe script never reads or prints token
values — it only inspects item *metadata* (names, timestamps). Paste the
script output blocks back into the GitHub issue.

1. **Baseline.** Run `scripts/macos-keychain-probe.sh status` → paste output.
2. **Login write (P1).** Run `claude`, `/login`, exit. Run the probe `status`
   again → paste. (Shows the item, its service name, creation/mod times.)
3. **Mid-session write-back (P2).** Note the item's `mdat` from step 2. Use a
   normal session for 15+ minutes, exit, run `status` → paste. Repeat once the
   next day with a session that lasts past a 401/re-login if one occurs.
4. **File precedence (P4).** Run
   `scripts/macos-keychain-probe.sh seed-bogus /tmp/hr-probe` then
   `CLAUDE_CONFIG_DIR=/tmp/hr-probe claude -p "reply OK"`.
   - If it replies OK → the Keychain won (file ignored when Keychain valid).
   - If it errors with an auth failure → the file won.
   Paste which happened, then delete `/tmp/hr-probe`.
5. **Token bypass (P3).** Run `claude setup-token` (do **not** paste the
   token). In a new shell:
   `export CLAUDE_CODE_OAUTH_TOKEN=<token>; CLAUDE_CONFIG_DIR=/tmp/hr-probe2 claude -p "reply OK"`.
   Run probe `status` → paste (expect: Keychain `mdat` unchanged from step 3).
   Then `unset CLAUDE_CODE_OAUTH_TOKEN` and delete `/tmp/hr-probe2`.

## Appendix B — tester-recruitment issue draft

> **Title:** Help wanted (macOS): 20-minute credential-storage probe for
> multi-account Claude OAuth support
>
> `hr` isolates Claude Code OAuth profiles per config dir, which works on
> Linux/Windows but not macOS, where credentials live in the Keychain
> (README "Known limitations"). We have a candidate design; it needs five
> observations from a real Mac that we can't make in CI.
>
> If you have a Mac, Claude Code, and a Claude subscription: run the checklist
> in `plans/designs/2026-06-11-claude-macos-keychain.design.md` (Appendix A)
> using `scripts/macos-keychain-probe.sh`, and paste the output blocks here.
> The script never reads or prints token values — only item names and
> timestamps. Thank you!
