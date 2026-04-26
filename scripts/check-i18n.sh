#!/usr/bin/env bash
# Verify every t('key') used in the frontend exists in every locale file.
#
# Why this exists: missing keys silently fall through to a literal string
# render (e.g. "paper.directoryBlurb" appearing on the page) because the
# i18n helper has no fallback policy. We refuse to ship that.
#
# Behaviour: prints per-locale missing keys and exits 1 if anything is
# missing. Run as a pre-commit hook or in CI.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FRONTEND="$ROOT/frontend"
LOCALES_DIR="$FRONTEND/src/lib/i18n"

# Locale files are TS objects keyed by quoted strings. We canonicalise to
# en.ts (every key used in code MUST land in en.ts), then ensure every
# locale matches en.ts's key set.
LOCALE_FILES=(en.ts zh.ts de.ts fr.ts)

# Extract every literal key passed to t('…') / t("…"). This intentionally
# only catches static keys — dynamic ones like t(`foo.${x}`) can't be
# verified statically and are out of scope.
USED_KEYS_FILE=$(mktemp)
trap 'rm -f "$USED_KEYS_FILE" /tmp/i18n-keys-*.txt 2>/dev/null' EXIT

# Each dotted segment must start with a letter so we don't pick up values
# like 'Loading...' or partial template-literal captures ('profile.degree.').
KEY_RE="[a-zA-Z][a-zA-Z0-9_]*(\.[a-zA-Z][a-zA-Z0-9_]*)+"

grep -rhoE "t\(['\"]${KEY_RE}['\"]" \
    --include='*.svelte' --include='*.ts' \
    "$FRONTEND/src" \
  | sed -E "s/t\(['\"]([^'\"]+)['\"].*/\1/" \
  | sort -u > "$USED_KEYS_FILE"

USED_COUNT=$(wc -l < "$USED_KEYS_FILE")
echo "Found $USED_COUNT static t() keys in source."

EXIT=0
for LOCALE in "${LOCALE_FILES[@]}"; do
    KEYS_FILE="/tmp/i18n-keys-${LOCALE%.ts}.txt"
    # Only the LHS keys ('foo.bar.baz':), never values, to avoid false
    # positives from values that happen to look like keys.
    grep -oE "'${KEY_RE}':" "$LOCALES_DIR/$LOCALE" \
      | tr -d "':" \
      | sort -u > "$KEYS_FILE"

    MISSING=$(comm -23 "$USED_KEYS_FILE" "$KEYS_FILE")
    if [ -n "$MISSING" ]; then
        echo
        echo "✗ $LOCALE missing $(echo "$MISSING" | wc -l) key(s):"
        echo "$MISSING" | sed 's/^/    /'
        EXIT=1
    else
        echo "✓ $LOCALE has all $USED_COUNT keys."
    fi
done

# Also flag keys defined in en.ts but missing from other locales (so
# secondary locales don't silently lag behind English additions).
EN_KEYS="/tmp/i18n-keys-en.txt"
for LOCALE in zh.ts de.ts fr.ts; do
    KEYS_FILE="/tmp/i18n-keys-${LOCALE%.ts}.txt"
    EXTRA_IN_EN=$(comm -23 "$EN_KEYS" "$KEYS_FILE")
    if [ -n "$EXTRA_IN_EN" ]; then
        echo
        echo "✗ $LOCALE missing $(echo "$EXTRA_IN_EN" | wc -l) key(s) that en.ts has:"
        echo "$EXTRA_IN_EN" | sed 's/^/    /'
        EXIT=1
    fi
done

exit $EXIT
