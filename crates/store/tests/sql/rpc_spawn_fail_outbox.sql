CREATE TRIGGER rpc_spawn_fixture_fail_outbox BEFORE INSERT ON event_outbox
WHEN EXISTS (SELECT 1 FROM events WHERE seq=NEW.event_sequence AND kind='rpc_spawn_observed')
BEGIN SELECT RAISE(ABORT, 'rpc spawn fixture outbox failure'); END;
