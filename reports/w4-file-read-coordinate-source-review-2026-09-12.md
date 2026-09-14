# File-read coordinate contract

Resumption: prior process3756 is missing. No result is inferred from that
interrupted run. Luna worker Descartes reported quota exhaustion; no completed
worker changes or tests are credited. Parent takes over without model fallback.

Before this follow-up edit, parent inspected discovery/test and construction
startIndex/endIndex extraction, string-call-chain extent validation and binding
source.slice use. Current discovery SHA256:
`3cbcdcb47b2c0e6d6cdc7e02052e6dfcd7cdba43f36add65ef77448343dbe87e`;
target: `146b10403add18cc8e3672069e06a047fa294fdacc955c5ea40223505ced658b`;
construction: `c59d2872b6b13941d985d5178d6bde078628f10bd1216d40ecc93774b9cbd505`;
discovery test: `a5fe664443f82c3dc2579e314c0f9f3c4f82fd01dd8d0294e077414d7207a85e`.

Adopt exact JS string slicing and Unicode distinction for public coordinates;
avoid interpreting them as UTF-8 byte positions in Rust. Existing coordinate
metadata was added in the interrupted turn, before this receipt (not a
retroactive source gate). Current test incorrectly expects UTF-16 length to
exceed UTF-8 length for an emoji; change to an exact two-unit difference and
exercise non-ASCII wrapper hops/parameter identities plus emitted CLI output.
Declare zero-based half-open extents in documentation. This step provides a
wire contract prerequisite; durable graph/SQL integration remains unfinished.

First rerun26787:6passed/1failed. Actual invocation coordinates match UTF-16,
but exact-shape assertion exposed that binding spreads the whole caller-owned
readInvocation (discovery passes FunctionCallEvidence) into public output. After
reading bindLocalFileRead, adopt explicit start/end projection and retain this
exact-shape assertion. No parser metadata should leak through a locator.

## Verified result

TypeScript emit71306 exited0. Native-required focused run51156 exited0:
62tests/7files, including12 emitted CLI tests. Emoji plus accented text/CRLF
prefix checks full-source hash, read/base extents, outer/inner invocation spans,
and the three-unit UTF-8 vs UTF-16 prefix difference. Direct discovery separately
checks the exact two-unit emoji difference. Source positions are zero-based
half-open; byteLength and lineCount retain their separate meanings.

Probe83446 exited0; receipt
`.harness/baselines/orders-file-reads-coordinate-20260912-01.json`:11candidates,
11captures,13states,19742output bytes, exact/one-under budget tests pass and
inputs unchanged. Old25136-byte receipt remains historical. The updated smoke
asserts both coordinate declarations and exact locator field keys.

Final binding SHA256:
`d3f1fc750cdf7e5f415174a0c0e725ed4e38d55942c26e06e1e94bf23ee78231`.
Discovery/target hashes above remain current. No Rust/domain/store/migration
implementation changed in this follow-up. Durable ingestion and SQL graph
publication remain open; focused success does not erase prior full-suite fail.
