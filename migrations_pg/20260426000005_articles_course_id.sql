-- Reviews and notes anchor to a COURSE (umbrella) instead of a single
-- iteration. The old anchor `articles.term_id` becomes optional
-- secondary metadata: an author may still tag "I took this in
-- Spring 2015", but the primary surface for course reviews/notes is
-- now the umbrella course detail page, listing every iteration's
-- contributions in one place.
--
-- term_id stays on the table (NOT dropped, NOT marked NOT NULL) — it's
-- the optional iteration tag now. course_id is the new primary anchor
-- for review/note articles. Backfill from the existing term_id → its
-- parent course (terms.course_id) so legacy rows surface correctly on
-- day one.

ALTER TABLE articles
    ADD COLUMN course_id VARCHAR(64) REFERENCES courses(id) ON DELETE SET NULL;

CREATE INDEX idx_articles_course_id ON articles(course_id) WHERE course_id IS NOT NULL;

-- Backfill: every existing review/note pinned to a term inherits that
-- term's umbrella course as its new primary anchor.
UPDATE articles
   SET course_id = t.course_id
  FROM terms t
 WHERE articles.term_id = t.id
   AND t.course_id IS NOT NULL;
