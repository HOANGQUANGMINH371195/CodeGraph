// Explicit local integration gate. Does not install or modify Archify, open a
// browser, contact a model, or claim all diagram views are implemented.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const product = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const [archifyInput, outputInput] = process.argv.slice(2);
assert.ok(archifyInput, 'Usage: node scripts/validate-deployment-diagram.mjs ARCHIFY_SKILL_ROOT [NEW_OUTPUT_DIR]');
const archify = path.resolve(archifyInput);
const cli = path.join(archify, 'bin/archify.mjs');
assert.ok(fs.statSync(cli).isFile(), 'Archify CLI must exist');
const output = outputInput ? path.resolve(outputInput) : fs.mkdtempSync(path.join(os.tmpdir(), 'graph-diagram-'));
if (outputInput) fs.mkdirSync(output); // Exclusive new directory: never overwrite a user artifact.
const root = path.join(output, 'source');
fs.mkdirSync(root);
const binary = path.join(product, 'target/debug/project-graph-agent');
const database = path.join(output, 'state.db');
const taskPath = path.join(output, 'task.json');
const citationPath = path.join(output, 'citation.json');
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const writeJson = (file, value) => fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
function execute(command, args, expected = 0) {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 30000, maxBuffer: 8 * 1024 * 1024, cwd: output });
  assert.ifError(result.error);
  assert.equal(result.signal, null);
  if (expected === 0) assert.equal(result.status, 0, `${command}: ${result.stderr}\n${result.stdout}`);
  else assert.notEqual(result.status, 0, 'invalid candidate unexpectedly accepted');
  return result;
}
const harness = (...args) => JSON.parse(execute(binary, ['--database', database, ...args]).stdout);
const source = fs.readFileSync(path.join(product, 'fixtures/orders/compose.yaml'));
fs.writeFileSync(path.join(root, 'compose.yaml'), source, { flag: 'wx' });
const project = {
  repository_id: 'orders-fixture', worktree_id: 'local-fixture', git_head: 'caller-supplied',
  working_tree_fingerprint: 'fixture', config_hash: 'fixture', ignore_policy_version: '1',
};
const lines = source.toString('utf8').split('\n').length - (source.at(-1) === 10 ? 1 : 0);
writeJson(citationPath, {
  schema_version: 1, id: 'compose-fixture', project, graph_version: 'diagram-fixture-v1',
  path: 'compose.yaml', content_sha256: digest(source), start_line: 1, end_line: lines, analysis_run: 'fixture',
});
writeJson(taskPath, {
  schema_version: 1, id: 'diagram-fixture', project, graph_version: 'diagram-fixture-v1',
  role: 'reader', account_lane: 'local', scope: ['compose.yaml'], dependencies: [],
  context_ref: 'fixture', expected_artifacts: ['diagram'], token_budget: 100,
});
harness('record-evidence', citationPath);
harness('publish-compose', 'compose-fixture', taskPath, root, '--expected-generation', '0');
const snapshot = harness('deployment', 'compose-fixture', taskPath);
const seed = snapshot.graph.nodes.find((node) => node.name === 'orders' && node.kind === 'service').id;
const bundle = harness('deployment-diagram', 'compose-fixture', taskPath, '--node', seed);
assert.equal(bundle.kind, 'deployment_diagram');
assert.equal(bundle.evidence_manifest.generation, 1);
assert.equal(bundle.evidence_manifest.relationship_verified, false);
assert.equal(bundle.evidence_manifest.source_bytes_verified, false);
assert.equal(bundle.evidence_manifest.analysis_run_verified, false);
assert.equal(bundle.diagram.components.length, 2);
assert.equal(bundle.diagram.connections.length, 1);
assert.equal(bundle.diagram.connections[0].variant, 'dashed');
assert.equal(bundle.diagram.connections[0].label, 'mounts');
for (const binding of bundle.connection_bindings) {
  const drawn = bundle.diagram.connections.find((edge) => edge.id === binding.diagram_id);
  const fact = bundle.evidence_manifest.edges[binding.context_edge_index];
  for (const [drawnKey, factKey] of [['from', 'source'], ['to', 'target']]) {
    const nodeBinding = bundle.component_bindings.find((node) => node.diagram_id === drawn[drawnKey]);
    assert.equal(bundle.evidence_manifest.nodes[nodeBinding.context_node_index].id, fact[factKey]);
  }
  assert.ok(bundle.evidence_manifest.citations.some((citation) => citation.id === fact.evidence_id));
}
const bundlePath = path.join(output, 'bundle.json');
const inputPath = path.join(output, 'architecture.json');
const htmlPath = path.join(output, 'architecture.html');
writeJson(bundlePath, bundle);
writeJson(inputPath, bundle.diagram);
const validation = JSON.parse(execute(process.execPath, [cli, 'validate', 'architecture', inputPath, '--json']).stdout);
assert.equal(validation.ok, true);
const delivery = JSON.parse(execute(process.execPath, [cli, 'deliver', 'architecture', inputPath, htmlPath, '--json']).stdout);
assert.equal(delivery.ok, true);
const html = fs.readFileSync(htmlPath);
assert.match(html.toString('utf8'), /Declared deployment neighborhood/);
assert.match(html.toString('utf8'), /Historical snapshot/);
assert.match(html.toString('utf8'), /Unknowns:/);
assert.doesNotMatch(html.toString('utf8'), /id="archify-source-evidence-data"/);
const badPath = path.join(output, 'invalid.architecture.json');
writeJson(badPath, { ...bundle.diagram, unsupported_evidence_claim: true });
const failed = JSON.parse(execute(process.execPath, [cli, 'deliver', 'architecture', badPath, htmlPath, '--json'], 1).stdout);
assert.equal(failed.ok, false);
assert.deepEqual(fs.readFileSync(htmlPath), html, 'failed candidate changed last-good artifact');
const revision = execute('git', ['-C', archify, 'rev-parse', 'HEAD']).stdout.trim();
const receipt = {
  schema_version: 1, kind: 'deployment_diagram_integration', ok: true,
  archify_revision: revision,
  archify_schema_sha256: digest(fs.readFileSync(path.join(archify, 'schemas/architecture.schema.json'))),
  harness_binary_sha256: digest(fs.readFileSync(binary)),
  input_sha256: digest(fs.readFileSync(inputPath)), artifact_sha256: digest(html),
  bundle_sha256: digest(fs.readFileSync(bundlePath)),
  validation, delivery, invalid_candidate: { ok: failed.ok, stage: failed.stage },
  last_good_preserved: true, runtime_semantics_verified: false, visual_review_completed: false,
  coverage: 'Orders Compose declared architecture neighborhood only; not five-view acceptance',
  output_directory: output,
};
writeJson(path.join(output, 'receipt.json'), receipt);
console.log(JSON.stringify({ ok: true, output_directory: output, artifact_sha256: receipt.artifact_sha256, last_good_preserved: true }));
