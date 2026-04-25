-- Exam-prep metadata on books. NULL = non-exam material; otherwise a flat
-- list of self-describing tags such as 'kaoyan-math-1', 'kaoyan-408'. The
-- canonical vocabulary lives in the frontend (`examTaxonomy.ts`); their
-- human labels resolve through i18n.

ALTER TABLE books ADD COLUMN IF NOT EXISTS exam_tags TEXT[];

-- GIN index for `tag = ANY(exam_tags)` filtering on the list endpoint.
CREATE INDEX IF NOT EXISTS idx_books_exam_tags
    ON books USING GIN (exam_tags);
