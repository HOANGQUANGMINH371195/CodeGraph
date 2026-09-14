# W1 — Streaming artifact content verification

Added application ArtifactReader port, verify_artifact and private-constructor
VerifiedArtifactContent receipt. The verifier checks declared length against
the host byte budget before opening input, then reads at most declared length
plus one sentinel byte using a fixed 32 KiB buffer. It verifies exact length
and SHA-256, accepts binary/empty content and returns metadata only. It retries
Interrupted reads and propagates other read failures. It does not buffer a
whole blob or return unbounded text to an agent.

DirectoryArtifacts is a read-only capability-scoped adapter in graph-source.
The host selects a private per-snapshot root; filenames are validated SHA-256
strings, not caller paths. Full ProjectRef and graph version are checked before
lookup. Only regular files are exposed; Unix opens use NONBLOCK to avoid FIFO
open waits. Directory capabilities prevent symlink path escape.

No trusted snapshot discovery, CAS ingestion/atomic publication, durable
verification receipt, authorization, redaction/encryption verification or
execution attestation is implemented here. A hash match proves bytes read now,
not future file immutability. A malicious custom reader or slow filesystem may
block: byte bounds are not deadlines, so supervisor cancellation is still
required. No semantic acceptance or ledger state changes occur. Blob ingestion
remains pending. Filesystem validation is Linux only.

Three integration tests exercise binary, empty and 100k-byte blobs; appended,
truncated and same-length corrupted content; pre-open budget rejection;
graph-version mismatch before missing-file access; directory rejection; and
external symlink denial. `sh scripts/validate-foundation.sh` passed 43 Rust +
10 Node tests, zero failures/ignored. Architecture gate passed with unchanged
six packages/29 dependencies. Format check passed.

Rust/domain skills informed the separation between a validated byte receipt
and metadata/protection declarations. No dependencies or migrations were added.

## CLI verification

Added `verify-artifact ID TASK.json ROOT --max-bytes N` (default 64 MiB).
The command resolves persisted metadata in the complete task snapshot before
opening a host-selected blob root. It returns metadata only with content_verified
true, protection_verified/execution_verified false, content_is_untrusted true,
snapshot_binding caller_supplied and receipt_persisted false. A successful check
does not persist trust or make a later metadata lookup report verified content.

One new subprocess test covers missing blob, exact success JSON, over-budget,
same-length hash tampering, truncation/append, wrong snapshot, unchanged lookup
trust and no task events. Help test now covers verify-artifact without database
creation. Errors have nonzero exit, stderr and empty stdout, per the CLI skill.
Foundation validation passed 44 Rust + 10 Node tests, zero failures/ignored;
format and dependency architecture gates passed. No deadline, ingestion or
durable receipt implementation is implied by the CLI addition.
