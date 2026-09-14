CREATE TRIGGER rpc_spawn_fixture_fail_event BEFORE INSERT ON events
WHEN NEW.kind='rpc_spawn_observed'
BEGIN SELECT RAISE(ABORT, 'rpc spawn fixture event failure'); END;
