-- ---------------------------------------------------------------------------
-- Papers as a first-class entity
--
-- Until now papers were articles with `category='paper'` decorated by a
-- `paper_metadata(repo_uri, source_path, venue, year, accepted)` row. That
-- works when a paper has exactly one PDF, but split-apart artefacts (arxiv
-- preprint vs publisher PDF vs camera-ready vs author-hosted post) live in
-- different articles and the discussion is fragmented across them.
--
-- Mirror the books pattern: a `papers` row is the durable identity, with
-- one or more `paper_versions` rows pointing at the various PDFs/articles
-- (`kind` differentiates preprint / accepted / published / native — where
-- "native" means the canonical text is hosted on NightBoat as an article).
-- Comments, Q&A, votes, bookmarks and learned-marks attach at paper
-- level via `content_uri = 'paper:{id}'`, reusing the existing content
-- machinery. Mirror PDF links sit alongside without splintering the
-- discussion.
-- ---------------------------------------------------------------------------

CREATE TABLE papers (
    id          varchar(64)  PRIMARY KEY,
    -- jsonb keyed by locale (mirrors books.title shape).
    title       jsonb        NOT NULL DEFAULT '{}'::jsonb,
    abstract    jsonb        NOT NULL DEFAULT '{}'::jsonb,
    -- Plain string list of author names; the canonical link to author rows
    -- goes through `paper_authors` so we get reverse lookups for free.
    authors     text[]       NOT NULL DEFAULT '{}'::text[],
    venue       varchar(255),
    -- 'conference' | 'journal' | 'workshop' | 'preprint' | 'thesis' | 'other'
    venue_kind  varchar(32),
    year        smallint,
    doi         varchar(128),
    arxiv_id    varchar(64),
    bibtex_key  varchar(128),
    -- Author-uploaded papers may sit in review state ("not yet accepted").
    -- Defaults to true since most papers we record will be public.
    accepted    boolean      NOT NULL DEFAULT true,
    created_by  varchar(255) NOT NULL,
    created_at  timestamptz  NOT NULL DEFAULT now(),
    removed_at  timestamptz
);

CREATE INDEX idx_papers_doi      ON papers (doi)      WHERE doi IS NOT NULL;
CREATE INDEX idx_papers_arxiv_id ON papers (arxiv_id) WHERE arxiv_id IS NOT NULL;
CREATE INDEX idx_papers_year     ON papers (year)     WHERE year IS NOT NULL;

CREATE TABLE paper_versions (
    id          varchar(64) PRIMARY KEY,
    paper_id    varchar(64) NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    -- 'preprint'   — arxiv etc. external mirror.
    -- 'accepted'   — author camera-ready post.
    -- 'published'  — official publisher copy.
    -- 'native'     — article hosted natively on NightBoat is the canonical
    --                text. `article_uri` carries the article URI; `url`
    --                stays NULL (the platform serves it).
    -- 'other'      — anything else (talks, slides, supplementary).
    kind        varchar(32)  NOT NULL,
    -- External mirror URL. NULL for kind='native'.
    url         text,
    -- Article URI for kind='native' (article is the canonical body for the
    -- paper). Matches articles' synthetic URI form.
    article_uri varchar(512),
    year        smallint,
    label       varchar(255),
    sort_order  smallint     NOT NULL DEFAULT 0,
    created_at  timestamptz  NOT NULL DEFAULT now(),
    -- Either an external URL or a hosted article must exist.
    CONSTRAINT paper_versions_target_present
        CHECK (url IS NOT NULL OR article_uri IS NOT NULL)
);

CREATE INDEX idx_paper_versions_paper   ON paper_versions (paper_id);
CREATE INDEX idx_paper_versions_article ON paper_versions (article_uri)
    WHERE article_uri IS NOT NULL;

CREATE TABLE paper_authors (
    paper_id   varchar(64) NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    author_id  varchar(64) NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    position   smallint    NOT NULL DEFAULT 0,
    -- 'author' (default), 'corresponding', 'editor', 'translator'.
    role       varchar(50) NOT NULL DEFAULT 'author',
    PRIMARY KEY (paper_id, author_id)
);

CREATE INDEX idx_paper_authors_author ON paper_authors (author_id);

-- Allow content_uri = 'paper:...' so comments / votes / bookmarks /
-- learned-marks / topic-tags can target a paper directly.
ALTER TABLE content DROP CONSTRAINT content_content_type_check;
ALTER TABLE content ADD  CONSTRAINT content_content_type_check CHECK (
    content_type = ANY (ARRAY[
        'article','series','question','answer','book','chapter',
        'book_series','coursegroup','paper'
    ])
);

-- ── Back-fill from paper_metadata ─────────────────────────────────────────
-- For each existing paper-flagged article, mint a papers row keyed on the
-- article's source-language localisation, then a kind='native' paper_version
-- pointing at that article. Existing votes / comments / bookmarks stay on
-- the article URI; future ones can target the paper.
--
-- `papers.authors text[]` is filled by aggregating article_authors.author_name
-- so the display string survives even when the curated authors row is absent.
-- `paper_authors` joins authors.did <-> article_authors.author_did to recover
-- the curated id when we have one; rows without a match are skipped (the
-- text[] fallback still renders the name on the paper page).

INSERT INTO papers (id, title, abstract, year, venue, accepted, authors, created_by)
SELECT
    'pap-' || substr(md5(a.repo_uri || a.source_path), 1, 16),
    jsonb_build_object('en', l.title),
    CASE WHEN COALESCE(l.summary,'') <> ''
         THEN jsonb_build_object('en', l.summary)
         ELSE '{}'::jsonb END,
    pm.year::smallint,
    pm.venue,
    pm.accepted,
    COALESCE(
        (SELECT array_agg(aa.author_name ORDER BY aa.position)
           FROM article_authors aa
          WHERE aa.repo_uri = a.repo_uri AND aa.source_path = a.source_path
            AND aa.author_name IS NOT NULL),
        '{}'::text[]
    ),
    a.author_did
FROM paper_metadata pm
JOIN articles a
  ON a.repo_uri = pm.repo_uri AND a.source_path = pm.source_path
JOIN article_localizations l
  ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path
 AND l.file_path = a.source_path
ON CONFLICT (id) DO NOTHING;

INSERT INTO content (uri, content_type)
SELECT 'paper:' || id, 'paper' FROM papers
ON CONFLICT (uri) DO NOTHING;

INSERT INTO paper_versions (id, paper_id, kind, article_uri, year, sort_order)
SELECT
    'pv-' || substr(md5(article_uri(pm.repo_uri, pm.source_path) || 'native'), 1, 16),
    'pap-' || substr(md5(pm.repo_uri || pm.source_path), 1, 16),
    'native',
    article_uri(pm.repo_uri, pm.source_path),
    pm.year::smallint,
    0
FROM paper_metadata pm
ON CONFLICT (id) DO NOTHING;

-- Curated authors backfill: only when authors.did matches article_authors.author_did.
INSERT INTO paper_authors (paper_id, author_id, position, role)
SELECT
    'pap-' || substr(md5(aa.repo_uri || aa.source_path), 1, 16) AS paper_id,
    a.id,
    aa.position,
    'author'
FROM article_authors aa
JOIN paper_metadata pm
  ON pm.repo_uri = aa.repo_uri AND pm.source_path = aa.source_path
JOIN authors a ON a.did = aa.author_did
ON CONFLICT (paper_id, author_id) DO NOTHING;
