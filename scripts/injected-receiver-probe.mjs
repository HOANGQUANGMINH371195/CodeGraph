import { mkdtempSync, writeFileSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { sourceFingerprint } from './source-fingerprint.mjs';

const [repoArg, outputArg] = process.argv.slice(2);
if (!repoArg || !outputArg) throw Error('usage: node scripts/injected-receiver-probe.mjs CODEGRAPH NEW_RECEIPT');
const repo = resolve(repoArg), output = resolve(outputArg);
if (existsSync(output)) throw Error('receipt exists');
process.env.CODEGRAPH_TELEMETRY = '0';
process.env.CODEGRAPH_NO_UPDATE_CHECK = '1';
const before = sourceFingerprint(repo);
const mod = await import(pathToFileURL(join(repo, 'dist/index.js')).href);
const CodeGraph = mod.default?.default ?? mod.default ?? mod.CodeGraph;
const cases = [
  ['direct', 'constructor() { this.store = new Store(); }', 'new Service().create();'],
  ['injected', 'constructor(store) { this.store = store; }', 'new Service(new Store()).create();'],
  ['unknown', 'constructor(store) { this.store = store; }', 'export function use(store) { return new Service(store).create(); }'],
];
const results = [];
for (const [name, constructor, invocation] of cases) {
  const dir = mkdtempSync(join(tmpdir(), 'injected-receiver-'));
  const source = `export class Store {\n  createOrder() { return 1; }\n}\n`
    + `export class Unrelated {\n  createOrder() { return 2; }\n}\n`
    + `export class Service {\n  ${constructor}\n  create() { return this.store.createOrder(); }\n}\n${invocation}\n`;
  let graph;
  try {
    writeFileSync(join(dir, 'app.mjs'), source);
    graph = CodeGraph.initSync(dir);
    const index = await graph.indexAll();
    const caller = graph.getNodesInFile('app.mjs').find(n => n.qualifiedName === 'Service::create');
    if (!caller) throw Error('caller not extracted');
    results.push({name, source, index, caller,
      callees: graph.getCallees(caller.id).map(x => ({target:x.node.qualifiedName, edge:x.edge}))});
  } finally {
    graph?.close(); rmSync(dir, {recursive:true});
  }
}
const receipt = {schema_version:1, source_revision:before.revision, source_sha256:before.sha256,
  source_unchanged:sourceFingerprint(repo).sha256 === before.sha256,
  dist_entry_sha256:createHash('sha256').update(readFileSync(join(repo, 'dist/index.js'))).digest('hex'),
  build_source_correspondence_verified:false, command:process.argv, results,
  scope:'Mechanism contrast only; no constructor-flow feature implemented', product_acceptance:false};
writeFileSync(output, JSON.stringify(receipt,null,2)+'\n', {flag:'wx'});
console.log(JSON.stringify({receipt:output, source_unchanged:receipt.source_unchanged,
  results:results.map(r=>({name:r.name,callees:r.callees.map(c=>c.target)}))}));
if (!receipt.source_unchanged) process.exitCode = 1;
