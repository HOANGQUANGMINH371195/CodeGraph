# W9 SQL parser adapter tests — source review and hash receipt

Date: 2026-09-12  
Source gate: ready for test implementation  
Owned files: `crates/system/tests/sql.rs`, this report  
Requested worker model: gpt-5.6-luna; effective model unknown. Parent launched
worker 01a09673-5c4d-7ab2-a213-40946b97e4a7, stopped it after this source receipt
but before a test file was delivered, and confirmed shutdown. Parent takes
ownership of the tests; do not attribute them to Luna.

## Source identity

The product worktree has no commit yet, so the pre-test-patch identity is the
SHA-256 of each relevant file. The worker did not record an outsource revision.
Parent verified Zvec HEAD 67ea1fa65ff99ee4c3a5bf4f2c0a6799c0390ead; hashes
below identify inspected source including local changes.

- outsource `zvec/src/db/sqlengine/parser/zvec_sql_parser.cc`:
  `547709901cf43b466bf4625647ed1d8170233b6cebad1b123f461ea431590cc8`
- outsource `zvec/src/db/sqlengine/parser/sql_info.h`:
  `82b86a71be270ad315a7964d5cf763319cee235d28ce180ba4207ad468454f64`
- product `crates/system/src/sql.rs`:
  `5081cb897d326b915e91532534e98522af9a09686187470bc1e02ca52d0c22b9`
- product `crates/system/tests/compose.rs`:
  `d3912b522b4b4fb35e328798f3cd3b461537ab4ff58510f76a09619392148202`
- product `crates/system/tests/compose_boundary.rs`:
  `47e61df82d204b827d5082ec2e6e528378be9e204948c66435d5ef7d09597211`
- pinned sqlparser `0.62.0/src/dialect/sqlite.rs`:
  `78c3a0cc649b15f312e7e27ddfc92fc7f141ee6e302d2302d0237443987e441d`
- pinned sqlparser `0.62.0/src/ast/visitor.rs`:
  `1ac7c9a71ecf8a4d499fe61cc5aed65f6a55878920d8d33e2d1e16160a5d1947`
- pinned sqlparser `0.62.0/src/parser/mod.rs`:
  `024cf521b6c9e4941bddd14e115920a49b0a503b7bebb68fabfb284bb9684a1c`
- pinned sqlparser `0.62.0/src/tokenizer.rs`:
  `63ffd30f5116ae74aadee929bb5a9e400329c633bea94819a7b2f9c729adfb60`

## Source and test study

Zvec `zvec_sql_parser.cc:37-119` installs lexer/parser error listeners,
parses a compilation unit, retries with LL after syntax errors, rejects lexer
or parser errors without returning a SQLInfo, and dispatches only SELECT in
`sql_info`. `:131-145` confirms `sql_type` returns SELECT only and otherwise
NONE. This is a syntax-parser precedent, not a complete operation contract;
the new adapter must also observe SQLite INSERT/UPDATE/DELETE/DDL and
transaction statements.

The current product `crates/system/src/sql.rs` checks the 128 KiB byte cap,
tokenizes with locations, checks 2048 tokens, parses SQLite statements with a
32 recursion limit, checks 256 statements, maps the public operation enum,
collects `sqlparser::visit_relations`, derives optional parser spans, hashes
the original bytes, and returns no partial report on any error. Errors are
typed and static, so malformed-source tests must verify that source text and
literal values are not echoed.

Existing system tests (`compose.rs`, `compose_boundary.rs`) use independent
integration tests, real `include_str!` fixtures, public APIs, explicit
operation/relationship assertions, and static redaction checks. The actual
Orders corpus has exactly 11 SQL files: three CREATE TABLE statements in
`schema.sql`, three transaction files, three SELECT/UPDATE/INSERT paths, and
the remaining INSERT/SELECT statements. Their source hashes are retained by
the fixture files and the test will assert each report hash rather than
normalizing or executing them.

Pinned sqlparser 0.62.0's `SQLiteDialect` recognizes SQLite identifier quoting,
SQLite transaction modifiers, conflict syntax, and SQLite-specific operators.
`Parser::parse_statements` ignores empty separators, preserves literal
semicolons inside tokenized string literals, and accepts a configured
recursion limit. `Tokenizer::tokenize_with_location` supplies locations but
returns an error before parsing on lexical failure. The visitor docs and unit
tests define `visit_relations` as depth-first syntactic relation traversal,
including nested queries and CREATE TABLE names; it does not resolve CTEs,
views, catalogs, or physical storage. Upstream spans are parser coordinates,
not guaranteed complete statement extents.

## Adopt / avoid

Adopt independent offline tests over the exported adapter, the same SQLite
dialect and visitor contract as production, all 11 real fixture files, fixed
hash expectations, ordered ordinals, operation mapping, relation visitation,
static error text, and boundary-focused limit cases. Include a CTE test whose
expected relation set deliberately treats both the CTE name and the underlying
source relation as syntax observations; it makes no physical-table claim.

Avoid copying Zvec's SELECT-only classification, executing SQLite, opening a
catalog, resolving relation roles, asserting exact full statement spans, or
accepting partial reports after malformed input. Avoid weakening assertions to
match a green implementation: token, statement, byte, and deep nesting cases
must fail at their documented limits. The test keeps the token case within the
byte cap and the 257-statement case below the token cap so each budget is
isolated.

## Test plan recorded before patch

1. Assert every actual Orders fixture parses with its expected operation,
   syntactic relations, statement ordinals, `semantic_verified == false`, and
   precomputed source hash; assert schema produces three ordered CREATE TABLE
   observations.
2. Cover the missing DELETE and Other operation variants using SQLite syntax,
   plus quoted identifiers/comments/string literals that must not fabricate
   relation names.
3. Cover a nested CTE, multiple statements, literal semicolons, trailing and
   empty separators, optional valid parser coordinates, and stable hash bytes.
4. Cover malformed/redacted failure with no partial report, exact byte and
   token boundaries, 257 statements below the token budget, and recursion
   deeper than 32 while keeping each case independently attributable.

No production file, manifest, dependency, fixture, or architecture policy is
changed by this test task.

## Parent implementation after worker shutdown

Parent wrote six tests; focused exit 0, 6 passed. Hash assertions calculate
SHA-256 over exact fixture bytes and separately assert comment changes alter
identity; the planned hard-coded digest list was not added. All 11 SQL files
are exercised, with schema's three statements checked in order. Exact-byte,
token and statement limits plus over-limit/recursion cases passed. Final
foundation included this target: 390 Rust pass/3 ignored +16 Node pass.
Parent test file SHA:
`73cb27efe26cf65a539140e6bb870a35b1be226facfb9344790982154c40e7a0`.
