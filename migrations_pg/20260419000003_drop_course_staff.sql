-- course_staff is redundant: everyone associated with a course (instructor,
-- TA, administrator) now lives in course_authors. Platform users whose
-- Author entity has a bound DID render as profile links; everyone else
-- links to the /author page. Single source of truth.
DROP TABLE IF EXISTS course_staff CASCADE;
