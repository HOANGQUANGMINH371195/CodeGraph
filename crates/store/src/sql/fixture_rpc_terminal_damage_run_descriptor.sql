UPDATE analysis_runs SET descriptor=json_set(descriptor, '$.analyzer_version', 'changed');
