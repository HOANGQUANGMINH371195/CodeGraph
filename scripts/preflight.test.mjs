import test from 'node:test';
import assert from 'node:assert/strict';
import { probe, preflight } from './preflight.mjs';

test('missing executable is a failed probe, not an exception', () => {
  const result = probe('missing', [], () => ({error: {code: 'ENOENT'}, status: null}));
  assert.equal(result.ok, false);
  assert.equal(result.error, 'ENOENT');
});

test('timeout and nonzero exit cannot report readiness', () => {
  assert.equal(probe('slow', [], () => ({error: {code: 'ETIMEDOUT'}, status: null})).ok, false);
  assert.equal(preflight(() => ({status: 1, stderr: 'failed'})).ready, false);
});

test('probes use bounded subprocesses without a shell', () => {
  let calls = 0;
  const result = preflight((command, args, options) => {
    calls++;
    assert.equal(options.shell, false);
    assert.equal(options.timeout, 10000);
    assert.ok(Array.isArray(args));
    return {status: 0, stdout: 'ok\n'};
  });
  assert.equal(calls, 8);
  assert.equal(result.probes.length, 8);
  assert.equal(result.probes.every(p => p.ok), true);
});
