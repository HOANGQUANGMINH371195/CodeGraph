// W0 diagnostic wrapper; reuses upstream probe, not a parallel scoring engine.
import { cpSync, mkdtempSync, readFileSync, readdirSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { sourceFingerprint } from './source-fingerprint.mjs';

const [repoArg, receiptArg] = process.argv.slice(2);
if (!repoArg || !receiptArg) throw Error('usage: node scripts/orders-graph-smoke.mjs CODEGRAPH_REPO NEW_RECEIPT');
const repo = resolve(repoArg), receiptPath = resolve(receiptArg);
if (existsSync(receiptPath)) throw Error('receipt exists');
const product = resolve(import.meta.dirname, '..');
const fixture = join(product, 'fixtures/orders');
const questionsPath = join(product, 'benchmarks/orders-v1.json');
const sqlTruthPath = join(product, 'benchmarks/orders-sql-v1.json');
const hashes = root => {
  const result = {};
  const visit = relative => {
    for (const entry of readdirSync(join(root, relative), {withFileTypes: true})) {
      const name = join(relative, entry.name);
      if (entry.isDirectory()) visit(name);
      else if (entry.isFile()) result[name] = createHash('sha256').update(readFileSync(join(root, name))).digest('hex');
      else throw Error('unexpected non-file entry');
    }
  };
  visit(''); return result;
};
process.env.CODEGRAPH_TELEMETRY = '0';
process.env.CODEGRAPH_NO_UPDATE_CHECK = '1';
const before = sourceFingerprint(repo);
const dist = hashes(join(repo, 'dist'));
const input = hashes(fixture);
const dir = mkdtempSync(join(tmpdir(), 'harness-orders-graph-'));
const receipt = {schema_version: 1, kind: 'orders_graph_smoke', node: process.version,
  command: process.argv, source_before: {revision: before.revision, clean: before.clean, sha256: before.sha256},
  dist_sha256: dist, fixture_sha256: input,
  questions_sha256: createHash('sha256').update(readFileSync(questionsPath)).digest('hex'),
  build_source_correspondence_verified: false, product_acceptance: false,
  scope: 'Index/watch API and upstream ToolHandler probes; no MCP transport or live agent benchmark', probes: []};
let cg;
try {
  cpSync(fixture, dir, {recursive: true});
  const mod = await import(pathToFileURL(join(repo, 'dist/index.js')).href);
  const CodeGraph = mod.default?.default ?? mod.default ?? mod.CodeGraph;
  cg = CodeGraph.initSync(dir);
  receipt.index = await cg.indexAll();
  receipt.stats = cg.getStats();
  const sqlTruthBytes = readFileSync(sqlTruthPath);
  const sqlTruth = JSON.parse(sqlTruthBytes);
  if (sqlTruth.schema_version !== 1 || sqlTruth.kind !== 'authored_sql_ground_truth'
    || input[sqlTruth.source.path] !== sqlTruth.source.sha256
    || sqlTruth.queries.some(query => input[query.path] !== query.sha256)) {
    throw Error('SQL ground truth is stale or unsupported');
  }
  receipt.sql_inventory = {
    ground_truth_sha256: createHash('sha256').update(sqlTruthBytes).digest('hex'),
    scope: 'SQL file inventory only; authored expectations are not extracted graph facts',
    linkage_verified: false, table_extraction_verified: false,
    files: sqlTruth.queries.map(query => ({
      path: query.path, expected_sha256: query.sha256,
      indexed_file: cg.getFile(query.path) ?? null,
      indexed_nodes: cg.getNodesInFile(query.path),
    })),
  };
  receipt.routes = cg.getNodesByKind('route').map(node => ({ node, edges: cg.getOutgoingEdges(node.id) }));
  receipt.route_flow_queries = [];
  for (const [routeName, file, targetName, expected] of [
    ['POST /orders', 'src/store.mjs', 'createOrder', 'candidate'],
    ['POST /dispatch', 'src/relay.mjs', 'dispatch', 'candidate'],
    ['POST /events', 'src/store.mjs', 'consume', 'candidate'],
    ['POST /dispatch', 'src/store.mjs', 'createOrder', 'not_found'],
  ]) {
    const route = receipt.routes.find(entry => entry.node.name === routeName)?.node;
    const target = cg.getNodesInFile(file).find(node => node.kind === 'method' && node.name === targetName);
    if (!route || !target) throw Error('route flow diagnostic endpoint missing');
    const result = await cg.getRouteFlow(route.id, target.id);
    receipt.route_flow_queries.push({ route: routeName, file, target: targetName, expected, result });
    if (result.status !== expected || result.runtimeVerified !== false) throw Error('route flow diagnostic mismatch');
  }
  receipt.nodes = ['src/main.mjs','src/http.mjs','src/service.mjs','src/store.mjs','src/relay.mjs']
    .flatMap(file => cg.getNodesInFile(file).map(node => ({node, callees: cg.getCallees(node.id)})));
  // Development-only observation of candidate wiring. The join is read-only
  // and not a public CodeGraph API or proof that the graph contains these edges.
  const { joinConstructorEvidence } = await import(pathToFileURL(join(repo, 'dist/resolution/constructor-join.js')).href);
  receipt.constructor_evidence = await joinConstructorEvidence(cg.resolver.context);
  const { reviewInjectedReceivers } = await import(pathToFileURL(join(repo, 'dist/resolution/receiver-decision.js')).href);
  receipt.receiver_decisions = reviewInjectedReceivers(receipt.constructor_evidence);
  const { mapInjectedFieldCalls } = await import(pathToFileURL(join(repo, 'dist/resolution/injected-call-mapping.js')).href);
  receipt.injected_call_mapping = mapInjectedFieldCalls(receipt.constructor_evidence, cg.resolver.context);
  const { mapParameterCallGraph, PARAMETER_CALL_PRODUCER } = await import(pathToFileURL(join(repo, 'dist/resolution/parameter-call-mapping.js')).href);
  receipt.parameter_call_mapping = mapParameterCallGraph(receipt.constructor_evidence, cg.resolver.context);
  const parameterMarker = cg.queries.getMetadata(`synthesis:${PARAMETER_CALL_PRODUCER}`);
  receipt.parameter_reconciliation = parameterMarker === null ? null : JSON.parse(parameterMarker);
  receipt.parameter_summary = {
    mapping_status: receipt.parameter_call_mapping.status,
    candidate_edges: receipt.parameter_call_mapping.edges.length,
    observations: receipt.parameter_call_mapping.evidence.length,
    lifecycle_status: receipt.parameter_reconciliation?.status ?? 'missing',
    inserted_edges: receipt.parameter_reconciliation?.counts?.inserted ?? null,
    conflicting_edges: receipt.parameter_reconciliation?.counts?.conflicts ?? null,
  };
  const parameterEdges = receipt.parameter_call_mapping.edges;
  receipt.parameter_query = parameterEdges.length
    ? cg.getParameterCallEvidence({ edges: parameterEdges, maxEvidence: 100 })
    : null;
  receipt.parameter_summary.query_status = receipt.parameter_query?.status ?? 'no-candidates';
  receipt.parameter_summary.query_observations = receipt.parameter_query?.evidence.length ?? 0;
  const toolsModule = await import(pathToFileURL(join(repo, 'dist/mcp/tools.js')).href);
  const ToolHandler = toolsModule.ToolHandler ?? toolsModule.default?.ToolHandler;
  const handler = new ToolHandler(cg);
  receipt.parameter_tool_responses = [];
  for (const [tool, endpoint] of [['codegraph_callers', 'target'], ['codegraph_callees', 'source']]) {
    for (const id of new Set(parameterEdges.map(edge => edge[endpoint]))) {
      const node = cg.getNode(id);
      if (!node) throw Error(`parameter endpoint missing: ${id}`);
      const args = { symbol: node.qualifiedName, file: node.filePath, limit: 100 };
      const result = await handler.execute(tool, args);
      receipt.parameter_tool_responses.push({ tool, endpoint_id: id, args, result });
    }
  }
  cg.close(); cg = null;
  for (const question of JSON.parse(readFileSync(questionsPath)).questions) {
    const command = [process.execPath, 'scripts/agent-eval/probe-explore.mjs', dir, question.question];
    const start = performance.now();
    const result = spawnSync(command[0], command.slice(1), {cwd: repo, env: process.env,
      encoding: 'utf8', timeout: 30000, maxBuffer: 2 * 1024 * 1024});
    receipt.probes.push({id: question.id, command, elapsed_ms: performance.now() - start,
      exit_code: result.status, signal: result.signal, error: result.error?.message,
      stdout: result.stdout, stderr: result.stderr});
  }
  cg = CodeGraph.openSync(dir);
  let syncCompleted = false;
  receipt.watch_errors = [];
  receipt.watch_started = cg.watch({debounceMs: 200,
    onSyncComplete: result => { receipt.watch_sync = result; syncCompleted = true; },
    onSyncError: error => { receipt.watch_errors.push(String(error)); },
  });
  if (!receipt.watch_started) throw Error('watch did not start');
  await new Promise(r => setTimeout(r, 100));
  const file = join(dir, 'src/service.mjs');
  writeFileSync(file, readFileSync(file, 'utf8') + '\nexport function ordersWatchProbe() { return 1; }\n');
  const start = performance.now();
  while (performance.now() - start < 8000) {
    if (syncCompleted && cg.getNodesInFile('src/service.mjs').some(n => n.name === 'ordersWatchProbe')) break;
    await new Promise(r => setTimeout(r, 100));
  }
  receipt.watch_observed = syncCompleted && cg.getNodesInFile('src/service.mjs').some(n => n.name === 'ordersWatchProbe');
  receipt.watch_elapsed_ms = performance.now() - start;
} catch (error) { receipt.error = String(error); }
finally {
  if (cg) { cg.unwatch(); cg.close(); }
  rmSync(dir, {recursive: true}); // Only this invocation's mkdtemp copy.
}
receipt.source_unchanged = sourceFingerprint(repo).sha256 === before.sha256;
receipt.dist_unchanged = JSON.stringify(hashes(join(repo, 'dist'))) === JSON.stringify(dist);
receipt.fixture_unchanged = JSON.stringify(hashes(fixture)) === JSON.stringify(input);
writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify({receipt: receiptPath, stats: receipt.stats,
  parameter_summary: receipt.parameter_summary,
  probes: receipt.probes.map(p => ({id:p.id, exit_code:p.exit_code})),
  watch_observed: receipt.watch_observed, error: receipt.error,
  source_unchanged: receipt.source_unchanged, product_acceptance: false}));
if (receipt.error || receipt.watch_errors?.length || !receipt.watch_observed || !receipt.source_unchanged || !receipt.dist_unchanged
  || !receipt.fixture_unchanged || receipt.probes.some(p => p.exit_code !== 0)) process.exitCode = 1;
