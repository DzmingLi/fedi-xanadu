-- Unify the `handout` attachment kind into `notes`.
--
-- "Notes" and "handout" were two ways to say the same thing: written
-- material distributed by the instructor. CS 6110 happened to use
-- handout, CS 164 used notes. The split forced two columns in the
-- course detail UI for what is conceptually one column. Drop the
-- distinction by rewriting handout → notes in every session row.
UPDATE course_sessions
SET attachments = (
    SELECT jsonb_agg(
        CASE
            WHEN a->>'kind' = 'handout'
                THEN jsonb_set(a, '{kind}', '"notes"')
            ELSE a
        END
    )
    FROM jsonb_array_elements(attachments) a
)
WHERE attachments::text LIKE '%"handout"%';
