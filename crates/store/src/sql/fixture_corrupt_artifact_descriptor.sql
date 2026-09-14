-- Isolated corruption-test database only; never run against a production store.
DROP TRIGGER artifacts_no_update;
UPDATE artifacts SET descriptor = json_set(descriptor, '$.id', 'wrong-id');
