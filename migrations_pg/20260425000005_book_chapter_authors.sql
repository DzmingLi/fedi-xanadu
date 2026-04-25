-- ---------------------------------------------------------------------------
-- Per-chapter authors for books
--
-- Edited volumes (e.g. "Twenty-Five Years of Constructive Type Theory")
-- credit different authors per chapter, but `book_chapters` only carried a
-- title/order/article_uri triple. This forced us to stuff author names into
-- the chapter title, e.g.
--   "An Intuitionistic Theory of Types (Per Martin-Löf, 1972 reprint)"
-- which loses author-side reverse lookups (Martin-Löf's author page can't
-- find the chapter) and bleeds metadata into display strings.
--
-- Mirror the `book_authors` pattern with a join table so each chapter can
-- reference one or more `authors` rows and we get clean reverse queries.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS book_chapter_authors (
    chapter_id  varchar(64) NOT NULL,
    author_id   varchar(64) NOT NULL,
    position    smallint NOT NULL DEFAULT 0,
    -- e.g. "author" (default), "translator", "editor" — leaves room for
    -- chapters whose contributors aren't all primary authors.
    role        varchar(50) NOT NULL DEFAULT 'author',
    PRIMARY KEY (chapter_id, author_id),
    FOREIGN KEY (chapter_id) REFERENCES book_chapters(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id)  REFERENCES authors(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_book_chapter_authors_author
    ON book_chapter_authors (author_id);
