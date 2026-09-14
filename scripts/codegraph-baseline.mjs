// Reuse CodeGraph's own deterministic probe and preserve its output with
// source identifiers. This is a W0 diagnostic, not an agent/token benchmark.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { sourceFingerprint } from './source-fingerprint.mjs';

const [repoArg, receiptArg] = process.argv.slice(2);
if (!repoArg || !receiptArg) {
  throw new Error('usage: node scripts/codegraph-baseline.mjs CODEGRAPH_REPO NEW_RECEIPT.json');
}
const repo = resolve(repoArg);
if (existsSync(resolve(receiptArg))) throw new Error('Receipt already exists; choose a new filename');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const files = {};
function fingerprint(relative) {
  for (const entry of readdirSync(join(repo, relative), {withFileTypes: true}).sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
    const path = `${relative}/${entry.name}`;
    if (entry.isDirectory()) fingerprint(path);
    else if (entry.isFile()) files[path] = hash(readFileSync(join(repo, path)));
    else throw new Error(`unexpected fixture entry: ${path}`);
  }
}
fingerprint('__tests__/fixtures/factory-closure-ts');
fingerprint('dist');
for (const path of ['package.json', 'package-lock.json', 'codegraph-kernel/Cargo.lock',
  'scripts/agent-eval/probe-factory-closure.mjs', 'dist/index.js', 'dist/mcp/tools.js']) files[path] = hash(readFileSync(join(repo, path)));
const kernelPath = `codegraph-kernel/prebuilds/${process.platform}-${process.arch}/codegraph-kernel.node`;
const kernelPresent = existsSync(join(repo, kernelPath));
if (kernelPresent) files[kernelPath] = hash(readFileSync(join(repo, kernelPath)));
const sourceBefore = sourceFingerprint(repo);
const revision = sourceBefore.revision;
const command = [process.execPath, 'scripts/agent-eval/probe-factory-closure.mjs', '--json'];
const startedAt = new Date().toISOString();
const start = performance.now();
const result = spawnSync(command[0], command.slice(1), {
  cwd: repo, encoding: 'utf8', timeout: 120000, maxBuffer: 8 * 1024 * 1024,
  env: {...process.env, CODEGRAPH_TELEMETRY: '0', CODEGRAPH_KERNEL: '1',
    CODEGRAPH_KERNEL_PATH: kernelPresent ? join(repo, kernelPath) : undefined,
    CODEGRAPH_KERNEL_DEBUG: '1'}, shell: false,
});
const elapsedMs = performance.now() - start;
const sourceAfter = sourceFingerprint(repo);
const stable = sourceBefore.sha256 === sourceAfter.sha256
  && Object.entries(files).every(([path, digest]) => hash(readFileSync(join(repo, path))) === digest);
let metrics = null;
let parseError = null;
try { metrics = JSON.parse(result.stdout); } catch (error) { parseError = error.message; }
const receipt = {
  schema_version: 2, kind: 'codegraph_factory_closure_baseline',
  repository: repo, revision, source_before: sourceBefore,
  source_after_sha256: sourceAfter.sha256, source_unchanged: stable,
  build_source_correspondence_verified: false,
  input_sha256: files, node: process.version, platform: process.platform,
  kernel_present: kernelPresent,
  kernel_loaded: (result.stderr ?? '').includes(`[codegraph-kernel] loaded ${join(repo, kernelPath)}`),
  command, started_at: startedAt, elapsed_ms: elapsedMs,
  exit_code: result.status, signal: result.signal,
  error: result.error?.message ?? parseError,
  metrics, stdout: result.stdout, stderr: result.stderr,
  scope: 'Upstream deterministic index/explore probe; no live agent, account, cost, token or whole-product acceptance.',
};
writeFileSync(resolve(receiptArg), `${JSON.stringify(receipt, null, 2)}\n`, {flag: 'wx'});
console.log(JSON.stringify({receipt: resolve(receiptArg), exit_code: result.status, source_unchanged: stable,
  inner_delivered: metrics?.innerDelivered, inner_total: metrics?.innerTotal,
  over_budget: metrics?.envelope?.overBudget ?? null,
  soft_budget_chars: metrics?.budget?.maxOutputChars ?? null,
  hard_ceiling_chars: metrics?.budget?.hardCeiling ?? null,
  exceeds_hard_ceiling: metrics?.budget?.hardCeiling == null ? null
    : metrics.envelope.chars > metrics.budget.hardCeiling,
  product_acceptance: false}));
if (result.error || result.status !== 0 || parseError || !stable) process.exitCode = 1;
