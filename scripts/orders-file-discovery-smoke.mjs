// Source-only discovery followed by bounded capture; no SQL execution/publication.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join, resolve } from 'node:path';

const [codegraphArg, receiptArg, ...extra] = process.argv.slice(2);
if (!codegraphArg || !receiptArg || extra.length) {
  throw Error('usage: node scripts/orders-file-discovery-smoke.mjs CODEGRAPH NEW_RECEIPT');
}
const codegraph = resolve(codegraphArg), receiptPath = resolve(receiptArg);
if (existsSync(receiptPath)) throw Error('receipt exists');
const product = resolve(import.meta.dirname, '..'), root = join(product, 'fixtures/orders');
const sourcePath = 'src/store.mjs';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const inputs = new Map();
const trackedRead = path => {
  const bytes = readFileSync(path);
  const actual = hash(bytes);
  if (inputs.has(path)) assert.equal(actual, inputs.get(path), 'input changed during probe');
  inputs.set(path, actual);
  return bytes;
};
const receipt = { schema_version: 1, kind: 'orders_file_discovery_smoke',
  scope: 'same-file imported fs readFileSync candidates, not runtime or graph acceptance',
  node: process.version, product_acceptance: false, runtime_verified: false,
  sql_executed: false, graph_published: false, build_source_correspondence_verified: false,
  transport: 'emitted-file-reads-cli', discovery: null, captures: [], error: null };
try {
  // Inventory inputs for reproducibility, not proof of the complete build closure.
  for (const name of ['file-read-discovery', 'file-read-binding', 'file-read-target',
    'local-method-target', 'string-call-chain', 'construction-sites', 'receiver-hazards']) {
    trackedRead(join(codegraph, 'src/resolution', `${name}.ts`));
    trackedRead(join(codegraph, 'dist/resolution', `${name}.js`));
  }
  trackedRead(join(codegraph, 'src/bin/codegraph.ts'));
  const cli = join(codegraph, 'dist/bin/codegraph.js');
  trackedRead(cli);
  const bytes = trackedRead(join(root, sourcePath));
  const run = (extra = []) => {
    const result = spawnSync(process.execPath, [cli, 'file-reads', sourcePath, '--path', root, ...extra], {
      timeout: 15000, maxBuffer: 2 * 1024 * 1024,
      env: { ...process.env, CODEGRAPH_NO_DAEMON: '1', CODEGRAPH_WASM_RELAUNCHED: '1',
        CODEGRAPH_TELEMETRY: '0', CODEGRAPH_NO_UPDATE_CHECK: '1' },
    });
    if (result.error || result.signal) throw Error('CLI launch/timeout/transport failure');
    return result;
  };
  const emitted = run();
  assert.equal(emitted.status, 0, 'file-reads CLI failed');
  assert.equal(emitted.stdout.at(-1), 10);
  const report = JSON.parse(emitted.stdout.toString('utf8'));
  assert.equal(report.schemaVersion, 1);
  assert.equal(report.kind, 'file_reads');
  assert.equal(report.sourceCoordinateEncoding, 'utf16-code-unit');
  assert.equal(report.status, 'observed');
  assert.equal(report.persisted, false);
  assert.equal(report.runtimeVerified, false);
  assert.equal(report.atomicSnapshotVerified, false);
  assert.equal(report.raceFreeContainmentVerified, false);
  assert.equal(report.requiresStableFilesystem, true);
  assert.equal(report.source.sha256, hash(bytes));
  const exact = run(['--max-output-bytes', String(emitted.stdout.length)]);
  assert.equal(exact.status, 0);
  assert.deepEqual(exact.stdout, emitted.stdout);
  const under = run(['--max-output-bytes', String(emitted.stdout.length - 1)]);
  assert.equal(under.status, 1);
  assert.equal(under.stdout.length, 0);
  receipt.output_bytes = emitted.stdout.length;
  receipt.exact_cap_passed = true;
  receipt.one_under_rejected = true;
  const discovery = report.discovery;
  receipt.discovery = discovery;
  receipt.captures = report.targets;
  assert.equal(discovery.sourceSha256, hash(bytes));
  assert.equal(discovery.status, 'observed');
  assert.equal(discovery.coverage, 'recognized-fs-imports-only');
  assert.equal(discovery.coordinateEncoding, 'utf16-code-unit');
  assert.equal(discovery.runtimeVerified, false);
  assert.equal(discovery.truncated, false);
  assert.equal(discovery.omittedIssues, 0);
  assert.deepEqual(discovery.issues, []);
  assert.equal(report.targets.length, discovery.candidates.length);
  for (const [index, candidate] of discovery.candidates.entries()) {
    assert.equal(candidate.status, 'candidate');
    assert.deepEqual(Object.keys(candidate.read.invocation).sort(), ['end', 'start']);
    const capture = report.targets[index];
    assert.equal(capture.status, 'captured-candidate');
    assert.equal(capture.containmentChecksPassed, true);
    assert.equal(capture.runtimeVerified, false);
    assert.equal(capture.atomicSnapshotVerified, false);
    assert.equal(capture.raceFreeContainmentVerified, false);
    assert.equal(capture.requiresStableFilesystem, true);
    assert.equal(capture.source.sha256, discovery.sourceSha256);
    assert.equal(capture.candidateIndex, index, 'target must refer to its canonical discovery binding');
    assert.equal(Object.hasOwn(capture, 'binding'), false, 'do not duplicate complete caller chains');
  }
  // Ground truth is deliberately unavailable to the discovery/capture stages.
  const truth = JSON.parse(trackedRead(join(product, 'benchmarks/orders-sql-v1.json')));
  assert.equal(truth.schema_version, 1);
  assert.equal(truth.kind, 'authored_sql_ground_truth');
  assert.deepEqual(truth.source, { path: sourcePath, sha256: discovery.sourceSha256 });
  assert.equal(truth.queries.length, 11);
  const actual = receipt.captures.map(c => ({ path: c.target.path, sha256: c.target.sha256 }));
  const expected = truth.queries.map(q => ({ path: q.path, sha256: q.sha256 }));
  const sorted = entries => entries.sort((a, b) => a.path.localeCompare(b.path));
  assert.deepEqual(sorted(actual), sorted(expected));
  for (const target of actual) {
    assert.equal(hash(trackedRead(join(root, target.path))), target.sha256);
  }
} catch (error) {
  receipt.error = error instanceof Error ? error.message : 'probe failed';
}
receipt.input_sha256 = Object.fromEntries(inputs);
try {
  receipt.inputs_unchanged = [...inputs].every(([path, digest]) => hash(readFileSync(path)) === digest);
} catch { receipt.inputs_unchanged = false; }
receipt.passed = receipt.error === null && receipt.captures.length === 11 && receipt.inputs_unchanged;
writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ receipt: receiptPath, passed: receipt.passed,
  candidates: receipt.discovery?.candidates.length ?? 0, captured: receipt.captures.length,
  visited_states: receipt.discovery?.visitedStates, error: receipt.error, product_acceptance: false }));
if (!receipt.passed) process.exitCode = 1;
