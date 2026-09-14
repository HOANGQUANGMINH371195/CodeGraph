# Kiểm toán truy hồi cục bộ cho Rust terminal graph harness

Ngày: 2026-09-11. Phạm vi độc lập: `zvec`, `sqlite-vector`, `icm`, `context-mode`, `LLMRouter`, dưới `/home/minh/projects/outsource`.

## 1. Kết luận kiến trúc

Thiết kế đích dùng **một tài khoản Codex, native subagents và swarm tự chủ có giới hạn**. Lớp harness cần lưu graph công việc, bằng chứng, ngân sách và trạng thái thực thi; các kho được khảo sát chỉ cung cấp cơ chế truy hồi, bộ nhớ hoặc mẫu kiểm thử. Không đưa model router, key rotation hay một scheduler agent khác thành kiến trúc bắt buộc.

Đề xuất triển khai đầu tiên: SQLite lưu graph/trạng thái và chỉ mục từ khóa; truy hồi trả về các đoạn có provenance và ngân sách đầu ra. Vector là backend tùy chọn có thể tái tạo. Học cách chunk/index/search từ `context-mode`, transaction và tách query/document embedding từ ICM. `sqlite-vector` là ứng viên thử nghiệm vector cùng SQLite; `zvec` là lựa chọn thử nghiệm khi cần engine ANN/hybrid riêng. Không có phép đo trong đợt này để kết luận backend nào nhanh hơn trên corpus của harness.

Năm phát hiện ưu tiên:

1. **`context-mode` có rủi ro mất scope khi refresh**: lời gọi re-index bỏ attribution; code insert mặc định `session_id=''`; bộ lọc cho chunk không có session xuất hiện xuyên dự án. Đây là chuỗi suy luận trực tiếp từ mã, chưa chạy tái hiện. Ngoài ra, thiếu/lỗi SessionDB có thể làm scope không được áp dụng. [23][25][26][27]
2. **ICM vẫn có thể thiếu kết quả hybrid sau lọc**: MCP xin nhiều ứng viên hơn, nhưng SQLite chỉ lấy tối đa 20 FTS + 20 vector. Một kết quả hợp lệ ngoài cả hai pool không được xét; hybrid trả rỗng sau lọc cũng không chuyển sang FTS. Kiểm thử starvation đã đọc dùng `embedder=None`, chưa chứng minh đường hybrid. [16][17][20]
3. **`sqlite-vector` full scan thực sự quét toàn bảng**; streaming ba tham số không bảo đảm thứ tự nearest-neighbor. `LIMIT k` đơn thuần không tương đương top-k. Bộ lọc dự án nằm ngoài global top-k có thể gây thiếu kết quả. [9][10][11]
4. **zvec có C ABI dùng được làm ranh giới Rust**, nhưng thành công cấp batch không đồng nghĩa từng document thành công; WAL auto-flush thất bại chỉ ghi log trong đoạn đã đọc. Không dùng vector index làm nguồn sự thật về task đã commit. [3][4][6]
5. **LLMRouter không phù hợp làm executor cho thiết kế mới**: đường batch gọi LiteLLM và chọn API key; chỉ nên học cách tách quyết định khỏi thực thi, kiểm thử replay và cách biểu diễn lỗi/usage. Tài liệu KNN nói có random fallback nhưng `route_single → load_model` thực tế ném lỗi khi thiếu checkpoint. [34][35][36][37][39]

## 2. Phương pháp, hướng dẫn và trạng thái snapshot

Đã đọc đầy đủ các skill `rust-router`, `domain-cli`, `m07-concurrency`. Chúng giúp định hướng ranh giới CLI, công việc blocking, ownership của trạng thái và giới hạn đồng thời; không dùng skill để mở rộng phạm vi thành viết code hay gọi agent khác. Đây là audit mã nguồn cục bộ, không phải khảo sát tài liệu sản phẩm Codex; native subagents là yêu cầu thiết kế do người dùng cung cấp, không phải khả năng API đã được xác minh trong năm kho này.

Đã kiểm tra `AGENTS.md` trên chuỗi thư mục cha và thư mục đích: không thấy hướng dẫn cha áp dụng ngoài `icm/AGENTS.md`. File này yêu cầu recall và store bộ nhớ [1]. Không chạy ICM: lệnh recall có thể cập nhật access/auto-decay [17], còn store vi phạm yêu cầu chỉ ghi báo cáo. Các `context-mode/configs/*/AGENTS.md` là file cấu hình ở nhánh con không nằm trong các source được khảo sát; không dùng chúng làm chỉ thị cho audit.

Chỉ chạy lệnh đọc: `rg`, `git rev-parse`, `git status`, `git ls-files`, `git check-ignore`, `nl`, `sed`, `ls` và kiểm tra văn bản. Không build, cài dependency, import runtime ML, gọi tài khoản, dùng mạng, chạy benchmark hay tạo subagent. **Số test thực thi: 0 ở cả năm repo.** “Có test” dưới đây chỉ có nghĩa đã đọc mã test; không có tuyên bố test pass. Không đọc/copy PLAN.md và không chỉnh sửa repo tham chiếu.

| Repo | HEAD kiểm tra | Trạng thái cục bộ trước/sau khảo sát | File tracked |
|---|---|---|---:|
| zvec | `67ea1fa65ff99ee4c3a5bf4f2c0a6799c0390ead` | `git status --porcelain=v1 --untracked-files=all` rỗng | 1539 |
| sqlite-vector | `0c2223ada9dce1fa33248c8835a15f51d9a0f655` | Rỗng | 99 |
| icm | `2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42` | Rỗng | 221 |
| context-mode | `ad7ef27106ee9ebbe9a75d0b75106c9187506e09` | Rỗng | 599 |
| LLMRouter | `d1490a37202b1bea799ab3a601cab349d2291b08` | Rỗng | 381 |

Các con số file dùng để nhận diện snapshot, không phải số file đã đọc. “Sạch” là kết quả Git, không khẳng định không có file ignored trong toàn repo. Các source SQLite của ICM đã đối chiếu bằng `git ls-files`, có tracked dù tìm kiếm mặc định ban đầu không liệt kê hết. Tham chiếu cuối báo cáo dẫn đến file và dòng của snapshot cục bộ; nội dung có thể đổi khi workspace được cập nhật.

## 3. zvec

### Phạm vi và luồng end-to-end

Đã đọc các đoạn thực thi Python `QueryExecutor`, C ABI insert/query/multi-query, C++ `CollectionImpl` write/query, WAL append/read, và test hybrid Python/WAL C++. Chưa đánh giá toàn bộ HNSW, DiskANN, tokenizer, optimizer hoặc crash-recovery engine.

