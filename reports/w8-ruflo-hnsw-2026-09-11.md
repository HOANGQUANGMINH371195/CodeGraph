# Ruflo handwritten HNSW measurements — 2026-09-11

Measured the actual `HNSWIndex` from
`ruflo/v3/@claude-flow/memory/src/hnsw-index.ts`, not a native/WASM bridge or
CLI memory search. Source revision: `a64f8b1ad89035c8b204f8b6e0893a2288551e06`;
source SHA-256: `e2372d7b3ca4a3f2e34a24f8484f5f7e8a7977d385ac1da922e557b57ebd2ab5`.
Ruflo was not modified. All runs report unchanged source fingerprints.

## Method

Linux x64, Intel i5-12450H, Node 22.22.1, TypeScript 5.9.3. The runner transpiles
the entire source module without changing its implementation. Only its
`node:events` runtime dependency is permitted; no native or mock fallback loads.
Each dataset runs in a separate process, without overlapping benchmark jobs.

Normalized uniform-random Float32 vectors, independent queries, fixed dataset
seed 424242 and graph-level seed 1337; cosine, k=10, M=16, efConstruction=200,
no quantization. Fifty queries per ef after ten warmups. The extended run
checks its top-k exact oracle against an independently full-sorted cosine scan
for three queries. Seeds, vector/query hashes, compiler/runner/source hashes,
per-ef metrics and command are in each receipt.

This is a preliminary synthetic benchmark: one build seed, fifty query samples,
not a statistical comparison across many corpora. Exact baseline is scalar JS
with corpus norms precomputed, not a native SIMD implementation. Synthetic
random geometry is not representative evidence for semantic code retrieval.

## Results

1,000 vectors × 128 dimensions: build 0.837 seconds. Exact p50 0.216 ms;
HNSW ef=100 p50 0.609 ms at 99.2% recall, ef=200 p50 0.808 ms at 100% recall.
[Receipt](../.harness/baselines/ruflo-hnsw-1000-128-20260911.json).

10,000 vectors × 384 dimensions, extended sweep: build 67.164 seconds.

| Search | Recall@10 | p50 ms | p95 ms |
|---|---:|---:|---:|
| Exact scalar JS | 100% (oracle) | 4.281 | 4.731 |
| HNSW ef=10 | 8.2% | 1.030 | 1.663 |
| HNSW ef=50 | 32.4% | 2.477 | 3.494 |
| HNSW ef=100 | 49.2% | 3.649 | 4.547 |
| HNSW ef=200 | 70.4% | 6.353 | 7.957 |
| HNSW ef=400 | 89.0% | 10.749 | 21.165 |
| HNSW ef=800 | 98.8% | 15.658 | 18.991 |
| HNSW ef=1600 | 99.8% | 21.515 | 24.227 |

[Extended receipt](../.harness/baselines/ruflo-hnsw-10000-384-high-recall-20260911.json).
An [earlier 10k sweep](../.harness/baselines/ruflo-hnsw-10000-384-20260911.json)
has identical recall at the overlapping ef values but different timings;
the repeated fixed seed is not an independent quality trial. p99 in raw receipts
is the maximum of only fifty samples and should not be treated as a production SLO.

## Scoped filtering and memory

Upstream `searchWithFilters` overfetches 3×k globally, filters, then takes k.
At 1% selectivity and ef=200 it returned on average 0.16/10 results at 1k and
0.36/10 at 10k (filtered recall 1.6% and 3.6%). Each dataset had at least ten
eligible neighbors, so underfill is not an empty-namespace explanation.
This is not an isolation leak, but it is unsuitable as proof of complete
project/snapshot-scoped retrieval. Filter-aware partition/search or a bounded
fallback with an explicit coverage gap is required before adapter acceptance.

The first 10k run's RSS rose from 164,724,736 to 206,536,704 bytes after indexing
and GC (about 39.9 MiB delta); upstream estimated 32,000,000 bytes. RSS includes
the corpus, compiler, oracle and V8 allocations; neither value proves isolated
index memory. Peak process RSS and each run's individual values are in receipts.

## Reproduce and next comparison

From the product repository, choose a new receipt filename:

```sh
node --expose-gc scripts/ruflo-hnsw-benchmark.mjs /home/minh/projects/outsource/ruflo /home/minh/projects/outsource/codegraph/node_modules/typescript/lib/typescript.js .harness/baselines/ruflo-hnsw-new-run.json 10000 384
```

The performance skill informed the recall/latency/RSS trade-off measurements;
no advertised speedup was used as a result. This TS implementation did not beat
the measured exact baseline at high recall on these datasets. That is not a
conclusion about all HNSW implementations, native Ruflo engines, larger corpora
or real code embeddings. SQLite-vector and Zvec have not yet been measured on
this corpus; **there is no three-engine ranking yet**. W8/P8.T11–T12 stay open,
including persistence, updates/deletes, concurrent readers, realistic filters,
real embedding datasets and recovery tests.
