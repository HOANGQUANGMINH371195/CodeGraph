// Explicit local SQL-candidate diagram gate. It does not run CodeGraph, SQL,
// a browser, a model, or claim that the static candidate executes.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const product = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const [archifyInput, outputInput] = process.argv.slice(2);
assert.ok(archifyInput, 'Usage: node scripts/validate-sql-link-diagram.mjs ARCHIFY_SKILL_ROOT [NEW_OUTPUT_DIR]');
const archify = path.resolve(archifyInput);
const cli = path.join(archify, 'bin/archify.mjs');
assert.ok(fs.statSync(cli).isFile(), 'Archify CLI must exist');
const output = outputInput ? path.resolve(outputInput) : fs.mkdtempSync(path.join(os.tmpdir(), 'graph-sql-diagram-'));
if (outputInput) fs.mkdirSync(output);
const root = path.join(output, 'source');
fs.mkdirSync(path.join(root, 'sql'), { recursive: true });
const binary = path.join(product, 'target/debug/project-graph-agent');
assert.ok(fs.statSync(binary).isFile(), 'Build project-graph-agent before this gate');
const database = path.join(output, 'state.db');
const taskPath = path.join(output, 'task.json');
const citationPath = path.join(output, 'citation.json');
const reportPath = path.join(output, 'file-reads.json');
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
const code = Buffer.from('const x = "😀";\n');
const sql = Buffer.from('SELECT 1;\n');
const codeHash = digest(code);
const sqlHash = digest(sql);
fs.writeFileSync(path.join(root, 'code.mjs'), code, { flag: 'wx' });
fs.writeFileSync(path.join(root, 'sql/q.sql'), sql, { flag: 'wx' });
const project = {
  repository_id: 'sql-diagram-fixture', worktree_id: 'local-fixture', git_head: 'caller-supplied',
  working_tree_fingerprint: 'fixture', config_hash: 'fixture', ignore_policy_version: '1',
};
writeJson(citationPath, {
  schema_version: 1, id: 'code-fixture', project, graph_version: 'diagram-fixture-v1',
  path: 'code.mjs', content_sha256: codeHash, start_line: 1, end_line: 1, analysis_run: 'fixture',
});
writeJson(taskPath, {
  schema_version: 1, id: 'diagram-fixture', project, graph_version: 'diagram-fixture-v1',
  role: 'reader', account_lane: 'local', scope: ['code.mjs'], dependencies: [],
  context_ref: 'fixture', expected_artifacts: ['diagram'], token_budget: 100,
});
writeJson(reportPath, {
  schemaVersion: 1, kind: 'file_reads', status: 'observed', sourceCoordinateEncoding: 'utf16-code-unit',
  source: { path: 'code.mjs', sha256: codeHash, byteLength: code.length, lineCount: 1 },
  discovery: { coordinateEncoding: 'utf16-code-unit', sourceSha256: codeHash, candidates: [
    { status: 'candidate', sourceSha256: codeHash, read: { invocation: { start: 0, end: 5 } } },
  ] },
  targets: [{
    status: 'captured-candidate', runtimeVerified: false, atomicSnapshotVerified: false,
    raceFreeContainmentVerified: false, containmentChecksPassed: true, requiresStableFilesystem: true,
    source: { path: 'code.mjs', sha256: codeHash, byteLength: code.length, lineCount: 1 },
    target: { path: 'sql/q.sql', sha256: sqlHash, byteLength: sql.length, lineCount: 1 }, candidateIndex: 0,
  }],
  runtimeVerified: false, atomicSnapshotVerified: false, raceFreeContainmentVerified: false,
  requiresStableFilesystem: true, persisted: false,
});
harness('record-evidence', citationPath);
harness('publish-sql-links', 'code-fixture', taskPath, reportPath, root, '--analysis-run', 'fixture', '--expected-generation', '0');
const link = 'file-read:0:sql/q.sql';
const bundle = harness('sql-link-diagram', 'code-fixture', taskPath, '--link', link);
assert.equal(bundle.kind, 'sql_link_diagram');
assert.equal(bundle.evidence_manifest.seed, link);
assert.equal(bundle.evidence_manifest.relationship_verified, false);
assert.equal(bundle.evidence_manifest.runtime_verified, false);
assert.equal(bundle.evidence_manifest.semantic_verified, false);
assert.equal(bundle.diagram.components.length, 2);
assert.equal(bundle.diagram.connections.length, 1);
assert.equal(bundle.diagram.connections[0].label, 'static file-read candidate');
assert.equal(bundle.diagram.connections[0].variant, 'dashed');
assert.doesNotMatch(JSON.stringify(bundle), /SELECT 1/);
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
assert.match(html.toString('utf8'), /Static code-to-SQL file candidate/);
assert.match(html.toString('utf8'), /not runtime traffic/);
assert.doesNotMatch(html.toString('utf8'), /SELECT 1/);
const badPath = path.join(output, 'invalid.architecture.json');
writeJson(badPath, { ...bundle.diagram, unsupported_evidence_claim: true });
const failed = JSON.parse(execute(process.execPath, [cli, 'deliver', 'architecture', badPath, htmlPath, '--json'], 1).stdout);
assert.equal(failed.ok, false);
assert.deepEqual(fs.readFileSync(htmlPath), html, 'failed candidate changed last-good artifact');
const revision = execute('git', ['-C', archify, 'rev-parse', 'HEAD']).stdout.trim();
const receipt = {
  schema_version: 1, kind: 'sql_link_diagram_integration', ok: true,
  archify_revision: revision,
  archify_schema_sha256: digest(fs.readFileSync(path.join(archify, 'schemas/architecture.schema.json'))),
  harness_binary_sha256: digest(fs.readFileSync(binary)), input_sha256: digest(fs.readFileSync(inputPath)),
  artifact_sha256: digest(html), bundle_sha256: digest(fs.readFileSync(bundlePath)),
  validation, delivery, invalid_candidate: { ok: failed.ok, stage: failed.stage },
  last_good_preserved: true, runtime_semantics_verified: false, visual_review_completed: false,
  coverage: 'One static file-read candidate only; not code execution, table/database identity, or architecture-wide data flow.',
  output_directory: output,
};
writeJson(path.join(output, 'receipt.json'), receipt);
console.log(JSON.stringify({ ok: true, output_directory: output, artifact_sha256: receipt.artifact_sha256, last_good_preserved: true }));
