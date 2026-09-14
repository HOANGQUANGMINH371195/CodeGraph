# Kiểm toán nguồn graph cho harness Rust terminal-native

Ngày khảo sát: **2026-09-11**. Phạm vi được giao: `codegraph`, `joern`, `codepropertygraph`, `archify`, `T3MP3ST` dưới `/home/minh/projects/outsource`. Đây là khảo sát mã nguồn local, không phải kết quả benchmark hoặc chứng nhận chạy thành công.

## 1. Kết luận kiến trúc

Thiết kế đích là **một tài khoản Codex, dùng native subagents, swarm tự chủ có giới hạn**. Không đưa multi-account, chọn provider/model cho từng vai trò, hoặc scheduler agent của repository tham khảo thành điều kiện kiến trúc. Năm repo này cung cấp các mảnh graph, evidence và context; không repo nào trong những luồng được kiểm tra chứng minh một tích hợp native Codex subagents đã hoàn chỉnh.

Khuyến nghị chính:

1. Lấy **CodeGraph làm mẫu cho graph phục vụ truy hồi tương tác**: parse/extract tách resolve, SQLite/FTS, provenance, reference chưa resolve và sync hội tụ. Kernel Rust hiện là thành phần N-API, chưa phải thư viện Rust độc lập thay toàn bộ engine. [1–15]
2. Dùng **Joern làm backend phân tích sâu tùy chọn**, có tiến trình riêng và coverage manifest. AST/call graph không tương đương dataflow; method bị bỏ qua vì ngưỡng tính toán phải hiện thành `partial`, không được suy ra “không có đường đi”. [16–26]
3. Học **schema phân lớp và diff-pass** từ Code Property Graph; không lấy raw graph ID làm định danh bền vững xuyên snapshot, không mở file tham khảo bằng loader mặc định trong luồng chỉ đọc. Loader có chuyển đổi/ghi và đường `persistTo` có thể xóa đích trước khi xác minh nguồn. [27–33]
4. Lấy **receipt gắn revision, source span, hash và commit artifact sau kiểm tra** từ Archify. `verified` ở đây xác minh tham chiếu tồn tại, không xác minh quan hệ kiến trúc do người/agent viết. HTML viewer là export tùy chọn; TUI vẫn là mặt sử dụng chính. [34–39]
5. Từ **T3MP3ST**, chỉ học inventory context, ghi rõ phần bị bỏ, giới hạn công việc và trạng thái kết quả. Không nhập bộ điều phối hai LLM, prompt che giấu mục tiêu, hay các stub swarm/scanner làm runtime mới. [40–52]

Ranh giới đề xuất: native Codex runtime sở hữu vòng đời subagent; harness Rust sở hữu snapshot graph, truy hồi, evidence, artifact và sổ theo dõi giới hạn. Sổ này kiểm tra quyền/giới hạn và phản ánh trạng thái native, không tự dựng một hàng đợi agent/provider cạnh tranh.

## 2. Phương pháp, hướng dẫn và trạng thái checkout

Đã kiểm tra đường cha tới workspace và thư mục báo cáo để tìm `AGENTS.md`; không thấy hướng dẫn ở các đường cha được kiểm tra. Đã đọc toàn bộ `codegraph/AGENTS.md`, đọc `T3MP3ST/AGENTS.override.md`, và tìm hướng dẫn lồng trong năm repo. `codegraph/docs/AGENTS.md` tồn tại nhưng không vào phạm vi file docs được nghiên cứu. Bốn repo còn lại không có `AGENTS.md` được tìm thấy trong phạm vi này. Override của T3MP3ST nói về quy trình merge, không phát sinh hành động merge trong khảo sát. [54]

Đã sử dụng skill `rust-router`, `domain-cli`, `m07-concurrency`: đánh giá từ yêu cầu terminal và giới hạn song song xuống hợp đồng dữ liệu, thay vì chọn thread pool trước. Không gọi subagent khác. Không cần áp dụng skill tạo diagram của Archify vì công việc là audit implementation, không author diagram.

Các thao tác thực hiện: `rg`, `nl`/`sed`/`cat`, đọc manifest/LICENSE, `git rev-parse HEAD`, `git status --porcelain`, đọc `git diff --stat` và diff resolver của CodeGraph. Không build, không install dependency, không chạy server/CLI phân tích, không truy cập mạng, không gọi tài khoản/model. **Tests executed: 0 cho cả năm repo.** Test trong các phần sau là **test source đã đọc**, không phải test đã pass trong phiên này. Chỉ file báo cáo này được tạo bằng `apply_patch`; không sửa repo tham khảo hay `PLAN.md`.

| Repository | HEAD đầy đủ | Working tree lúc kiểm tra | Phạm vi thực sự đọc |
|---|---|---|---|
| codegraph | `3ed73bc127323e63153bf6ec8354afa82ce36aaf` | Dirty: 15 tracked modified, 3 untracked | index/sync, extraction store, kernel ABI/routing, resolver persistence, schema, named flow, MCP output, tests convergence/parity/render/limit |
| joern | `7c1163d96705d354d7c1957531487a63af34dda6` | Clean | parse CLI, Python generator/frontend, default overlays, reaching definitions, dataflow query engine, Python dataflow tests |
| codepropertygraph | `e7b6e8da670e4b58a64ba153d197041c25fd798a` | Clean | schema assembly/metadata, loader/Proto conversion, diff-pass lifecycle, loader/pass tests |
| archify | `18911058008f17dc065af23a2cdc9bfeff6d3f7a` | Clean | deliver/render CLI, architecture renderer, shared loader, repository evidence, brand resolution, repository-evidence tests |
| T3MP3ST | `29824d5625ede419ac8cdae418c8f4c72c6270f7` | Clean | whitebox HTTP→ingest→packing→decomposition, parse fallback, graph/reachability, stubs, tests ingest/packing/whitebox/stub honesty |

“Clean” chỉ nghĩa là Git không báo tracked/untracked thay đổi theo lệnh trên; không chứng minh dependency, ignored files hoặc binary tương ứng HEAD. Có `dist` và `node_modules` trong CodeGraph nhưng không dùng chúng làm bằng chứng runtime. Các liên kết dòng trong báo cáo trỏ working tree local, có thể dịch chuyển khi người khác sửa file.

Danh sách dirty CodeGraph, để không lẫn thay đổi local với upstream:

```text
 M CHANGELOG.md
 M __tests__/explore-output-budget.test.ts
 M __tests__/kernel-dart-parity.test.ts
 M __tests__/kernel-tsjs-parity.test.ts
 M __tests__/mcp-callers-truncation.test.ts
 M __tests__/object-literal-methods.test.ts
 M __tests__/react-native-bridge.test.ts
 M __tests__/ts-chained-receiver.test.ts
 M codegraph-kernel/src/dart.rs
 M codegraph-kernel/src/tsjs/extractors.rs
 M scripts/agent-eval/probe-factory-closure.mjs
 M src/extraction/languages/dart.ts
 M src/extraction/tree-sitter.ts
 M src/mcp/tools.ts
 M src/resolution/index.ts
?? __tests__/explore-output-limit.test.ts
?? src/mcp/explore-output-limit.ts
?? src/resolution/store-accessor.ts
```

Diff tracked tại thời điểm đọc: 274 dòng thêm, 56 dòng xóa. Không đọc sâu implementation `store-accessor.ts`; chỉ xác nhận diff resolver đã thêm guard nested member call và gọi accessor trước prefilter. Vì vậy không kết luận chất lượng toàn bộ thay đổi này. Cơ chế final output ceiling [13–14] là **local untracked**, không được ghi thành năng lực của HEAD đã pin.

