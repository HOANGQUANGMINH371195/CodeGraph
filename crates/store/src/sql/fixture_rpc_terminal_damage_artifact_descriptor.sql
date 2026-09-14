UPDATE artifacts SET descriptor=json_set(descriptor, '$.byte_length', 1) WHERE id='stderr';
