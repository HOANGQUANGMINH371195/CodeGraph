SELECT (SELECT count(*) FROM rpc_terminal_receipts),
       (SELECT count(*) FROM events), (SELECT count(*) FROM event_outbox);
