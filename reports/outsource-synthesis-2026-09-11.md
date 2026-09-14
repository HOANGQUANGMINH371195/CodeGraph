# Thiết kế single-account native swarm từ 22 repository

## Quyết định chính

Project Graph Harness là sản phẩm terminal-native hoàn chỉnh: một phiên Codex
điều phối các subagent native cùng authority tài khoản, mặc định kế thừa model.
Harness bổ sung graph context, giao việc có giới hạn, quyền ghi và cổng kiểm
chứng; không dựng executor LLM thứ hai hay chia credential giữa worker.
Giữ phạm vi W0–W13, embedding, phân tích sâu, browser, sandbox và trải nghiệm
terminal. Loại bỏ sự bắt buộc của account pool, cheap-model routing và proxy
Responses riêng. Đây là quyết định kiến trúc, chưa phải runtime đã triển khai.
[1][2][5]

Đã khảo sát các luồng thực thi và test liên quan ở **cả 22 repo**, không có nghĩa
đã đọc mọi file hay chứng minh mọi đường chạy. Năm báo cáo nguồn ghi revision,
phạm vi, nhánh fallback, test đã đọc và các khoảng trống. Đợt audit này không
chạy build/test của các repo tham khảo. Các rủi ro suy từ mã cần fixture tái hiện;
test mock, test bị skip và test có tên live không tự chứng minh runtime thật.
CodeGraph có thay đổi cục bộ sẵn có, nên HEAD riêng không định danh đủ snapshot.
[1–5]

## Luồng sản phẩm đích

```text
Yêu cầu người dùng + quyền đã cấp + snapshot dự án
  → root Codex: xác định việc, đường găng và các khoảng trống hiểu biết
  → harness: task DAG + admission + context packets theo scope
  → native Codex: spawn / send / observe / resume / close
       ├─ explorer: tìm luồng và bằng chứng còn thiếu
       ├─ implementer: sửa trong write scope/worktree được giao
       └─ critic: kiểm tra giả thuyết, diff và điều kiện nghiệm thu
  → kết quả ứng viên + source spans + artifact hashes + execution receipts
  → kiểm chứng độc lập + giải quyết mâu thuẫn
  → một GraphWriter xuất bản facts/delta theo snapshot
  → context mới cho root và những task thực sự bị ảnh hưởng
```

Codex sở hữu cây agent và vòng đời thread. DAG công việc không đồng nhất với
cây agent: một child có thể làm nhiều attempt, một task có thể cần nhiều lượt
kiểm chứng. `Done` chỉ là quan sát runtime; dependency chỉ được mở khi kết quả
đạt cổng kiểm chứng. Native thread fork không tạo worktree cách ly. [1][2]

Tự chủ nghĩa là root chủ động chia công việc trong phạm vi được giao, không
phải sinh worker không giới hạn. Mặc định thiết kế ban đầu là tối đa bốn child
hoạt động, budget chung cho toàn cây, giới hạn depth/tổng lượt spawn, chống
trùng việc và thời hạn toàn run. Các con số là policy của harness, không phải
quota dịch vụ Codex. Thiếu hook enforce phải báo capability gap; prompt hay
observer không được gắn nhãn hard enforcement. [1][2][5]

## Ma trận học hỏi và loại bỏ

