-- Q&A: questions and answers are articles with different `kind`.

-- 1. Add kind, question_uri, answer_count to articles
ALTER TABLE articles
  ADD COLUMN kind VARCHAR(20) NOT NULL DEFAULT 'article',
  ADD COLUMN question_uri VARCHAR(512) REFERENCES articles(at_uri) ON DELETE CASCADE,
  ADD COLUMN answer_count INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_articles_kind ON articles(kind);
CREATE INDEX idx_articles_question ON articles(question_uri) WHERE question_uri IS NOT NULL;

-- 2. Expand content table to accept question/answer types
ALTER TABLE content DROP CONSTRAINT content_content_type_check;
ALTER TABLE content ADD CONSTRAINT content_content_type_check
  CHECK (content_type IN ('article', 'series', 'question', 'answer'));

-- 3. Update content trigger to use kind
CREATE OR REPLACE FUNCTION content_insert_article() RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO content (uri, content_type)
    VALUES (NEW.at_uri, NEW.kind)
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 4. Generalize comments: article_uri → content_uri
ALTER TABLE comments RENAME COLUMN article_uri TO content_uri;
-- FK still valid: answers are in articles table too

-- 5. Answer count trigger (auto-maintain on insert/delete)
CREATE OR REPLACE FUNCTION update_answer_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.kind = 'answer' AND NEW.question_uri IS NOT NULL THEN
        UPDATE articles SET answer_count = answer_count + 1 WHERE at_uri = NEW.question_uri;
    ELSIF TG_OP = 'DELETE' AND OLD.kind = 'answer' AND OLD.question_uri IS NOT NULL THEN
        UPDATE articles SET answer_count = answer_count - 1 WHERE at_uri = OLD.question_uri;
    END IF;
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_answer_count
    AFTER INSERT OR DELETE ON articles
    FOR EACH ROW EXECUTE FUNCTION update_answer_count();

-- 6. Question merge log (admin)
CREATE TABLE question_merges (
    from_uri VARCHAR(512) NOT NULL,
    into_uri VARCHAR(512) NOT NULL REFERENCES articles(at_uri),
    merged_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (from_uri)
);