Luồng truy vấn hybrid: `Collection.query` dựng context → `QueryExecutor.execute` phân nhánh theo số query → nhiều query cần reranker; RRF/Weighted/Callback dựng `_MultiQuery`, mỗi nhánh có `num_candidates=max(topk,10)` → binding gọi C++ `CollectionImpl::query(MultiQuery)` → khóa schema, validate từng target, lấy segments, đưa filter chung vào mỗi `SearchQuery` → tạo SQL engine cho từng nhánh → một segment thì chạy nhánh qua query thread pool, nhiều segment thì tuần tự ở tầng này để tránh chồng fan-out → nếu nhánh lỗi trả lỗi toàn query; nếu đủ kết quả gọi reranker và trả documents. C ABI `zvec_collection_multi_query` gọi cùng collection và chuyển kết quả sang mảng handle. [2][3][5]

Luồng ghi bổ sung: `insert` → `write_impl` lặp từng document và thu `Status` → C ABI đếm success/error. Nếu outer result hợp lệ, mã trả về vẫn có thể là `ZVEC_OK` dù một số document lỗi. Có API `insert_with_results` trả status từng phần để adapter không mất thông tin. [3][4]

### Cơ chế nên dùng

- Ranh giới C ABI giúp Rust không cần giữ Python trong hot path. Adapter phải sở hữu/free rõ collection, query và document handles; trước khi tích hợp thật cần audit FFI và kiểm thử sanitizer riêng. Báo cáo này chưa chứng minh Rust wrapper nào tồn tại. [3]
- Phân biệt số ứng viên của từng nhánh với `topk` cuối; hợp nhất bằng rank giúp tránh trộn trực tiếp distance và BM25 khác thang đo. Chọn số ứng viên theo corpus của harness, không sao chép mặc định 10 như một bảo đảm recall. [2][5]
- Shared schema lock và snapshot schema trong `query_result_snapshot_impl` là mẫu hữu ích để giữ schema nhất quán khi có DDL. Đây không phải snapshot task/graph của ứng dụng. [5]

### Gaps và hành vi không nên suy diễn

**Batch không atomic theo nghĩa tất cả-hoặc-không** ở vòng ghi đã đọc: status được thu từng document, không có rollback batch tại đây. Cần phân biệt: `write_impl` validate/sanitize toàn batch trước khi ghi ở dòng 1609–1611, nên document sai schema/dimension có thể làm fail sớm cả batch; lỗi trong vòng ghi sau đó mới có thể tạo mixed per-document status. Adapter phải biểu diễn cả batch rejection và `partial`, cho phép retry theo ID, không đánh dấu cả generation đã xuất bản khi chỉ kiểm tra mã outer. [3][4]

**Query không bảo đảm snapshot xuyên write batch**: comment và khóa ở đoạn delete phân biệt plain query chấp nhận view giữa lúc ghi, còn delete cần shared write lock cho bước chọn tập. Một task graph cần revision/generation riêng; shared schema lock không đủ bảo đảm kết quả thuộc cùng commit ứng dụng. [5]

**WAL không đồng nghĩa durable acknowledgment**: `LocalWalFile::append` tạo CRC, ghi record; auto-flush thất bại chỉ log, rồi trả 0. `next()` trả chuỗi rỗng cả khi EOF lẫn khi read/CRC lỗi ở lớp này. Cần kiểm tra tầng recovery phía trên trước khi kết luận mức mất dữ liệu; không giả định lỗi đọc toàn hệ thống bị nuốt. [6]

**Reranker Python tùy biến đổi đường thực thi**: native built-ins dùng C++, reranker khác gọi từng query tuần tự rồi merge ở Python. Đây là fallback thực, không phải mock; không quảng bá mọi reranker đều có cùng mức parallelism. [2]

Test đã đọc: `test_collection_fts_vector_hybrid.py` tạo collection thật trong thư mục tạm, dùng vector tự tạo 16 chiều, insert và kiểm tra thứ hạng/filter/FTS không match. Một assertion ranking chỉ yêu cầu một trong hai document đúng có mặt ở top 3, nên không phải bằng chứng recall trên code Việt/Anh. `test_query_executor.py` có `MagicMock` và schema mock; không được coi test wrapper là test ANN. WAL test ghi, flush và reopen thật nhưng đoạn đã đọc không mô phỏng power loss. [7][8][40]

**Quyết định:** giữ làm backend thử nghiệm giai đoạn sau; không bắt buộc zvec làm backend mặc định; vẫn giữ embedding và nghiệm thu vector trong phạm vi sản phẩm đầy đủ. Lưu canonical graph, artifacts và ingestion journal ngoài vector engine, kèm generation để tái tạo index.

## 4. sqlite-vector

### Phạm vi và luồng end-to-end

Đã đọc extension registration, `vector_init`, virtual-table planner/filter, full scan/top-k heap/streaming, quantization savepoint, validation và các test C. Không khảo sát toàn bộ SIMD kernels hay chạy phép đo quantization.

Luồng đầy đủ: load/register `sqlite3_vector_init` → đăng ký hàm `vector_init` và module `vector_full_scan`/`vector_quantize_scan` → ứng dụng tạo bảng có PK và BLOB, gọi `vector_init(table,column,options)` để kiểm tra bảng, type, dimension và metric → SQL `vector_full_scan(table,column,query,k)` đi qua `vFullScanBestIndex`/`vFullScanCursorFilter` → `vCursorFilterCommon` kiểm tra tham số, byte length query, lấy table context và cấp vùng top-k → `vFullScanRun` chuẩn bị `SELECT pk, vector FROM table`, quét từng row, gọi distance function, thay phần tử xấu nhất trong heap → heapsort tăng dần distance → cursor trả `id,distance`. [9][10][11][12]

### Cơ chế nên dùng

Vector nằm trong bảng SQLite thông thường, thuận tiện join với chunk/revision và dùng transaction của SQLite. Cơ chế heap top-k chỉ cần bộ nhớ theo k ở đoạn scan, nhưng tổng chi phí vẫn có thành phần đọc toàn bộ N vector và tính N khoảng cách; không gọi nó là ANN. Query BLOB được kiểm tra đúng số byte trước khi distance function đọc dữ liệu. [9][10]

Quantization rebuild dùng `SAVEPOINT quantize`, drop/recreate bảng quantized, dựng lại dữ liệu, serialize options rồi release; lỗi rollback đến savepoint. Nếu trước đó đã preload, wrapper gọi preload lại sau rebuild thành công. Đây là cơ chế đáng học cho derived index cập nhật có transaction. [13]

