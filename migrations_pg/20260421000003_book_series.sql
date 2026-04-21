-- Book series: first-class entity grouping related books (e.g. 王道考研 4 本,
-- 剑桥雅思系列). Mirrors the books table structure — i18n JSONB fields, edit log,
-- polymorphic content registration — while adding ordered membership and
-- independent ratings.

CREATE TABLE book_series (
    id           VARCHAR(64) PRIMARY KEY,
    title        JSONB NOT NULL DEFAULT '{}',
    subtitle     JSONB NOT NULL DEFAULT '{}',
    description  JSONB NOT NULL DEFAULT '{}',
    cover_url    VARCHAR(1024),
    created_by   VARCHAR(255) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    removed_at   TIMESTAMPTZ
);

CREATE INDEX idx_book_series_active ON book_series(created_at DESC) WHERE removed_at IS NULL;

-- Ordered member books. A book can belong to multiple series (reorganized
-- bundles, cross-discipline sets).
CREATE TABLE book_series_members (
    series_id   VARCHAR(64) NOT NULL REFERENCES book_series(id) ON DELETE CASCADE,
    book_id     VARCHAR(64) NOT NULL REFERENCES books(id)       ON DELETE CASCADE,
    position    SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (series_id, book_id)
);
CREATE INDEX idx_book_series_members_book ON book_series_members(book_id);
CREATE INDEX idx_book_series_members_order ON book_series_members(series_id, position);

-- Independent series rating (aggregate over member books is computed at query
-- time from book_ratings, not materialized here).
CREATE TABLE book_series_ratings (
    series_id   VARCHAR(64) NOT NULL REFERENCES book_series(id) ON DELETE CASCADE,
    user_did    VARCHAR(255) NOT NULL,
    rating      SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 10),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (series_id, user_did)
);

CREATE TABLE book_series_edit_log (
    id          VARCHAR(64) PRIMARY KEY,
    series_id   VARCHAR(64) NOT NULL REFERENCES book_series(id) ON DELETE CASCADE,
    editor_did  VARCHAR(255) NOT NULL,
    old_data    JSONB NOT NULL DEFAULT '{}',
    new_data    JSONB NOT NULL DEFAULT '{}',
    summary     TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_book_series_edit_log_series ON book_series_edit_log(series_id, created_at DESC);

-- Register in polymorphic content table so comments/tags can hang off a series
-- URI the same way they do for articles, books, chapters.
ALTER TABLE content DROP CONSTRAINT content_content_type_check;
ALTER TABLE content ADD CONSTRAINT content_content_type_check
    CHECK (content_type = ANY (ARRAY[
        'article','series','question','answer',
        'book','chapter','book_series'
    ]));

CREATE FUNCTION content_insert_book_series() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO content (uri, content_type)
    VALUES ('book_series:' || NEW.id, 'book_series')
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END; $$;

CREATE FUNCTION content_delete_book_series() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM content WHERE uri = 'book_series:' || OLD.id;
    RETURN OLD;
END; $$;

CREATE TRIGGER trg_book_series_content_insert
    AFTER INSERT ON book_series FOR EACH ROW EXECUTE FUNCTION content_insert_book_series();
CREATE TRIGGER trg_book_series_content_delete
    BEFORE DELETE ON book_series FOR EACH ROW EXECUTE FUNCTION content_delete_book_series();

-- Backfill: older books/chapters may have been inserted without the explicit
-- content registration. Make sure every existing book and chapter has a
-- content row so the upcoming comments FK change doesn't orphan anything.
INSERT INTO content (uri, content_type)
SELECT 'book:' || id, 'book' FROM books
ON CONFLICT DO NOTHING;

INSERT INTO content (uri, content_type)
SELECT 'chapter:' || id, 'chapter' FROM book_chapters
ON CONFLICT DO NOTHING;
