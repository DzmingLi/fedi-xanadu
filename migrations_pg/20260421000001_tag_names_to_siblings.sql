-- Expand tag_labels.names jsonb translations into sibling tag_labels rows.
--
-- Each (origin_tag, translated_lang, translated_name) becomes its own row
-- sharing the origin's group_id, so tagStore.localize() — which walks
-- sibling rows by group_id — can surface translations without also having
-- to read the `names` jsonb column. 21 rows get materialized on the
-- production snapshot; new translations go through upsert logic in the
-- service layer (see update_tag_names).

INSERT INTO tag_labels (id, name, lang, group_id, created_by, names)
SELECT DISTINCT
  n.value AS id,
  n.value AS name,
  n.key   AS lang,
  t.group_id,
  t.created_by,
  jsonb_build_object(n.key, n.value) AS names
FROM tag_labels t, LATERAL jsonb_each_text(t.names) n
WHERE n.key <> t.lang
  AND n.value <> ''
  -- Skip when a sibling with this (group_id, lang) already exists; the
  -- jsonb entry is then a duplicate of that sibling's own row.
  AND NOT EXISTS (
    SELECT 1 FROM tag_labels sib
    WHERE sib.group_id = t.group_id AND sib.lang = n.key
  )
  -- Defensive: if the translation text collides with an existing tag id
  -- in a different group, skip. Production has zero such collisions today
  -- but this keeps the migration idempotent for future reruns.
  AND NOT EXISTS (
    SELECT 1 FROM tag_labels e WHERE e.id = n.value
  );

-- After expansion, every row in a group should advertise the same
-- translation map. Rebuild the cache from current sibling state so old
-- half-populated jsonb values (and the single-entry jsonb we just wrote
-- on the freshly-inserted siblings) are replaced with the union.
UPDATE tag_labels dst
SET names = sub.map
FROM (
    SELECT group_id, jsonb_object_agg(lang, name) AS map
    FROM tag_labels
    GROUP BY group_id
) sub
WHERE dst.group_id = sub.group_id
  AND dst.names IS DISTINCT FROM sub.map;
