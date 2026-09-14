// Reproducible emitted-CLI syntax check, not graph/runtime acceptance.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const [binaryArg, receiptArg, ...extra] = process.argv.slice(2);
if (!binaryArg || !receiptArg || extra.length) {
  throw Error('usage: node scripts/orders-sql-cli-smoke.mjs BINARY NEW_RECEIPT');
}
const binary = resolve(binaryArg), receiptPath = resolve(receiptArg);
if (existsSync(receiptPath)) throw Error('receipt exists');
const product = resolve(import.meta.dirname, '..');
const root = join(product, 'fixtures/orders');
const truthPath = join(product, 'benchmarks/orders-sql-v1.json');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const receipt = { schema_version: 1, kind: 'orders_sql_cli_smoke',
  scope: 'Authored expectations versus emitted CLI SQLite syntax, not graph/runtime acceptance',
  node: process.version, product_acceptance: false,
  build_source_correspondence_verified: false, reports: [], checks: [], error: null };
const dir = mkdtempSync(join(tmpdir(), 'harness-orders-sql-cli-'));
const inputs = new Map();
try {
  receipt.binary_sha256 = hash(readFileSync(binary));
  const truthBytes = readFileSync(truthPath);
  inputs.set(truthPath, hash(truthBytes));
  receipt.ground_truth_sha256 = hash(truthBytes);
  const truth = JSON.parse(truthBytes);
  assert.equal(truth.schema_version, 1);
  assert.equal(truth.kind, 'authored_sql_ground_truth');
  assert.equal(truth.queries.length, 11, 'versioned Orders corpus must be complete');
  assert.equal(new Set(truth.queries.map(q => q.path)).size, 11);
  for (const input of [truth.source, ...truth.queries]) {
    assert.match(input.path, /^(src|sql)\/[a-zA-Z0-9_.-]+$/);
    const path = join(root, input.path);
    const actual = hash(readFileSync(path));
    assert.equal(actual, input.sha256, 'authored expectations are stale');
    inputs.set(path, actual);
  }
  const project = { repository_id: 'orders-fixture', worktree_id: 'local',
    git_head: 'caller-supplied', working_tree_fingerprint: 'authored-fixture',
    config_hash: 'default', ignore_policy_version: '1' };
  const run = args => {
    const result = spawnSync(binary, ['--database', join(dir, 'state.db'), ...args],
      { timeout: 10000, maxBuffer: 1024 * 1024 });
    if (result.error || result.signal) throw Error('CLI launch/timeout/transport failure');
    return result;
  };
  const success = result => {
    assert.equal(result.status, 0, 'CLI command failed');
    assert.ok(result.stdout.length > 0);
    assert.equal(result.stdout.at(-1), 10, 'JSON output must end in LF');
    return JSON.parse(result.stdout.toString('utf8'));
  };
  for (const query of truth.queries) {
    const bytes = readFileSync(join(root, query.path));
    const text = bytes.toString('utf8');
    const lines = text.split('\n').length - Number(text.endsWith('\n'));
    const evidence = { schema_version: 1, id: query.path, project, graph_version: 'sql-demo-v1',
      path: query.path, content_sha256: query.sha256, start_line: 1, end_line: lines,
      analysis_run: 'unverified-demo-run' };
    const task = { schema_version: 1, id: 'sql-probe', project, graph_version: 'sql-demo-v1',
      role: 'reader', account_lane: 'local', scope: [query.path], dependencies: [],
      context_ref: 'ctx', expected_artifacts: ['report'], token_budget: 100 };
    const citationPath = join(dir, 'citation.json'), taskPath = join(dir, 'task.json');
    writeFileSync(citationPath, JSON.stringify(evidence));
    writeFileSync(taskPath, JSON.stringify(task));
    success(run(['record-evidence', citationPath]));
    const args = ['analyze-sql', query.path, taskPath, root];
    const emitted = run(args);
    const report = success(emitted);
    assert.equal(report.schema_version, 1);
    assert.equal(report.kind, 'sql_analysis');
    assert.equal(report.dialect, 'sqlite');
    assert.deepEqual(report.evidence, evidence);
    for (const field of ['candidate_only', 'source_is_untrusted', 'content_hash_and_lines_verified']) {
      assert.equal(report[field], true);
    }
    for (const field of ['persisted', 'analysis_run_verified', 'semantic_verified', 'relationship_verified']) {
      assert.equal(report[field], false);
    }
    assert.equal(report.snapshot_binding, 'caller_supplied');
    assert.equal(report.relation_semantics, 'syntactic_unresolved');
    assert.equal(report.span_coverage, 'parser_hint_not_full_statement');
    assert.ok(report.statements.length > 0);
    const operation = ({ ddl: 'create_table', select: 'query' })[query.operation] ?? query.operation;
    for (const [ordinal, statement] of report.statements.entries()) {
      assert.equal(statement.ordinal, ordinal);
      assert.equal(statement.operation, operation);
    }
    assert.deepEqual(report.statements.flatMap(s => s.relations), query.tables);
    const exact = run([...args, '--max-output-bytes', String(emitted.stdout.length)]);
    success(exact);
    assert.deepEqual(exact.stdout, emitted.stdout, 'exact-cap output must be stable');
    const short = run([...args, '--max-output-bytes', String(emitted.stdout.length - 1)]);
    assert.notEqual(short.status, 0);
    assert.equal(short.stdout.length, 0, 'failure must not emit partial JSON');
    receipt.reports.push(report);
    receipt.checks.push({ path: query.path, output_bytes: emitted.stdout.length,
      exact_cap_passed: true, one_under_rejected: true });
  }
  receipt.binary_unchanged = hash(readFileSync(binary)) === receipt.binary_sha256;
  assert.equal(receipt.binary_unchanged, true);
} catch (error) {
  receipt.error = error instanceof Error ? error.message : 'probe failed';
} finally {
  try {
    receipt.inputs_unchanged = [...inputs].every(([path, expected]) => hash(readFileSync(path)) === expected);
  } catch { receipt.inputs_unchanged = false; }
  try { rmSync(dir, { recursive: true }); } // Only this invocation's mkdtemp directory.
  catch { receipt.cleanup_failed = true; }
}
receipt.passed = receipt.error === null && receipt.reports.length === 11
  && receipt.inputs_unchanged === true && receipt.binary_unchanged === true && !receipt.cleanup_failed;
writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ receipt: receiptPath, passed: receipt.passed,
  files: receipt.reports.length, statements: receipt.reports.reduce((n, r) => n + r.statements.length, 0),
  error: receipt.error, product_acceptance: false }));
if (!receipt.passed) process.exitCode = 1;
