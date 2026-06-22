# macOS Keychain probe — tester runbook (harness-router / KEYCHN)

Thanks for running this. It takes ~20–30 min (one ~15-min idle session in the
middle) and answers whether `hr` can isolate two `claude` OAuth profiles on
macOS. **You won't expose any secrets:** the probe script only reads Keychain
item *metadata* (service/account names + timestamps) — never token values. Its
output is safe to paste back as-is (it does include your macOS username and
Claude Code version).

## You need

- A Mac with **Claude Code** installed and logged in (`claude /login` done).
- A **Pro / Max / Team / Enterprise** subscription — required for step **P3**
  (the load-bearing one). P1/P2/P4 work on any logged-in Mac.

## Get the script (single file, no clone needed)

```bash
curl -fsSL https://raw.githubusercontent.com/joshuaboys/harness-router/main/scripts/macos-keychain-probe.sh -o /tmp/hr-probe.sh
chmod +x /tmp/hr-probe.sh
```

Paste each command's output under the matching probe in the table at the bottom.

---

## P1 — baseline + that `/login` writes the Keychain

```bash
/tmp/hr-probe.sh status            # (a) baseline — note the mdat timestamp
claude /login                      # complete the browser login
/tmp/hr-probe.sh status            # (b) after login — note mdat again
```

**Record:** the `svce`/`acct` names shown, and whether `mdat` advanced between
(a) and (b). *Expected: item service is `"Claude Code-credentials"`, account =
your username; `mdat` advances after `/login`.*

## P2 — no mid-session write-back

```bash
/tmp/hr-probe.sh status            # (a) before
claude                             # start a session, use it normally, then leave it idle ≥15 min, then quit
/tmp/hr-probe.sh status            # (b) after
```

**Record:** `mdat` before vs after. *Expected: unchanged across an
uninterrupted session (only a re-`/login` should rewrite it). If you hit a 401 /
forced re-login during the session, note that — it's a real data point.*

## P4 — file-vs-Keychain precedence (decides if Option B is even viable)

```bash
/tmp/hr-probe.sh seed-bogus /tmp/hr-probe-dir       # writes a deliberately INVALID .credentials.json
CLAUDE_CONFIG_DIR=/tmp/hr-probe-dir claude -p "reply OK"
rm -rf /tmp/hr-probe-dir                            # cleanup
```

**Record which happened:**
- **Auth error / login prompt** → the bogus *file won* (file is read even when
  the Keychain is valid).
- **`OK` reply** → the *Keychain won* (file ignored when Keychain is available).

## P3 — token bypass leaves the Keychain untouched  ⭐ load-bearing

This confirms Option A: a `CLAUDE_CODE_OAUTH_TOKEN` session should neither read
nor write the Keychain. **Do the export in your terminal — don't paste the token
into chat or the results table.**

```bash
/tmp/hr-probe.sh status            # (a) before — note mdat
claude setup-token                 # browser flow; copy the printed token
export CLAUDE_CODE_OAUTH_TOKEN=<paste-the-token-here-in-your-terminal-only>
claude -p "reply OK"               # should answer using the token
unset CLAUDE_CODE_OAUTH_TOKEN
/tmp/hr-probe.sh status            # (b) after — note mdat
```

**Record:** `mdat` before vs after, and whether the `OK` reply succeeded.
*Expected (Option A holds): the session replies OK and `mdat` is unchanged.*

---

## Results table (paste output blocks under each row)

| Probe | Claim | Result (pass/fail/note) | Evidence (mdat before→after, etc.) | Date |
| ----- | ----- | ----------------------- | ---------------------------------- | ---- |
| P1 | Keychain item service/account names; `/login` writes it | | | |
| P2 | No mid-session write-back | | | |
| P3 | `CLAUDE_CODE_OAUTH_TOKEN` session touches Keychain not at all | | | |
| P4 | File-vs-Keychain precedence when both exist | | | |

> Reminder: paste **metadata only** (service/account names, `cdat`/`mdat`,
> mode, pass/fail) — never token values.
