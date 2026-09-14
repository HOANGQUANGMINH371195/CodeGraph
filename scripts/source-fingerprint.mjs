// Identify the actual Git worktree, including unpublished and untracked source.
// This is an identity check, not proof that a build corresponds to that source.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { lstatSync, readFileSync, readlinkSync } from 'node:fs';
import { join } from 'node:path';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');

export function sourceFingerprint(repo) {
  const git = args => {
    const result = spawnSync('git', ['-C', repo, ...args], {
      encoding: 'utf8', timeout: 10000, maxBuffer: 32 * 1024 * 1024,
    });
    if (result.error || result.status !== 0) throw new Error('Git source fingerprint failed');
    return result.stdout;
  };
  const revision = git(['rev-parse', 'HEAD']).trim();
  const status = git(['status', '--porcelain=v1', '-z', '--untracked-files=all']);
  const paths = [...new Set(git(['ls-files', '--cached', '--others', '--exclude-standard', '-z'])
    .split('\0').filter(Boolean))].sort();
  const files = Object.create(null);
  for (const path of paths) {
    const absolute = join(repo, path);
    let stat;
    try { stat = lstatSync(absolute); }
    catch (error) {
      if (error.code !== 'ENOENT') throw error;
      files[path] = { kind: 'deleted' };
      continue;
    }
    if (stat.isSymbolicLink()) {
      files[path] = { kind: 'symlink', sha256: hash(readlinkSync(absolute)) };
    } else if (stat.isFile()) {
      files[path] = { kind: 'file', executable: !!(stat.mode & 0o111), sha256: hash(readFileSync(absolute)) };
    } else {
      // A submodule must have its own receipt; don't silently omit its state.
      throw new Error(`Unsupported source entry: ${path}`);
    }
  }
  return { revision, clean: status === '', status, files,
    sha256: hash(JSON.stringify({ revision, status, files })) };
}
