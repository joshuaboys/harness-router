#!/usr/bin/env bash
# macOS credential-storage probe for the KEYCHN-001 spike.
# See plans/designs/2026-06-11-claude-macos-keychain.design.md (Appendix A).
#
# Privacy: this script NEVER reads or prints credential values. It inspects
# Keychain item metadata only (service/account names, timestamps) and never
# passes `-w` or `-g` to `security`. Output is safe to paste into a GitHub
# issue, though it does include your macOS username and Claude Code version.
set -euo pipefail

SERVICES=("Claude Code-credentials" "Claude Code")

usage() {
  cat <<'EOF'
Usage:
  macos-keychain-probe.sh status            Print Keychain/file state for Claude Code
  macos-keychain-probe.sh seed-bogus <dir>  Write a deliberately invalid .credentials.json
                                            into <dir> for the file-precedence test (P4)
EOF
  exit 1
}

item_metadata() {
  local service="$1"
  # Without -w/-g, find-generic-password prints attributes only — no secret.
  if out=$(security find-generic-password -s "$service" 2>/dev/null); then
    echo "--- item found: service=\"$service\" ---"
    # Keep only the attribute lines that matter for the probe.
    echo "$out" | grep -E '"(svce|acct)"|cdat|mdat' || true
  else
    echo "--- no item for service=\"$service\" ---"
  fi
}

status() {
  echo "=== hr keychain probe: $(sw_vers -productVersion 2>/dev/null || echo 'not macOS') ==="
  echo "claude version: $(claude --version 2>/dev/null || echo 'claude not found')"
  echo "user: ${USER:-$(id -un 2>/dev/null || echo unknown)}"
  for svc in "${SERVICES[@]}"; do
    item_metadata "$svc"
  done
  for dir in "${CLAUDE_CONFIG_DIR:-$HOME/.claude}"; do
    if [ -f "$dir/.credentials.json" ]; then
      echo "credentials file: PRESENT at $dir/.credentials.json (mode $(stat -f '%Lp' "$dir/.credentials.json"))"
    else
      echo "credentials file: absent in $dir"
    fi
  done
  echo "=== end probe ==="
}

seed_bogus() {
  local dir="$1"
  mkdir -p "$dir"
  # Plausible shape, deliberately invalid token values: if Claude Code reads
  # this file while a valid Keychain item exists, the session fails with an
  # auth error — which is exactly the signal the P4 test needs.
  cat > "$dir/.credentials.json" <<'EOF'
{
  "claudeAiOauth": {
    "accessToken": "hr-probe-deliberately-invalid",
    "refreshToken": "hr-probe-deliberately-invalid",
    "expiresAt": 4102444800000,
    "scopes": ["user:inference", "user:profile"]
  }
}
EOF
  chmod 600 "$dir/.credentials.json"
  # %q quotes the path so the printed commands stay safe to copy/paste even
  # when <dir> contains spaces or shell metacharacters.
  local qdir
  printf -v qdir '%q' "$dir"
  echo "Wrote bogus credentials file to $dir/.credentials.json"
  echo "Now run: CLAUDE_CONFIG_DIR=$qdir claude -p \"reply OK\""
  echo "  replies OK        -> Keychain took precedence (file ignored)"
  echo "  auth/401 error    -> file took precedence"
  echo "Afterwards delete the directory: rm -rf $qdir"
}

case "${1:-}" in
  status) status ;;
  seed-bogus) [ $# -eq 2 ] || usage; seed_bogus "$2" ;;
  *) usage ;;
esac
