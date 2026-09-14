SELECT id, payload FROM outbox WHERE delivered = 0 ORDER BY id LIMIT 100;
