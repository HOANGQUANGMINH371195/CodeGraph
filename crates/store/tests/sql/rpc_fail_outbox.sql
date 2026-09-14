CREATE TRIGGER rpc_fixture_fail_outbox BEFORE INSERT ON event_outbox
WHEN EXISTS (
    SELECT 1 FROM events WHERE seq = NEW.event_sequence
    AND kind IN ('rpc_launch_registered', 'rpc_launch_claimed')
)
BEGIN SELECT RAISE(ABORT, 'rpc fixture outbox failure'); END;
