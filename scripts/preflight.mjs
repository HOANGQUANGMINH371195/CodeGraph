import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { existsSync } from 'node:fs';

// Read-only probes: no installs, login, network calls, or environment dumps.
export function probe(command, args, run = spawnSync) {
  const result = run(command, args, {
    encoding: 'utf8', timeout: 10000, maxBuffer: 65536,
    windowsHide: true, shell: false,
  });
  return {
    command: [command, ...args],
    ok: !result.error && result.status === 0,
    exit_code: result.status ?? null,
    error: result.error?.code ?? null,
    stdout: (result.stdout ?? '').trim(),
    stderr: (result.stderr ?? '').trim(),
  };
}

export function preflight(run = spawnSync) {
  const localNpm = fileURLToPath(new URL('../.harness/corepack/npm/10.9.2/bin/npm-cli.js', import.meta.url));
  const probes = [
    probe(process.execPath, ['--version'], run),
    probe(process.execPath, ['-e', "const {DatabaseSync}=require('node:sqlite');const db=new DatabaseSync(':memory:');db.exec('CREATE VIRTUAL TABLE probe USING fts5(content)');db.close();"], run),
    existsSync(localNpm)
      ? probe(process.execPath, [localNpm, '--version'], run)
      : probe('npm', ['--version'], run),
    probe('cargo', ['--version'], run),
    probe('rustc', ['--version'], run),
    probe('rustfmt', ['--version'], run),
    probe('cargo', ['clippy', '--version'], run),
    probe('git', ['--version'], run),
  ];
  const nodeMajor = Number(process.versions.node.split('.')[0]);
  return {
    schema_version: 1,
    platform: process.platform,
    architecture: process.arch,
    node_supported: nodeMajor >= 22 && nodeMajor < 25,
    ready: nodeMajor >= 22 && nodeMajor < 25 && probes.every(p => p.ok),
    probes,
    scope: 'Tool availability only; not a build, MSRV, adapter, or product acceptance receipt.',
  };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const report = preflight();
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  process.exitCode = report.ready ? 0 : 1;
}
