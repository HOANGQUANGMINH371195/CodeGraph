CREATE TRIGGER reject_deployment_edge BEFORE INSERT ON deployment_edges BEGIN SELECT RAISE(ABORT,'fixture rejection'); END;
