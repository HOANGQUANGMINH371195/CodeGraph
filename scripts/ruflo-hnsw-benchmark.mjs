// Source-bound benchmark of Ruflo's handwritten TS HNSW, not its CLI/native bridge.
// Run each size in a separate process; synthetic vectors are NOT embedding quality evidence.
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve, join } from 'node:path';
import { cpus } from 'node:os';
import { sourceFingerprint } from './source-fingerprint.mjs';

const [repoArg, tsArg, receiptArg, nArg = '1000', dimArg = '128'] = process.argv.slice(2);
if (!repoArg || !tsArg || !receiptArg) throw Error('usage: node --expose-gc scripts/ruflo-hnsw-benchmark.mjs RUFLO TYPESCRIPT_JS NEW_RECEIPT N DIM');
const n = Number(nArg), dim = Number(dimArg), k = 10, queryCount = 50;
if (!Number.isInteger(n) || n < 100 || n > 10000 || !Number.isInteger(dim) || dim < 8 || dim > 1536) throw Error('bounded run requires 100 <= N <= 10000 and 8 <= DIM <= 1536');
if (existsSync(receiptArg)) throw Error('Receipt exists');
const hash = data => createHash('sha256').update(data).digest('hex');
const repo = resolve(repoArg), tsPath = resolve(tsArg);
const before = sourceFingerprint(repo);
const sourcePath = 'v3/@claude-flow/memory/src/hnsw-index.ts';
const source = readFileSync(join(repo, sourcePath), 'utf8');
const require = createRequire(import.meta.url);
const ts = require(tsPath);
const js = ts.transpileModule(source, { compilerOptions: {
  target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
}, reportDiagnostics: true });
if (js.diagnostics?.some(d => d.category === ts.DiagnosticCategory.Error)) throw Error('Transpile error');
const mod = { exports: {} };
new Function('require', 'exports', 'module', js.outputText)(name => {
  if (name !== 'node:events') throw Error(`Unexpected runtime dependency: ${name}`);
  return require(name);
}, mod.exports, mod);
const { HNSWIndex } = mod.exports;