### Gaps và fallback

- **Streaming khác top-k:** ba tham số chọn streaming; planner ghi rõ không có sorting guarantee. Muốn nearest k trên đường này phải `ORDER BY distance LIMIT k`; `LIMIT k` riêng chỉ giới hạn số row đi qua. Test streaming cơ bản chỉ kiểm tra có kết quả và distance không âm, không kiểm tra nearest. [9][11][14]
- **Filter sau global top-k:** scan nội bộ không nhận project predicate trong SQL đọc bảng. Join kết quả top-k toàn cục rồi `WHERE project=A` có thể trả rỗng dù A có vector phù hợp. Phương án cô lập scope: partition vật lý theo phạm vi hợp lệ, hoặc stream toàn tập rồi filter/sort; nếu chỉ oversample phải công khai giới hạn recall. [10][11]
- **Dữ liệu lưu sai kích thước có thể bị bỏ qua:** query BLOB sai byte length bị lỗi, nhưng full scan bỏ row NULL/undersized, còn BLOB dài hơn không bị chặn bởi phép kiểm tra `< expected_bytes`. Adapter cần validate trước ghi và đếm row hỏng; không coi ít kết quả là corpus thực sự ít tài liệu. [9][10]
- **Quantized state là derived snapshot:** đoạn rebuild/preload và module scan không thiết lập cơ chế tự đồng bộ DML của bảng nguồn; tìm kiếm `CREATE TRIGGER`/`sqlite3_update_hook` trong file extension không thấy kết quả. Đây là khoảng trống hợp đồng cần kiểm chứng bằng insert/update/delete sau quantize, đặc biệt giữa hai connection; không giả định luôn tự refresh. [13]
- Thiếu quantization table trả lỗi cụ thể; không thấy nhánh âm thầm tạo vector giả hay chuyển sang full scan trong `vCursorFilterCommon`. [9]

Test đã đọc: full scan qua nhiều type/metric, top-k đủ ba row và sorted; TurboQuant preload/streaming, `WITHOUT ROWID`, qbits không hỗ trợ và phá BLOB quantized phải trả lỗi. Những fixture này có assertion thực; chưa đủ xác nhận cache invalidation xuyên connection, transaction ngoài rollback, hay recall của corpus mã nguồn. [14][15]

**Quyết định:** ứng viên vector nhẹ để làm spike trong SQLite; trước hết dùng exact scan làm oracle. Không chọn quantization chỉ dựa vào tên thuật toán hoặc benchmark README. **ICM dùng `sqlite-vec`/`vec0`, không dùng repo `sqlite-vector` này**; thay thế cần adapter và migration SQL, không phải đổi tên dependency. [18]

## 5. ICM

### Phạm vi và luồng end-to-end

Đã đọc MCP dispatch/store/recall, `Embedder`, SQLite transaction/search/schema migration và tests MCP/search. Backend PostgreSQL/OpenSearch có trong repo nhưng ngoài luồng khảo sát sâu này; không suy rộng kết luận SQLite sang chúng.

Luồng store: MCP JSON-RPC `tools/call` → `handle_tools_call` lấy name/arguments → `call_tool_with_config` → `tool_store` kiểm tra topic/content → optional document embedding → tìm near-duplicate cùng topic, có thể merge; nếu mới thì auto-link → `store.store` → SQLite `BEGIN IMMEDIATE`, `store_inner` và commit/rollback → cập nhật backref best-effort sau store → trả ID. Luồng này không biến quan hệ similarity thành dependency thực thi. [19][21][22][41]

Luồng recall: cùng dispatcher → `tool_recall` chạy auto-decay best-effort, chuẩn hóa limit 1..20 và project/topic/keyword → `embed_query` → SQLite hybrid lấy FTS OR và `vec0` cosine KNN → chuẩn hóa BM25 bằng `abs(rank)/(1+abs(rank))`, vector similarity `1-distance`, trộn 0.3/0.7 → lọc phạm vi → mở rộng `related_ids` một hop với discount 0.5 → lọc lại neighbors, truncate → cập nhật access → format kết quả. Nếu embedding/hybrid lỗi, chuyển FTS rồi keyword; nếu hybrid thành công nhưng sau lọc rỗng, trả “no memories” ngay. [16][17][19]

### Cơ chế nên dùng

- `Embedder` tách `embed` và `embed_query`; đây là contract cần thiết cho model dùng tiền tố passage/query. `dimensions` không đủ nhận diện model, cần thêm model ID/version/hash ở harness. [42]
- Transaction cho memory và vector sync, dedup theo nội dung/topic, giữ importance cao hơn khi merge là mẫu cho derived memory. Canonical evidence của graph nên bất biến; summary có thể cập nhật và luôn trỏ đến evidence. [21][22]
- Giới hạn recall và one-hop expansion có thể chuyển thành ngân sách số chunk/cạnh của harness. Bộ lọc phải áp dụng lại cho neighbor là bài học cụ thể từ mã. [17]

### Gaps và hành vi đặc biệt

**Pool bị chặn ở 20 mỗi nhánh.** `query_limit=(limit*10).min(200)` ở MCP không buộc backend lấy đủ ứng viên. Backend dùng `pool_size=limit.clamp(1,5)*4`, nên tối đa 40 ID trước dedup. Ví dụ 21 tài liệu dự án B đứng trước tài liệu A ở cả FTS/vector: query cho A có thể không thấy A dù caller xin 200. Đây là suy luận tĩnh, chưa phải regression đã chạy; sửa cần predicate trước candidate selection hoặc pagination có giới hạn rõ. [16][17]

**Fallback không có nghĩa một score tin cậy.** Nhánh FTS dùng score tổng hợp 1.0 để mở rộng graph rồi đổi sang -1.0 khi hiển thị, chủ động tránh giả mạo confidence. Tuy nhiên lỗi FTS bên trong hybrid được bỏ qua bởi `if let Ok`, còn lỗi embedding/hybrid ở MCP không được trả thành structured degraded reason. Harness nên trả `mode` và `warnings` độc lập với kết quả. [16][17]

**Recall có side effect**: auto-decay và access-count updates làm đọc bộ nhớ khác với đọc evidence bất biến. Không gọi MCP recall trên canonical event store khi muốn replay định danh. [17]

**Near-duplicate merge có thể làm embedding không khớp summary**: nhánh merge đặt `summary=merge_summaries(existing,content)` nhưng `embedding=Some(query_emb.clone())`, trong khi query_emb được tính từ memory mới trước merge. Cần re-embed summary đã merge hoặc giữ embeddings theo phiên bản evidence; không coi hai phát biểu gần nhau là cùng một sự kiện. [21]

