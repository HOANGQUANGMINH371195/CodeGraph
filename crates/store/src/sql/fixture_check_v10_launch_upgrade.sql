SELECT (SELECT count(*) FROM execution_launch_claims),
       (SELECT count(*) FROM execution_plans WHERE run_id='old-run' AND descriptor='legacy plan');
