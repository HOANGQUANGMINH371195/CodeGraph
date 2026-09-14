INSERT INTO notifications(id, order_id) VALUES (?, ?) ON CONFLICT(id) DO NOTHING;