**Backref có thể bất đối xứng**: forward links trước store, backref sau store và chỉ log nếu lỗi. Dùng quan hệ đó như gợi ý truy hồi, không dùng làm cạnh “phải hoàn thành trước” của scheduler. [41]

**Đổi dimension tự hủy index vector cũ**: schema mở DB thấy dimension đổi thì transaction drop `vec_memories`, NULL embeddings và tạo lại bảng. CLI có cơ chế đọc dimension lưu sẵn khi không có embedder, nhưng đổi model thật vẫn cần kế hoạch re-index; hai model cùng dimension cũng không được phân biệt chỉ bởi dimension. [18][43]

Test đã đọc: store/recall roundtrip MCP không embedder; starvation test tạo năm noise và một target rồi gọi recall với `None`; test SQLite sanitize Unicode, giới hạn token và escape `%`/`_`. Chúng chứng minh ý định kiểm tra FTS/API; không chứng minh hybrid vượt pool 20 hay chất lượng semantic bằng model thật. [20][44]

**Quyết định:** học contract và transaction; không cài nguyên hệ thống hooks/memory nudge vào harness, không nhập global preferences làm quyền truy cập mặc định. Bộ nhớ tác nhân là view dẫn xuất với provenance, không phải task ledger.

## 6. context-mode

### Phạm vi và luồng end-to-end

Đã đọc MCP index/search handlers, ContentStore schema/chunk insertion/RRF/fuzzy/refresh, unified search, worker pool, executor timeout, và tests store/staleness/flood guard. Chưa kiểm toán toàn bộ hook, adapter Codex, security subsystem hay session recovery.

Luồng index → search: `ctx_index` handler gọi `store.index` với attribution → chọn content hoặc đọc file, chia Markdown, tính SHA-256 khi có path → transaction xóa source trùng label và insert vào cả hai FTS5 `porter unicode61` + `trigram` → trả số section → `ctx_search` tính scope và quota → `searchWithFallback` tự refresh nguồn cũ → hai truy vấn OR có BM25, RRF K=60 → filter session → proximity/title rerank → chỉ khi chưa có kết quả mới fuzzy-correct rồi chạy lại → MCP format/cap đầu ra. [23][24][25][28][29][48]

Luồng timeline: `searchAllSources` luôn lấy ContentStore, nhưng chỉ khi `sort='timeline'` mới thêm SessionDB và auto-memory; bắt lỗi từng nguồn, chuẩn hóa timestamp, sort tăng dần và slice limit. Vì thế “unified” không có nghĩa chế độ relevance tìm trên cả ba nguồn. Timestamp thiếu của ContentStore được thay bằng thời điểm gọi search, không phải thời gian sự kiện gốc. [26]

### Cơ chế nên dùng

FTS cục bộ không cần model download phù hợp khởi động offline. Chunk văn bản, source hash, transaction replace và RRF là các cơ chế nhỏ có thể chuyển sang Rust/SQLite. Giữ `matchLayer` như `rrf`/`rrf-fuzzy` để phân biệt cách match; không gọi rank là xác suất đúng. [23][24][25]

Flood guard đã tách quota theo actor, test pure có 10 subagents × 2 calls không bị quota của một agent khác chặn. Đây là bài học trực tiếp cho swarm cùng tài khoản. Worker pool bảo toàn thứ tự input và kết quả fulfilled/rejected, giới hạn công việc cùng lúc; chỉ học mẫu cho indexing/IO trong harness, không dựng scheduler agent cạnh tranh native runtime. [29][30][31]

### Gaps và hành vi cần sửa khi áp dụng

**Scope có thể fail-open**: nếu `projectScope` có giá trị nhưng SessionDB không mở được hoặc lookup session thất bại, `allowSet` vẫn undefined và ContentStore không lọc. Legacy empty-session vẫn được công khai xuyên dự án theo chủ ý code. Đây không thể dùng như isolation boundary cho task/private repo. [25][26][29]

**Refresh mất attribution**: nguồn có session A được refresh bằng `this.index({content,path,source})` không truyền attribution. Insert thay source cũ và viết session/event thành chuỗi rỗng; session filter cho phép `sid==''`. Chuỗi này có thể khiến dữ liệu A xuất hiện khi tìm dự án B trong shared store. Cần regression thực trước phát hành; báo cáo không gọi đây là exploit đã chạy. [23][25][27]

**Identity dựa label/title dễ va chạm**: insert xóa mọi source cùng label; RRF dùng `${source}::${title}` làm khóa hợp nhất thay vì chunk ID. Hai task đặt label `build-output` có thể ghi đè; hai chunk cùng source/title có thể bị gộp. Harness cần `project/revision/artifact/chunk` identity, còn label chỉ để hiển thị. [23][24]

**Relevance và quota khác mô tả đơn giản**: handler bình thường `Math.min(limit,2)`, sau soft cap còn 1; hard block mặc định sau 8 calls trong cửa sổ 60 giây. Actor không resolve được rơi vào `__default__`, có thể gộp subagent. Cần truyền actor ID từ bridge native thay vì suy đoán qua cwd/session hiện tại. [29]

**Refresh không chứng minh dữ liệu đang tồn tại/được phép đọc**: file deleted giữ cache; deny-check chỉ bỏ re-read; mtime phải lớn hơn indexed_at mới hash lại. Chuyển policy hoặc mtime không tăng có thể giữ nội dung cũ; cần revoke/tombstone và trạng thái stale riêng. Không khẳng định policy check ở tầng khác cũng bị thiếu vì chưa audit toàn security path. [25][27]

**Executor “background timeout” không phải task hoàn thành**: nó trả `exitCode:0`, `timedOut:true`, `backgrounded:true` khi process vẫn chạy. Khi không truyền timeout thì không đặt timer ở tầng này. Vì vậy adapter đọc riêng exitCode sẽ báo thành công sai; cần trạng thái Running/TimedOut/Completed. `runPool` có concurrency cap nhưng không nhận cancellation/deadline; callback `onSettled` ném lỗi nằm ngoài catch của job có thể làm worker dừng. Không coi pool này là bounded swarm hoàn chỉnh. [30][32]

Test đã đọc: wildcard source escaping qua Porter/trigram/fallback; file thay đổi thì refresh và nội dung cũ biến mất; flood guard kiểm tra độc lập bằng synthetic actor IDs, không mở MCP transport. Các test được đọc không xác minh native Codex actor attribution, refresh giữ project scope hay collision giữa tác nhân. [31][33][45]

