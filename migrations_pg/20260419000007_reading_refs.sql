-- Extend course_sessions.readings beyond plain text. The new
-- reading_refs column stores an array of {label, url?} entries so a
-- lecture can reference multiple textbook chapters with clickable
-- links ("§1.1", "§2.3, §2.4 (Alg 170-198)"). The plain-text `readings`
-- column stays for short annotations that don't need links.
ALTER TABLE course_sessions
    ADD COLUMN reading_refs JSONB NOT NULL DEFAULT '[]'::jsonb;
