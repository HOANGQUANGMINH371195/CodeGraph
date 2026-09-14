# Orders architecture retrieval: root cause and remaining work

## Current-source and executed diagnostic

Parent inspected `CodeGraph.findRelevantContext`,
`ContextBuilder.findRelevantContext`, `extractSearchTerms`, and
`ToolHandler.handleExplore` before the empty-subgraph return. Also inspected
Orders HTTP implementation and Compose services. The diagnostic ran against
the fresh emitted build using an owned temporary fixture copy, then closed
the graph and removed only that copy (process exit 0).

CodeGraph revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty source
fingerprint `5a0098c34f5018b16340834fad4da7da255538ba203f52758912755525b674b7`.

| Query | Context nodes / roots | Tool output |
| --- | --- | --- |
| Vietnamese architecture/sequence/data-flow request | 0 / 0 | No relevant code found |
| Describe the system architecture | 0 / 0 | No relevant code found |
| ordersServer notificationsServer OrderStore OutboxRelay | 36 / 8 | 25 symbols across 4 files; 5,411 chars |
| compose.yaml src/main.mjs sql/schema.sql | 27 / 1 | 5 symbols across 1 file; only one file pinned |

The last output contains the strings compose.yaml and sql/schema.sql, but
that is NOT proof their source was returned: the query heading and unmatched
path guidance contain them. Do not score evidence presence using substring
checks alone. Actual inventory includes Compose plus six JavaScript files;
`getNodesInFile('compose.yaml')` is empty and SQL files are not indexed.
`getNodesByKind('route')` returns zero.

## Cause and implementation implications

The broad-query failure occurs before rendering: exact names, name-segment
seeds and textual search have no matching seed, and explore returns when its
subgraph is empty. English reproduces the failure, so translating the query
alone is not a fix. The named query proves some code connectivity exists,
not that deployment/data-flow architecture has been reconstructed.

The current HTTP source uses node:http callbacks with conditional method/URL
checks. Compose declares two services and distinct volume/database paths.
Neither those deployment facts nor the route boundaries currently exist as
the required graph entities. Named-symbol retrieval cannot substitute for
W9 adapters and W11 evidence-linked architecture compilation.

Adopt the PLAN's orientation → module → symbol → evidence progression and
explicit coverage/unknowns. Preserve upstream explore schema compatibility.
Do not hardcode Orders symbol names into production fallback, manufacture
service edges from search words, or equate a nonempty source answer with a
validated diagram. The authored Orders diagram requirement remains unchanged.
