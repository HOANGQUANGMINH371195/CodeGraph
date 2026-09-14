# Compose declared-fact adapter — source study and architecture decision

## Resumed parser implementation source check

Before resuming implementation, main reread current OpenDev frontmatter parsing
(same SHA-256 below), Codex migration source and product architecture tests.
Fetched yaml-rust2 is **0.11.1**, not an inferred latest version. Read its actual
`parser.rs` Event/next_token and `scanner.rs` Marker/ScanError/scalar-style APIs;
parser SHA-256 `ecee17f228209b20e6c57e0af89f63bac0adfe56aa8384a32598b38ea8694bde`.
Markers are one-based lines. Read the dependency MIT license; use its public API,
not copied implementation. Adapt event iteration to bounded recursive syntax
construction; strip parser error text to a static category and line. Preserve
quoted null as a scalar, reject duplicate/complex keys and all anchors/tags.
The input byte limit also bounds scanner lookahead before an event is returned.

Parent boundary-test review reread OpenDev `frontmatter_tests.rs`, Codex
`external-agent-migration/src/lib_tests.rs` frontmatter tests, source evidence
construction, the new Luna Compose draft and tests before adding independent
integration controls. The first draft uses colon-concatenated IDs, silently
ignores several fields, cites reference container lines rather than item lines,
and rejects unsupported mounts instead of preserving unknowns. These are
explicit requested-contract gaps, not acceptable shortcuts for a green fixture.
Add adversarial scope collisions, changed-byte citation IDs, exact reference
lines, unresolved references/interpolation and short/long-volume controls.

Additional source-line boundary found before follow-up patch: product verifier
counts LF-delimited physical source lines, while yaml-rust2 scanner
`skip_linebreak`/`char_traits::is_break` also counts bare CR. Read both actual
implementations. Adapt marked YAML logical lines through a precomputed LF-line
map, preserving raw source/hash rather than normalizing bytes or rejecting CR
input. Multiple YAML lines on one physical source line cite that same full line.

Main agent read the current OpenDev frontmatter parser and Codex migration
frontmatter parser, plus this product's SourceEvidence, verify_source,
DirectorySource, architecture policy/tests and W9 contract before coding.

- OpenDev revision `d32c660e4eed1a8e988d1fd58da88e41ba641d08`;
  frontmatter.rs SHA-256 `733ca566be927604b43a61f76efe79b2ae70e9bc4a7f71dfdfd306635e0a91cf`.
- Codex revision `818f1cca8ccf8899f0f4d59336baebaccf358eed`;
  external-agent-migration/src/subagents.rs SHA-256
  `88816a9d45d136096faec0056f2e8511c58c437f59fca53950ef04f09ff94c06`.
- Product SourceEvidence SHA-256
  `1abbd2090921b1b7bd0ac377f3a42ceeb7c5fb189be34bc15a9e5a1ee684768d`;
  application source verifier
  `8cb050b18786a160747db6869b25dbdff8c3afc94dade55cadce935ca69596cf`;
  architecture policy
  `fbc84a387851c3bf61e7edd9cfe6f4f4ada3658b58eb0918a69939203c2ac26c`.

Adopt typed parsing and explicit errors from these references. Avoid OpenDev's
`.ok()` loss of parse diagnostics, and do not use line regexes as YAML parsing.
Neither inspected repo provides a Compose-to-graph extractor. The adaptation
requires marked YAML nodes so each emitted declaration points to a real source
line, including flow-style YAML. Compare declarations, not runtime resources.

Primary references fetched during this study:

