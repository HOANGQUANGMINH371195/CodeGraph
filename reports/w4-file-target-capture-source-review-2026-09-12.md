# File target capture — before-code source gate

Read outsource route-flow.ts capture/open/fstat/bounded-read/hash flow,
utils.ts lexical and realpath containment (including ENOENT fallback), and
security.test.ts lexical/symlink escape assertions. Revision
3ed73bc127323e63153bf6ec8354afa82ce36aaf. Source hashes:
- route-flow.ts: c9f96e322f8a8fd60e01c7c7f28da21719f28c4d25cbb2258c484f3a62b7ea69
- utils.ts: 634a86b3dc0241a00460d062934c3854e7f1839b77202f1ddd7f6c56a88265ce

Adopt regular-descriptor capture, nonblocking open, cap+1 reads and fatal UTF-8
decoding; reuse strict validatePathWithinRoot with no escape waiver. Unlike
generic indexing, reject symlink components for both code and target here.
Check descriptor identity and size/timestamps against path metadata after read;
this detects ordinary replacement/drift, not adversarial race-free containment.
Root stability and non-atomic source/target capture must remain explicit.

API reads source from a caller-selected root/path, compares required expected
source SHA, computes file-read binding from those bytes, resolves conditional
URL against the source file URL, rejects non-file/query/fragment/outside paths,
captures the target (128KiB max), then recaptures source for drift rejection.
Return hashes/byte and line counts without file bodies. Source256KiB max;
exact user limits accepted within caps. No SQL execution, graph mutation or
claimed atomic snapshot. Native filesystem races/platform release gates open.

Tests use owned temporary roots only: successful cited capture, changed source,
target hash update, exact/over caps, malformed UTF-8, symlinks/outside URLs,
nonregular/missing target and no stdout/source-body leakage. Target bytes are
fresh observations, not a persisted domain SourceEvidence yet.

## Implementation and focused verification

Added file-read-target.ts. Source is captured/fatal-UTF8 decoded with BOM
preserved, checked against required SHA, then re-derived binding resolves the
conditional URL. Only file URLs without query/fragment are accepted. Strict
containment plus no-symlink component checks precede regular-descriptor bounded
reads. Descriptor/path identity, size/mtime/ctime are checked around reads;
source is recaptured after target. These are stable-filesystem checks, not
race-free confinement or an atomic multi-file snapshot; flags stay false.
Capture returns source/target hashes, byte lengths and LF line counts, no bodies.

Parent reviewed Luna Pascal's actual source-study receipt and six capture tests,
closed the completed worker, and reran them with nine parent boundary tests and
existing string/binding regressions: **233 passed/11 files**, native required,
terminal exit0 (92319). tsc emit exit0 (93918); git diff --check exit0.
Linux FIFO, symlink components, escape/remote/query/fragment URLs, invalid UTF8,
source drift, missing/directory targets and exact byte caps covered. Platform
gates and adversarial race testing remain open; byte caps are not I/O deadlines.

Actual Orders fixture: all11 targets captured, target hashes match authored
ground truth and source hash checked. Receipt:
.harness/baselines/orders-file-target-capture-20260912-01.json.
No repository SQL executed or graph ledger modified. Captured target metadata
does not upgrade conditional fs/URL observations into runtime execution proof.

SHA-256:
- file-read-target.ts: 20afc8cc3a0ea390033ca3bba248c4dc7c09a7a795dba87ab91c9546caffaa0f
- worker tests: 2b032f0a14dee605e2c08c6e77294cdcf2920f7c9f61033e27770ffc1906735e
- boundary tests: 2e5cf3623a6c768d0b9ec61d109e17d14bea96411d03d9bc7218ae26bbeab076

Full native-required suite launched with --maxWorkers4 --minWorkers1 (same
assertions/timeouts), expected receipt
.harness/baselines/codegraph-file-target-capture-20260912-01.json.
Previous full-suite timing failures are preserved, not waived. Durable graph
and SQL analysis integration/automatic discovery/context remain next work.
## W4 bounded-sidecar timing receipt (2026-09-12)

This report's prior live/running handoff is stale and is superseded by the
terminal receipts below. No source, test, threshold, plan, discovery file, or
baseline was edited by this review.

### Before-update source receipt

Requested model: `gpt-5.6-luna`. Effective model was not observed or inferred.
The parent owns discovery source, tests, plans, and reports; this bounded
sidecar made no recursive delegation and writes only this report.

Relevant current source and receipt fingerprints read before this report
update:

- `codegraph/__tests__/ui-server-api.test.ts` SHA-256:
  `5ceefb65da8e3bc045aa66a3d87134573e4aab6312136dede5778643e28c6369`
- `.harness/baselines/codegraph-file-target-capture-20260912-01.json`
  SHA-256:
  `8431905170be2f716917f0554f50d01319270327c6c916be8fad4477eef1e661`
- Baseline product revision:
  `3ed73bc127323e63153bf6ec8354afa82ce36aaf`
- Baseline embedded before/after fingerprint:
  `f6dbcb7a1ace0c1334fc278eca32d110440a1c1500b2c9b2645096edee8bc2f0`
  (unchanged: `true`).

The relevant assertion is `ui-server-api.test.ts:631–647`: warm the node
request, measure the second request with `performance.now()`, require 500+
callers and 300 returned rows, then require `elapsed < 100` ms. Adopt the
existing warm-cache measurement and its explicit response-shape counts as the
source-backed timing contract. Avoid changing the threshold, warming behavior,
or source/test code; avoid treating one isolated pass as proof that the full
suite failure is gone.

### Full receipt and isolated outcome

The product baseline receipt ran from `2026-09-12T17:39:11.609Z` through
`2026-09-12T17:45:10.585Z`, terminal exit **1**, signal `null`. It reports:

```text
❯ |engine| __tests__/ui-server-api.test.ts (62 tests | 1 failed | 1 skipped) 3707ms
  × GET /api/node/<id> — the busiest symbol > caps the caller list, keeps the true total, and stays fast 258ms
    → expected 127.5792960000008 to be less than 100
AssertionError: expected 127.5792960000008 to be less than 100
❯ __tests__/ui-server-api.test.ts:647:21
```

Its full terminal aggregate was **5449 passed, 1 failed, 9 skipped**
(`5459` tests; `312` test files passed and `1` failed).

The exact isolated command, from the codegraph cwd, was:

```text
/home/minh/projects/project-graph-agent/scripts/with-local-tools node node_modules/vitest/vitest.mjs run __tests__/ui-server-api.test.ts
```

It exited **0**: **1** test file passed; **61 passed, 1 skipped, 0 failed**
out of 62 tests. Vitest reported duration `11.35s` and test execution
`2.28s`. The isolated success does **not** erase the full-suite failure above;
the timing assertion remains a full-suite failure at the recorded baseline.

The literal relative path `product/scripts/with-local-tools` was also checked
from the codegraph cwd and failed before Vitest with exit **127** because that
path does not exist there; the resolved product script above is the command
that produced the isolated receipt. No threshold or source edit was made.
