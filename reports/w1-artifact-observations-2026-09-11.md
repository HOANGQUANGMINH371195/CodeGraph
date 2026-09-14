# Artifact content observations

V6 adds immutable historical content observations linked by full snapshot to
registered artifact metadata. Writes through ArtifactObservationRepository take
VerifiedArtifactContent, require exact registered descriptor equality, and
record a nonnegative host-supplied timestamp plus sha256-length-v1. Observation
IDs replay only for identical artifact/time/version; conflicts fail. Reads return
ArtifactContentObservation data, not a fresh verification capability.

V1–V5 were unchanged. Two store tests cover missing/different metadata,
idempotence/conflict, update/delete denial, restart and full snapshot isolation,
V5 upgrade preserving artifacts without fabricated observations, and a raw SQL
cross-scope FK rejection. The full gate passed 52 Rust + 10 Node tests before
the native-subagent research pivot. No new dependencies were added.

The API does not prove when a remote analyzer executed, who authenticated the
timestamp, whether the blob remains unchanged or whether redaction is valid.
Custom reader adapters are trusted host components; direct database tampering is
outside this type boundary. No execution acceptance/outbox event is emitted.
CLI and ingestion receipt persistence are not connected yet. Rust/domain skills
informed the distinction between historical records and verified capabilities.
