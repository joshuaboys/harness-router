# Action Plan: KEYCHN probe (KEYCHN-002 entry criteria)

| Field     | Value                                                                       |
| --------- | --------------------------------------------------------------------------- |
| Source    | [../modules/01-claude-macos-keychain.aps.md](../modules/01-claude-macos-keychain.aps.md) |
| Work Item | KEYCHN-002 — Keychain-aware OAuth isolation (entry-criteria probe)          |
| Status    | In Progress                                                                 |

## Prerequisites

- [x] KEYCHN-001 design accepted (Option A primary, Option C fallback)
- [x] Probe script present (`scripts/macos-keychain-probe.sh`)
- [ ] A Mac with Claude Code + a Claude subscription (Pro+ for P3)

## Purpose

Confirm the four design claims (design "What must be verified before KEYCHN-002
starts") on real macOS before any `src/` work begins. P3 is load-bearing for the
accepted Option A; P1/P2/P4 de-risk the Option C fallback. Run via the Appendix A
checklist; the probe script inspects item *metadata* only — it never reads or
prints token values.

A self-contained, hand-off-ready version of these steps for an external macOS
tester lives in [KEYCHN-probe-runbook.md](./KEYCHN-probe-runbook.md) (single-file
`curl`, no clone needed); its results table maps 1:1 onto the Results table below.

## Actions

### Action 1 — Capture baseline + login-write metadata (P1)

**Purpose**
Establish the Keychain item's service/account names and that `/login` writes it.

**Produces**
`status` output before and after `claude` `/login` recorded in the results table.

**Checkpoint**
Item service/account names observed; `mdat` advances after `/login`.

**Validate**
`scripts/macos-keychain-probe.sh status`

### Action 2 — Observe mid-session write-back (P2)

**Purpose**
Confirm a normal session does not rewrite the Keychain mid-session (refresh
unused); only a re-`/login` does.

**Produces**
`mdat` before/after a 15+ min session (and across a 401/re-login if it occurs).

**Checkpoint**
`mdat` unchanged across an uninterrupted session.

### Action 3 — Test file-vs-Keychain precedence (P4)

**Purpose**
Decide whether Option B (seeded `.credentials.json`) is even viable.

**Produces**
Recorded result of `CLAUDE_CONFIG_DIR=/tmp/hr-probe claude -p "reply OK"` with a
bogus seeded credentials file present and a valid Keychain item.

**Checkpoint**
"Keychain won" or "file won" recorded; `/tmp/hr-probe` deleted afterwards.

**Validate**
`scripts/macos-keychain-probe.sh seed-bogus /tmp/hr-probe`

### Action 4 — Confirm token bypass leaves Keychain untouched (P3) — load-bearing

**Purpose**
Verify Option A's core guarantee: a `CLAUDE_CODE_OAUTH_TOKEN` session neither
reads nor writes the Keychain.

**Produces**
`mdat` before/after a `setup-token` session, recorded (token value never pasted).

**Checkpoint**
Keychain `mdat` unchanged after a `CLAUDE_CODE_OAUTH_TOKEN` session.

## Results

> Paste probe output blocks here as steps complete. Record only metadata
> (service/account names, `cdat`/`mdat`, mode) — never token values.
>
> **Tester env:** macOS 26.2, Claude Code 2.1.185 (tester @benjaminszymkow), 2026-06-22.

| Probe | Claim | Result | Evidence | Date |
| ----- | ----- | ------ | -------- | ---- |
| P1 | Keychain item service/account names as reported; `/login` writes it | PASS | `svce`/`acct` = `"Claude Code-credentials"`/username; `/login` advanced `mdat` 13:13:06Z→13:41:24Z, `cdat` stable (in-place update); no `.credentials.json` on disk | 2026-06-22 |
| P2 | No mid-session write-back (refresh unused) | _pending_ | | |
| P3 | `CLAUDE_CODE_OAUTH_TOKEN` session touches Keychain not at all | _pending_ | | |
| P4 | File-vs-Keychain precedence when both exist | _pending_ | | |

## Completion

- [ ] P1–P4 recorded in the Results table with evidence
- [ ] P3 confirmed (Option A guarantee holds) — or Option C fallback triggered
- [ ] Findings folded back into KEYCHN-002 entry criteria; Q-001 answered in `../issues.md`
- [ ] KEYCHN-002 cleared to start in source module