LICENSE đọc tại checkout: CodeGraph MIT, Archify MIT, Joern và Code Property Graph Apache-2.0, T3MP3ST AGPL-3.0. Đây là nhận diện văn bản license local, không phải thẩm định pháp lý hoặc toàn bộ license phụ thuộc. Đề xuất ưu tiên tự triển khai contract nhỏ khi chỉ cần ý tưởng; nếu vendor code phải có bước kiểm tra nguồn/license của phần được lấy. [53]

## 3. CodeGraph — graph truy hồi, Rust kernel và tính nhất quán

### 3.1. Luồng đầu-cuối đã trace

Luồng index đến trả context:

1. `CodeGraph.indexAll()` lấy mutex nội bộ rồi file lock; đánh dấu `index_state=indexing`, bật bulk load, gọi `ExtractionOrchestrator.indexAll()` với signal/progress. Với DB mới có nhánh journal MEMORY, synchronous OFF; DB cũ đi đường WAL. [1: `src/index.ts:472,494,525,541`]
2. Orchestrator nhận kết quả parse theo sequence, ghi theo thứ tự file. `storeResult()` chờ WAL backpressure; có thể chuyển buffer tới writer worker, hoặc materialize rồi gọi `storeExtractionResult()`. `STORE_WRITER_WINDOW` giới hạn lượng gửi chưa xử lý. [2: `src/extraction/index.ts:1990,2003,2021`]
3. Route native: `tryKernelExtract()` gọi `kernel.extractFile()` rồi decode về `ExtractionResult`. Rust `extract_file()` dispatch theo language dưới stack guard, xuất năm buffer `meta/nodes/edges/refs/arena`; TS fallback đi `TreeSitterExtractor` khi kernel trả null. [3–5]
4. Store tính hash, bỏ qua nội dung không đổi, snapshot incoming cross-file edges trước xóa dữ liệu file, ghi node/edge/reference. ID dựa file/kind/name/line khiến chỉnh dòng có thể đổi ID, nên restore cross-file edge phải dựa thêm tên/kind/path. [2: `2602–2669`]
5. Sau parse, index reinitialize resolver, chạy post-extract và `resolveReferencesBatched()`. Resolver tạo/persist edges, xóa resolved refs, giữ failed refs để sync thử lại. Schema có `unresolved_refs.status`, `name_tail`, FK cascade và `edges.provenance`. [1: `576,628,1274`; 6–7]
6. Query MCP vào `ToolHandler.execute()`; chờ catch-up có giới hạn, chọn query pool nếu healthy/ready hoặc xử lý cùng tiến trình, rồi thêm worktree/staleness notice. Explore gọi `resolveNamedSymbolFlow()`, lấy callees để dựng flow và source quanh callsite, giới hạn text cuối cùng và cập nhật phần thực sự đã giao cho session. [10–11,13]

Đây là trace bằng mã nguồn qua cả index và query; không chạy index thực tế.

### 3.2. Cơ chế nên lấy

**Tách extraction khỏi semantic resolution.** Kernel ABI kiểm tra version và bảng NodeKind/EdgeKind trước nhận binary. Đây là contract cụ thể để tránh đọc nhầm enum index khi JS và Rust lệch phiên bản. Một boundary crossing/file và các bảng phẳng là lựa chọn đáng thử nếu phép đo thực tế cho thấy chi phí serialization đáng kể. Không cần sao chép N-API vào harness Rust thuần: có thể tách domain extraction khỏi binding thành crate riêng. Crate hiện khai báo `cdylib`, dùng `napi` và module nội bộ; “có Rust” không đồng nghĩa “cargo add rồi có full graph engine”. [3–4]

**Một writer, parse song song có backpressure.** Thứ tự commit độc lập với thứ tự worker xong giúp kết quả dễ tái lập. Phân biệt pool phân tích CPU với subagent LLM: đây là implementation detail của graph backend, không phải hệ điều phối agent cần đưa ra sản phẩm. Chỉ áp dụng bulk/WAL tuning sau khi có profile; nhánh fresh init OFF/MEMORY không phù hợp để lưu audit ledger không thể mất. [1–2]

**Sync cần xem xét file không đổi.** Khi định nghĩa thêm/xóa, caller ở file nguyên vẹn vẫn có thể cần rebind. `definitionDelta` dẫn tới `resurrectStaleResolutionEdges`; failed refs cũng cần được kích hoạt lại khi symbol mới xuất hiện. Copy kiểu “hash file đổi rồi reparse riêng file ấy” sẽ làm graph lệch dần. [1: `872–976`; 7]

**Lưu provenance tới wiring site.** Cạnh callback/event emitter được đánh `heuristic`, `synthesizedBy`, `registeredAt`; MCP duyệt raw incident edges để không mất heuristic edge khi có static edge trùng cặp. Contract đích phải giữ loại chứng cứ này, không flatten thành một boolean `calls`. [11–12]

**Kiểm tra source thật đã giao.** Local limiter chừa chỗ cho notice, đóng code fence, cắt theo dòng/file, tính lại header và không checkpoint range bị cắt. Đây là cơ chế tốt để native subagent tiếp theo không bị từ chối source với lý do “đã đọc” khi source thực tế chưa tới context. Hạn mức hiện tính JS string units, không phải byte/token. [13–14]

### 3.3. Khoảng trống và hành vi fallback

- Kernel load/contract mismatch trả null; extraction error fallback WASM, lỗi `defer:` im lặng theo thiết kế. Stack guard cũng đi đường defer. Cần trả `backendUsed`, `fallbackReason`, `grammarRevision` ra telemetry; chỉ gắn nhãn “Rust enabled” là không đủ. [3–5]
- Parity TS/JS có `describe.skipIf(!kernelBuilt)` ở dòng 60. Assertions so canonicalized node/edge/ref và yêu cầu số node không rỗng, nhưng suite có thể không chạy. Comment nói `CODEGRAPH_KERNEL_EXPECT=1` được kiểm tra ở suite scaffold khác; phần scaffold đó chưa được audit. [15]
- `NotIndexedError` trả success-shaped text để hướng dẫn, trong khi PathRefusal trả error. Adapter machine không nên suy ra graph sẵn sàng từ việc MCP thiếu `isError`. Cần `status=not_indexed` có kiểu. Catch-up có thể tiếp tục nền nên kết quả đầu tiên chưa chắc thuộc snapshot mới nhất. [11: `2110–2121,2186–2200`]
- `sync()` không lấy được file lock trả toàn bộ counter bằng 0, giống no-op; `indexAll()` lại trả `success:false` và lỗi. Harness phải phân biệt `busy` với `unchanged`. Đây là hành vi trực tiếp trong code, chưa kiểm tra bằng contention runtime. [1: `475–477,780–783`]
- Flow query bị chặn seed/candidate/hop/visit. Named mode ưu tiên chain dài trong budget, directed mode ưu tiên đường ngắn; không phải thuật toán liệt kê mọi đường. Không thấy đường chỉ nên đọc là “không tìm thấy trong phạm vi truy vấn này”. [10: `154–185,395–422,559–615`]
- Store có lọc node thiếu field và edge/reference tới node bị loại. Nếu reuse, thống kê các phần bị loại để không biến extraction lỗi thành graph sạch. [2: `2671–2685`]
- Trạng thái fast-init/bulk load phức tạp và ghi file lớn theo chunk. Không giả định toàn bộ index/file luôn là một transaction ACID. Snapshot publication của harness cần ranh giới riêng. [1–2]

### 3.4. Test đã đọc và quyết định

