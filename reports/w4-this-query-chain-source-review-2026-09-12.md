# Local this-query chain — source gate and implementation evidence

## Current implementation result

Added src/resolution/string-call-chain.ts with exact invocation/expression
extents, conditional same-file parameter substitution and per-hop provenance.
Reuse existing lexical/receiver collectors; opt-in calleeExtent distinguishes
inner this.query from outer .run() at the same start. Default collection output
remains unchanged. Chain length1..8, source256KiB, expression2048 steps/depth64,
expanded strings4096 UTF-16 units. No JS execution, target file reads, graph
writes or runtime authority. Receiver coverage remains class-local-only with
explicit runtime assumptions, not whole-program mutation/subclass analysis.

Known local method/prototype writes, receiver escapes, duplicate/getter/static
targets, inheritance/decorators, computed members and foreign ordinary-function
this are rejected. Intermediate calls must be inside the previous exact target;
that syntactic containment does not prove invocation/reachability. The original
single-hop substitution API is retained unchanged.

Final focused run with CODEGRAPH_KERNEL_EXPECT=1: **201 passed/8 files**, exit0
(session5701). tsc emit exit0 (58232), git diff --check exit0. Parent reviewed
Luna Hypatia's six test assertions and source receipt, then reran them with
the parent-owned fifteen boundary cases and six related suites. Worker receipt
records test-reference hashes; parent source review above is the integration
basis. Worker completed and close requested; no further edits delegated.

Actual Orders emitted-build probe: all11 candidate query-path strings match
authored expectations, source hash matches ground truth, expression/hop extents
retained. Latest receipt:
.harness/baselines/orders-string-call-chain-20260912-02.json.
It includes source/build hashes and no runtime/file-binding claim. Earlier01
receipt predates the added output expression extent; preserved as history.

SHA-256:
- string-call-chain.ts: 64056afbfed8fe612b321d0847ba7cff7e4d6cc40654842e96cbe98f3a79f495
- construction-sites.ts: cb82b3a7115dc55b2e58be5153a74f772306d98bbe5ecf6af2276cf5411e69c9
- worker tests: f4c952c08372f86e878e52f1f1368fd03f0994df178dd1bcc9f6bc436e983c16
- boundary tests: f218dc650ca66f88458c1519d23cc8137d844b84867c3593823841b3924d356b

Full regression for this newer chain snapshot has not run. The preceding
full-suite4 timing failures remain recorded, not converted into green by the
focused pass. Next: fs/URL identity, containment and target hash evidence,
then durable graph/context/diagram integration. W4/W9 remain incomplete.

## Historical source study and process observations

Parent reread receiver-hazards.ts member-effect collection, common class
hazards, class/function/arrow boundaries and complete member-this-effects tests.
Also read receiver-decision.ts root validation and explicit runtime assumptions.
Reference revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf.
Source hashes:
- receiver-hazards.ts: 5f88d36e9d2ffd5c693901a8db149d4f6db5c8e05091289bf0b280239075a059
- member-this-effects.test.ts: 7f7a52faa3e196b08ddec87eaf34b4285c9b68a0ce067eed31dde48cf041ddfd

Read-only Orders probe confirms class extent182..1064, query method290..340,
seven direct instance-this query-call effects, no local member hazards. Existing
collector distinguishes lexical arrows, ordinary nested functions, nested
classes, computed keys, method-call chains and property writes. Reuse it rather
than a textual this.query match. Evidence is class-local only: unseen subclass,
external mutations/reflection and arbitrary runtime receivers remain assumptions.

Chain implementation must use full invocation extents (start AND end): nested
calls in this.query('enqueue').run() share a starting offset. Reject ambiguous
locators, foreign expression/parameter ownership, methods/getters/fields with
duplicate names, unknown computed members, inheritance/decorators, relevant
this/prototype mutation and receiver escape. Require an exact instance-this
effect for the call, a unique callable non-static method in that same class,
and exact corresponding function evidence before parameter substitution.

Then validate each inner call is within the previous target's extent and
propagate literal inputs through its exact parameter identities. Keep bounded
chain depth/expansion and complete per-hop provenance. Never claim actual
invocation, fs/URL identity, file containment or database access from this chain.
Tests need chained .run(), shadowed/nested this, duplicates/getters/static,
mutations, call-location ambiguity, broken chains and same-name foreign classes.

No code for this integration written yet. Full preceding snapshot remains live
in95117; source is held unchanged until that receipt finishes.

Additional read-only probe confirmed duplicate invocation starts for Orders
enqueue: outer .run() extent542..600 (two arguments), inner this.query()
extent542..563 (one argument). Both are correctly retained by the extractor;
this is not a parser bug. The new chain API must accept full extents rather
than relaxing the existing start-only helper's ambiguity refusal.

At handoff, suite95117 was polled again and remained running; ps independently
showed parent vitest1123211 and six active workers. All probe sessions are
terminal (16157 and91722 exit0). No subagents active and no implementation
edits were made to the frozen CodeGraph snapshot in this investigation.

Update:95117 subsequently completed exit1,5393 pass/4 fail/9 skip, unchanged
source fingerprint. Freeze is released. See substitution receipt for the three
5000ms timeouts and highlight443ms/<400ms failure; focused rerun is separate
from whole-suite acceptance. The next implementation gate is ready, not coded.
## Implementation resumption source gate

Parent reread current string-substitution.ts completely and receiver-hazards
member/effect/common-hazard branches, plus substitution tests before coding.
Build a separate bounded exact-extent chain helper, retaining the single-hop
API unchanged. Reuse lexical and member-this evidence; no guessed global method
edges. Additional guards inspect observed constructor/prototype effects.
Worker owns only its dedicated test/receipt files; parent owns production and
independent boundary tests. No full-suite process remains live at this point.
