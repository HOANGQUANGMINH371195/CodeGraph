# Orders benchmark corpus v1

Owned test corpus, not a production service or a Harness implementation stack.
Node 22.22.1, no npm dependencies: `node --test test/*.test.mjs` from this folder.
Two HTTP services and separate SQLite databases. The outbox is the durable queue;
`POST /dispatch` explicitly drains it over HTTP, not a background broker.

Deployment description: compose.yaml/Dockerfile. Docker has not been validated;
the image tag is versioned but not digest-pinned for release acceptance. Tests run
both service listeners in one process with separate databases and real loopback
HTTP. No external service, model or account is contacted.

This intentionally small corpus tests structural relationships, not scalability,
token savings, production authentication or crash/power-loss safety. Delivery is
at-least-once; notification deduplication assumes immutable event IDs/payloads.
No email is sent. Keep benchmark answer keys outside the indexed source corpus.
