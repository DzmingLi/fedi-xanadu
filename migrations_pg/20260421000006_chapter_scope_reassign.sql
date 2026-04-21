-- Reassign what articles.book_chapter_id / course_session_id are used for.
--
-- Before: only category='review' could carry chapter / session scope (via the
--   CategoryMetadata::Review variant). But book reviews are typically about the
--   whole book, not a single chapter.
--
-- After:  the columns are used by category='note' (via a new
--   CategoryMetadata::Note variant) and by kind='question' articles (via
--   top-level fields on CreateArticle). category='review' no longer sets these
--   columns; existing review rows that populated them have their chapter/
--   session reset to NULL so the book-level review still makes sense.
--
-- The columns themselves are kept (same names, wider scope). App-layer
-- enforcement in article_service ensures category='review' writes NULL.

UPDATE articles
   SET book_chapter_id   = NULL,
       course_session_id = NULL
 WHERE category = 'review'
   AND (book_chapter_id IS NOT NULL OR course_session_id IS NOT NULL);
