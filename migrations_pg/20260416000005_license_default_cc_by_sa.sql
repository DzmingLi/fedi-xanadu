-- Change default license from CC-BY-NC-SA-4.0 to CC-BY-SA-4.0
ALTER TABLE articles ALTER COLUMN license SET DEFAULT 'CC-BY-SA-4.0';
ALTER TABLE drafts ALTER COLUMN license SET DEFAULT 'CC-BY-SA-4.0';
