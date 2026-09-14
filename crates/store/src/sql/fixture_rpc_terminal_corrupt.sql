DROP TRIGGER rpc_terminal_no_update;
UPDATE rpc_terminal_receipts SET descriptor='{"secret":"broken"}';
