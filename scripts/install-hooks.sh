#!/usr/bin/env bash
# Install local git pre-commit hooks for this repo.
# Idempotent: re-running just refreshes the symlinks.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS_DIR="$ROOT/.git/hooks"

cat > "$HOOKS_DIR/pre-commit" <<'EOF'
#!/usr/bin/env bash
# Auto-installed by scripts/install-hooks.sh
# Bypass with `git commit --no-verify` if you must.
set -e
ROOT="$(git rev-parse --show-toplevel)"

# Only run i18n check if any frontend file is staged.
if git diff --cached --name-only | grep -qE '^frontend/src/.*\.(svelte|ts)$'; then
    "$ROOT/scripts/check-i18n.sh"
fi
EOF

chmod +x "$HOOKS_DIR/pre-commit"
echo "Installed pre-commit hook at $HOOKS_DIR/pre-commit"