| Repo | Cơ chế nên lấy | Ranh giới / điều cần kiểm chứng |
|---|---|---|
| codex | AgentControl, authority inheritance, child IDs, proactive mode seam | Probe protocol và effective mode; không invent agent/spawn RPC [1] |
| opendev | Rust terminal UX, tiến độ theo instance, nhóm tool đọc song song | Không port executor LLM; kiểm cancellation token, resume và lifetime slots [2] |
| orca | Quan sát fleet, lineage, polling và reconnect | Transcript có thể thiếu; child biến mất không phải thành công [2] |
| t3code | Native app-server adapter, child reducer, event/projection/receipt | collabAgent events nội bộ không phải native RPC; stop phải có deadline tổng [2] |
| ruflo | Source fingerprints, fenced scope contracts, HNSW đối chứng | Registration không phải execution; lease in-memory không phải distributed lock [2] |
| codegraph | Incremental index/rebind, provenance, context retrieval | Ghi backend/coverage thực, kiểm clean rebuild và semantic golden riêng [3] |
| joern | CFG/dataflow sâu theo use case | Timeout, method limits và unsupported frontend phải hiện partial [3] |
| codepropertygraph | Schema/pass lifecycle và import contracts | Không dùng ID cục bộ nối snapshot; ghi loss khi import [3] |
| archify | Evidence references, validation trước xuất bản artifact | Source span hợp lệ chưa chứng minh connection đúng ngữ nghĩa [3] |
| T3MP3ST | Vòng phản biện và khám phá giả thuyết bảo mật | Không lấy swarm stub hay bộ điều phối hai LLM làm runtime [3] |
| zvec | Native vector/hybrid, C ABI, candidate/reranking contracts | Partial batch, generation publication và durability cần fault injection [4] |
| sqlite-vector | Exact scan oracle và vector cùng SQLite | Streaming LIMIT không phải nearest top-k; quantization cần invalidation [4] |
| icm | Query/document embedding riêng, memory transaction và bounded expansion | Dùng sqlite-vec khác sqlite-vector; filter sau pool có thể bỏ sót [4] |
| context-mode | Chunking, FTS/RRF, output retrieval, quota theo actor | Không sao chép scope fail-open, refresh mất attribution, identity bằng label [4] |
| LLMRouter | Policy replay, tách quyết định/thực thi, usage provenance | Không đưa model router, Python ML stack hay API-key executor vào core [4] |
| browser | Headless content/CDP phục vụ nghiên cứu web | Text-raster screenshot không chứng minh CSS/layout; cần browser capability [5] |
| OpenSandbox | Lifecycle, resource ownership, execution isolation adapter | close không phải destroy; profile widening không bật ngầm [5] |
| ghostty | Terminal state, incremental rendering, parser stress cases | Không fork GUI để làm CLI; libghostty-vt cần ABI/lifetime tests [5] |
| rtk | Compact output view và giữ exit semantics | Lưu raw evidence độc lập; byte/4 không phải usage token thực [5] |
| grit | Worktree ownership và merge candidate lifecycle | Tái hiện read→write lock và retry sau nhả lock trước reuse [5] |
| temp-rs-ddd | Module boundaries, dependency inversion, repository contracts | Không copy Kafka/cloud boilerplate; health/handler/commit phải có semantics thật [5] |
| codex-multi-auth | Ledger/replay và refresh-lock như tham khảo cục bộ | Loại account routing, rotation, credential copying và proxy khỏi runtime đích [5] |

Ma trận là quyết định chọn lọc cơ chế, không cho phép sao chép code bỏ qua giấy
phép. Trước reuse phải có inventory theo package/file, commit/feature/ABI pin,
và adapter regression tương ứng. Audit source không thay legal/license gate.
[3][5]

## Hiểu dự án mà không đọc nguyên file nghìn dòng

Không có repo nào được khảo sát chứng minh khả năng hiểu toàn bộ kiến trúc
chỉ bằng một index. Thiết kế kết hợp graph nhẹ, cấu hình/build/API/deployment,
truy vấn sâu và bằng chứng runtime. CPG là mô hình dữ liệu/phân tích mà Joern
sử dụng và mở rộng, không phải sản phẩm thay thế Joern để tự giải quyết mọi
quan hệ hệ thống. Diagram là một projection có chứng cứ, không là bằng chứng
rằng tác giả diagram đã hiểu đúng mọi luồng. [3]

ContextBroker trả tầng orientation → module → symbol → source span, kèm
callers/callees, cấu hình nối luồng, provenance, freshness và gaps. Khi gặp
dynamic dispatch, reflection, code sinh hoặc thông tin chưa index, child được
giao một câu hỏi cụ thể với ngân sách đọc bổ sung. Kết quả âm phải phân biệt
“không tìm thấy trong phạm vi đã xét” với “không tồn tại”. Đọc nhiều source vẫn
có thể cần thiết khi thiếu bằng chứng; mục tiêu là không đọc cả file mặc định,
không phải cấm đọc source bằng mọi giá. [3][4]

Swarm cải thiện graph bằng cách đề xuất facts và các khoảng trống cần truy
tiếp. Mỗi đề xuất gắn snapshot, span/hash, phương pháp và trạng thái kiểm chứng.
Critic kiểm nguồn và phản ví dụ; đa số agent đồng ý không biến suy luận thành
fact. GraphWriter tích hợp có transaction và invalidation; embedding similarity
không tạo cạnh calls/depends_on và không mở dependency công việc. [2–4]

## Storage, embedding và chất lượng truy hồi

