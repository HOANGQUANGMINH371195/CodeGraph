// Snapshot local reference checkouts without fetching or changing their Git state.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const [rootArg, outputArg] = process.argv.slice(2);
if (!rootArg || !outputArg) throw new Error('usage: node scripts/lock-references.mjs REFERENCE_ROOT NEW_LOCK.json');
const root = resolve(rootArg);
const sha256 = data => createHash('sha256').update(data).digest('hex');
const repositories = [];
for (const name of readdirSync(root).sort()) {
  const path = join(root, name);
  if (!existsSync(join(path, '.git'))) continue;
  const git = args => {
    const result = spawnSync('git', ['-C', path, ...args], {encoding: 'utf8', timeout: 10000});
    if (result.error || result.status !== 0) throw new Error(`failed Git query for ${name}`);
    return result.stdout.trim();
  };
  const head = git(['rev-parse', 'HEAD']);
  const status = git(['status', '--porcelain']);
  const files = {};
  const rootFiles = readdirSync(path, {withFileTypes: true});
  const licenses = rootFiles.filter(entry => entry.isFile() && /^(LICENSE|LICENCE|COPYING|NOTICE)(\.|$)/i.test(entry.name))
    .map(entry => entry.name).sort();
  for (const file of [...licenses, 'Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json', 'pyproject.toml', 'uv.lock']) {
    if (existsSync(join(path, file))) files[file] = sha256(readFileSync(join(path, file)));
  }
  if (head !== git(['rev-parse', 'HEAD']) || status !== git(['status', '--porcelain'])) {
    throw new Error(`checkout changed during snapshot: ${name}`);
  }
  repositories.push({name, path, commit: head, branch: git(['rev-parse', '--abbrev-ref', 'HEAD']),
    clean: status === '', status_sha256: sha256(status), files_sha256: files,
    license_files: licenses, license_review: 'pending', compatibility: 'pending'});
}
if (!repositories.length) throw new Error('no reference Git repositories found');
const manifest = {schema_version: 1, captured_at: new Date().toISOString(), root,
  scope: 'Commit and root manifest/license fingerprints; nested/submodule licenses, binary/toolchain pins and compatibility require separate review.',
  repositories};
writeFileSync(resolve(outputArg), `${JSON.stringify(manifest, null, 2)}\n`, {flag: 'wx'});
console.log(JSON.stringify({output: resolve(outputArg), repositories: repositories.length,
  clean: repositories.filter(repo => repo.clean).length}));