`sync-rebuild-convergence.test.ts` so **tập edge**, không chỉ counts; rebuild thực sự dùng `CodeGraph.recreate()` rồi index lại, tránh so index với chính nó. Test thêm competing definition xác minh caller không đổi vẫn rebind. Kill switch `CODEGRAPH_NO_REBIND=1` là ý tưởng mutation test hữu ích: suite phải phát hiện khi vô hiệu hóa cơ chế cần bảo vệ. Convergence chỉ chứng minh hai pipeline nhất quán, chưa chứng minh cả hai đúng về ngữ nghĩa; vẫn cần oracle edge kỳ vọng. [9]

`explore-named-symbol-render.test.ts` kiểm tra definition line xuất hiện trong output, không chỉ tên symbol trong header. `kernel-tsjs-parity` so node/edge/ref; local `explore-output-limit` kiểm tra Unicode, fence và partial-delivery. Những test này có giá trị về thiết kế oracle, nhưng **không test nào được thực thi ở đây**. [14–15,55]

**Adopt:** schema truy hồi nhỏ, provenance, failed-ref retry/rebind, oracle rebuild độc lập, query budget và delivered-range accounting. **Avoid:** port toàn bộ MCP/installer/UI/daemon vào harness mới; coi kernel là toàn bộ semantic engine; áp dụng local patch chưa pin như baseline upstream.

## 4. Joern — semantic analysis theo lớp, backend sâu có giới hạn

### 4.1. Luồng đầu-cuối đã trace

Với frontend Python source:

1. `JoernParse.run(config)` kiểm tra input, chọn/đoán language, gọi generator, rồi default overlays. Python generator chạy executable `pysrc2cpg` với input và output; post-processing dùng danh sách pass frontend-specific. [16–17]
2. Entry `NewMain` dùng `X2CpgMain(new Py2CpgOnFileSystem(), ...)`. `createCpg()` tạo CPG rỗng, xác định file `.py`, loại venv/ignored dirs, tạo input providers đọc source, gọi `Py2Cpg.buildCpg()`. [18]
3. `buildCpg()` tạo metadata/global namespace/ANY type, apply diff, chạy `CodeToCpg`, config-file và dependency pass. `CodeToCpg.runOnPart()` parse Python, convert bằng visitor rồi absorb AST diff. Nếu lỗi, tạo file node và log warning. [19–20]
4. `DefaultOverlays.create()` load CPG, chạy Base→ControlFlow→TypeRelations→CallGraph rồi `OssDataFlow`. `OssDataFlow.create()` gọi `ReachingDefPass.createAndApply()`: mỗi method là một part, tính problem, check threshold, solve rồi thêm reaching-def edges. [21–22]
5. Query `sink.reachableByFlows(source)` chuẩn hóa starting points, gọi `Engine.backwards()`, giải task ngược từ sink, gom/dedup kết quả; adapter DSL lọc node không visible theo semantics trước tạo `Path`. Engine phải shutdown sau dùng. [23–25]

Luồng này cho thấy dataflow là lớp tính toán trên graph, không chỉ query `calls` với BFS.

### 4.2. Giá trị tái sử dụng

**Overlay manifest và semantics cấu hình được.** Giữ rõ graph đã có AST/CFG/call graph/dataflow gì, phiên bản frontend, semantics và tham số query. Python dataflow tests cho cùng source/call nhưng kết quả đổi khi cung cấp `NilSemantics` hoặc flow mapping khác: semantics là phần identity của kết quả/cache, không phải chi tiết có thể bỏ qua. [21,26]

**Task fingerprint/cache trong phân tích.** Engine dùng fingerprint để tránh solve lại, giữ task phụ thuộc, gom kết quả theo sink. Có thể học cách memoize graph query và cắt chu kỳ. Không đưa work-stealing engine này thành scheduler cho Codex agents: đây là scheduler thuật toán dataflow ở backend. [24–25]

**Backend tùy chọn qua artifact.** Nên gọi một tiến trình phân tích được giới hạn CPU/RAM/deadline, dùng scratch directory riêng, xuất graph/slice có version. TUI vẫn có thể mở snapshot nhẹ khi Joern chưa có sẵn. Không bắt người dùng xây toàn bộ Joern để sử dụng harness cơ bản.

### 4.3. Giới hạn cần hiển thị

- ReachingDef mặc định ngưỡng 4.000 definitions/method. Vượt ngưỡng thì log “Skipping” và return, không có reaching-def edges cho method đó. Manifest “overlay đã chạy” không đủ chứng minh coverage method. [21–22]
- Query `EngineConfig.maxCallDepth=4`; `TaskCreator` loại task vượt depth hoặc lặp stack. Các giới hạn args/output expansion cũng là một phần config. Kết quả âm chỉ có ý nghĩa dưới cấu hình này. [24–25]
- Một task solve ném lỗi thì engine log và tiếp tục giảm running counter, cuối cùng vẫn trả danh sách kết quả thu được. Adapter cần thu failure/partial information, không chỉ serialize list paths. [24: `96–130`]
- Python parse lỗi vẫn tạo FILE node; đếm file node không đo được số file parse đầy đủ. [20: `35–59`]
- `reachableByInternal()` gọi shutdown sau `backwards()` chứ không trong `finally` ở đoạn đọc. Nếu `backwards` ném lỗi ra ngoài, có rủi ro không dọn executor qua đường này; chưa tái hiện. [23: `76–83`]
- **Ứng viên lỗi cần regression:** `JoernParse.generator` là biến global chưa khởi tạo; `enhanceOnly` bỏ qua generate, trong khi `applyDefaultOverlays` vẫn gọi `generator.applyPostProcessingPasses(cpg)`. Trong tiến trình mới chạy `--overlaysonly` với enhance bật, có đường dereference giá trị chưa gán. Đây là suy luận từ control flow, chưa chạy CLI và chưa kết luận mọi mode bị lỗi. [16: `16,133–159`]

### 4.4. Test và lựa chọn

Đã đọc Python `DataFlowTests` với source literal→call→print, exact flow code/line, và negative tests thay external-call semantics. Đây là test parser/graph/dataflow thực qua fixture, không phải LLM simulation; chưa chạy. Chưa audit toàn bộ frontend C/C++/Rust, toàn bộ querydb hoặc compatibility giữa hai checkout Joern/CPG hiện tại. [26]

**Adopt:** overlay identity, typed source/sink query, bounded analysis, semantics-sensitive cache và oracle flow code/line. **Avoid:** bắt buộc JVM stack ở critical path TUI, dùng output rỗng như bằng chứng an toàn, hoặc coi checkout codepropertygraph cạnh bên tự động đúng version Joern cần.

## 5. Code Property Graph — schema, import và pass lifecycle

### 5.1. Luồng đầu-cuối đã trace

Luồng import protobuf CPG vào graph có thể query/persist:

1. `CpgLoader.load(path)` kiểm tra tồn tại và magic bytes. ZIP/proto hoặc OverflowDB được chuyển sang sibling `.fg`; nhánh còn lại mở storage trực tiếp. Bản có `persistTo` hỗ trợ copy hoặc convert sang đích khác. [29]
2. Proto loader mở ZIP, đọc `CpgStruct`, chạy hai lượt: tạo raw nodes và map proto ID→graph node; sau đó set property và tạo edge qua mapping. Cuối cùng `DiffGraphApplier.applyDiff` rồi trả CPG. [30: `21–76`]
3. Một `CpgPass.createAndApply()` tạo builder, `runWithBuilder()` gọi init→generateParts→runOnPart→merge→onAccumulatorComplete; finish chạy trong finally. Sau đó apply diff vào graph. Các part đọc trạng thái ban đầu, chưa thấy diff của part khác. [31]
4. Test loader mở graph copy, thêm `NewMethod`, close, mở lại đích và kiểm tra count tăng, nguồn vẫn giữ count cũ. Đây là call flow import→mutation→persist→reload có test source trực tiếp. [33]

