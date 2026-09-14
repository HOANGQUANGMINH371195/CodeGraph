// Development diagnostic, not a replacement for the real HTTP latency test.
import { createRequire } from 'node:module';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { sourceFingerprint } from './source-fingerprint.mjs';

const [repoArg, receiptArg] = process.argv.slice(2);
if (!repoArg || !receiptArg) throw Error('usage: ui-node-profile.mjs CODEGRAPH_REPO NEW_RECEIPT');
const repo = resolve(repoArg);
const before = sourceFingerprint(repo);
process.env.CODEGRAPH_TELEMETRY = '0';
process.env.CODEGRAPH_NO_UPDATE_CHECK = '1';
const require = createRequire(join(repo, 'package.json'));
const CodeGraph = require('./dist/index.js').default;
const { buildNode } = require('./dist/ui-server/api/node.js');
const when = require('./dist/ui-server/api/when.js');
const guards = require('./dist/graph/branch-guards.js');
const dir = mkdtempSync(join(tmpdir(), 'harness-ui-node-profile-'));
let cg;
let metrics = {};
const restorers = [];
const record = (name, start) => {
  const row = metrics[name] ??= { calls: 0, ms: 0 };
  row.calls++;
  row.ms += performance.now() - start;
};
function wrap(object, key, async = false) {
  const original = object[key];
  object[key] = async ? async function (...args) {
    const start = performance.now();
    try { return await original.apply(this, args); }
    finally { record(key, start); }
  } : function (...args) {
    const start = performance.now();
    try { return original.apply(this, args); }
    finally { record(key, start); }
  };
  restorers.push(() => { object[key] = original; });
}
const receipt = { kind: 'ui_node_component_profile', node: process.version,
  source_sha256: before.sha256, revision: before.revision,
  build_source_correspondence_verified: false, http_sla_verified: false,
  note: 'Inclusive nested timings overlap; do not sum them. Instrumentation adds overhead.', samples: [] };
try {
  writeFileSync(join(dir, 'hot.ts'), 'export function hot(n: number): number {\n return n * 2;\n}\n\n' +
    Array.from({length: 500}, (_, i) => `export function caller${i}(): number {\n return hot(${i});\n}`).join('\n\n'));
  cg = CodeGraph.initSync(dir, {config: {include: ['**/*.ts'], exclude: []}});
  await cg.indexAll();
  cg.resolveReferences();
  const id = cg.getNodesInFile('hot.ts').find(n => n.name === 'hot' && n.kind === 'function')?.id;
  if (!id) throw Error('missing hot fixture node');
  wrap(when, 'annotateWhen', true);
  wrap(guards, 'guardsForFile', true);
  for (const name of ['getNode','getIncomingEdges','getOutgoingEdges','getAncestors',
    'getNodesByIds','getFanIn','getFanOut','getCallers','getImpactRadius','getUnresolvedReferencesFrom']) wrap(cg, name);
  for (let i = 0; i < 11; i++) {
    metrics = {};
    const start = performance.now();
    const payload = await buildNode(cg, dir, id);
    const elapsed = performance.now() - start;
    if (payload.incoming.total !== 500 || payload.incoming.shown !== 300 || payload.blast.withinHops !== 500)
      throw Error('fixture semantics changed');
    receipt.samples.push({warm: i !== 0, elapsed_ms: elapsed, metrics});
  }
} finally {
  for (const restore of restorers.reverse()) restore();
  cg?.close();
  rmSync(dir, {recursive: true}); // Only this invocation's mkdtemp fixture.
}
receipt.source_unchanged = sourceFingerprint(repo).sha256 === before.sha256;
writeFileSync(resolve(receiptArg), JSON.stringify(receipt, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify(receipt, null, 2));
if (!receipt.source_unchanged) process.exitCode = 1;
