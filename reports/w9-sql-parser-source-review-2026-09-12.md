# SQLite syntax adapter — source gate before code

Existing outsource gap revalidated: Zvec zvec_sql_parser.cc sql_info/sql_type
dispatches SELECT only (SHA 547709901cf43b466bf4625647ed1d8170233b6cebad1b123f461ea431590cc8).
Its AST/error-first approach is useful, but copying that parser cannot cover
Orders INSERT/UPDATE/transactions. Prior CodeGraph SQL admission remains
file-only; this task does not alter that provider or pretend it supplies SQL AST.

Selected Apache sqlparser 0.62.0 (Apache-2.0), fetched with cargo info, not a
new database engine. Official source/docs:
https://github.com/apache/datafusion-sqlparser-rs/tree/v0.62.0
https://docs.rs/sqlparser/0.62.0/sqlparser/
SQLite dialect tests read from the pinned upstream tests/sqlparser_sqlite.rs:
CREATE TABLE quoting/untyped columns/conflict options and PRAGMA examples.
Local source read: SQLiteDialect complete; Parser recursion/token-input API;
Tokenizer token spans; Statement/DML variants; visitor relation traversal.
SQLiteDialect SHA 78c3a0cc649b15f312e7e27ddfc92fc7f141ee6e302d2302d0237443987e441d;
visitor SHA 1ac7c9a71ecf8a4d499fe61cc5aed65f6a55878920d8d33e2d1e16160a5d1947.

Adopt syntax AST and explicit SQLite dialect, recursion limit plus byte/token
and statement caps. Keep default recursive protection and enable visitor only;
do not add serde AST payloads. Parser success is not SQLite semantic validity.
Upstream spans are incomplete for some AST nodes: expose optional parser span
as a hint, not a verified complete-statement source slice. Source hash + ordinal
remain the stable observation anchor. Relations are syntactic references,
not resolved physical tables: CTE names/views/catalog binding remain unknown.
No SQL execution, connection, filesystem or network in graph-system adapter.

Read product system compose.rs/yaml.rs error/budget conventions. Skills
rust-router/m11-ecosystem keep dependency outside domain; m06-error-handling
requires typed redacted failure, m04-zero-cost favors concrete report/enums and
the upstream visitor rather than a new generic parser framework.

Planned public API: analyze_sqlite(&str) -> Result<SqlSyntaxReport, SqlError>.
Report carries SHA-256, ordered statements (ordinal, operation, syntactic
relations, optional parser span), semantic_verified=false. Operations include
Query/Insert/Update/Delete/CreateTable/Begin/Commit/Rollback/Other. Unsupported
classification stays Other, not silent omission. No graph authority publication
or code→query join yet; downstream integration remains an explicit open task.

Before changing dependency policy, read complete scripts/check-architecture.mjs
and its tests. Explicitly allow sqlparser only in graph-system, extending the
existing YAML adapter boundary test to reject it in domain/application. Keep
database/network/store/source dependencies forbidden there. Cargo resolved
eight additional packages including recursive/stacker/psm and visitor derive;
default recursion protection is retained, not disabled to minimize the lockfile.
No transitive runtime-isolation or supply-chain audit completion is claimed.

## Implementation and focused results

`graph-system` now exports analyze_sqlite and concrete syntax report/error
types. Parser source byte/token/statement/recursion caps are enforced, named
relations retain visitation order, Other is explicit, and no body/error snippet
or catalog/execution claim is returned. Optional upstream spans are hints.
No CLI, persistence, source-evidence capability or CodeGraph binding yet.

Cargo check/build passed; dependency guard reports 8 packages / 49 direct
declarations and 11 boundary tests passed. Initial clippy -D warnings stopped
in domain with 164 diagnostics; this is not a clean strict workspace check.
Normal clippy exit 0 had 258 diagnostics across dependencies/modules; the two
SQL doc-markdown warnings were fixed and a subsequent filtered check found
zero diagnostics in crates/system/src/sql.rs. No lints were disabled.

Luna worker 01a09673-5c4d-7ab2-a213-40946b97e4a7 (requested gpt-5.6-luna,
effective unknown) produced source-study receipt, but no tests before root
stopped it and received shutdown. Root read the receipt, corrected its worker
identity/revision claims, took over test ownership and wrote six integration
tests. Six passed, including all 11 Orders SQL files (schema has 3 statements),
quoted names, comment/literal exclusions, semicolons, CTE syntax, malformed
redaction, source identity and exact/over budget boundaries. Do not attribute
these tests to Luna. Dependency choice was source-backed, not an evaluation of
all SQL parsers on the market.

Final hashes: sql.rs
`5081cb897d326b915e91532534e98522af9a09686187470bc1e02ca52d0c22b9`;
tests/sql.rs `73cb27efe26cf65a539140e6bb870a35b1be226facfb9344790982154c40e7a0`;
Cargo.lock `7c64412e6b8640580bc519ea38af7b8b5def9fd1af6c094972c7bccd079618d6`.

Final foundation receipt:
`.harness/baselines/foundation-sql-parser-20260912-01.json`, exit 0,
390 Rust passed / 0 failed / 3 ignored + 16 Node passed; hashes of the eight
changed manifest/lock/source/test/policy files match before/after. This is not
a fingerprint of every worktree file. cargo fmt --all -- --check passed.
An earlier foundation run passed before the new SQL test target was present;
the final receipt is the current gate and includes those six tests.
