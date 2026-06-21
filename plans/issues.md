# Issues & Questions Tracker

> Development-time discoveries that emerge while building. Not a bug tracker replacement — a lightweight log for planning-level concerns that need visibility.

---

## Issues

### ISS-001: Claude Code never uses its OAuth refresh token (upstream)

| Field | Value |
|-------|-------|
| Status | Open |
| Severity | Medium |
| Discovered | KEYCHN-001 |
| Module | KEYCHN |

**Context:** Community-reported upstream bug (anthropics/claude-code #31095,
#12447): Claude Code stores access + refresh tokens but never refreshes;
sessions 401 after ~1–8 h and force a re-`/login`, which rewrites the macOS
Keychain item.

**Impact:** Good news for snapshot-style isolation (no silent mid-session
write-back), bad news for long sessions on any platform. Re-login is the one
event that mutates the Keychain mid-session — any swap-style strategy must
capture after exit, not only before launch. If upstream fixes refresh, P2
assumptions need re-checking.

### ISS-002: Keychain service-name mismatch in some Claude Code versions (upstream)

| Field | Value |
|-------|-------|
| Status | Open |
| Severity | Low |
| Discovered | KEYCHN-001 |
| Module | KEYCHN |

**Context:** anthropics/claude-code #9403 reports the login path writing
service `"Claude Code-credentials"` while the read path looks up
`"Claude Code"` in some versions.

**Impact:** Any strategy that manipulates the item by service name (design
option C) inherits this fragility; the probe checks both names.

---

## Questions

### Q-001: Does .credentials.json take precedence over the Keychain on macOS?

| Field | Value |
|-------|-------|
| Status | Open |
| Priority | High |
| Discovered | KEYCHN-001 |
| Assigned | macOS tester (probe step P4) |

**Context:** Claude Code reads `<config-dir>/.credentials.json` on macOS when
the Keychain is unavailable (community-reported, SSH scenarios). Unknown:
which source wins when both exist. Decides whether design option B
(per-profile seeded credentials file — full Linux parity) is viable.

**Options considered:**
1. Keychain wins → option B dead, option A (setup-token) stands
2. File wins → option B becomes a serious contender; re-open design

---

## Resolved

_(Nothing resolved yet)_

---

## Quick Reference

| ID Type  | Format  | Example |
| -------- | ------- | ------- |
| Issue    | ISS-NNN | ISS-001 |
| Question | Q-NNN   | Q-001   |

**Severities:** Critical > High > Medium > Low

**Reference from other docs:** `See ISS-001` or `Related: ISS-001, Q-002`