**Quyết định:** áp dụng chọn lọc retrieval pipeline và test patterns; không nhập toàn executor/hooks thành quyền thực thi mới. Bắt buộc identity/scope tường minh và output envelope trước tích hợp.

## 7. LLMRouter

### Phạm vi và luồng end-to-end

Đã đọc CLI `route_query`, KNN constructor/single/batch, model loader, Longformer embedding và LiteLLM API caller; test KNN inference, RACER replay/validation và mock HTTP tool-call. Không audit tất cả router, trainer hoặc OpenClaw server.

Luồng KNN đầy đủ: constructor kế thừa MetaRouter, lấy routing train data và chọn model có performance cao nhất mỗi query để dựng dữ liệu → `route_single` tải checkpoint → `get_longformer_embedding` lazy-load tokenizer/model, tokenize tối đa 4096, mean-pool theo attention mask → KNN predict model name → CLI `route_query` trả quyết định hoặc structured error. Đường `route_batch` tự route từng query, định dạng prompt, chọn API endpoint/service → `call_api` parse khóa, chọn round-robin, gọi LiteLLM completion → lấy response/usage hoặc estimate tokens → batch trả response, token counts, success và metric khi có ground truth. [34][35][36][37][38]

### Cơ chế có thể học và thứ nên tránh

Tách quyết định khỏi thực thi ở `route_query` là mẫu cho **chọn retrieval strategy hoặc vai trò task**, không phải lý do giữ model routing. RACER tests cho thấy cách pin seed, giữ RNG độc lập, kiểm tra embedding hữu hạn, model chưa train, candidate order checkpoint và expected replay; có thể chuyển thành test task-policy deterministic của harness. Không cần tải PyTorch/Longformer để học contract này. [34][39]

Không dùng `route_batch` hay OpenClaw proxy làm executor của swarm một tài khoản. Những đường đó có API endpoint, service và key-selection độc lập, không cung cấp bằng chứng về native Codex subagent lifecycle. Chúng tăng thêm auth, latency và lỗi có thể tránh trong thiết kế đã chọn. [35][36]

### Fallback/mocked/misleading behavior

- KNN docstring nói thiếu trained model có thể random fallback; code `load_model` ném `FileNotFoundError`, không thấy random fallback trên luồng đã trace. `import random` không chứng minh hành vi. [35][37]
- Prompt formatting lỗi thì dùng query gốc; đây là thay đổi semantics thực của input. [35]
- Thiếu usage thì tự đếm bằng `_count_tokens`; các field output không tự phân biệt measured/estimated trong đoạn đã đọc. API lỗi trả một chuỗi `API Error: ...` ở trường response kèm `error` và số token 0. Consumer phải kiểm tra error, không index chuỗi lỗi như câu trả lời hay coi 0 là chi phí đo được. [36]
- Model initialization có `from_pretrained`, nên lần chạy đầu có thể cần model cache/network. Checkpoint pickle được deserialize bằng `pickle.load`; chỉ dùng artifact do mình kiểm soát nếu từng làm spike. Audit này không import/chạy chúng. [37][38]
- `tests/inference_test/test_knnrouter.py` là script main gọi batch thật và in kết quả, không có assertions về chất lượng; còn thông báo init ghi nhầm `LargestLLM`. Không gọi tập này là offline unit suite. Test HTTP dùng `MockResponse`, `RecordingAsyncClient`, mock model và URL example.test; chỉ kiểm tra dạng request/response, không chứng minh provider thật thực thi tool. RACER test dùng fixture và policy được initialize, không phải evaluation model đã train ngoài thực tế. [39][46][47]

**Quyết định:** loại LLMRouter khỏi runtime/dependency bắt buộc; giữ các ý tưởng về contract, replay, provenance usage và test deterministic. Không tạo task “multi-account model routing” trong kế hoạch mới.

## 8. Hợp đồng tích hợp đề xuất

Các contract sau là đề xuất của audit, chưa tồn tại như một API thống nhất trong năm repo.

| Contract | Trường và invariant cần có | Nguồn bài học |
|---|---|---|
| Artifact/chunk | `project_id`, `repo_id`, `revision`, `artifact_id`, `content_hash`, `chunk_id`, path và line/byte span; label không làm khóa | Label replace/RRF collision [23][24] |
| Ingest | Idempotency key, expected revision, per-item status, `index_generation`; chỉ publish khi transaction/generation hoàn tất | zvec partial writes [3][4]; SQLite savepoint [13] |
| Search request | Scope bắt buộc, query, `final_k`, `candidate_budget`, `max_bytes`, deadline, allowed generations; có exact/lexical/hybrid mode | Pool starvation [16][17]; streaming khác top-k [9][11] |
| Search response | Stable hit ID, raw score + score kind/direction, match layer, provenance, stale flag, `partial`, warnings, effective limit, số candidate đã xét | Synthetic score [17], quota [29], lỗi/usage [36] |
| Embedding space | Model ID/version/hash, dimension, metric, normalization, query/document prefix; namespace theo toàn bộ space ID | `Embedder` [42], migration chỉ dimension [18] |
| Memory/graph | Summary dẫn xuất trỏ evidence; cạnh `similar_to` khác `depends_on`; vòng đời task lấy từ event native, không từ score vector | Best-effort backrefs [41] |
| Native execution bridge | `run_id`, `task_id`, `native_agent_id`, parent, trạng thái và result reference; idempotent event ingestion; không có API-key pool | Executor background [32], quota actor [29] |
| Budget | Giới hạn native agents đang active, tổng nhiệm vụ, deadline, retries, tool/output bytes; số token thực đo nullable và tagged estimated | Pool chỉ cap inflight [30], usage estimate [36] |

Rust harness nên sở hữu một dịch vụ retrieval cục bộ, cho phép chạy blocking SQLite/vector work ngoài vòng render TUI và ngoài futures xử lý sự kiện. Scope do caller đáng tin cậy cấp, không suy từ tên topic hoặc text prompt. Index và memory được sửa qua command có transaction; worker đọc kết quả với generation rõ. Cơ chế giới hạn việc spawn/wait/cancel đặt ở bridge tới native runtime và graph policy đã có; không thêm queue/scheduler độc lập từ các kho này.

Mất model/cache phải trả lexical mode có cảnh báo rõ. Mất scope resolver phải trả lỗi có cấu trúc hoặc tập rỗng có lý do, không tìm toàn cục. Retrieval chỉ cung cấp nội dung dữ liệu; văn bản tìm được không được trở thành quyền gọi tool hay chỉ thị cho subagent.

### Luồng triển khai có giới hạn

