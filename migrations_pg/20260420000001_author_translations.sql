-- Per-locale name variants for authors.
--
-- `name` is the canonical display name used when no locale-specific entry
-- applies — conventionally the English form for authors who publish in
-- English.
--
-- `original_names` holds the author's own authoritative forms in other
-- languages. Example: Terence Tao publishes as both "Terence Tao" and
-- "陶哲轩"; both are originals. `original_names = {"zh": "陶哲轩"}` (the
-- English form lives in `name`).
--
-- `translations` holds transliterations / translated renderings that the
-- author does not use themselves. Example: Paul Krugman → 保罗·克鲁格曼.
-- `translations = {"zh": "保罗·克鲁格曼"}`.
--
-- Resolution order for a locale L: original_names[L] → translations[L] → name.
ALTER TABLE authors ADD COLUMN IF NOT EXISTS original_names JSONB NOT NULL DEFAULT '{}';
ALTER TABLE authors ADD COLUMN IF NOT EXISTS translations   JSONB NOT NULL DEFAULT '{}';
