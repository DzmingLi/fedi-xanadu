-- Allow reviews to be scoped to a specific chapter of a book or a
-- specific lecture of a course. Both null = review is about the whole
-- book/course. Only relevant on articles with category='review'; other
-- kinds leave them null.
ALTER TABLE articles
    ADD COLUMN book_chapter_id   VARCHAR(64) REFERENCES book_chapters(id)  ON DELETE SET NULL,
    ADD COLUMN course_session_id VARCHAR(64) REFERENCES course_sessions(id) ON DELETE SET NULL;

CREATE INDEX idx_articles_book_chapter   ON articles (book_chapter_id)   WHERE book_chapter_id   IS NOT NULL;
CREATE INDEX idx_articles_course_session ON articles (course_session_id) WHERE course_session_id IS NOT NULL;
