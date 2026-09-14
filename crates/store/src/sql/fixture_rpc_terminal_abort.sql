CREATE TRIGGER fixture_terminal_abort BEFORE INSERT ON event_outbox
WHEN (SELECT kind FROM events WHERE seq=NEW.event_sequence)='rpc_terminal_recorded'
BEGIN SELECT RAISE(ABORT, 'injected terminal outbox failure'); END;