Giữ SQLite làm canonical store cho trạng thái, provenance, migrations và
journal; vector là derived index có thể rebuild. Embedding vẫn thuộc phạm vi
sản phẩm đầy đủ. Lựa chọn backend phải dựa trên exact oracle, corpus code thực,
recall@k sau scope filtering, p50/p95, RAM, index size, build/rebuild và crash
recovery; không thể suy “mạnh hơn” từ tên HNSW hay benchmark README. [4]

Namespace embedding phải chứa model/version và cấu hình cần thiết, không chỉ
dimension. Filter phải nằm trước chọn top-k hoặc có pagination có giới hạn,
trả partial khi hết budget. Refresh giữ nguyên scope/owner; mất scope database
phải fail closed. Chunk ID dựa artifact/snapshot/span, label chỉ để hiển thị.
Publish generation chỉ khi xử lý đủ trạng thái từng item. [4]

Benchmark Ruflo trước đây vẫn là đối chứng synthetic riêng, không phải chứng
minh HNSW thắng zvec/sqlite-vector trên code embeddings. Đợt source audit này
không chạy lại phép đo đó. PLAN giữ benchmark nhiều backend và tiêu chí chất
lượng thay vì chốt handwritten HNSW làm lựa chọn mặc định. [6]

## Checklist tích hợp và cổng nghiệm thu

Các acceptance chi tiết trong báo cáo nguồn được ghép vào PLAN, không xem là
đã triển khai khi chỉ hoàn tất nghiên cứu:

- [x] Quyết định một account, inherited model và native lifecycle; gỡ yêu cầu
  cross-account routing khỏi kiến trúc mặc định.
- [x] Khảo sát luồng liên quan ở đủ 22 repo và lập ma trận chọn lọc.
- [ ] Native bridge: protocol probe, requested/effective mode, lineage qua
  compaction/resume, restart reconciliation và không dispatch trùng.
- [ ] Admission: budget toàn cây và active lifetime; permission không tăng;
  stop có deadline tổng, receipt và kiểm tra side effect thực sự ngừng.
- [ ] Evidence: event/projection/receipt transactional với payload digest;
  raw output có hash; usage unknown không thành 0; candidate giữ qua merge lỗi.
- [ ] Graph: coverage receipts, dirty identity, convergence oracle và semantic
  goldens; import loss/partial rõ; authored architecture không tự semantic-verified.
- [ ] Retrieval: scope fail-closed, refresh attribution, filtered recall,
  serialized byte caps, vector generation và fault-injection durability.
- [ ] Infrastructure: lock contention, bounded streams, sandbox cleanup,
  browser capability, terminal ABI/lifetime, error/health semantics và license pin.
- [ ] Đánh giá single agent so swarm 2/4/8 cùng account/model và cùng chất lượng:
  latency, uncached input, token usage thực nếu có, số lỗi và tỉ lệ nghiệm thu.

Nguồn task chi tiết: runtime §8; graph G01–G12; retrieval R01–R14;
infrastructure I-01–I-16; Codex acceptance. Các task trùng lifecycle được hợp
nhất trong P9.T11–T15; không tạo năm executor riêng theo năm báo cáo. [1–5]

## Sources

1. [Codex: source flows, proactive mode, authority và protocol](/home/minh/projects/project-graph-agent/reports/outsource-codex-audit-2026-09-11.md).
2. [Runtime: Ruflo, OpenDev, Orca và T3 Code; nguồn đánh số §9](/home/minh/projects/project-graph-agent/reports/outsource-runtime-audit-2026-09-11.md).
3. [Graph: CodeGraph, Joern, CPG, Archify và T3MP3ST; nguồn đánh số §10](/home/minh/projects/project-graph-agent/reports/outsource-graph-audit-2026-09-11.md).
4. [Retrieval: zvec, sqlite-vector, ICM, context-mode và LLMRouter; nguồn đánh số §11](/home/minh/projects/project-graph-agent/reports/outsource-retrieval-audit-2026-09-11.md).
5. [Hạ tầng: browser, OpenSandbox, Ghostty, RTK, Grit, Rust DDD và multi-auth; nguồn đánh số §12](/home/minh/projects/project-graph-agent/reports/outsource-infrastructure-audit-2026-09-11.md).
6. [Phép đo Ruflo HNSW trước đợt audit này](/home/minh/projects/project-graph-agent/reports/w8-ruflo-hnsw-2026-09-11.md).
