CREATE TRIGGER fail_launch_event BEFORE INSERT ON event_outbox
BEGIN SELECT RAISE(ABORT, 'fixture launch outbox failure'); END;
