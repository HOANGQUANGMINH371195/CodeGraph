// P0.T02: real stdio MCP smoke for the emitted CodeGraph build.
// The direct index/watch smoke intentionally stays separate: this script only
// proves initialize -> tools/list -> status/explore over JSON-RPC.
import { cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { sourceFingerprint } from './source-fingerprint.mjs';

const [repoArg, fixtureArg, receiptArg] = process.argv.slice(2);
if (!repoArg || !fixtureArg || !receiptArg) {
  throw Error('usage: node scripts/codegraph-mcp-smoke.mjs CODEGRAPH_REPO FIXTURE NEW_RECEIPT');
}

const engine = resolve(repoArg);
const fixture = resolve(fixtureArg);
const receiptPath = resolve(receiptArg);
const binary = join(engine, 'dist/bin/codegraph.js');
const hashBytes = bytes => createHash('sha256').update(bytes).digest('hex');
const hashTree = root => {
  const out = {};
  const visit = relative => {
    for (const entry of readdirSync(join(root, relative), { withFileTypes: true })) {
      const name = join(relative, entry.name);
      if (entry.isDirectory()) visit(name);
      else if (entry.isFile()) out[name] = hashBytes(readFileSync(join(root, name)));
      else throw Error(`unexpected non-file entry: ${name}`);
    }
  };
  visit('');
  return out;
};
const waitFor = (messages, predicate, timeoutMs = 10000) => new Promise((resolveResult, reject) => {
  const started = Date.now();
  const tick = () => {
    const found = messages.find(predicate);
    if (found) return resolveResult(found);
    if (Date.now() - started >= timeoutMs) {
      return reject(Error(`timeout waiting for MCP response; messages=${JSON.stringify(messages)}`));
    }
    setTimeout(tick, 20);
  };
  tick();
});

if (existsSync(receiptPath)) {
  throw Error(`receipt exists: ${receiptPath}`);
}

const sourceBefore = sourceFingerprint(engine);
const distBefore = hashTree(join(engine, 'dist'));
const fixtureBefore = hashTree(fixture);
const tempProject = mkdtempSync(join(tmpdir(), 'harness-mcp-orders-'));
const receipt = {
  schema_version: 1,
  kind: 'codegraph_mcp_smoke',
  command: process.argv,
  source_before: { revision: sourceBefore.revision, clean: sourceBefore.clean, sha256: sourceBefore.sha256 },
  dist_sha256: distBefore,
  fixture_sha256: fixtureBefore,
  scope: 'Real emitted CodeGraph stdio MCP: initialize, tools/list, status and explore; temporary indexed fixture only',
  product_acceptance: false,
};

let child = null;
try {
  cpSync(fixture, tempProject, { recursive: true });
  const module = await import(pathToFileURL(join(engine, 'dist/index.js')).href);
  const CodeGraph = module.default?.default ?? module.default ?? module.CodeGraph;
  if (typeof CodeGraph?.initSync !== 'function') throw Error('dist CodeGraph.initSync unavailable');
  const graph = CodeGraph.initSync(tempProject);
  receipt.direct_index = await graph.indexAll();
  graph.close();

  const messages = [];
  let buffer = '';
  let stderr = '';
  child = spawn(process.execPath, [binary, 'serve', '--mcp', '--no-watch', '--path', tempProject], {
    cwd: tempProject,
    execArgv: [],
    env: {
      ...process.env,
      CODEGRAPH_NO_DAEMON: '1',
      CODEGRAPH_NO_UPDATE_CHECK: '1',
      CODEGRAPH_TELEMETRY: '0',
    },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  child.stdout.on('data', chunk => {
    buffer += chunk.toString('utf8');
    let end;
    while ((end = buffer.indexOf('\n')) >= 0) {
      const line = buffer.slice(0, end).trim();
      buffer = buffer.slice(end + 1);
      if (!line) continue;
      try { messages.push(JSON.parse(line)); }
      catch { messages.push({ parse_error: line }); }
    }
  });
  child.stderr.on('data', chunk => { stderr += chunk.toString('utf8'); });
  const send = message => child.stdin.write(`${JSON.stringify(message)}\n`);
  send({
    jsonrpc: '2.0', id: 1, method: 'initialize',
    params: {
      protocolVersion: '2025-11-25', capabilities: {},
      clientInfo: { name: 'project-graph-agent-p0-t02', version: '1' },
      rootUri: `file://${tempProject}`,
    },
  });
  const initialized = await waitFor(messages, message => message.id === 1 && message.result);
  send({ jsonrpc: '2.0', method: 'notifications/initialized' });
  send({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} });
  const listed = await waitFor(messages, message => message.id === 2);
  send({ jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'codegraph_status', arguments: {} } });
  const status = await waitFor(messages, message => message.id === 3);
  send({
    jsonrpc: '2.0', id: 4, method: 'tools/call',
    params: { name: 'codegraph_explore', arguments: { query: 'POST /orders OrderService createOrder' } },
  });
  const explored = await waitFor(messages, message => message.id === 4);

  const toolNames = (listed.result?.tools ?? []).map(tool => tool.name);
  const statusText = status.result?.content?.[0]?.text ?? '';
  const exploreText = explored.result?.content?.[0]?.text ?? '';
  receipt.mcp = {
    initialize_ok: Boolean(initialized.result),
    tool_names: toolNames,
    status: {
      ok: !status.error && !status.result?.isError,
      has_header: statusText.includes('CodeGraph Status'),
      has_file_count: statusText.includes('Files indexed'),
      chars: statusText.length,
    },
    explore: {
      ok: !explored.error && !explored.result?.isError,
      has_route: exploreText.includes('POST /orders'),
      has_service: exploreText.includes('OrderService'),
      has_target: exploreText.includes('createOrder'),
      chars: exploreText.length,
    },
    stderr: stderr.trim(),
  };
  if (!receipt.mcp.initialize_ok || !toolNames.includes('codegraph_explore')
    || !receipt.mcp.status.ok || !receipt.mcp.status.has_header
    || !receipt.mcp.explore.ok || !receipt.mcp.explore.has_route
    || !receipt.mcp.explore.has_target) {
    throw Error(`MCP assertions failed: ${JSON.stringify(receipt.mcp)}`);
  }
} catch (error) {
  receipt.error = String(error);
} finally {
  if (child && !child.killed) child.kill('SIGKILL');
  rmSync(tempProject, { recursive: true, force: true });
}

receipt.source_unchanged = sourceFingerprint(engine).sha256 === sourceBefore.sha256;
receipt.dist_unchanged = JSON.stringify(hashTree(join(engine, 'dist'))) === JSON.stringify(distBefore);
receipt.fixture_unchanged = JSON.stringify(hashTree(fixture)) === JSON.stringify(fixtureBefore);
receipt.product_acceptance = false;
writeFileSync(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`, { flag: 'wx' });
console.log(JSON.stringify({
  receipt: receiptPath,
  direct_index: receipt.direct_index,
  mcp: receipt.mcp,
  error: receipt.error,
  source_unchanged: receipt.source_unchanged,
  dist_unchanged: receipt.dist_unchanged,
  fixture_unchanged: receipt.fixture_unchanged,
  product_acceptance: false,
}));
if (receipt.error || !receipt.source_unchanged || !receipt.dist_unchanged || !receipt.fixture_unchanged) {
  process.exitCode = 1;
}
