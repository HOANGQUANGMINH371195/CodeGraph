-- Events are immutable input to independently checkpointed projections.
CREATE TABLE event_outbox (
    event_sequence INTEGER PRIMARY KEY REFERENCES events(seq)
);

-- Preserve events produced before outbox support was installed.
INSERT INTO event_outbox(event_sequence) SELECT seq FROM events;

CREATE TRIGGER enqueue_event_outbox AFTER INSERT ON events
BEGIN
    INSERT INTO event_outbox(event_sequence) VALUES (NEW.seq);
END;

CREATE TABLE projection_cursors (
    consumer TEXT PRIMARY KEY CHECK(length(trim(consumer)) > 0),
    event_sequence INTEGER NOT NULL REFERENCES event_outbox(event_sequence)
);