Đây là luồng đề xuất để nhóm chính ghép với audit Codex native đã có; tên thao tác dưới đây là khái niệm, không khẳng định tên API của Codex. Các ngưỡng là cấu hình của run do harness chốt trước, không lấy số mặc định của repo làm cam kết sản phẩm.

1. **Ingest một artifact:** xác nhận scope/revision → giới hạn byte đầu vào và số chunk → hash/chunk → ghi canonical artifact và ingestion intent trong transaction → xử lý index bằng số worker IO cố định → thu kết quả từng item → publish generation nếu đủ điều kiện. Retry chỉ item chưa commit, tối đa `max_index_retries`; hết hạn giữ artifact và đánh dấu index pending/failed, không mất provenance. Áp dụng bài học partial writes và transaction replace. [3][13][23]
2. **Phục vụ một lần retrieval:** nhận native actor ID và scope từ bridge → kiểm tra quota actor/toàn run → chọn generation → lexical search với predicate phạm vi → optional vector cùng embedding space → fusion theo chunk ID → mở rộng tối đa một hop, với `max_neighbors` và filter scope lần nữa → serialize dưới `max_bytes` → trả hits cùng partial/degraded reason. Mỗi nhánh chỉ dùng `candidate_budget`, timeout cố định của request và tối đa một fallback lexical; không lặp đến vô hạn khi không có hit. [16][17][24][29]
3. **Một lượt native swarm:** đọc các node ready từ graph hiện có → cấp context theo retrieval budget → dispatch không quá `max_active_native_agents` qua native bridge → nhận sự kiện tiến độ/kết thúc → commit event và result reference idempotent → cập nhật dependency của graph → mới xét node tiếp theo. `max_total_tasks`, `max_attempts_per_task`, deadline toàn run và quyền đã cấp chặn dispatch mới. Hết ngân sách không spawn thêm; wait/cancel/reconcile phải theo contract native đã xác minh ở audit runtime, không dùng process background exitCode để thay thế completion event. [30][32]
4. **Refresh/recovery:** kiểm tra content hash/revision, giữ nguyên owner/scope → tạo generation mới thay vì đổi identity → tombstone nguồn xóa/revoke → publish atomic. Khi khởi động lại, chỉ replay ingestion intent chưa hoàn tất và native events chưa commit; không chạy lại task đã terminal. Rebuild vector là công việc có giới hạn độc lập với lifecycle tác nhân, không tự tải model hay chuyển tài khoản để giải quyết thiếu dependency. [18][23][25][27]

Mỗi luồng dừng ở một artifact, một retrieval request, một lượt dispatch hoặc một batch recovery đã giới hạn. Không yêu cầu một hệ thống scheduler mới để thực hiện các bước này.

## 9. Công việc cụ thể để đưa vào kế hoạch và acceptance tests

Ưu tiên là đề xuất triển khai; tất cả acceptance tests dưới đây **chưa chạy**. Dùng corpus cố định offline trước, sau đó mới benchmark quy mô. Không cần tài khoản live để xác nhận các invariant storage/policy.

| ID | Ưu tiên / công việc | Điều kiện nghiệm thu cụ thể |
|---|---|---|
| R01 | P0 — Chốt ranh giới một tài khoản/native agents | Config mặc định không yêu cầu model router, provider key hay account pool. Fake native adapter phát spawn/result/cancel; graph ghi đúng native ID, không tạo second executor. Việc xác minh API native thật thuộc nghiên cứu runtime khác, không được test fake thay thế. |
| R02 | P0 — Artifact identity/provenance | Hai task cùng label `build-output`, hai file cùng heading và hai revision cùng path đều truy hồi riêng được. Mọi hit mở được đúng artifact hash/span. Không mất chunk khi RRF dedup. |
| R03 | P0 — Scope fail-closed và refresh | A/B có nội dung cùng từ khóa; query A không trả B. Lặp sau refresh A, mất SessionDB, lookup lỗi và source legacy không session. Refresh giữ nguyên owner/scope; dữ liệu legacy cần migration scope tường minh. |
| R04 | P0 — Filter trước top-k hoặc bounded pagination | Tạo ít nhất 50 noise của B xếp cao hơn một target A ở FTS và vector. Query A vẫn tìm target; hoặc trả `partial`/budget-exhausted rõ, không “không có dữ liệu”. Không chỉ dùng fixture năm noise và embedder None như test ICM đã đọc. |
| R05 | P0 — Trạng thái thực thi và cancellation | Background trả exitCode 0 + timedOut vẫn là Running/TimedOut, không Completed. Cancel/timeout ngăn dispatch thêm, kết quả trễ không hồi sinh task terminal; mỗi attempt chỉ commit một lần. Tiến độ/telemetry lỗi không được làm mất result hoặc bỏ trống task. |
| R06 | P0 — Ingest transaction/generation | Batch sai dimension bị reject rõ trước ghi nếu backend validate toàn batch; fixture lỗi từng item trong vòng ghi phải trả status riêng/partial. Generation chưa hoàn chỉnh không được publish. Crash giữa commit canonical artifact và index update: reopen/replay idempotent, không lặp chunk và không mất artifact. |
| R07 | P0 — Output/usage contract | Search một kết quả quá lớn hoặc batch nhiều query vẫn nằm trong byte budget sau serialize, không chỉ kiểm tra cap trước vòng lặp. Usage thiếu là unknown/estimated; API error text không được lưu như successful answer. |
| R08 | P1 — Lexical retrieval baseline | Fixture Việt có dấu/không dấu, identifier `snake_case`, `%`, `_`, đường dẫn, quote, câu dài, heading trùng; golden expected IDs. Không dùng rank âm/dương lẫn lộn. Kết quả vẫn dùng được khi không có model/vector backend. |
| R09 | P1 — Embedding namespace và memory merge | Hai model cùng dimension bị tách namespace; model khác dimension không tự phá index đang phục vụ. Summary merge được re-embed theo nội dung cuối; embedding failure có degraded reason và job tái tạo. |
| R10 | P1 — sqlite-vector spike có exact oracle | So sánh full-scan bốn tham số với stream `ORDER BY distance LIMIT k` trên fixture có ties. Chứng minh stream `LIMIT k` không được dùng làm nearest. Test NULL, short/oversized BLOB, NaN/Inf, dimension sai, metric direction và câu query nhiều scope. |
| R11 | P1 — Quantization invalidation | Quantize/preload → insert/update/delete → truy vấn hai connection → outer transaction rollback → reopen. Adapter phát hiện stale generation và rebuild hoặc exact fallback; đánh giá recall@k so exact oracle, báo RAM/latency riêng, không lấy marketing làm ngưỡng. |
| R12 | P1 — Quota toàn run và riêng actor | 10 native actor IDs × 2 searches không bị quota của một actor chặn; một actor spam không ảnh hưởng actor khác. Đồng thời tổng inflight và byte/token budget toàn run vẫn bị chặn. Thiếu actor ID báo lỗi/giới hạn rõ, không âm thầm gộp mọi actor. |
| R13 | P2 — zvec C ABI/durability spike | Wrapper tạo/query/free qua C ABI với leak/double-free checks; mixed-success batch và retry idempotent. Fault injection write/flush/CRC/reopen; document durability contract trước khi dùng ACK. Kiểm thử query trong concurrent batch với generation của harness. |
| R14 | P2 — Evidence-aware graph expansion | One-hop `similar_to` đúng budget, không vượt scope; backref thiếu không làm task chạy sớm. Xóa/revoke artifact cập nhật tombstone; replay task graph giữ nguyên dù memory access_count hoặc rank đổi. |

