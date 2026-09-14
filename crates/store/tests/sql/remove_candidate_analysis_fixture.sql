-- Only for an owned corrupted-database test fixture with foreign keys disabled.
DELETE FROM analysis_runs WHERE id = ?1;
