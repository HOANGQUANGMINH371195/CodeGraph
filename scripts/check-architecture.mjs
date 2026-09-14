import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

// Intentional direct dependencies, not an inferred "whatever exists" baseline.
// A new adapter or library requires an explicit architecture review here.
const policy = {
  'graph-domain': { normal: ['thiserror'] },
  'graph-application': { normal: ['graph-domain', 'thiserror', 'sha2'] },
  'graph-protocol': { normal: ['graph-domain', 'serde', 'serde_json', 'thiserror'] },
  'graph-store': { normal: ['graph-application', 'graph-domain', 'graph-protocol', 'refinery', 'rusqlite', 'serde_json', 'thiserror'], dev: ['tempfile'] },
  'graph-source': { normal: ['graph-domain', 'graph-application', 'cap-std', 'sha2', 'thiserror', 'rustix'], dev: ['tempfile'] },
  'graph-system': { normal: ['graph-domain', 'graph-application', 'sha2', 'thiserror', 'yaml-rust2', 'sqlparser'] },
  'graph-execution': { normal: ['graph-domain', 'graph-application', 'graph-protocol', 'serde_json', 'sha2', 'thiserror', 'rustix'], dev: ['tempfile', 'graph-store', 'graph-source', 'rustix'] },
  'project-graph-agent': { normal: ['graph-domain', 'graph-application', 'graph-store', 'graph-protocol', 'graph-source', 'graph-system', 'clap', 'serde_json'], dev: ['tempfile'] },
};

export function checkArchitecture(metadata) {
  if (metadata?.version !== 1 || !Array.isArray(metadata.packages) || !Array.isArray(metadata.workspace_members)
      || metadata.workspace_members.length === 0) throw new Error('Invalid or empty Cargo metadata');
  const errors = [];
  const packages = new Map(metadata.packages.map(p => [p.id, p]));
  const members = metadata.workspace_members.map(id => {
    const p = packages.get(id);
    if (!p || !Array.isArray(p.dependencies)) throw new Error(`Missing workspace package: ${id}`);
    return p;
  });
  const byName = new Map();
  for (const member of members) {
    if (byName.has(member.name)) errors.push(`Duplicate workspace package name: ${member.name}`);
    byName.set(member.name, member);
    if (!Object.hasOwn(policy, member.name)) errors.push(`Unclassified workspace package: ${member.name}`);
  }
  for (const name of Object.keys(policy)) if (!byName.has(name)) errors.push(`Required workspace package missing: ${name}`);
  let checked = 0;
  for (const member of members) {
    for (const dep of member.dependencies) {
      checked++;
      const kind = dep.kind ?? 'normal';
      const label = `${member.name} -> ${dep.name} (${kind}${dep.rename ? `, alias=${dep.rename}` : ''}${dep.target ? `, target=${dep.target}` : ''}${dep.optional ? ', optional' : ''})`;
      if (!Object.hasOwn(policy, member.name) || !policy[member.name][kind]?.includes(dep.name)) errors.push(`Forbidden dependency: ${label}`);
      if (Object.hasOwn(policy, dep.name)) {
        const target = byName.get(dep.name);
        if (!target || !dep.path || dep.source != null || resolve(dep.path, 'Cargo.toml') !== resolve(target.manifest_path)) {
          errors.push(`Internal dependency is not the workspace package: ${label}`);
        }
      } else if (dep.path || !dep.source?.startsWith('registry+')) {
        errors.push(`Unreviewed dependency origin: ${label}`);
      }
    }
  }
  return { schema_version: 1, ok: errors.length === 0, packages: members.length, declared_dependencies: checked, errors };
}

export function inspectWorkspace(root) {
  const result = spawnSync('cargo', ['metadata', '--format-version', '1', '--no-deps', '--locked', '--offline'], {
    cwd: root, encoding: 'utf8', timeout: 60000, maxBuffer: 16 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) throw new Error(`Cargo metadata failed: ${result.error?.message ?? result.stderr.trim()}`);
  return { ...checkArchitecture(JSON.parse(result.stdout)),
    metadata_sha256: createHash('sha256').update(result.stdout).digest('hex'),
    scope: 'Declared direct dependencies, including optional, build, dev and target-specific declarations; not source-level or transitive runtime isolation.' };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const report = inspectWorkspace(resolve(process.argv[2] ?? '.'));
    process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
    process.exitCode = report.ok ? 0 : 1;
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
