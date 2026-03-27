-- Unified content tag system: articles and series share the same tag relationship tables.
-- Three relationships: teaches (what it teaches), topics (what it belongs to), prereqs (what it requires).

-- 1. Content identity table
CREATE TABLE content (
    uri TEXT PRIMARY KEY,
    content_type TEXT NOT NULL CHECK (content_type IN ('article', 'series'))
);

-- Populate from existing articles and series
INSERT INTO content (uri, content_type)
SELECT at_uri, 'article' FROM articles;

INSERT INTO content (uri, content_type)
SELECT id, 'series' FROM series;

-- 2. Unified tag relationship tables
CREATE TABLE content_teaches (
    content_uri TEXT NOT NULL REFERENCES content(uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (content_uri, tag_id)
);
CREATE INDEX idx_content_teaches_tag ON content_teaches(tag_id);

CREATE TABLE content_topics (
    content_uri TEXT NOT NULL REFERENCES content(uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    PRIMARY KEY (content_uri, tag_id)
);
CREATE INDEX idx_content_topics_tag ON content_topics(tag_id);

CREATE TABLE content_prereqs (
    content_uri TEXT NOT NULL REFERENCES content(uri) ON DELETE CASCADE,
    tag_id VARCHAR(255) NOT NULL REFERENCES tags(id),
    prereq_type VARCHAR(50) NOT NULL DEFAULT 'required',
    PRIMARY KEY (content_uri, tag_id)
);
CREATE INDEX idx_content_prereqs_tag ON content_prereqs(tag_id);

-- 3. Migrate data from old tables
INSERT INTO content_teaches (content_uri, tag_id)
SELECT article_uri, tag_id FROM article_teaches
ON CONFLICT DO NOTHING;

INSERT INTO content_prereqs (content_uri, tag_id, prereq_type)
SELECT article_uri, tag_id, prereq_type FROM article_prereqs
ON CONFLICT DO NOTHING;

-- Migrate series.tag_id → content_topics
INSERT INTO content_topics (content_uri, tag_id)
SELECT id, tag_id FROM series
ON CONFLICT DO NOTHING;

-- 4. Drop old tables and column
DROP TABLE article_teaches;
DROP TABLE article_prereqs;
ALTER TABLE series DROP COLUMN tag_id;

-- 5. Add triggers to auto-insert into content table
CREATE OR REPLACE FUNCTION content_insert_article() RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO content (uri, content_type) VALUES (NEW.at_uri, 'article') ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION content_insert_series() RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO content (uri, content_type) VALUES (NEW.id, 'series') ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION content_delete_article() RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM content WHERE uri = OLD.at_uri;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION content_delete_series() RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM content WHERE uri = OLD.id;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_article_content_insert
    AFTER INSERT ON articles FOR EACH ROW EXECUTE FUNCTION content_insert_article();

CREATE TRIGGER trg_series_content_insert
    AFTER INSERT ON series FOR EACH ROW EXECUTE FUNCTION content_insert_series();

CREATE TRIGGER trg_article_content_delete
    BEFORE DELETE ON articles FOR EACH ROW EXECUTE FUNCTION content_delete_article();

CREATE TRIGGER trg_series_content_delete
    BEFORE DELETE ON series FOR EACH ROW EXECUTE FUNCTION content_delete_series();
