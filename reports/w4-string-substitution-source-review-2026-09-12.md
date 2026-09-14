# Source-bound local string substitution — before-code gate

Reread outsource parameter-call-mapping.ts exact function extents, parameter
matching and source snapshot checks; its tests for unrelated same-named classes,
repeated invocation provenance and missing exact nodes. Reuse these principles
with current construction-sites lexical function/parameter/string evidence.
Reference revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf; no external code
execution, name-only dispatch, SQL execution or repository file publication.

Implement a source-bound query accepting source/language and exact invocation
and argument-expression offsets. Re-extract opt-in evidence from those bytes;
require unique local callee/function identity and an expression within its
extent. Substitute only exact stable parameter identities using uniquely
positioned literal arguments; missing/spread/foreign/symbolic inputs unresolved.
Bound source bytes, recursion/steps and final expanded characters. Return a
source hash and substitution-site provenance, candidate-only/runtime false.
Do not treat the resulting string as a contained path or existing SQL file.

Test actual template→local call composition, const alias callee, foreign/same-name
function rejection, spread/missing/unknown/symbolic arguments, mutation and
limits. Imported functions and this.query method dispatch remain unresolved
until cross-file/receiver binding is integrated, not guessed in this helper.

## Implementation and verification

Added string-substitution.ts: a source-bound internal helper, not a new CLI/MCP
tool or graph writer. It computes a SHA-256 from supplied bytes and re-extracts
one file's evidence; positions are UTF-16 units. Candidate value means conditional
expression substitution only, not proof the expression executes. Source cap
256KiB, expanded text cap4096 code units, evaluation256 steps/depth64. The input
string is already resident and hashed; these limits do not bound caller I/O.

12 new tests; combined string/construction/function-parameter/mapping suites:
161 pass across5 files, terminal exit0 (68006). tsc emit exit0 (63536),
git diff --check exit0. Hashes:
- string-substitution.ts: 07ffa1dc06ad866d23988f0525246dd9b86b1db31b4bdf43d21f7cce2968113d
- string-substitution.test.ts: 598b60f3af633b39cc8ab1b400f2e0a8758503cdb3714f66b9b4cf19276a5f6b

Actual Orders source/build probe exit0: four candidate values
../sql/schema.sql, ../sql/begin.sql, ../sql/commit.sql, ../sql/rollback.sql;
sql(name) remains unresolved (argument-not-literal). Receipt:
.harness/baselines/orders-local-string-substitution-20260912-01.json.
No filesystem target was opened by substitution, fileBindingVerified remains
false. This does not yet resolve this.query methods or all11 query files.

Full native-required CodeGraph suite launched for this source snapshot, session
95117; expected receipt
.harness/baselines/codegraph-string-substitution-20260912-01.json.
Process completed exit1: 5393 passed,4 failed,9 skipped across308 files,
374.47 seconds; before/after source fingerprints match. Failures: two tests
in sync-rebuild-convergence and one in typed-this-field-lifecycle exceeded
5000ms; ui-highlight cost assertion observed443ms against unchanged <400ms.
No evidence yet establishes these failures as harmless. Earlier
string-arguments full receipt had one timing failure and is preserved.

Focused rerun of all three failing files with CODEGRAPH_KERNEL_EXPECT=1:
43 passed/3 files, terminal exit0 (46076). No threshold/test/source edits.
The formerly timed-out tests completed in1369ms,760ms and1800ms; highlight
suite passed its existing cost assertion. This supports load sensitivity as
a hypothesis, not a full-suite green claim. No tests remain running here.
