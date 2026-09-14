# String parameter initialization order — before-code follow-up

Parent reread current outsource construction-sites.ts scope construction,
parameterOrigin position check and opt-in stringExpression parameter branch;
also read callback-parameter-defaults.test.ts ownership/negative assertions.
Revision remains 3ed73bc127323e63153bf6ec8354afa82ce36aaf; pre-edit source hash
0cc0c74322f893eefb56d9f5c22372d4cb055eedb409d6dac2f6c0526f0b5138.

Read-only probe against the emitted build reproduces a gap: both
function f(a=query(b),b){} and function f(b,a=query(b)){} produce a symbolic
parameter b. The first reads a later, uninitialized parameter and must not be
used as a substitution input. Adopt the existing parameterOrigin position
check (node position against binding end), keep default execution ownership
separate from body invocation, preserve the earlier-parameter positive case.

Planned focused fix: stringExpression refuses parameter uses before the end of
their declaration. Test both ordering directions and normal body use across
JS/JSX/TS/TSX. This is conservative lexical evidence, not runtime invocation
proof; deferred closures in early defaults may remain unknown.

Full suite for the preceding source snapshot is still running (44649). Do not
modify that source until it completes, so its receipt fingerprint stays valid.

## Implementation and verification

Waited until session 44649 completed before editing: its receipt retained an
unchanged fingerprint, exit 1 (one UI wall-clock assertion; details in the
preceding source receipt). Added binding.end > node.startIndex refusal to the
string parameter branch. Four language-parametrized tests cover late-parameter
negative, earlier-parameter positive and normal body-use positive.

Focused string/construction/function-parameter suites: 135 passed, 3 files,
exit 0 (session 27578). String suite now has 27 tests. tsc emit exit 0 (74252),
git diff --check exit 0. Isolated UI suite: 61 passed/1 skipped, exit 0 (74823),
unchanged timing threshold; not a whole-suite timing or regression guarantee.

After-edit SHA-256:
- construction-sites.ts: 4ef9406144990c8a069ff8c8c1d154ace71b397db21539b4c4e5846c6ccd7185
- string-argument-evidence.test.ts: 15d17fe7dda5604e6e5bb671a4edd74cee2f6ef6193aafef4524d9fcd0766c2a

No workers or tests remain running from this follow-up. Full native-required
regression for the after-edit snapshot has not yet run. Wrapper substitution,
fs/URL binding and cited file resolution are still unimplemented; this guard
prevents a known false substitution input rather than claiming flow completion.