function rng(seed) {
  let x = seed >>> 0;
  return () => { x ^= x << 13; x ^= x >>> 17; x ^= x << 5; return (x >>> 0) / 4294967296; };
}
const random = rng(424242);
function vector() {
  const v = Float32Array.from({ length: dim }, () => random() * 2 - 1);
  const norm = Math.sqrt(v.reduce((s, x) => s + x * x, 0));
  for (let i = 0; i < dim; i++) v[i] /= norm;
  return v;
}
const vectors = Array.from({ length: n }, vector);
const queries = Array.from({ length: queryCount }, vector);
const digest = arrays => {
  const h = createHash('sha256');
  for (const v of arrays) h.update(new Uint8Array(v.buffer, v.byteOffset, v.byteLength));
  return h.digest('hex');
};
const norms = vectors.map(v => Math.sqrt(v.reduce((s, x) => s + x * x, 0)));
function exact(q, filter = () => true) {
  const norm = Math.sqrt(q.reduce((s, x) => s + x * x, 0));
  const best = [];
  for (let i = 0; i < n; i++) {
    if (!filter(i)) continue;
    let dot = 0;
    for (let j = 0; j < dim; j++) dot += q[j] * vectors[i][j];
    const distance = 1 - dot / (norm * norms[i]);
    if (best.length === k && distance >= best[k - 1].distance) continue;
    let pos = best.findIndex(r => distance < r.distance);
    if (pos < 0) pos = best.length;
    best.splice(pos, 0, { id: String(i), distance });
    if (best.length > k) best.pop();
  }
  return best;
}
const stats = times => {
  const sorted = [...times].sort((a, b) => a - b);
  const pct = p => sorted[Math.ceil(p * sorted.length) - 1];
  return { p50_ms: pct(.5), p95_ms: pct(.95), p99_ms: pct(.99), mean_ms: times.reduce((a,b)=>a+b,0)/times.length };
};
// Warm the exact oracle before measuring; corpus norm preparation is excluded.
for (const q of queries.slice(0, 10)) exact(q);
const truth = [], exactTimes = [];
for (const q of queries) { const t = performance.now(); truth.push(exact(q)); exactTimes.push(performance.now() - t); }
// Independently sort a full cosine scan to check the bounded top-k oracle.
for (let qi = 0; qi < 3; qi++) {
  const q = queries[qi];
  const ordered = vectors.map((v, id) => {
    let dot = 0, qnorm = 0, vnorm = 0;
    for (let j = 0; j < dim; j++) { dot += q[j] * v[j]; qnorm += q[j] ** 2; vnorm += v[j] ** 2; }
    return { id: String(id), distance: 1 - dot / Math.sqrt(qnorm * vnorm) };
  }).sort((a,b)=>a.distance-b.distance).slice(0,k);
  if (ordered.some((r,i)=>r.id!==truth[qi][i].id)) throw Error('Exact oracle disagrees with full-sort reference');
}
global.gc?.();
const rssBefore = process.memoryUsage().rss;
const index = new HNSWIndex({ dimensions: dim, M: 16, efConstruction: 200, maxElements: n + 1, metric: 'cosine' });
const savedRandom = Math.random;
Math.random = rng(1337);
const start = performance.now();
try {
  for (let i = 0; i < n; i++) {
    await index.addPoint(String(i), vectors[i]);
    if ((i + 1) % 1000 === 0) console.error(`indexed ${i + 1}/${n}`);
  }
} finally { Math.random = savedRandom; }
const buildMs = performance.now() - start;
global.gc?.();
const rssAfter = process.memoryUsage().rss;
const curves = [];
for (const ef of [10, 50, 100, 200, 400, 800, 1600].filter(ef => ef <= n)) {
  for (const q of queries.slice(0, 10)) await index.search(q, k, ef);
  const times = [], recalls = [];
  for (let i = 0; i < queries.length; i++) {
    const t = performance.now(); const result = await index.search(queries[i], k, ef); times.push(performance.now() - t);
    const wanted = new Set(truth[i].map(r => r.id));
    if (new Set(result.map(r=>r.id)).size !== result.length || result.some(r=>!Number.isFinite(r.distance))) throw Error('Invalid result');
    recalls.push(result.filter(r => wanted.has(r.id)).length / k);
  }
  curves.push({ ef, recall_at_10: recalls.reduce((a,b)=>a+b,0)/recalls.length, ...stats(times) });
}
// Post-filtering is a distinct workload: do not report its latency as unfiltered recall.
const filteredTruth = queries.map(q => exact(q, i => i % 100 === 0));
const filtered = [];
for (let i = 0; i < queries.length; i++) {
  const result = await index.searchWithFilters(queries[i], k, id => Number(id) % 100 === 0, 200);
  const wanted = new Set(filteredTruth[i].map(r => r.id));
  filtered.push({ returned: result.length, recall: result.filter(r=>wanted.has(r.id)).length / wanted.size });
}
const after = sourceFingerprint(repo);
const receipt = {
  schema_version: 1, kind: 'ruflo_handwritten_hnsw_benchmark', captured_at: new Date().toISOString(),
  backend: 'HNSWIndex TypeScript executed as transpiled CommonJS; no WASM/native/CLI bridge',
  revision: before.revision, source_fingerprint: before.sha256, source_unchanged: before.sha256 === after.sha256,
  source_path: sourcePath, source_sha256: hash(source), transpiled_sha256: hash(js.outputText),
  typescript: ts.version, compiler_sha256: hash(readFileSync(tsPath)), runner_sha256: hash(readFileSync(new URL(import.meta.url))),
  node: process.version, platform: process.platform, arch: process.arch, cpu: cpus()[0]?.model,
  command: [process.execPath, ...process.execArgv, ...process.argv.slice(1)],
  dataset: { n, dim, k, queries: queryCount, seed: 424242, graph_seed: 1337, distribution: 'normalized uniform random Float32, independent queries', vectors_sha256: digest(vectors), queries_sha256: digest(queries) },
  config: { M: 16, efConstruction: 200, metric: 'cosine', quantization: false },
  oracle_checked_against_full_sort_queries: 3,
  build_ms: buildMs, exact_js_precomputed_norms: stats(exactTimes), curves,
  one_percent_postfilter: { mean_returned: filtered.reduce((s,r)=>s+r.returned,0)/queryCount, recall_at_10: filtered.reduce((s,r)=>s+r.recall,0)/queryCount },
  memory: { gc_available: !!global.gc, rss_before_index: rssBefore, rss_after_index: rssAfter, rss_delta: rssAfter-rssBefore,
    max_rss_kib: process.resourceUsage().maxRSS, upstream_estimate_not_rss: index.getStats().memoryUsage },
  limitations: ['single build seed and one 50-query pass per ef; latency tails are exploratory', 'synthetic geometry, no semantic embeddings', 'RSS includes corpus, oracle and compiler; delta is not isolated index memory', 'exact baseline is scalar JS, not optimized SQLite/Zvec native', 'persistence, updates, concurrency and crash recovery not measured'],
};
writeFileSync(receiptArg, JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ receipt: receiptArg, build_ms: buildMs, exact: receipt.exact_js_precomputed_norms, curves, filtered: receipt.one_percent_postfilter }));
if (!receipt.source_unchanged) process.exitCode = 1;
