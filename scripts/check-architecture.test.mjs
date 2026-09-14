import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkArchitecture, inspectWorkspace } from './check-architecture.mjs';
import { fileURLToPath } from 'node:url';

const names = ['graph-domain', 'graph-application', 'graph-protocol', 'graph-store', 'graph-source', 'project-graph-agent', 'graph-execution', 'graph-system'];
const metadata = () => ({ version: 1, workspace_members: [...names],
  packages: names.map(name => ({ id: name, name, manifest_path: `/workspace/${name}/Cargo.toml`, dependencies: [] })) });
const external = (name, extra = {}) => ({ name, kind: null, source: 'registry+https://github.com/rust-lang/crates.io-index', ...extra });
const internal = (name, extra = {}) => ({ name, kind: null, source: null, path: `/workspace/${name}`, ...extra });

test('real workspace satisfies declared architecture', () => {
  const report = inspectWorkspace(fileURLToPath(new URL('..', import.meta.url)));
  assert.equal(report.ok, true, report.errors.join('\n'));
  assert.equal(report.packages, 8);
  assert.ok(report.declared_dependencies > 20);
});

test('JSON decoding is allowed in protocol but not domain or application', () => {
  for (const [index, allowed] of [[0, false], [1, false], [2, true]]) {
    const data = metadata();
    data.packages[index].dependencies.push(external('serde_json'));
    assert.equal(checkArchitecture(data).ok, allowed);
  }
});

test('YAML and SQL parsing stay in system adapter, which cannot acquire database or network dependencies', () => {
  for (const parser of ['yaml-rust2', 'sqlparser']) {
    for (const [index, allowed] of [[0, false], [1, false], [7, true]]) {
      const data = metadata();
      data.packages[index].dependencies.push(external(parser));
      assert.equal(checkArchitecture(data).ok, allowed);
    }
  }
  for (const dependency of [external('rusqlite'), external('reqwest'), internal('graph-store'), internal('graph-source')]) {
    const data = metadata();
    data.packages[7].dependencies.push(dependency);
    assert.equal(checkArchitecture(data).ok, false);
  }
});

test('source snapshot hashing stays in the source adapter boundary', () => {
  const source = metadata();
  source.packages[4].dependencies.push(external('sha2'));
  assert.equal(checkArchitecture(source).ok, true);
  const domain = metadata();
  domain.packages[0].dependencies.push(external('sha2'));
  assert.equal(checkArchitecture(domain).ok, false);
});

test('CLI composes system adapter without reversing protocol or domain dependencies', () => {
  for (const [index, allowed] of [[0, false], [1, false], [2, false], [5, true]]) {
    const data = metadata();
    data.packages[index].dependencies.push(internal('graph-system'));
    assert.equal(checkArchitecture(data).ok, allowed);
  }
});

test('CLI can name domain contracts but domain cannot acquire CLI dependencies', () => {
  const inward = metadata();
  inward.packages[5].dependencies.push(internal('graph-domain'));
  assert.equal(checkArchitecture(inward).ok, true);
  for (const dependency of [internal('project-graph-agent'), external('clap')]) {
    const outward = metadata();
    outward.packages[0].dependencies.push(dependency);
    assert.equal(checkArchitecture(outward).ok, false);
  }
});

test('rejects reverse, renamed, optional, target and build dependencies', () => {
  for (const extra of [{}, { rename: 'harmless' }, { optional: true }, { target: 'cfg(windows)' }, { kind: 'build' }, { kind: 'dev' }]) {
    const data = metadata();
    data.packages[0].dependencies.push(internal('graph-store', extra));
    assert.equal(checkArchitecture(data).ok, false, JSON.stringify(extra));
  }
});

test('recognizes dependency package name rather than alias', () => {
  const data = metadata();
  data.packages[1].dependencies.push(internal('graph-domain', { rename: 'domain' }));
  assert.equal(checkArchitecture(data).ok, true);
  data.packages[0].dependencies.push(external('rusqlite', { rename: 'thiserror' }));
  assert.equal(checkArchitecture(data).ok, false);
});

test('rejects impersonated internal packages and unreviewed external origins', () => {
  for (const dependency of [external('graph-domain'), internal('graph-domain', { path: '/outside/domain' })]) {
    const data = metadata(); data.packages[1].dependencies.push(dependency);
    assert.equal(checkArchitecture(data).ok, false);
  }
  const data = metadata(); data.packages[0].dependencies.push(external('thiserror', { source: 'git+https://example.invalid/repo' }));
  assert.equal(checkArchitecture(data).ok, false);
});

test('new, missing and malformed workspace members fail closed', () => {
  assert.throws(() => checkArchitecture({}), /Invalid/);
  const data = metadata(); data.workspace_members.push('missing');
  assert.throws(() => checkArchitecture(data), /Missing workspace/);
  data.packages.push({ id: 'missing', name: 'new-adapter', dependencies: [] });
  assert.equal(checkArchitecture(data).ok, false);
  const removed = metadata(); removed.workspace_members.pop();
  assert.equal(checkArchitecture(removed).ok, false);
});

test('dev allowances cannot leak into production or build dependencies', () => {
  const data = metadata(); data.packages[3].dependencies.push(external('tempfile', { kind: 'dev' }));
  assert.equal(checkArchitecture(data).ok, true);
  data.packages[3].dependencies[0].kind = null;
  assert.equal(checkArchitecture(data).ok, false);
  data.packages[3].dependencies[0] = external('rusqlite', { kind: 'build' });
  assert.equal(checkArchitecture(data).ok, false);
});

test('execution composition adapters remain dev-only', () => {
  for (const name of ['graph-store', 'graph-source']) {
    for (const kind of ['dev', null, 'build']) {
      const data = metadata();
      data.packages[6].dependencies.push(internal(name, { kind }));
      assert.equal(checkArchitecture(data).ok, kind === 'dev');
    }
  }
});