### 5.2. Contract đáng lấy

`CpgSchema` ghép các lớp FileSystem, Namespace, Type, Method, AST, CallGraph, CFG, Dominators, PDG, Tags, Findings; lớp Metadata có language, root, version, hash và danh sách overlays theo thứ tự. Harness nên học **phân biệt loại quan hệ và provenance lớp sinh ra**, không lấy toàn bộ schema lớn nếu UI chỉ cần symbol/call/import/evidence. [27–28]

Diff-pass cung cấp mẫu “worker đọc snapshot, trả delta, coordinator apply”. Trong harness, subagent trả đề xuất/evidence/patch artifact; nó không tự sửa authoritative graph bằng một kết nối DB chia sẻ. Apply delta phải kiểm tra snapshot đầu vào, duplicate và dangling endpoints. `finish()` trong finally là mẫu cleanup cần giữ khi task lỗi. [31–32]

Định danh proto ID qua map cho thấy phải tách **backend ID** khỏi **harness ID**. Đề xuất luôn namespace bằng backend+snapshot; liên kết xuyên revision dùng symbol key và source evidence riêng. Không nối graph từ hai import chỉ vì numeric ID bằng nhau. [30]

### 5.3. Rủi ro cụ thể

- Loader không phải read-only utility. `load(path)` có thể sinh sibling `.fg`; flatgraph có persistence lúc close (test chủ động copy fixture). Luồng inspect phải dùng bản sao scratch và đích riêng. [29,33]
- `load(from,persistTo)` gọi `Files.deleteIfExists(persistTo)` trước kiểm tra `from` tồn tại và trước format validation. Không truyền output đã tin cậy làm `persistTo`. Tình huống nguồn thiếu/format sai phải được wrapper chặn trước, publish qua rename sau validation. [29: `56–74`]
- Proto converter bỏ qua node kind không biết với warning và bỏ edge có endpoint không map được. Với unknown node có property, đường truy cập lazy `gNode` còn có thể throw; không thể mô tả đây là tolerant import hoàn toàn. Cần báo rejected node/edge và failure reason, không chỉ số node đã load. [30: `54–69,98–107`]
- Chỉ hỗ trợ tối đa một property trên edge trong converter flatgraph; edge có nhiều property sẽ throw. Muốn mang đồng thời callsite, confidence, extractor và evidence cần bảng sidecar hoặc evidence node/quan hệ riêng, không nhét tùy ý vào CPG edge. [30: `123–133`]
- Fork/join giữ toàn bộ parts và diff trong RAM trước apply; không có bound streaming theo batch trong pass này. Apply một diff không tự chứng minh transaction rollback khi apply nửa chừng lỗi. Chỉ khẳng định build-then-apply và cleanup theo code, chưa chứng minh crash atomicity. [31: `43–64,160–234`]

### 5.4. Test và quyết định

`CpgPassNewTests` có schema violation, init/finish đúng một lần, finish cả khi run lỗi, và accumulator merge. Test accumulator đầu tiên tự đặt `isParallel=false`, nên không dùng nó làm chứng minh race-free ở chạy song song. `CpgLoaderTests` bao phủ ba format và persist riêng. Tất cả là đọc source, không thực thi. [32–33]

**Adopt:** schema có version, lớp graph rõ nghĩa, delta build/apply, lifecycle cleanup. **Avoid:** raw binary làm exchange contract không version; loader mặc định trên reference artifact; port cả flatgraph vào Rust chỉ để có graph.

## 6. Archify — evidence và giao artifact đáng tin

### 6.1. Luồng đầu-cuối đã trace

Luồng `deliver architecture input.json output.html --repo-root ... --json`:

1. `commandDeliver` parse args, đọc bytes specification, resolve output, tạo staging và ghi snapshot specification bằng `wx`. Nó spawn renderer vào candidate path. [34: `821–847,941–969`]
2. Architecture renderer gọi `loadDiagramWithBrandMarks`; shared loader parse JSON, validate schema/guided views/relationship IDs/engineering profile, rồi gọi `verifyRepositoryEvidence`. [35–36]
3. Evidence verifier yêu cầu commit SHA 40 ký tự, canonical repository identity và root Git đúng top-level/origin. `cat-file -e SHA^{commit}`, `cat-file -t SHA:path`, `git show SHA:path` kiểm tra blob và line range. Nó dùng commit blob, không source dirty hiện tại. [37]
4. Renderer chạy layout validation, dựng SVG và `writeDiagram` ghép template HTML với source-evidence payload. Deliver kiểm tra artifact candidate; lỗi thì giữ artifact cũ. Nó tính SHA-256 specification/artifact, tạo receipt, kiểm tra lại output path, rename candidate sang output rồi in receipt. [34–36]

### 6.2. Điều nên mang vào harness

**Evidence bằng snapshot content.** Tách `RepoIdentity`, `revision`, `path`, `line/endLine` khỏi nhãn do agent viết. Với local dirty snapshot, thêm working-tree content digest và đánh dấu nguồn là working-tree; không sinh link commit như thể commit có chứa thay đổi chưa commit. Test Archify sửa working file nhưng kiểm tra source cũ tại commit, một ví dụ rất rõ cho sự khác biệt này. [37–38]

**Artifact staging + receipt.** Freeze input, validate candidate, hash cả input/output, publish cuối cùng. Dùng cho báo cáo, graph snapshot và patch artifact. Hash chỉ chứng minh bytes được receipt đề cập; muốn xác minh suy luận đúng vẫn cần kiểm tra evidence edge/source. [34]

**Diagnostic có cấu trúc.** `code`, `severity`, `subject`, `evidence`, `supportedFixes` tốt hơn parse chuỗi log tự do để TUI quyết định nút retry hoặc cần dữ liệu nào. [37: `10–18`]

### 6.3. Những điều không được suy quá

- `verified:true` chỉ có nghĩa repository/commit/file/range thỏa kiểm tra. Verifier không đọc AST để chứng minh component, connection hoặc câu giải thích đúng; ít nhất một component source là đủ để receipt tồn tại. Không đổi thành nhãn “architecture verified”. [37: `160–235`]
- Evidence được hỗ trợ theo đường đang đọc cho architecture; các mode workflow/dataflow/lifecycle không tự có cùng contract. `hasRepositoryEvidence` trả false với loại khác. [37: `68–79`]
- SHA regex 40 ký tự, origin bắt buộc và root phải là Git top-level: không bê nguyên vào harness cần local repo không remote hoặc định danh hash đa dạng. Đây là giới hạn implementation, không phải yêu cầu người dùng mới. [37: `86,132–151`]
- Render có thể truy cập mạng: `loadDiagramWithBrandMarks()` gọi `prepareDiagramBrandMarks`; brand object URL được capture lại và so digest. Digest pin không đồng nghĩa offline. Tích hợp export phải chỉ nhận local/preset asset hoặc có offline asset store; không chạy renderer tùy ý với brand URL trong workflow local-only. Không thực hiện request nào trong audit này. [35,39]
- `render` đi thẳng write HTML; guarantee staging/validate/preserve-old mô tả ở trên thuộc `deliver`. Receipt thành công được in stdout, không mặc định là một log bền vững harness có thể mở lại. [34–35]

### 6.4. Test và quyết định

