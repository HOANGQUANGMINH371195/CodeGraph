PRAGMA ignore_check_constraints=ON;
UPDATE rpc_launch_claims SET claimed_at_ms=-1 WHERE launch_id='launch';