- [yaml-rust2 parser](https://docs.rs/yaml-rust2/latest/yaml_rust2/parser/struct.Parser.html):
  token-by-token events support stopping on depth/event budgets.
- [marked events](https://docs.rs/yaml-rust2/latest/yaml_rust2/parser/enum.Event.html).
- [Compose services](https://docs.docker.com/reference/compose-file/services/),
  [volumes](https://docs.docker.com/reference/compose-file/volumes/), and
  [interpolation](https://docs.docker.com/reference/compose-file/interpolation/).

## Reviewed boundary

Add `graph-system`, a synchronous offline adapter crate, depending only on
graph-domain, graph-application, sha2, thiserror and yaml-rust2 (no default
encoding feature; input is already verified UTF-8). Parser 0.11 API is the
initial pinned minor target; inspect fetched source/lock before claiming its
runtime behavior. No domain YAML/JSON/DB dependency and no Docker daemon,
filesystem reads, environment expansion, network, shell or container launch.
Update the architecture allowlist and its negative tests explicitly for this
adapter; do not relax existing domain/application/store boundaries.

Input is a verified full-file SourceSlice; independently check that its text
hash is the full-file citation hash. Output is a candidate deployment projection:
service/volume/network declarations, explicit mounts/dependencies/attachments,
source citations and unknowns. Identity includes repository/worktree, logical
Compose path, declaration kind and name, not line number. Citation identity
includes content hash. Source verification does not establish runtime truth.

Reject duplicate keys, malformed/multiple YAML documents and exhausted budgets
without returning a partial-success graph. Anchors/aliases/custom tags initially
return an explicit unsupported error rather than expanding or guessing. Do not
resolve interpolation from host env. Unsupported fields and unresolved declared
targets remain diagnostics; never infer traffic or a database from a service name.
Do not expose arbitrary environment values/secrets in projected facts/errors.

Skills used: rust-router, m09-domain, m11-ecosystem, domain-cloud-native and
m06-error-handling. The cloud-native skill informs deployment boundaries only;
this is intentionally a local-first offline tool, not a stateless hosted service.
This increment does not close W9 or replace its remaining Compose/manifest/API,
SQL, graph persistence/query, invalidation and diagram requirements.

## Parent implementation and verification result

The parent implemented bounded YAML event parsing and independent boundary tests;
Luna supplied the Compose adapter and initial integration tests. Parent reviewed
the actual source and corrected identity collisions, raw/unsupported semantics,
reference line accuracy and the CR/LF boundary. Source-review attribution errors
in the worker receipt were corrected rather than treated as source evidence.
Final integration separates source validation, section parsing/projection and
long-mount decoding; no production lint suppression was added.

Implemented candidate declarations: Service, Volume, Network; Mounts, DependsOn
and AttachedTo edges. Both short named mounts (including ro/rw) and long named
volume mounts (read_only true/false) work. Access modes are explicitly unmodeled
diagnostics, not preserved security assertions. Unresolved targets, interpolation,
bind/anonymous/unsupported mounts retain unknowns, not invented resources. Field
values such as environment secrets are not copied into diagnostics or errors.
IDs are length-prefixed SHA-256 identities scoped by repo/worktree/path/kind/name;
citations additionally bind full project/run/graph/source hash and actual line.

Parent verification on final implementation:

- `cargo test -p graph-system --locked --offline`: 18 passed (3 marked-parser,
  8 Compose/source integration, 7 independent boundary tests). The final full
  workspace run below reran all 18 after parent integration/refactoring.
- `scripts/with-local-tools cargo clippy -p graph-system --lib --locked --offline
  --no-deps -- -D warnings`: exit 0, no warnings. This is scoped library Clippy,
  not a claim that every workspace test target is warning-free.
- `scripts/with-local-tools sh scripts/validate-foundation.sh`, session `78317`:
  terminal exit 0; **301 Rust passed, 0 failed, 3 ignored; 14 Node tests passed**.
  Architecture inspection: 8 workspace packages / 46 declared dependencies.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- `git diff --check`: exit 0 (product is unborn/untracked; this is not a complete
  untracked-file whitespace audit).

Final SHA-256 identities:

- compose.rs: `a74d8095cb1b390cfa77658861c8ec9284f959e830a84df7d8aa0907e1dfaf4b`
- yaml.rs: `85bb6496ae6bfc38f5ce2380cf16c456127690c0f8625b6c11796abd6e3964b3`
- tests/compose.rs: `d3912b522b4b4fb35e328798f3cd3b461537ab4ff58510f76a09619392148202`
- tests/compose_boundary.rs: `8a462c7db042f83778da2cba0ff97d7193e9a5513c360f85139124612672a49b`
- Cargo.lock: `5ea60cf9a49d548be541136078b6b752667051ed322ae73e5c88c3eef91c8b56`

Remaining: CLI/protocol exposure, persistence and stale-removal/reindexing,
graph-aware retrieval/compiler integration, build/module mapping, ports and
cross-service runtime evidence, SQL/API/other IaC adapters and actual rendered
diagram acceptance. These candidate DTOs confer no trusted graph-write authority.
There is no Docker/env/network/filesystem access in graph-system. This adapter
is not a complete Compose schema validator: unsupported semantics remain unknown
and YAML anchors/tags/multi-document input remain explicit errors. W9 stays open.