`repository-evidence.test.mjs` tạo Git repo fixture thực, commit file, đổi origin và gọi CLI subprocess. Có test local-only bỏ web link nhưng vẫn verify, dirty working file không đổi evidence tại commit, path escape, revision thiếu, line quá giới hạn và giữ artifact cũ. Đây là test deterministic Git/filesystem cho evidence; không chứng minh ngữ nghĩa graph. Không chạy suite vì trong file còn có đường browser/preview, và nhiệm vụ không cần tạo artifact phụ. [38]

**Adopt:** typed evidence receipt, snapshot pin, diagnostic, publish cuối cùng. **Avoid:** browser viewer thành UI chính, brand capture tự phát, coi authored diagram là codegraph extractor.

## 7. T3MP3ST — context hữu ích, semantic graph và swarm chưa phải nền tảng đích

### 7.1. Luồng đầu-cuối đã trace

Luồng whitebox local:

1. HTTP `POST /api/whitebox/analyze` kiểm tra objective rồi gọi `runWhiteboxAnalysis`. [40]
2. Whitebox resolve input nguồn; với path local thực hiện canonical containment. `ingestRepository(createMultiLangIngestConfig(...))` crawl file, đọc text, parse block, build call graph, tìm entrypoint, BFS reachability, classify, sort priority. [41–44]
3. `packAnalysisUnits` tạo source context có budget. Whitebox tạo config orchestrator/worker và gọi `DecompositionOrchestrator.run()`. Guard chỉ từ chối khi `stats.files===0`, không từ chối trường hợp files>0 nhưng blocks/source context rỗng. [41: `319–359`]
4. `run()` parse source bundle, lặp round: decompose→dispatch→synthesize→tích lũy kiến thức, dừng nếu không có query, synthesis yêu cầu dừng hoặc hết maxRounds; sau đó final synthesis và usage totals. Dispatch mặc định batch `Promise.all`, concurrency=4, tối đa 8 queries/round, 5 rounds. [45]
5. Worker nhận query và context explicit hoặc context pack rồi gọi `worker.chat()`. Nó không phải native Codex child thread; constructor tạo hai `LLMBackbone`. Kết quả có answered/refused/error và token usage nếu provider trả. [45: `112–132,287–339`]

Không gọi HTTP/LLM hoặc khởi tạo runtime trong khảo sát. Luồng remote clone và token auth tồn tại trong whitebox helper, nhưng không được chạy và không cần đưa vào local harness. [41: `111–174`]

### 7.2. Cơ chế đáng học

- Repo map giữ inventory trước phần source; rank file theo relevance; output có included/dropped và dấu middle-elided khi cắt file lớn. Điều này cho native subagent biết đang thiếu gì để truy hồi tiếp. [46]
- Analysis unit giữ path/line, callers/callees, representative reachability path, priority. Có thể dùng hình dạng này để chọn frontier graph và xây task packet, nhưng exposure/priority chỉ là heuristic lựa chọn, không phải phát hiện lỗi đã xác nhận. [42–43,50]
- Số vòng/query và trạng thái answered/error là điểm bắt đầu cho contract bounded work. Cần chuyển thành policy quanh native runtime, với ledger bền và deadline thực; không copy vòng `LLMBackbone` làm engine thứ hai. [45]

### 7.3. False, mocked và fallback đã xác định

**Call graph không phải semantic resolver.** `buildCallGraph()` tạo regex `name\s*\(` cho từng tên, scan body đã bỏ dòng đầu, rồi nối tới mọi block cùng tên. Không disambiguate import/receiver/scope, loại self-edge, có thể match text trong comment/string; single-line definition mất lời gọi cùng dòng. BFS chỉ ghi một shortest representative path. Vì vậy một `reachable:true` có thể dựa trên tên trùng, còn `false` có thể do parser/matcher bỏ sót. Không dùng backend này để đưa ra kết luận reachability chắc chắn. [43]

**Không có grammar và parse timeout/lỗi đều thành `[]`.** Python luôn đi regex legacy. Non-Python grammar thiếu trả empty; parse bị progressCallback hủy hoặc throw cũng trả empty sau reset parser. `ingestRepository` đã tăng processedFiles trước bước đó, nên files>0/blocks=0 có thể đi vào LLM với source rỗng. Không đồng nhất “grammar chưa load”, “file không có function”, “parse timeout” và “không có mã liên quan”. [42,44,41]

**Budget chưa phải hard cap đầu-cuối.** `estimateTokens` dùng chars/4. Repo-map minimum 200 tokens có thể vượt budget rất nhỏ; `tokensUsed` cộng các section nhưng không bao trọn mọi separator của text cuối. Test chấp nhận `estimateTokens(text) <= budget + 50`, xác nhận đây là xấp xỉ. Explicit `query.context` bypass `packForWorker`, accumulatedKnowledge tăng qua round, và synthesis/final prompt có overhead riêng. Nên giới hạn bytes/text và token ước lượng riêng, reserve cả prompt scaffold, không gọi nó là hard token bound. [45–46,49]

**Telemetry planning có nguy cơ khác prompt thật.** `decompose()` pack bằng `this.planningTokenBudget` để emit telemetry, nhưng gọi `ORCHESTRATOR_DECOMPOSE_PROMPT` không truyền budget ấy. Prompt tự dùng `PLANNING_TOKEN_BUDGET` khi `truncateForPlanning`. Nếu caller đặt custom budget, telemetry và nội dung gửi có thể khác. Đây là suy luận trực tiếp từ hai callsite, chưa có runtime reproduction. Giải pháp đích là pack một lần, gửi đúng packet đó, hash packet vào ledger. [45: `243–263`; 47: `51–68,124–134`]

**Bounded theo mặc định không đồng nghĩa được kiểm soát mọi đầu vào.** Whitebox nhận bất kỳ `maxRounds` kiểu number >0; constructor spread config không validate finite integer/upper bound. Vòng batch tạo barrier: một worker chậm giữ cả batch, không thấy abort/deadline xuyên `run()` trong luồng đọc. Usage được cộng sau call, không phải reserve ngân sách trước dispatch. Shared fields trên instance được reset đầu run, nên cùng instance chạy concurrent còn có nguy cơ lẫn state. Không tái sử dụng orchestrator này như singleton multi-run. [41,45]

**Synthesis chưa phải bằng chứng có kiểm tra.** Parser nhận findings/sourceQueryIds do model đưa, default confidence=0.5 nếu thiếu; không thấy kiểm tra sourceQueryIds tồn tại hay source span chứng minh finding. Unstructured response giữ prose và dừng vòng. Đây là kết quả LLM cần adjudication, không phải oracle tự động. [45: `389–438`]

**Swarm/scanner có stub thật.** `SwarmController.initialize()` trả `[]`; `ScannerOrchestrator.scan()` phát completed với findings=0 và trả empty. Stub honesty test khẳng định đúng các hình dạng này. Không có fabricated agent, nhưng machine consumer vẫn có thể hiểu nhầm “scan complete, zero findings” là đã quét. Workflow stub có topological walk nhưng node result `notExecuted:true`, overall failed. Contract đích nên `unsupported/not_executed`, không `completed` với tập rỗng. [51–52]

**Test whitebox có mock orchestration.** `whitebox-multilang.test.ts` dùng parser/filesystem thực để kiểm tra Go/TS/Python tới sourceContext; test `runWhiteboxAnalysis` spy/mocks `DecompositionOrchestrator.prototype.run` trả `{}`. Test này chứng minh wiring source, không chứng minh multi-round reasoning hoặc worker thực chạy. [48]