Không đưa benchmark hoặc test live vào R01–R09. Khi benchmark R10–R13, ghi hardware, corpus hash, embedding space, số documents, k/candidate budget, p50/p95, peak RSS, index size, thời gian build/rebuild và recall@k so exact oracle. Chưa có số liệu audit nào cho phép đặt cam kết tốc độ hoặc tỷ lệ tiết kiệm token.

## 10. Mức tin cậy và giới hạn

Độ tin cậy cao cho hành vi nhánh code/SQL được dẫn dòng; trung bình cho lỗi tích hợp suy ra giữa nhiều lớp, vì chưa chạy tái hiện. Chưa xác minh ABI portability, crash durability toàn engine, hiệu quả semantic trên code, runtime Codex native, sandbox enforcement hay policy tải model. Không kết luận repo “fake”: mock tests, vector fixture thủ công, sentinel score và runtime fallback được phân loại riêng trong từng phần.

Báo cáo đưa ra các cơ chế có thể tái dùng và điều kiện chấp nhận chúng. Không có thay đổi nào trong năm repo tham chiếu, không chỉnh PLAN.md, không gửi dữ liệu ra ngoài. Artifact duy nhất được ghi là báo cáo này.

## 11. Danh mục nguồn cục bộ có đánh số

Mỗi mục là nguồn đã đọc, với dòng neo và vùng nội dung liên quan; số dòng trong phần mô tả có thể là một khoảng, link mở tại dòng đầu. Liên kết áp dụng cho HEAD ở mục 2.

1. [icm/AGENTS.md:1][1] — bắt buộc recall/store theo hướng dẫn repo.
2. [zvec/python/zvec/executor/query_executor.py:120][2] — dispatch, native/Python reranker, candidate count, dòng 120–210.
3. [zvec/src/binding/c/c_api.cc:6956][3] — insert/count và per-item results 6956–7026; query/multi-query 7396–7465.
4. [zvec/src/db/collection.cc:1543][4] — insert/upsert và vòng `write_impl` đến 1650.
5. [zvec/src/db/collection.cc:1773][5] — write-view, schema snapshot và multi-query đến 1951.
6. [zvec/src/db/index/storage/wal/local_wal_file.cc:29][6] — CRC, auto-flush error, EOF/read-error sentinel đến 67.
7. [zvec/python/tests/test_collection_fts_vector_hybrid.py:40][7] — collection fixture và hybrid assertions đến 249.
8. [zvec/python/tests/test_query_executor.py:17][8] — mock imports/schema và wrapper test.
9. [sqlite-vector/src/sqlite-vector.c:2602][9] — query type/length validation, streaming/top-k branch, missing quant table, đến 2765.
10. [sqlite-vector/src/sqlite-vector.c:3126][10] — heap sort và toàn bộ full scan đến 3196; streaming chuẩn bị từ 3503.
11. [sqlite-vector/src/sqlite-vector.c:2798][11] — virtual-table planner và ordering contract đến 2850.
12. [sqlite-vector/src/sqlite-vector.c:3780][12] — init validation, extension entrypoint 3853 và registration 3944–3953.
13. [sqlite-vector/src/sqlite-vector.c:2232][13] — quantization rebuild/savepoint/rollback/preload đến 2330.
14. [sqlite-vector/test/test_vector.c:225][14] — full-scan top-k/stream assertions đến 301.
15. [sqlite-vector/test/test_vector.c:441][15] — preload, streaming, qbits, WITHOUT ROWID, corrupt blob đến 507.
16. [icm/crates/icm-store/src/store/memory.rs:261][16] — FTS, vec0 KNN, hybrid candidate pool và scoring đến 446.
17. [icm/crates/icm-mcp/src/tools.rs:1242][17] — recall/filters/expansion/fallback/side effects đến 1404.
18. [icm/crates/icm-store/src/schema.rs:17][18] — vec0 cosine/dimension; dimension migration tại 613–638.
19. [icm/crates/icm-mcp/src/server.rs:330][19] — JSON-RPC tools/call dispatch đến 363.
20. [icm/crates/icm-mcp/src/tools.rs:2739][20] — no-embedder roundtrip; starvation fixture từ 2839.
21. [icm/crates/icm-mcp/src/tools.rs:984][21] — store validation/embedding/dedup merge đến 1105.
22. [icm/crates/icm-store/src/store/memory.rs:11][22] — transaction store; update validation/vector-sync từ 51.
23. [context-mode/src/store.ts:838][23] — index/chunk/hash; insert transaction/default attribution 1032–1082.
24. [context-mode/src/store.ts:1244][24] — RRF key/candidate merge, proximity rerank đến 1335.
25. [context-mode/src/store.ts:1340][25] — fallback, session filter, stale checks đến 1431.
26. [context-mode/src/search/unified.ts:70][26] — scope resolution, partial-source handling, timeline/default behavior đến 175.
27. [context-mode/src/store.ts:1433][27] — refresh gọi index không attribution và catch đến 1457.
28. [context-mode/src/server.ts:2417][28] — MCP index gọi store và response/error đến 2434.
29. [context-mode/src/server.ts:2457][29] — actor bucket; search scope/quota/limits/backend calls 2632–2735.
30. [context-mode/src/runPool.ts:42][30] — cap concurrency, settled result, callback và worker lifetime đến 80.
31. [context-mode/tests/core/search-flood-guard.test.ts:17][31] — pure unit test, fake actor identity và quota scenarios đến 100.
32. [context-mode/src/executor.ts:470][32] — absent timeout và background response/kill đến 508.
33. [context-mode/tests/stale-detection.test.ts:54][33] — file hash và file mutation refresh đến 120.
34. [LLMRouter/llmrouter/cli/router_inference.py:207][34] — route-only decision/error contract đến 274.
35. [LLMRouter/llmrouter/models/knnrouter/router.py:13][35] — docstring/constructor/single route; batch API path đến 245.
36. [LLMRouter/llmrouter/utils/api_calling.py:310][36] — API keys, LiteLLM, measured/estimated tokens, errors đến 427.
37. [LLMRouter/llmrouter/utils/model_loader.py:55][37] — missing checkpoint error, pickle/torch load đến 106.
38. [LLMRouter/llmrouter/utils/embeddings.py:43][38] — lazy pretrained model, truncation, mean pooling đến 107.
39. [LLMRouter/tests/test_racerrouter_inference.py:10][39] — synthetic ready policy, RNG, validation, replay đến 75.
40. [zvec/tests/db/index/storage/wal_file_test.cc:43][40] — real file lifecycle test, flush/reopen đến 100.
41. [icm/crates/icm-mcp/src/tools.rs:1112][41] — auto-link, store, best-effort backrefs, auto-consolidation đến 1153.
42. [icm/crates/icm-core/src/embedder.rs:3][42] — query/document distinction, batch, dimensions.
43. [icm/crates/icm-cli/src/main.rs:1782][43] — resolve persisted embedding dims/no-embeddings behavior.
44. [icm/crates/icm-store/src/store/tests/search.rs:5][44] — sanitize, Unicode, caps, wildcard regression đến 145.
45. [context-mode/tests/store.test.ts:818][45] — source LIKE and wildcard escaping across search paths đến 874.
46. [LLMRouter/tests/inference_test/test_knnrouter.py:6][46] — main script, live-capable batch and console output đến 40.
47. [LLMRouter/tests/test_openclaw_http_tool_calls.py:11][47] — MockResponse/RecordingAsyncClient/test configuration đến 100.
48. [context-mode/src/store.ts:463][48] — FTS5 Porter/trigram schema và attribution columns đến 501.

