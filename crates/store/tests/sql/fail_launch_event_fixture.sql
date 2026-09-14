CREATE TRIGGER fail_launch_event BEFORE INSERT ON events
WHEN NEW.kind = 'execution_launch_claimed'
BEGIN SELECT RAISE(ABORT, 'fixture launch event failure'); END;