**Prompt architecture không phù hợp mục tiêu mới.** Prompt của decomposition hướng tới worker riêng không biết full objective và cơ chế context isolation nhằm tránh refusal. Không mang policy/prompt này sang native subagents. Child cần task scope, mục tiêu liên quan, constraints và evidence đủ để đánh giá trung thực. Chỉ học cấu trúc packet bounded; không học chiến lược che giấu mục tiêu. [47]

### 7.4. Coverage test và quyết định

Đã đọc tests `code-ingest` cho handler→helper, reverse caller, BFS depth, classification, ordered packing, path:line và redaction; `context-pack` cho inventory, priority, tail marker, budget xấp xỉ; `whitebox-multilang` với mock rõ ràng; `stub-honesty` với expected no-op. Không chạy tests. Chưa audit toàn bộ mission recovery, agents, attack graph, arsenal, persistence hay security của HTTP server; không suy ra tình trạng toàn hệ thống từ các module được chọn.

**Adopt:** inventory và omission accounting, task/result envelope nhỏ, test phân biệt real source path với mocked model path. **Avoid:** regex callgraph làm authority, two-model/provider loop, stub swarm và prompt refusal-routing; không cần phụ thuộc T3MP3ST runtime để triển khai các contract này.

## 8. Hợp đồng tích hợp đề xuất

Các hợp đồng dưới đây là **đề xuất mới cho harness**, không phải API đã được chứng minh tồn tại trong Codex hoặc năm repo. Việc ánh xạ native runtime cụ thể phải được kiểm tra riêng trong phạm vi Codex; audit này không đọc repo `codex` vì nằm ngoài scope được giao.

| Contract | Trường tối thiểu | Invariant cần bảo vệ |
|---|---|---|
| `Snapshot` | schema version, repo ID/root, base revision, working-tree digest, extractor/grammar/resolver versions, overlays, config digest | Mọi query/evidence cùng một snapshot; commit và dirty bytes không bị lẫn |
| `ExtractionReceipt` | file/content hash, backend used, status, errors, node/edge counts, fallback reason, skipped ranges | Empty-success khác unsupported/timeout/error; native fallback được nhìn thấy |
| `GraphEdge` | namespaced ID, source/target, kind, provenance, evidence spans, producer/version | Heuristic khác resolved/observed/authored; metadata không mất khi nhập CPG |
| `QueryResult` | snapshot ID, query/config, nodes/edges/paths, limits, visited count, truncated, gaps, status | `not_found` chỉ trong coverage/budget; path giữ từng bước và chứng cứ cạnh |
| `ContextPacket` | packet digest, objective/task scope, source spans+hash, included/partial/dropped ranges, actual size | Chỉ đánh delivered range thực sự còn trong payload cuối; không suppress source dựa tên file |
| `NativeTaskReceipt` | native child ID, parent/task ID, snapshot/packet ID, allowed write scope, limits, lifecycle events, terminal reason | Native runtime là nguồn trạng thái child; không tạo provider/model/account registry |
| `Finding` | claim, evidence IDs, derivation, confidence basis, verifier/test receipt, unresolved gaps | LLM prose/confidence không tự nâng thành verified; sourceQueryIds phải tham chiếu hợp lệ |
| `ArtifactReceipt` | input/output hash, snapshot ID, validations, produced files, disposition | Chỉ publish candidate đã validate; trước đó không thay artifact tốt đang tồn tại |

Luồng thực thi đích:

```text
Người dùng → Codex parent/native subagents
                  │  task + scope + budget, native child ID
                  ▼
            Harness Rust (CLI/TUI + ledger)
                  │  snapshot-scoped query/context
                  ▼
     Graph nhẹ ── optional request ── Joern trong scratch
                  │
                  ▼
       Evidence/finding → validation → artifact receipt
```

Parent tự phân rã và tổng hợp; harness kiểm tra tối đa số child đang active, số task/vòng, tổng hạn mức, deadline và phạm vi ghi trước hành động. Nếu native runtime đã có một giới hạn thì dùng effective limit nhỏ hơn giữa policy và capability; không dựng một runtime agent thứ hai. Khi native unavailable thì trả capability error/single-agent mode được ghi rõ; không tự fallback sang tài khoản/provider khác.

TUI nên hiển thị graph frontier, task/child state, budget remaining, snapshot freshness và gaps. Chuỗi trạng thái công việc và chuỗi trạng thái index là hai domain riêng: `task completed` không kéo theo `index complete`, `tests passed` hoặc `finding verified`. CLI machine xuất JSON data trên stdout; progress/diagnostic sang stderr. Backend phân tích CPU có thể dùng worker pool riêng nhưng không được tiêu thụ vô hạn tài nguyên khi native children tăng.

## 9. Công việc cụ thể nên đưa vào kế hoạch và acceptance tests

Không sửa `PLAN.md`. Danh sách sau là đầu việc để chủ dự án chọn/ghép vào kế hoạch chính; mỗi đầu việc có tiêu chí kiểm chứng, không yêu cầu live account trong unit/contract tests.

| ID / ưu tiên | Đầu việc | Acceptance tests bắt buộc |
|---|---|---|
| G01 / P0 | Chốt contract single-account/native-subagent và capability boundary | Không có config bắt buộc account pool/provider router; fake native adapter ghi nhận cùng session authority; một con hết slot không spawn thêm; không fallback provider khi native lỗi. Fake adapter chỉ chứng minh contract, phải ghi rõ chưa chứng minh runtime native thật. |
| G02 / P0 | Snapshot identity cho commit và dirty worktree | Hai snapshot cùng HEAD nhưng khác bytes không dùng chung cache; evidence dirty không sinh commit permalink giả; query khi snapshot thay trả stale hoặc snapshot cũ có nhãn rõ. |
| G03 / P0 | Extraction/coverage receipt chuẩn hóa | Thiếu grammar, parse timeout, parse error, file thật không symbol, kernel ABI mismatch trả năm trạng thái phân biệt; files processed không thay thế parsed coverage; receipt ghi backend thật. |
| G04 / P0 | Graph store nhỏ với provenance và reference lifecycle | Cùng symbol name ở hai module không nối tùy ý; heuristic edge có wiring span; unresolved giữ được qua restart; thêm định nghĩa ở file khác tạo được cạnh khi caller không đổi. |
| G05 / P0 | Incremental convergence oracle độc lập | Thêm/xóa/rename/move-line/competing definition/import đổi; tập edges+provenance của sync bằng clean rebuild trên snapshot giống nhau; cố ý tắt rebind làm test fail; golden fixture riêng kiểm tra semantic đúng, tránh hai pipeline cùng sai. |
| G06 / P0 | Context packet cuối cùng có hard size budget | Đo đúng payload gửi, gồm header/notice/scaffold; budget 0/nhỏ, Unicode tiếng Việt/emoji, một dòng khổng lồ, tên file dài; actual bytes không vượt cap; partial source không đánh delivered; custom budget và telemetry cùng packet hash. |
| G07 / P0 | Bounded native work và recovery ledger | Validate limit finite integer/có trần; reserve trước spawn; cancel/deadline dừng dispatch mới; restart reconcile native child ID thay vì spawn trùng; late result không commit vào task đã cancel; write scope của hai child không chồng ngoài điều phối cho phép. |
| G08 / P1 | Query semantics rõ named/directed và coverage | Overload > candidate cap, cycle, long chain, heuristic hop, dynamic boundary; hit cap luôn có `truncated/gaps`; không-path không đổi thành “unreachable” toàn cục; TUI và CLI dùng cùng query derivation. |
| G09 / P1 | Joern adapter chỉ tùy chọn và không mutate input | Chạy fixture parser/backend trong scratch, original bytes giữ nguyên sau close; method > threshold và query depth cap hiện partial; một worker error hiện diagnostic; unsupported frontend không trigger install/network ngầm. |
| G10 / P1 | CPG import có mapping/loss receipt | Unknown kind/dangling edge/multi-property edge được reject hoặc ghi loss rõ; nguồn thiếu/format sai không xóa artifact đích; round-trip không dùng raw numeric ID để nối hai snapshot; version mismatch bị chặn. |
| G11 / P1 | Evidence và artifact publication theo mẫu Archify | Invalid path/line/revision, dirty source, wrong repo, schema fail đều giữ artifact tốt cũ; hash input/output đúng bytes; authored connection không được nhãn semantic-verified; export offline không gọi brand URL. |
| G12 / P1 | Honest capability và verification gates | Stub scanner/swarm trả `not_executed`, không completed-zero; mocked tests được ghi test-double; findings có sourceQueryId sai bị reject; native parity job thiếu binary fail thay vì xanh do skip. |

