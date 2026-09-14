CREATE TRIGGER rpc_fixture_fail_event BEFORE INSERT ON events
WHEN NEW.kind IN ('rpc_launch_registered', 'rpc_launch_claimed')
BEGIN SELECT RAISE(ABORT, 'rpc fixture event failure'); END;
