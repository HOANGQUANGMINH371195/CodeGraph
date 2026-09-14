# W1 — Artifact metadata contract

Implemented immutable domain Artifact and strict versioned JSON wire contract.
Fields: id, full ProjectRef, graph_version, analysis_run, content_sha256,
byte_length, kind, retention, declared_protection. The digest describes exact
stored bytes (after any transformation), not the pre-redaction plaintext.
Empty output is allowed. Lengths above signed 64-bit ledger capacity are rejected.

Retention is ephemeral/evidence/user_pinned; protection is
unreviewed/redacted/encrypted/redacted_and_encrypted. Protection is explicitly
a producer declaration, never proof or permission to expose content. Retention
is a requested lifecycle classification, not authority to pin/delete content.
Reference counts are deliberately absent from the wire: the future ledger
must derive references transactionally from consumers, not trust supplied counts.
No path, secret, encryption key, verified flag or deletion method was added.

Rust domain/type-driven skills informed private fields, validated construction
and closed policy enums, without a new dependency or runtime.

Validation: `sh scripts/validate-foundation.sh` passed 36 Rust and 10 Node tests,
zero failures/ignored. Two protocol tests cover every policy combination,
roundtrip, empty bytes, future schema, malformed hashes, length overflow and
all required scope/identity fields. A JSON test in the CLI test target uses its
existing serde_json dependency to cover serialization and rejection of unknown
fields, caller reference_count/verified/path, unknown enums and negative lengths.
This is a contract test, not evidence of an artifact CLI command.

## Ledger registration

Added ArtifactRepository and SQLite V5 metadata table with immutable triggers,
composite foreign key to analysis run id/project/graph_version and separate SQL
files. V1–V4 were not edited. Exact duplicate registration returns false;
different content at the same ID conflicts. Missing/different-scope run is
rejected before insertion; the FK also blocks raw cross-scope SQL inserts.
Lookup validates decoded metadata and its run registration. No blob access,
acceptance event, verified protection or task transition is implied.

Two additional tests cover restart, replay/conflict, update/delete refusal,
all six ProjectRef components plus graph version, missing IDs and run absence,
no automatic outbox event, actual V4 upgrade preserving runs and SQL FK checks.
`sh scripts/validate-foundation.sh` passed 38 Rust + 10 Node tests, zero failures
or ignored tests; architecture gate remains six packages/29 dependencies.
Rust/domain skills informed the application port and adapter-owned persistence.
The two migration-count assertions now expect five versions.

## CLI metadata access

Added `record-artifact ARTIFACT.json` and `artifact ID TASK.json`. Versioned
responses explicitly report content_verified/protection_verified/execution_verified
as false even if declared_protection says redacted_and_encrypted. Lookup returns
artifact:null for missing or differently scoped metadata. Task JSON is a scope
selector, not authorization; no bytes are ingested by these commands.

Two subprocess tests cover registration before/after matching run creation,
replay across processes, ID conflict, invalid wire fields/digests/schema,
unknown run, all six project fields and graph version, preservation after
rejected writes, no task events and help without database creation. Errors
produce nonzero exit/stderr with no success JSON, following the CLI skill.
`sh scripts/validate-foundation.sh` passed 40 Rust + 10 Node tests with zero
failures/ignored; architecture gate and format check passed.

Remaining: blob
ingestion, byte verification, bounded reads, protection receipts; transactional
references/pins and GC; task attempt/execution receipts and verifier integration.
No existing string-based submission/integration behavior was upgraded by this
contract. P1.T07 and W1 remain incomplete.
