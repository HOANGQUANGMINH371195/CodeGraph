# Final explore ceiling — 2026-09-11

The factory-closure probe now includes `budget` as well as `envelope` so a
soft target cannot be confused with pre-finalization length or the hard cap.
[Probe receipt](../.harness/baselines/factory-closure-hard-ceiling-20260911.json):
15,939 delivered, 15,936 before header finalization, 13,000 soft target,
19,500 hard ceiling. The previous “three characters over budget” interpretation
was incorrect; the older reports have been corrected without changing raw data.

Code inspection nevertheless found two hard-ceiling gaps: the omission note
was appended after cutting to the ceiling, and the survivor header was expanded
after the size check. The new final limiter includes both, recomputes the
surviving-file header, cuts at section/line boundaries and closes code fences.
It labels omitted context instead of claiming partial source is complete.
The limits follow upstream JavaScript string length, not bytes or tokens.

`npm run build` succeeded, followed by:

```sh
npm test -- __tests__/explore- --maxWorkers=2 --minWorkers=1 --reporter=json --outputFile=/home/minh/projects/project-graph-agent/.harness/baselines/explore-hard-ceiling-20260911.json
```

All **290 tests in 24 files passed**. Source fingerprints matched before/after
this run. [Results](../.harness/baselines/explore-hard-ceiling-20260911.json).
The deterministic factory probe also passed with its native kernel loaded.

A subsequent safety guard excludes partially cut file sections from the session
checkpoint, so the next call cannot withhold source ranges never delivered.
After that addition, the output-limit, session-state and diagnostics suites
passed **49 tests across 3 files**. The earlier build/probe and 290-test run
precede that guard. Follow-up added checks over 189 tight ceilings with Unicode
source lines, multiple source blocks and dropped-file checkpoint detection;
the output-limit and session-state suites passed 39 tests. A fresh build after
the guard succeeded. A complete regression run on the final patch remains pending.

This fixes the upstream text finalizer, not the Rust harness's future typed
ContextEnvelope, tokenizer policy, snapshot authority or complete W4 gate.
