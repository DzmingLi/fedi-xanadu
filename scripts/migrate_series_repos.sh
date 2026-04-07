#!/usr/bin/env bash
# Migrate existing series articles to series repos.
#
# For series without pijul repos:
#   1. Init a series pijul repo
#   2. Copy article source files from independent repos into chapters/
#   3. Update series.pijul_node_id in DB
#
# Run on the server as the fedi-xanadu service user or with appropriate permissions.
# Usage: PIJUL_STORE=/var/lib/fedi-xanadu/pijul-store DB=fedi_xanadu ./migrate_series_repos.sh

set -euo pipefail

PIJUL_STORE="${PIJUL_STORE:-/var/lib/fedi-xanadu/pijul-store}"
DB="${DB:-fedi_xanadu}"
DRY_RUN="${DRY_RUN:-0}"

log() { echo "[migrate] $*"; }
run() {
    if [ "$DRY_RUN" = "1" ]; then
        echo "[dry-run] $*"
    else
        eval "$@"
    fi
}

# Get all series without pijul repos that have articles
SERIES=$(sudo -u postgres psql -d "$DB" -t -A -c "
    SELECT s.id
    FROM series s
    JOIN series_articles sa ON s.id = sa.series_id
    WHERE s.pijul_node_id IS NULL
    GROUP BY s.id
")

for SERIES_ID in $SERIES; do
    NODE_ID="series_${SERIES_ID}"
    REPO_PATH="${PIJUL_STORE}/${NODE_ID}"

    log "Processing series: $SERIES_ID → $REPO_PATH"

    # 1. Create series repo directory structure
    if [ ! -d "$REPO_PATH" ]; then
        run "mkdir -p '$REPO_PATH/chapters' '$REPO_PATH/cache'"
        log "  Created repo directory"
    else
        run "mkdir -p '$REPO_PATH/chapters' '$REPO_PATH/cache'"
    fi

    # 2. Copy article source files from independent repos
    ARTICLES=$(sudo -u postgres psql -d "$DB" -t -A -F'|' -c "
        SELECT sa.article_uri, a.content_format::text
        FROM series_articles sa
        JOIN articles a ON sa.article_uri = a.at_uri
        WHERE sa.series_id = '$SERIES_ID'
        ORDER BY sa.order_index
    ")

    COPIED=0
    for ROW in $ARTICLES; do
        URI=$(echo "$ROW" | cut -d'|' -f1)
        FORMAT=$(echo "$ROW" | cut -d'|' -f2)

        # Extract TID from URI: at://did/collection/TID → TID
        TID=$(echo "$URI" | rev | cut -d'/' -f1 | rev)

        # Determine file extension
        case "$FORMAT" in
            typst)    EXT="typ" ;;
            markdown) EXT="md" ;;
            html)     EXT="html" ;;
            *)        EXT="typ" ;;
        esac

        # Source: independent article repo
        ARTICLE_NODE_ID=$(echo "$URI" | sed 's|/|_|g; s|:|_|g')
        ARTICLE_REPO="${PIJUL_STORE}/${ARTICLE_NODE_ID}"

        CHAPTER_DEST="${REPO_PATH}/chapters/${TID}.${EXT}"

        if [ -f "$CHAPTER_DEST" ]; then
            log "  Skip $TID.$EXT (already exists)"
            continue
        fi

        # Try to find source file
        SRC_FILE=""
        if [ -f "${ARTICLE_REPO}/content.${EXT}" ]; then
            SRC_FILE="${ARTICLE_REPO}/content.${EXT}"
        elif [ -f "${ARTICLE_REPO}/content.typ" ]; then
            SRC_FILE="${ARTICLE_REPO}/content.typ"
        elif [ -f "${ARTICLE_REPO}/content.md" ]; then
            SRC_FILE="${ARTICLE_REPO}/content.md"
        elif [ -f "${ARTICLE_REPO}/content.html" ]; then
            SRC_FILE="${ARTICLE_REPO}/content.html"
        fi

        if [ -n "$SRC_FILE" ]; then
            run "cp '$SRC_FILE' '$CHAPTER_DEST'"
            log "  Copied $SRC_FILE → chapters/$TID.$EXT"
            COPIED=$((COPIED + 1))
        else
            log "  WARNING: No source found for $URI (repo: $ARTICLE_REPO)"
        fi
    done

    log "  Copied $COPIED article(s)"

    # 3. Update DB: set pijul_node_id
    run "sudo -u postgres psql -d '$DB' -c \"UPDATE series SET pijul_node_id = '$NODE_ID' WHERE id = '$SERIES_ID'\""
    log "  Updated pijul_node_id = $NODE_ID"

    log "Done: $SERIES_ID"
    echo
done

# Summary: also init repos for series that have NO articles but no pijul_node_id
EMPTY_SERIES=$(sudo -u postgres psql -d "$DB" -t -A -c "
    SELECT id FROM series WHERE pijul_node_id IS NULL
")

for SERIES_ID in $EMPTY_SERIES; do
    NODE_ID="series_${SERIES_ID}"
    REPO_PATH="${PIJUL_STORE}/${NODE_ID}"

    if [ ! -d "$REPO_PATH" ]; then
        run "mkdir -p '$REPO_PATH/chapters' '$REPO_PATH/cache'"
        log "Init empty repo for $SERIES_ID"
    fi

    run "sudo -u postgres psql -d '$DB' -c \"UPDATE series SET pijul_node_id = '$NODE_ID' WHERE id = '$SERIES_ID'\""
done

log "Migration complete."
