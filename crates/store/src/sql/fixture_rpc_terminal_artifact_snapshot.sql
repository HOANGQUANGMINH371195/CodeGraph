SELECT json_group_array(json_array(id, project, graph_version, analysis_run, descriptor))
FROM (SELECT * FROM artifacts ORDER BY id);
