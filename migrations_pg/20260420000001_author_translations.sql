-- Per-locale name variants for authors.
--
-- `name` is the canonical display name used when no locale-specific entry
-- applies — conventionally the English form for authors who publish in
-- English.
--
-- Three per-locale buckets, in display-priority order:
--
--   1. `original_names` — forms the author uses themselves in other
--      languages. Terence Tao publishes as both "Terence Tao" and "陶哲轩";
--      both are originals. `original_names = {"zh": "陶哲轩"}`.
--
--   2. `official_translations` — widely-accepted translations the field has
--      settled on; admin-curated. Shown automatically in matching locale.
--      Example: Richard Feynman → 理查德·费曼; David Graeber → 大卫·格雷伯.
--
--   3. `translations` — other transliterations/variant renderings. Stored
--      for search and display on the author's own page under "other
--      translations", but NOT used as the default display anywhere else.
--
-- Resolution order for a locale L:
--   original_names[L] → official_translations[L] → name.
ALTER TABLE authors ADD COLUMN IF NOT EXISTS original_names        JSONB NOT NULL DEFAULT '{}';
ALTER TABLE authors ADD COLUMN IF NOT EXISTS official_translations JSONB NOT NULL DEFAULT '{}';
ALTER TABLE authors ADD COLUMN IF NOT EXISTS translations          JSONB NOT NULL DEFAULT '{}';