[1]: /home/minh/projects/outsource/icm/AGENTS.md:1
[2]: /home/minh/projects/outsource/zvec/python/zvec/executor/query_executor.py:120
[3]: /home/minh/projects/outsource/zvec/src/binding/c/c_api.cc:6956
[4]: /home/minh/projects/outsource/zvec/src/db/collection.cc:1543
[5]: /home/minh/projects/outsource/zvec/src/db/collection.cc:1773
[6]: /home/minh/projects/outsource/zvec/src/db/index/storage/wal/local_wal_file.cc:29
[7]: /home/minh/projects/outsource/zvec/python/tests/test_collection_fts_vector_hybrid.py:40
[8]: /home/minh/projects/outsource/zvec/python/tests/test_query_executor.py:17
[9]: /home/minh/projects/outsource/sqlite-vector/src/sqlite-vector.c:2602
[10]: /home/minh/projects/outsource/sqlite-vector/src/sqlite-vector.c:3126
[11]: /home/minh/projects/outsource/sqlite-vector/src/sqlite-vector.c:2798
[12]: /home/minh/projects/outsource/sqlite-vector/src/sqlite-vector.c:3780
[13]: /home/minh/projects/outsource/sqlite-vector/src/sqlite-vector.c:2232
[14]: /home/minh/projects/outsource/sqlite-vector/test/test_vector.c:225
[15]: /home/minh/projects/outsource/sqlite-vector/test/test_vector.c:441
[16]: /home/minh/projects/outsource/icm/crates/icm-store/src/store/memory.rs:261
[17]: /home/minh/projects/outsource/icm/crates/icm-mcp/src/tools.rs:1242
[18]: /home/minh/projects/outsource/icm/crates/icm-store/src/schema.rs:17
[19]: /home/minh/projects/outsource/icm/crates/icm-mcp/src/server.rs:330
[20]: /home/minh/projects/outsource/icm/crates/icm-mcp/src/tools.rs:2739
[21]: /home/minh/projects/outsource/icm/crates/icm-mcp/src/tools.rs:984
[22]: /home/minh/projects/outsource/icm/crates/icm-store/src/store/memory.rs:11
[23]: /home/minh/projects/outsource/context-mode/src/store.ts:838
[24]: /home/minh/projects/outsource/context-mode/src/store.ts:1244
[25]: /home/minh/projects/outsource/context-mode/src/store.ts:1340
[26]: /home/minh/projects/outsource/context-mode/src/search/unified.ts:70
[27]: /home/minh/projects/outsource/context-mode/src/store.ts:1433
[28]: /home/minh/projects/outsource/context-mode/src/server.ts:2417
[29]: /home/minh/projects/outsource/context-mode/src/server.ts:2457
[30]: /home/minh/projects/outsource/context-mode/src/runPool.ts:42
[31]: /home/minh/projects/outsource/context-mode/tests/core/search-flood-guard.test.ts:17
[32]: /home/minh/projects/outsource/context-mode/src/executor.ts:470
[33]: /home/minh/projects/outsource/context-mode/tests/stale-detection.test.ts:54
[34]: /home/minh/projects/outsource/LLMRouter/llmrouter/cli/router_inference.py:207
[35]: /home/minh/projects/outsource/LLMRouter/llmrouter/models/knnrouter/router.py:13
[36]: /home/minh/projects/outsource/LLMRouter/llmrouter/utils/api_calling.py:310
[37]: /home/minh/projects/outsource/LLMRouter/llmrouter/utils/model_loader.py:55
[38]: /home/minh/projects/outsource/LLMRouter/llmrouter/utils/embeddings.py:43
[39]: /home/minh/projects/outsource/LLMRouter/tests/test_racerrouter_inference.py:10
[40]: /home/minh/projects/outsource/zvec/tests/db/index/storage/wal_file_test.cc:43
[41]: /home/minh/projects/outsource/icm/crates/icm-mcp/src/tools.rs:1112
[42]: /home/minh/projects/outsource/icm/crates/icm-core/src/embedder.rs:3
[43]: /home/minh/projects/outsource/icm/crates/icm-cli/src/main.rs:1782
[44]: /home/minh/projects/outsource/icm/crates/icm-store/src/store/tests/search.rs:5
[45]: /home/minh/projects/outsource/context-mode/tests/store.test.ts:818
[46]: /home/minh/projects/outsource/LLMRouter/tests/inference_test/test_knnrouter.py:6
[47]: /home/minh/projects/outsource/LLMRouter/tests/test_openclaw_http_tool_calls.py:11
[48]: /home/minh/projects/outsource/context-mode/src/store.ts:463