Mốc triển khai đề xuất: G01–G07 trước; một fixture Rust/TS/Python nhỏ đủ kiểm tra UI+graph+packet+native-adapter contract; sau đó G08–G12. Chỉ thêm Joern khi có use case cần CFG/dataflow và fixture chứng minh giá trị so với graph nhẹ. Native runtime integration test thật là một gate riêng với phiên Codex được phép, không được tuyên bố đã đạt từ mock và cũng không cần live account cho bước thiết kế contract này.

Các phép đo nên thu sau khi implementation có thể chạy: cold index/warm sync latency, p95 query, peak RSS/WAL, queue wait, parser fallback/coverage, edges sai theo golden fixture, payload bytes/token estimate, số lần phải lấy thêm source và task cancellation latency. Không dùng các con số marketing hoặc benchmark trong comment của repository như số đo trên máy này.

## 10. Nguồn local đánh số và giới hạn kết luận

Liên kết dưới đây là nguồn đã đọc. Một mục có thể dẫn đầu file/hàm; các dòng bổ sung được ghi ngay cạnh để truy được chính xác. `WT` = working tree dirty; `U` = untracked local. Các nhận định “nguy cơ/ứng viên lỗi” là static reasoning, chưa có runtime reproduction. Không có phát hiện nào ở đây là kết quả live penetration test.

