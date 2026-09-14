SELECT output_run, stdout_id, stderr_id, finished_at_ms, descriptor
FROM rpc_terminal_receipts WHERE launch_id=?1;
