import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync, symlinkSync, chmodSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { sourceFingerprint } from './source-fingerprint.mjs';

test('fingerprints dirty tracked bytes, untracked files, deletion and link identity', () => {
  const root = mkdtempSync(join(tmpdir(), 'harness-fingerprint-'));
  const git = (...args) => {
    const result = spawnSync('git', ['-C', root, ...args], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  };
  try {
    git('init');
    writeFileSync(join(root, 'source.txt'), 'original');
    git('add', 'source.txt');
    git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
      '-c', 'commit.gpgsign=false', 'commit', '-m', 'fixture');
    const initial = sourceFingerprint(root);
    assert.equal(initial.clean, true);
    assert.deepEqual(sourceFingerprint(root), initial);
    writeFileSync(join(root, 'source.txt'), 'patch one');
    const one = sourceFingerprint(root);
    writeFileSync(join(root, 'source.txt'), 'patch two');
    const two = sourceFingerprint(root);
    assert.equal(one.status, two.status); // porcelain alone misses this change
    assert.notEqual(one.sha256, two.sha256);
    writeFileSync(join(root, 'new.txt'), 'new implementation');
    const added = sourceFingerprint(root);
    assert.notEqual(added.sha256, two.sha256);
    assert.ok(added.files['new.txt']);
    writeFileSync(join(root, '__proto__'), 'ordinary source filename');
    assert.equal(sourceFingerprint(root).files.__proto__.kind, 'file');
    if (process.platform !== 'win32') {
      chmodSync(join(root, 'new.txt'), 0o755);
      assert.notEqual(sourceFingerprint(root).sha256, added.sha256);
      symlinkSync('missing-target', join(root, 'link'));
      assert.equal(sourceFingerprint(root).files.link.kind, 'symlink');
    }
    rmSync(join(root, 'source.txt'));
    assert.equal(sourceFingerprint(root).files['source.txt'].kind, 'deleted');
  } finally { rmSync(root, { recursive: true, force: true }); }
});