1. [CodeGraph index/sync](/home/minh/projects/outsource/codegraph/src/index.ts:472) — 472–628 index; 778–846 sync lock/store; 872–976 retry/rebind; 1274 batched resolver.
2. [Extraction commit/store](/home/minh/projects/outsource/codegraph/src/extraction/index.ts:1990) — 1990–2038 ordered store/backpressure; 2602–2697 materialize/hash/cross-file edges/filter.
3. [Rust kernel](/home/minh/projects/outsource/codegraph/codegraph-kernel/src/lib.rs:233) — 59–103 buffer/contract; 233–262 dispatch; [Cargo manifest](/home/minh/projects/outsource/codegraph/codegraph-kernel/Cargo.toml:9) — cdylib/N-API dependencies.
4. [Kernel loader ABI gate](/home/minh/projects/outsource/codegraph/src/extraction/kernel/loader.ts:124) — 124–166 contract/load fallback.
5. [Kernel extraction fallback](/home/minh/projects/outsource/codegraph/src/extraction/kernel/index.ts:282) — 282–314; [TS route, WT](/home/minh/projects/outsource/codegraph/src/extraction/tree-sitter.ts:7196).
6. [SQLite graph schema](/home/minh/projects/outsource/codegraph/src/db/schema.sql:20) — node/edge/files/unresolved refs, 20–125.
7. [Resolver persistence, WT](/home/minh/projects/outsource/codegraph/src/resolution/index.ts:1296) — persist/delete/park refs; local guard at 897 onward was also inspected via git diff.
8. [Sync definition delta](/home/minh/projects/outsource/codegraph/src/index.ts:940) — stale edge resurrection and re-resolution.
9. [Convergence test](/home/minh/projects/outsource/codegraph/__tests__/sync-rebuild-convergence.test.ts:109) — independent rebuild; 133–173 competing-definition cases; 61 edge-set query.
10. [Named/directed flow](/home/minh/projects/outsource/codegraph/src/graph/named-symbol-flow.ts:559) — bounded modes; 154–185 caps; 395–422 BFS; 52 exact symbol lookup.
11. [MCP dispatch/output, WT](/home/minh/projects/outsource/codegraph/src/mcp/tools.ts:2104) — 2104–2200 dispatch/status; 2745 shared flow; 6073–6095 actual delivery accounting.
12. [Heuristic synthesis](/home/minh/projects/outsource/codegraph/src/resolution/callback-synthesizer.ts:190) — provenance/wiring site; 351 event emitter.
13. [Final output ceiling, U](/home/minh/projects/outsource/codegraph/src/mcp/explore-output-limit.ts:16) — final string cap and partial-delivery helper.
14. [Ceiling tests, U](/home/minh/projects/outsource/codegraph/__tests__/explore-output-limit.test.ts:10) — fence, omitted range, Unicode and header growth.
15. [Kernel parity tests, WT](/home/minh/projects/outsource/codegraph/__tests__/kernel-tsjs-parity.test.ts:60) — skip gate; 79–95 canonical comparison.
16. [Joern parse CLI](/home/minh/projects/outsource/joern/joern-cli/src/main/scala/io/joern/joerncli/JoernParse.scala:81) — 81–91 pipeline; 16 global generator; 133–159 enhance-only branch.
17. [Python generator](/home/minh/projects/outsource/joern/console/src/main/scala/io/joern/console/cpgcreation/PythonSrcCpgGenerator.scala:18) — subprocess args and post-processing.
18. [Python filesystem frontend](/home/minh/projects/outsource/joern/joern-cli/frontends/pysrc2cpg/src/main/scala/io/joern/pysrc2cpg/Py2CpgOnFileSystem.scala:66) — file selection/input providers; [entry](/home/minh/projects/outsource/joern/joern-cli/frontends/pysrc2cpg/src/main/scala/io/joern/pysrc2cpg/Main.scala:42).
19. [Python build passes](/home/minh/projects/outsource/joern/joern-cli/frontends/pysrc2cpg/src/main/scala/io/joern/pysrc2cpg/Py2Cpg.scala:33).
20. [Python AST conversion/error fallback](/home/minh/projects/outsource/joern/joern-cli/frontends/pysrc2cpg/src/main/scala/io/joern/pysrc2cpg/CodeToCpg.scala:21).
21. [Default overlays](/home/minh/projects/outsource/joern/joern-cli/src/main/scala/io/joern/joerncli/DefaultOverlays.scala:18), [base overlay list](/home/minh/projects/outsource/joern/joern-cli/frontends/x2cpg/src/main/scala/io/joern/x2cpg/X2Cpg.scala:374), [OssDataFlow](/home/minh/projects/outsource/joern/dataflowengineoss/src/main/scala/io/joern/dataflowengineoss/layers/dataflows/OssDataFlow.scala:23).
22. [Reaching definitions bailout](/home/minh/projects/outsource/joern/dataflowengineoss/src/main/scala/io/joern/dataflowengineoss/passes/reachingdef/ReachingDefPass.scala:23) — solver and 4000-definition default/threshold.
23. [Dataflow query DSL](/home/minh/projects/outsource/joern/dataflowengineoss/src/main/scala/io/joern/dataflowengineoss/language/ExtendedCfgNode.scala:40) — visible paths; 76–83 engine lifecycle.
24. [Query engine](/home/minh/projects/outsource/joern/dataflowengineoss/src/main/scala/io/joern/dataflowengineoss/queryengine/Engine.scala:96) — failure handling; 133–178 dedup; 320 config caps.
25. [Task loop/depth pruning](/home/minh/projects/outsource/joern/dataflowengineoss/src/main/scala/io/joern/dataflowengineoss/queryengine/TaskCreator.scala:22).
26. [Python dataflow tests](/home/minh/projects/outsource/joern/joern-cli/frontends/pysrc2cpg/src/test/scala/io/joern/pysrc2cpg/dataflow/DataFlowTests.scala:17) — expected paths and semantics-sensitive negatives through line 122.
27. [CPG schema composition](/home/minh/projects/outsource/codepropertygraph/schema/src/main/scala/io/shiftleft/codepropertygraph/schema/CpgSchema.scala:5).
28. [CPG metadata schema](/home/minh/projects/outsource/codepropertygraph/schema/src/main/scala/io/shiftleft/codepropertygraph/schema/MetaData.scala:28).
29. [CPG loader](/home/minh/projects/outsource/codepropertygraph/codepropertygraph/src/main/scala/io/shiftleft/codepropertygraph/cpgloading/CpgLoader.scala:33) — 33–74 conversion/copy/delete behavior.
30. [Proto import](/home/minh/projects/outsource/codepropertygraph/codepropertygraph/src/main/scala/io/shiftleft/codepropertygraph/cpgloading/ProtoCpgLoader.scala:39) — two-pass mapping, dropped endpoints, unknown node kinds and edge property limit at 123.
31. [CPG diff pass](/home/minh/projects/outsource/codepropertygraph/codepropertygraph/src/main/scala/io/shiftleft/passes/CpgPass.scala:160) — apply; 196–234 lifecycle/merge; memory model documented 43–64.
32. [Pass lifecycle/schema tests](/home/minh/projects/outsource/codepropertygraph/codepropertygraph/src/test/scala/io/shiftleft/passes/CpgPassNewTests.scala:37) — schema failure, finish on error, accumulator with sequential mode at 110.
33. [Loader persistence tests](/home/minh/projects/outsource/codepropertygraph/codepropertygraph/src/test/scala/io/shiftleft/codepropertygraph/cpgloading/CpgLoaderTests.scala:54) — separate output/reopen/source unchanged; fixture copied because close persists at 80.
34. [Archify deliver CLI](/home/minh/projects/outsource/archify/archify/bin/archify.mjs:821) — freeze/render 941–969, check 991–1030, receipt 1080–1108, rename 1137, output 1175; direct render at 742.
35. [Archify shared load/write](/home/minh/projects/outsource/archify/archify/renderers/shared/cli.mjs:19) — validation/evidence; async brand at 44; direct write at 53.
36. [Architecture renderer](/home/minh/projects/outsource/archify/archify/renderers/architecture/render-architecture.mjs:55) — load; final validation/SVG/write at 1065.
37. [Repository evidence verifier](/home/minh/projects/outsource/archify/archify/renderers/shared/repository-evidence.mjs:74) — 74–238 full verifier; path guard at 42.
38. [Repository evidence tests](/home/minh/projects/outsource/archify/archify/test/repository-evidence.test.mjs:79) — pinned commit vs dirty source; 131–200 failure/preserve-old cases.
39. [Remote brand preparation](/home/minh/projects/outsource/archify/archify/renderers/shared/brand-marks.mjs:454) — URL capture and digest comparison, preset alternative.
40. [Whitebox HTTP entry](/home/minh/projects/outsource/T3MP3ST/src/server.ts:6566).
41. [Whitebox pipeline](/home/minh/projects/outsource/T3MP3ST/src/recon/whitebox.ts:319) — ingest/pack/LLM; 290–307 two-provider defaults; 192 containment; 111–174 clone/local selector.
42. [Ingest pipeline](/home/minh/projects/outsource/T3MP3ST/src/recon/code-ingest.ts:832) — parsed blocks, stats/truncation, graph/classification/sorting.
43. [Name-based graph and BFS](/home/minh/projects/outsource/T3MP3ST/src/recon/code-ingest.ts:575) — regex graph; entry detection 637; representative shortest reachability 658.
44. [Multi-language parser fallback](/home/minh/projects/outsource/T3MP3ST/src/recon/ts-parse.ts:89) — Python regex; missing grammar/timeout/error empty result.
45. [Decomposition orchestrator](/home/minh/projects/outsource/T3MP3ST/src/orchestration/orchestrator.ts:112) — defaults/two LLMs; run 142; telemetry 243; batch 271; worker 287; synthesis parsing 415.
46. [Context packer](/home/minh/projects/outsource/T3MP3ST/src/orchestration/context-pack.ts:269) — map minimum, rank, size accounting; estimator at 62, head/tail at 238.
47. [Planning prompt/budget](/home/minh/projects/outsource/T3MP3ST/src/orchestration/prompts.ts:51) — separate-worker purpose/context policy and independent planning budget at 68; packing helper at 124.
48. [Whitebox test with orchestrator mock](/home/minh/projects/outsource/T3MP3ST/src/__tests__/whitebox-multilang.test.ts:42).
49. [Context budget tests](/home/minh/projects/outsource/T3MP3ST/src/__tests__/context-pack.test.ts:88) — estimate plus slack; inventory and head/tail cases through 148.
50. [Ingest/graph/packing tests](/home/minh/projects/outsource/T3MP3ST/src/__tests__/code-ingest.test.ts:225) — handler→helper, reach depth; ordered context/source-span/redaction 314–371.
51. [Swarm stub](/home/minh/projects/outsource/T3MP3ST/src/stubs/index.ts:363), [scanner stub](/home/minh/projects/outsource/T3MP3ST/src/stubs/index.ts:105) — empty/no-op result; workflow notExecuted at 1317–1325.
52. [Stub honesty tests](/home/minh/projects/outsource/T3MP3ST/src/__tests__/stub-honesty.test.ts:68) — swarm empty, workflow failed/notExecuted; scanner expectation at 36.
53. License headers: [CodeGraph](/home/minh/projects/outsource/codegraph/LICENSE:1), [Joern](/home/minh/projects/outsource/joern/LICENSE:2), [CPG](/home/minh/projects/outsource/codepropertygraph/LICENSE:2), [Archify](/home/minh/projects/outsource/archify/LICENSE:1), [T3MP3ST](/home/minh/projects/outsource/T3MP3ST/LICENSE:1).
54. Hướng dẫn áp dụng: [CodeGraph AGENTS](/home/minh/projects/outsource/codegraph/AGENTS.md:1), [T3MP3ST override](/home/minh/projects/outsource/T3MP3ST/AGENTS.override.md:1); skill đã đọc: [rust-router](/home/minh/.codex/skills/rust-router/SKILL.md:1), [domain-cli](/home/minh/.codex/skills/domain-cli/SKILL.md:1), [m07-concurrency](/home/minh/.codex/skills/m07-concurrency/SKILL.md:1).
55. [Explore renders definition-line test](/home/minh/projects/outsource/codegraph/__tests__/explore-named-symbol-render.test.ts:139).

Mức tin cậy cao cho cấu trúc call flow, guard/fallback và hình dạng assertions tại checkout đã đọc; trung bình cho rủi ro suy từ control flow chưa tái hiện; chưa xác định cho tốc độ thực, chất lượng semantic toàn repo, compatibility binary, tính đúng native Codex integration và hành vi live model. Các khoảng chưa xác định này có acceptance task tương ứng ở mục 9.
