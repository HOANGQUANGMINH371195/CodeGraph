# Kiểm toán hạ tầng outsource cho graph harness Rust trong terminal

Ngày khảo sát: 2026-09-11. Phạm vi độc lập: browser, OpenSandbox, ghostty, rtk, grit, temp-rs-ddd, codex-multi-auth trong /home/minh/projects/outsource.

## 1. Kết luận kiến trúc

Thiết kế đích là **một tài khoản Codex, native subagents, tự trị có giới hạn**. Graph harness cung cấp ngữ cảnh đồ thị, giao diện terminal, bằng chứng, ranh giới ghi và giới hạn tài nguyên. Codex cha giữ quyền điều phối subagent. Không đưa account pool, model router, Responses proxy, scheduler swarm độc lập hoặc message broker vào đường chạy bắt buộc.

Những cơ chế đáng lấy từ bảy repo nằm ở lớp bên dưới: thu gọn output có truy hồi của RTK; trạng thái VT và dirty rendering của Ghostty; transaction và ý tưởng lease/worktree của Grit; hợp đồng vòng đời của OpenSandbox; extraction của Lightpanda; phân pha khởi động/dừng từ temp-rs-ddd; ledger có schema và chặn khi không biết chi phí từ codex-multi-auth. Mức tái sử dụng chủ yếu là **thiết kế có kiểm chứng và adapter hẹp**, không nhập nguyên bộ sản phẩm.

Năm phát hiện ảnh hưởng lớn nhất:

1. Screenshot của Lightpanda là **raster hóa văn bản thành các block**, không phải chụp giao diện CSS. API trả PNG thật, nhưng dùng nó để xác nhận giao diện web sẽ tạo bằng chứng sai loại. [4]
2. Grit có transaction chống double writer, nhưng nhánh cùng agent nâng read → write bỏ qua reader khác. Ngoài ra, merge thất bại vẫn nhả lock; gọi done lần nữa có thể dừng sớm vì không còn lock. [21][22][23]
3. RTK giữ exit code nhưng raw capture bị giới hạn; kênh streaming không giới hạn hàng đợi và bộ nhớ filtered vẫn tăng. Chỉ số token là ước lượng byte/4. Không thể xem RTK là bộ lưu bằng chứng đầy đủ hoặc bộ đo ngân sách Codex. [16][17][18]
4. OpenSandbox có cleanup và readiness thực, nhưng close không hủy sandbox; Docker extension bật bwrap isolation còn thêm SYS_ADMIN và bỏ giới hạn AppArmor/seccomp. Không suy diễn “isolation” đồng nghĩa tăng an toàn ở mọi lớp. [8][11]
5. temp-rs-ddd chứa health “ok” cố định, application test cộng 2+2, handler không tồn tại vẫn thành công, và cơ chế commit Kafka có thể vượt qua message lỗi. Đây là template tham khảo, chưa phải lõi runtime đáng nhập. [28][29][30][31]

## 2. Phương pháp, revision và giới hạn bằng chứng

Đã đọc AGENTS.md ở gốc các repo có file: browser, OpenSandbox, ghostty, grit, codex-multi-auth; thêm browser/CONTRIBUTING.md, OpenSandbox/{server,sdks,specs}/AGENTS.md, ghostty/src/terminal/c/AGENTS.md, codex-multi-auth/{lib,test}/AGENTS.md. Không thấy AGENTS.md tại các thư mục cha đã kiểm tra hoặc tại đích báo cáo. rtk và temp-rs-ddd không có AGENTS.md trong kết quả tìm kiếm.

Skill đã áp dụng: rust-router, domain-cli, m07-concurrency và unsafe-checker. Chúng định hướng kiểm tra stdout/stderr, exit code, bounded channels, ownership và ABI. Đã đọc OpenAI Docs; không thực hiện bước tra cứu mạng của skill vì yêu cầu người dùng giới hạn local. Không dùng ICM recall/store trong Grit: audit chỉ dựa mã nguồn, không đọc kho memory ngoài phạm vi hay phát sinh ghi trạng thái.

Không dùng mạng, live account, Docker daemon, broker, database server hoặc subagent khác. Không build, không chạy test repo. Lý do: mục tiêu là khảo sát nguồn; browser/Ghostty cần toolchain/dependency lớn, còn các suite khác có thể ghi cache, khởi chạy dịch vụ hoặc tạo fixture. **Tất cả kết quả dưới đây là đọc nguồn và kiểm thử tĩnh; không phải chứng nhận test đã pass.** Các tình huống lỗi suy ra được ghi là suy luận, chưa tái hiện.

Revision lấy bằng git rev-parse HEAD; trạng thái bằng git status --short:

| Repo | Commit đầy đủ | Worktree khi khảo sát |
|---|---|---|
| browser | d693f49872fc72676baecfb3626b1876eadaf57d | Sạch; không có dòng status |
| OpenSandbox | eed301cca02b261256b2c5e5a23bb1a570c90160 | Sạch |
| ghostty | 44f2a44df7e8c4a0c6df3f7d872ef3d7ead88e51 | Sạch |
| rtk | 79347d5e0e20a61002b8dffe190d377846cb1e59 | Sạch |
| grit | 0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe | Sạch |
| temp-rs-ddd | 12398c3c6ebbc67998c4ac10a60332f281eeb793 | Sạch |
| codex-multi-auth | f71768223f8cc5d9375b115046b624ce49e7db55 | Sạch |

Trạng thái sạch không bao gồm kiểm toán nội dung ignored file, submodule hay provenance dependency. Không đọc credential store cá nhân. Các liên kết số ở cuối báo cáo trỏ vào checkout local và dòng bắt đầu của đoạn bằng chứng; phạm vi dòng được nêu bằng văn bản nếu cần. Khi checkout thay đổi, phải đối chiếu lại với commit trong bảng.

### Ma trận bao phủ

| Repo | Mã nguồn đã đọc theo đường chạy | Test đã đọc | Test thực thi | Chưa bao phủ |
|---|---|---|---|---|
| browser | main fetch, session/pages, navigate callbacks, CDP screenshot/emulation, Zig→Rust rasterizer | resolveWaitUntil; CDP screenshot và PNG dimensions | Không | Toàn bộ Web APIs, CDP/BiDi, SSRF policy, browser compatibility suite |
| OpenSandbox | Python create adapter → lifecycle route → Docker provisioning/container start → readiness/cleanup; middleware auth | security defaults, provisioning failure cleanup, cancellation, destroy, transport ownership | Không | execd/egress implementation, Kubernetes/FSB, snapshots, các SDK còn lại, isolation thực tế |
| ghostty | C exports → terminal VT write → render state → dirty rows/cells; mailbox | split escape/combining mark, begin/end render update | Không | GTK/macOS, GPU, đầy đủ PTY/shell integration, Rust wrapper chạy thật |
| rtk | CLI cargo test → CargoTestHandler → shared runner → stream → tee/retriever; token estimate | failure preservation, exit 42, capture cap, guard, successful-run storage policy | Không | Tất cả filter/hook, telemetry network path, recall store internals |
| grit | CLI claim/done → SQLite lock → worktree/rebase/merge → notification; symbol/dependency scan | read/write locks, separate-connection race, TTL | Không | Cloud lock backends, đầy đủ shell benchmark/harness, semantic resolver |
| temp-rs-ddd | HTTP health; Kafka routing/dispatch/ack/commit; stop/drain; connection container | application add; telemetry propagator tests | Không | Từng DB/client adapter, live Kafka, correctness của toàn bộ telemetry |
| codex-multi-auth | wrapper transport setup → proxy auth/selection/fetch/forward → usage ledger/budget; policy evaluation | budget unknown price, ledger redaction, policy load failure | Không | OAuth flow, account refresh, toàn bộ wrapper/storage suites; không kiểm chứng native Codex protocol |

## 3. browser / Lightpanda

### Luồng end-to-end đã truy

Đường fetch: main.fetchThread tạo Browser và gọi lp.fetch → tạo Notification và **một Session** → tạo Page cho từng URL → Frame.navigate tạo HTTP transfer với header/data/done/error callbacks → runner chờ → thu lỗi/status → writeResults. Có defer đóng session và browser. [1][2][3]

Điểm đáng chú ý: comment mô tả “fresh session” không có nghĩa mỗi URL được cô lập bằng một Session riêng. Mã tại lightpanda.zig:224 tạo session trước vòng lặp URL tại :245. Nhiều trang trong cùng lần fetch chia sẻ session/cookie jar. Không dùng batch URL cho những tác vụ yêu cầu cách ly danh tính/cookie. [2]

Đường screenshot: CDP Page.captureScreenshot → kiểm tra format PNG → resolve frame/viewport/clip → screenshot.preparePng → collect DOM thành LpBlock → Renderer của Rust → layout văn bản, tạo Pixmap và encode PNG → base64 trong CDP response. Rasterizer dùng block/span, font shaping và nền trắng; nguồn tự mô tả rõ text-only, còn render() cho thấy việc bố trí block và clamp kích thước raster. [4][5]

### Hành vi giới hạn và fallback

- PNG là output thật, không phải ảnh mẫu cố định. Tuy nhiên đây là ảnh nội dung văn bản, không bảo toàn bố cục CSS của website. Phải gắn metadata kiểu text_raster, tuyệt đối không dùng nhãn browser_visual_verification. [4]
- Format khác PNG trả lỗi -32000; quality chỉ cảnh báo rồi tiếp tục. Emulation.setEmulatedMedia, setFocusEmulationEnabled và setTouchEmulationEnabled trả kết quả thành công mà không thực hiện tương ứng. Capability negotiation phải dựa trên chức năng thực, không chỉ việc method được nhận. [5][6]
- fail_on_http_error mặc định false. Chờ wait_until có thể hết ngân sách rồi dump trạng thái hiện có; điều kiện selector/script chưa thỏa mới được mô tả là timeout. Có giá trị khi extraction best-effort, nhưng harness phải lưu riêng readiness, HTTP status và error. [2]
- wait_ms dùng ngân sách còn lại khi đi qua selector/script; đây là mẫu tốt để tránh nhân timeout theo số trang. Lỗi khởi tạo trong vòng tạo/navigate vẫn có try trả ra sớm; không nâng comment “mọi trang đều được ghi” thành bảo đảm tuyệt đối cho mọi loại lỗi. [2]

### Kiểm thử và quyết định

Đã đọc test resolveWaitUntil và CDP captureScreenshot: format không hỗ trợ, header PNG đúng width/height, cùng các test nhánh clip/scale được xác định trong file. Những test này không chứng minh rendering CSS đúng; chính đường rasterizer cho thấy mục tiêu khác. [5][7]

**Lấy:** adapter extraction DOM/Markdown, deadline chung, per-page evidence, cancellation và capability matrix. **Tránh:** nhúng browser làm nền tảng UI harness, coi CDP parity là đầy đủ, dùng screenshot này cho visual regression. Nguồn rasterizer có header AGPL-3.0-or-later; nếu sao chép/nhúng mã, cần xem xét giấy phép riêng, báo cáo này không kết luận nghĩa vụ phân phối. [4]

Contract đề xuất: BrowserEvidence gồm URL yêu cầu/cuối, session_scope, HTTP status, điều kiện ready, timed_out, engine_revision, artifact_kind, artifact_hash, unsupported_or_ignored_options. Một session chỉ phục vụ scope đã định; fallback sang engine khác phải tạo evidence mới có nhãn engine, không thay thế âm thầm.

## 4. OpenSandbox

### Luồng end-to-end đã truy

Sandbox.create của Python → SandboxesAdapter.create_sandbox chuyển model và gọi generated post_sandboxes → POST /sandboxes của FastAPI validate extension và gọi service → DockerSandboxService kiểm tra timeout/resource/network/platform → thread _provision_sandbox → container_ops tạo host_config/container, chuẩn bị execd runtime, start container → trả Running và đăng ký expiration → SDK resolve execd/egress endpoints song song → check_ready → trả Sandbox. [8][10][11][12]

Running ở control plane là container đã start. SDK readiness là bước riêng; skip_health_check bỏ bước này và nguồn có log cảnh báo sandbox có thể chưa sẵn sàng. Đây là ranh giới hữu ích để biểu diễn Provisioning/Running/Ready độc lập. [8][11]

### Cơ chế tái sử dụng

- Tách create/connect/close/kill/destroy; destroy gọi kill trong try và close trong finally. Lỗi hủy remote không bị biến thành thành công. [8]
- Nếu create đã nhận sandbox_id rồi lỗi ở endpoint/readiness, SDK bắt BaseException, thử kill sandbox, đóng owned transport và truyền lại CancelledError. Có test cancellation cụ thể cho trường hợp đã nhận ID nhưng đang chờ endpoint. [8][13]
- Container provisioning có cleanup container, sidecar, volume/mount và release reserved ports theo từng bước. Tests dùng Docker mock xác nhận cleanup khi chuẩn bị runtime lỗi. [11][12][14]
- Contract tài nguyên explicit: timeout, resource limits, network policy, platform, volumes. Docker từ chối lifecycle hooks và poolRef; không âm thầm giả lập tính năng Kubernetes. [11]

### Rủi ro và giới hạn

1. **close chỉ đóng tài nguyên local.** Khi timeout=None, nguồn nêu cần cleanup thủ công. Harness dùng sandbox phải có destroy/reconcile rõ ràng và deadline ngoài tiến trình; async Drop đơn thuần không đủ bảo đảm remote đã hủy. [8]
2. **Auth có chế độ bỏ qua.** Nếu không tenant provider và không API key, middleware cho request đi qua. Không phải mọi deployment đều không auth; đây là nhánh cấu hình cụ thể cần kiểm tra lúc kết nối. [9]
3. **Isolation có đánh đổi ở lớp container ngoài.** Docker với bootstrap.execd.isolation=enable thêm SYS_ADMIN, thay AppArmor và seccomp thành unconfined để bwrap hoạt động. Không bật extension này mặc định chỉ vì tên “isolation”. Cần kiểm thử negative capability và chọn profile đã được đánh giá. [11]
4. **Cancel trước khi client nhận ID chưa được test đã đọc bảo đảm.** Server tạo daemon thread provisioning; cancellation của await future không đồng nghĩa thread dừng. SDK chỉ cleanup theo ID đã biết. Suy luận: timeout/mất response có thể để lại sandbox chưa gắn với local run cho đến expiration/reconcile. Cần metadata run_id và bước tìm lại resource thay vì retry create mù. [8][11]
5. Spec network policy riêng cho FSB nói rõ trả “policy intent”, chưa chắc đã enforcement/convergence. Đây là cảnh báo trong contract; audit này chưa đọc implementation FSB để kết luận sâu hơn. [15]

**Lấy:** hợp đồng lifecycle và adapter tùy chọn cho lệnh build/test rủi ro. **Tránh:** nhập control plane/Kubernetes/pool vào đường chạy local bắt buộc; đặt một Codex account trong mỗi sandbox; mount toàn bộ trạng thái Codex/credential vào sandbox.

Contract đề xuất: SandboxLease chứa run_id, sandbox_id, provider, resource_profile, expires_at, ready_state, cleanup_state. Không đồng nhất với native_agent_id: một sandbox là môi trường tool execution, không phải một agent scheduler. Adapter cần tạo/hủy idempotent ở cấp harness bằng operation record và reconcile; chưa có bằng chứng audit này xác nhận native create API hỗ trợ idempotency key.

## 5. ghostty / libghostty-vt

### Luồng end-to-end đã truy

Symbol export ghostty_terminal_new/ghostty_terminal_vt_write nằm trong lib_vt.zig → terminal.new tạo Zig terminal và stream wrapper → vt_write đưa byte vào stream.nextSlice → render_state_begin_update chụp trạng thái cần thiết từ terminal → end_update hoàn tất phần việc chỉ dựa dữ liệu render state → get(row_iterator) gắn slice hàng/cell/dirty → row_iterator_next_dirty duyệt thay đổi → row_cells_get đọc dữ liệu cell → clean sau frame đã vẽ. [19][20]

Đây là engine VT và API phục vụ renderer tùy biến. Đường API đã đọc không tự tạo cửa sổ TUI Rust hoặc khởi chạy native subagent; harness vẫn cần transport/process/PTY và cách vẽ phù hợp. Ghostty có termio riêng trong ứng dụng, không nên nhập cả ứng dụng GUI để có terminal-native graph.

### Cơ chế nên lấy

- Parser giữ trạng thái qua chunk: test chia escape sequence và combining mark qua hai vt_write, so sánh plainString. Phù hợp stream output không theo ranh giới Unicode/CSI. [19]
- Render state hai pha rút ngắn thời gian giữ quyền truy cập terminal; có test xác nhận pending styles được hoàn tất và bold còn đúng. Dirty toàn frame và từng hàng độc lập; gọi clean chỉ khi đã vẽ thành công. [20]
- Mailbox dùng queue capacity 64. Khi đầy, đánh thức consumer và nhả mutex liên quan trước khi chờ push, tránh deadlock do consumer cần cùng mutex. Đây là cơ chế bounded transport đáng học cho event/UI. [27]

### Điều kiện tích hợp và giới hạn

Rust wrapper phải giữ opaque handle, free đúng một lần, bảo vệ truy cập terminal khi begin/update, giới hạn lifetime của row/cell view. get(row_iterator) cấp view trỏ vào memory của RenderState; không giữ view qua mutation/update hoặc free. Không gắn Send/Sync chỉ vì handle là con trỏ. ABI feature/export và header phải cùng revision. [19][20]

Mailbox là bounded queue nhưng không phải durable log: notify lỗi sẽ log và drop message; nhánh chờ forever cũng không có deadline toàn cục tại đây. Không sao chép cơ chế drop vào event hoàn thành task hoặc commit evidence. [27]

**Lấy:** mặc định chạy TUI của harness bên trong terminal thông thường, Ghostty là một terminal host khả dụng; chỉ thêm libghostty-vt khi cần pane hiển thị terminal con. **Tránh:** bắt buộc mọi người cài Ghostty, fork GUI, hoặc để PTY transcript quyết định trạng thái native agent. Output terminal chỉ là evidence; lifecycle agent phải lấy từ protocol native đã được xác minh riêng.

Contract đề xuất: TerminalPane nhận byte chunks và resize có sequence; sở hữu một terminal handle; sinh render snapshot có generation; input/responses đi qua transport explicit. Cancel pane/process và cancel native agent là hai hành động khác nhau.

## 6. rtk

### Luồng end-to-end đã truy

CLI Commands::Cargo/Test → cargo_cmd.run → run_test → run_cargo_streamed tạo Command bằng argv → BlockStreamFilter<CargoTestHandler> → core.runner.run_inner → stream.run_streaming spawn child, relay signal, đọc stdout/stderr → filter summary/failure → tee_and_hint → TimedExecution.track → trả exit code child. [16][17][18]

CargoTestHandler bỏ dòng test thành công, gom test-result và giữ failure detail. Test fixture có panic và kiểm tra tên test/panicked còn trong output. Bản chất đây là bộ lọc trình bày; không phải người phán quyết test pass. [16]

### Cơ chế nên lấy

- Command argv cho adapter cargo tránh phải tự nối shell string. Ngược lại wrapper err/test trong cmds/rust/runner.rs thực sự chạy sh -c/cmd /C; nếu tái sử dụng phải phân biệt API argv với API shell. [16]
- Exit code được truyền lại, có test exit 42; lỗi signal không bị xem là success trong nhóm test đã xác định. [17]
- never_worse fallback về raw khi estimate của filtered lớn hơn raw, có unit tests. Hữu ích cho token presentation, nhưng không kiểm chứng tính đầy đủ thông tin. [18]
- Recovery hint dẫn tới recall hash hoặc tee; giúp agent đọc ngắn trước rồi truy xuất phần thô khi cần. [18]

### Gaps ảnh hưởng bounded swarm

- raw_stdout/raw_stderr có RAW_CAP khoảng 10 MiB mỗi stream; quá giới hạn có cảnh báo và bỏ phần capture dư. “full output” được lưu sau bước capture không mặc nhiên là toàn bộ output gốc. Harness phải lưu truncation flag và dùng spool riêng nếu cần đầy đủ. [17]
- Streaming dùng std::sync::mpsc::channel không giới hạn, đồng thời filtered.push_str tích lũy nội dung. RAW_CAP không phải giới hạn RAM tổng. Consumer chậm hoặc một dòng cực dài vẫn là rủi ro; chỉ kết luận từ nguồn, chưa đo RAM. [17]
- Trong SQLite recovery mode, tee_and_hint bỏ qua exit 0 kể cả tee_on_success=true; test xác nhận không tạo DB cho run thành công. Disabled/config-load-failure/tiny output cũng có thể không tạo hint. Vì vậy không dùng RTK store thay evidence store của graph harness. [18]
- estimate_tokens dùng text.len()/4 làm tròn lên; len là byte UTF-8. Đây không phải tokenizer model hoặc usage native, sai lệch có thể lớn với tiếng Việt và Unicode. Tên metric nên là estimated_output_tokens/bytes_saved, không booked_codex_tokens. [18]
- ChildGuard Drop gọi wait, không cung cấp deadline kill ở đó. Harness vẫn phải có deadline/cancel/process cleanup độc lập. [17]

**Lấy:** adapter nén output với raw artifact riêng, preserve exit/signal, version filter và recover-on-demand. **Tránh:** global rewrite hook áp vào mọi lệnh Codex trước khi kiểm chứng; dùng text summary/emoji để quyết định task hoàn tất; dùng savings làm ngân sách tài khoản.

Contract đề xuất: ToolResult gồm argv, cwd, exit_code hoặc signal, start/end, stdout/stderr artifact hashes, captured/truncated, compact_text, filter_revision và estimate_method. Chính result này là bằng chứng kiểm thử; compact_text chỉ là view.

## 7. grit

### Luồng end-to-end đã truy

main → cli.run → cmd_claim validate read/write và symbols → ensure registry + resolve LockStore → tùy chọn mở rộng transitive dependencies thành read locks → try_lock từng symbol → SQLite BEGIN IMMEDIATE, xóa lock hết hạn, kiểm tra holder, INSERT/UPDATE và commit → tạo worktree agent → thông báo JSON qua Unix socket. [21][24][25]

Đường hoàn tất: cmd_done đọc locks → GitRepo.merge_worktree lấy merge.lock → kiểm tra main worktree dirty → rebase trong agent worktree, có fallback plain merge → nếu merge thành công thì xóa worktree/branch → nhả locks, dequeue/promote → phát AgentDone → cuối cùng trả lỗi nếu merge thất bại. [22][23]

### Cơ chế tốt đã có test

BEGIN IMMEDIATE bao quanh check-then-set chống hai process riêng cùng thấy lock trống; mutex Rust nội bộ một process không đủ. Test separate_connections mở 16 store/kết nối, assert đúng một writer và một row lock. Test này mô phỏng tranh chấp nhiều connection bằng thread; không phải đã chạy 16 process OS. Read/read, read/write và TTL có test riêng. [21][26]

Worktree/branch được giữ lại khi merge lỗi là cải tiến đáng lấy; serialize merge cùng guard worktree dirty giảm rủi ro mất thay đổi. [22][23]

### Lỗi/rủi ro suy ra từ nguồn

**G1 — Nâng lock vi phạm exclusivity.** SQLite try_lock:99 kiểm tra có row của chính agent rồi UPDATE mode và Granted trước khi kiểm tra locks của agent khác. Chuỗi tối thiểu: A read(S), B read(S), A write(S). Lần cuối đi nhánh cùng agent, B vẫn read trong DB. Transaction chống race không sửa sai invariant này. Tests read/write đã đọc không có kịch bản upgrade khi còn reader khác. Chưa chạy tái hiện. [21][26]

**G2 — Multi-symbol claim không atomic.** Claim lặp try_lock từng symbol. Khi không wait hoặc đến lượt cuối, granted vẫn được giữ dù một symbol blocked; tạo worktree rồi trả lỗi nếu không queue. Khi có queue, blocked có thể trả Ok vì đã xếp hàng. Nếu một try_lock lỗi giữa vòng lặp, toán tử ? cũng thoát với grants trước đó còn tồn tại. Chỉ nhánh retry mới cố release grants. Harness không được suy luận “claim trả lỗi = không sở hữu gì” hoặc “exit 0 = sở hữu đủ scope”. [24]

**G3 — Merge lỗi và retry done không nối liền.** done gọi release_all ngay cả khi merge thất bại rồi báo lỗi. Lần gọi tiếp theo kiểm tra locks.is_empty tại :896 và return Ok trước merge. Vì vậy thông điệp “sửa rồi run grit done again” ở merge guard không đủ khôi phục nếu không reacquire locks. Điều kiện là merge lỗi đã đi tới release_all, không phải mọi lỗi đầu hàm. [22][23]

**G4 — Event AgentDone có thể xuất hiện trước lỗi merge được trả.** Thông báo được gửi tại :966, lỗi tại :974. Room.notify bỏ qua lỗi kết nối/write; protocol không có sequence, ACK hoặc replay. Không dùng room socket làm source of truth cho completion. [22][25]

**G5 — Graph dependency là heuristic tên.** scan_with_deps map function_name → mọi symbol cùng tên và thêm edge cho mọi callee tương ứng, không resolve import/type/receiver. Symbol ID file::name còn có nguy cơ trùng tên method trong một file. Phù hợp gợi ý conflict scope, không phải semantic proof cho Rust. [25]

**G6 — Advisory locking và stale lock.** LockStore không chặn agent dùng filesystem trực tiếp. merge.lock dùng exclusive file create và PID liveness, fallback thời gian khi không xác định; vẫn cần xử lý owner identity/fencing thay vì sao chép nguyên cơ chế cleanup file. [21][23]

**Lấy:** worktree riêng cho write scope, SQLite transaction, explicit lock intent, serialize integration. **Tránh:** Grit queue/assign/session/watch trở thành scheduler thứ hai; tự merge branch ngay khi agent nói done; toàn bộ lock protocol chưa sửa invariant.

Contract đề xuất: WriteScopeLease {run_id, native_agent_id, repository_id, base_commit, scope, mode, expires_at, fencing_generation}. Acquire nhóm scope phải atomic hoặc trả danh sách partial explicit. MergeCandidate có branch/commit/evidence riêng và trạng thái merge_failed/review_ready/merged; không phụ thuộc việc lease còn tồn tại để retry integration. Với scope độc lập đã giao trước, worktree riêng và một parent integrator có thể đủ; chưa cần symbol lease cho mọi read.

## 8. temp-rs-ddd

### Luồng end-to-end đã truy

HTTP: run::http.start → interface HTTP router → health_router → get_health → ApiResponse::ok → IntoResponse JSON. Handler nhận _state nhưng không dùng; lời gọi HealthCheckUseCase chỉ là TODO. PgHealthyRepo có SELECT 1 thật nhưng chưa được nối vào đường health này. [28][32]

Kafka: runtime.start tạo ack channel 2048 và topic pipelines → consumer.recv chuyển MQMessage → topic queue → router chọn worker theo partition modulo worker_count → handler::dispatch → handler deserialize/log → Ok tạo CommitAck(next_offset=offset+1) → CommitState.record thay entry theo topic/partition → flush drain pending và commit Async. Đây là một đường đầy đủ từ input tới side effect commit, không chỉ sơ đồ DDD. [29][30]

### Cơ chế đáng học

Bootstrap/run/drain phân pha rõ; giữ connection handles trong AppState/Connections và dừng telemetry sau các server. Pipeline dùng bounded mpsc và cùng partition đi cùng worker, là mẫu backpressure/ordering dễ hiểu. Chỉ lấy nguyên tắc quản lý lifecycle và message result, không lấy broker runtime cho subagents. [30][32][33]

### Hành vi mẫu/không đầy đủ

- Health luôn ok; không phản ánh trạng thái PgHealthyRepo hoặc backend. Application crate chỉ export add và test 2+2; domain có test module rỗng. Telemetry có test thực nhưng các propagator test đã đọc chủ yếu xác nhận không panic, không kiểm chứng context đi xuyên hệ thống. [28][31][34]
- ApiResponse::fail chỉ đặt status trong JSON; IntoResponse trả Json(self), không map trường status sang HTTP status. Suy luận: dùng fail(503, ...) vẫn trả HTTP 200 nếu không có lớp khác sửa status. Không nên lấy response wrapper này làm mẫu error contract. [32]
- Unknown Kafka handler log “dropping message” rồi Ok; pipeline tiếp tục gửi ack. Đây là drop thành công thật trong production path, không chỉ test mock. [29][30]
- Offset n bị handler lỗi không ack, nhưng n+1 cùng partition vẫn được xử lý và ack n+2. CommitState chỉ giữ offset mới, không giữ gap thất bại. Suy luận: commit sau đó có thể bỏ qua n dù log nói “offset will not be committed”. Không có cơ chế retry/barrier/DLQ đã thấy ở đường này. [29][30]
- flush drain pending trước async commit; lỗi commit chỉ log. Chưa có bảo đảm giữ pending để thử lại hoặc xác nhận durable completion. [29]
- Dừng HTTP/WS/gRPC dùng JoinHandle.abort, bỏ kết quả await. Không phải graceful drain request đang chạy chỉ vì tên hàm drain. Kafka đợi worker nhưng không có timeout ở wait_all; runtime còn await topic_tx.send bên trong select branch, nên áp lực đầy queue có thể trì hoãn xử lý shutdown/ack. Đây là rủi ro cần test với consumer chậm, không kết luận deadlock đã tái hiện. [30][33]

**Lấy:** package boundaries vừa đủ cho domain graph, application commands, adapters và TUI; structured tracing; thứ tự dừng. **Tránh:** nhập toàn bộ AppState có Pg/Redis/Scylla/S3/Kafka/gRPC, health template, commit algorithm hoặc abort-as-drain.

Contract đề xuất: kết quả processing phải có Completed/Failed/Cancelled rõ; event completion chỉ commit khi evidence durable và dependency trước đó đáp ứng. Không bắt chước watermark Kafka bằng “offset lớn nhất đã thấy” cho graph task.

## 9. codex-multi-auth

### Luồng end-to-end đã truy

Wrapper codex.js chọn transport context → load proxy module → startRuntimeRotationProxy và tạo provider/shadow context ở nhánh tương ứng → client request tới loopback → authenticate client trước khi phân loại endpoint → policy/account selection → refresh token/header outbound → fetch có AbortController/timeout → forwardStreamingResponse chờ drain khi downstream chậm → scan usage → record ledger theo outcome. [35][36][37][38]

Không được mô tả mọi launch đều dùng shadow home: source hiện có nhánh app-server và interactive/resume dùng canonical home, vì shadow mirror gặp rắc rối app-server-control và snapshot thread index. Đây là ví dụ chi phí bảo trì phát sinh khi wrapper can thiệp state của official CLI. [35]

### Phần có thể lấy độc lập

- Ledger JSONL schema version, normalize field, hash account identifier, tạo directory 0700/file 0600, append queue và lock liên process. Có test đọc file raw để xác nhận không lưu email/account ID thô. Với harness một account, tốt hơn bỏ hẳn identity không cần thiết khỏi audit event. [39][40]
- Budget guard trả allowed/reasons theo requests/tokens/cost. Model không biết giá làm cost budget không thể đánh giá và bị chặn; test có case unpriced thực. Có thể lấy nguyên tắc Unknown ≠ Zero. [41]
- Stream forwarding có backpressure, timeout stall, cancel reader khi client đóng; có thể học kỹ thuật transport cho local adapter, không cần proxy Responses. [38]
- Test runtime policy unreadable assert 503 và zero upstream calls, là mẫu acceptance hữu ích cho policy lỗi không được bỏ qua. [42]

### Phần phải loại khỏi kiến trúc bắt buộc

Default codexRuntimeRotationProxy=true ở config.ts:192; runtime load AccountManager, chọn tài khoản theo scheduling strategy, refresh và rewrite outbound auth. Tất cả account pool, affinity, routing profile, model/account fallback, app-bind, shadow CODEX_HOME và refresh guardian nằm ngoài mục tiêu một account/native subagents. Không “đơn giản hóa” bằng cách dùng pool có đúng một phần tử: vẫn giữ proxy, credential plumbing và failure modes không cần thiết. [35][36][43]

Có fallback wrapper khi module/proxy không có hoặc khởi động lỗi: nếu không yêu cầu configured upstream, wrapper log rồi tiếp tục baseContext không rotation; configured-upstream branch có thể fail hard. Không lấy kiểu fallback này cho policy bắt buộc của harness: việc policy không hoạt động phải biểu diễn rõ, không chỉ log rồi chạy. [35]

Budget hiện đọc summary lịch sử rồi evaluate; ledger record ở cuối stream. Trong các đoạn đã truy không có atomic reservation cho chi phí của các request đang bay. Suy luận: nhiều subagents đồng thời có thể cùng thấy usage dưới cap rồi vượt cap sau đó. Append lock không đồng nghĩa lock bảo vệ check+reserve budget. Stale append lock chỉ kiểm tra mtime khi dọn; không kiểm tra owner PID tại bước đó. Cần thiết kế giới hạn pending/reservation riêng nếu muốn hứa hard bound. [37][39][41][44]

Các tests đã đọc dùng summary nhân tạo, temp directory, mock policy và recording fetch/SSE; không phải live account integration, không chứng minh tính tương thích native subagents. AGENTS.md có thống kê test/coverage nhưng báo cáo không dùng những con số đó như kết quả đã xác nhận.

**Lấy:** event schema, redaction tối thiểu, Unknown budget state, transport backpressure. **Tránh:** import package hoặc sử dụng wrapper/proxy làm cách chạy Codex mặc định. Native protocol cần được nhóm nghiên cứu Codex riêng xác minh; audit bảy repo này không tự đặt tên RPC rồi khẳng định nó tồn tại.

## 10. Hợp đồng tích hợp tối thiểu cho harness

Các cấu trúc dưới đây là **đề xuất mới**, không phải API đã tồn tại trong các repo. Chúng là adapter data contracts và trạng thái quan sát, không phải runtime scheduler cạnh tranh Codex.

| Ranh giới | Dữ liệu tối thiểu | Invariant/owner |
|---|---|---|
| Native Codex adapter | native session/thread/agent IDs, parent ID, capability/version, event ID, lifecycle/result/cancel state | Codex sở hữu agent lifecycle và dispatch; harness không sinh worker/model router độc lập |
| Run policy | max active descendants, max depth, deadline, bounded retry, output/storage ceiling; usage actual/estimated/unknown | Giới hạn được truyền qua native capability đã kiểm chứng; nếu capability thiếu thì không quảng bá hard enforcement |
| Graph context | repo revision, graph generation, edge provenance/confidence, source spans | Heuristic edge như Grit không được nâng thành semantic dependency chắc chắn |
| Write scope | repo/base commit, worktree, owner native_agent_id, scope, lease generation | Parent cấp scope và tích hợp; hết lease không chứng minh worker đã dừng |
| Tool evidence | argv/cwd, exit/signal, raw artifact refs, compact output, truncation, filter revision | Raw và structured outcome là nguồn chứng cứ; RTK là projection |
| Browser evidence | final URL/status, wait condition, capability flags, engine, artifact kind | text_raster khác browser_pixels; unknown/ignored option không được tính là đã kiểm thử |
| Sandbox lease | resource ID/profile, expiry/readiness, cleanup attempt/outcome, run metadata | Sandbox thuộc tool scope; create không đồng nghĩa ready, close không đồng nghĩa destroy |
| Terminal pane | chunk sequence, dimensions, render generation, owned VT handle | UI state không quyết định task success; output không tin cậy không được tự phát lệnh |
| Durable result | run/task/native IDs, artifact hash, evidence status, merge state, sequence | Completion task và integration success tách nhau; event delivery best-effort không phải durable ACK |

### Tự trị có giới hạn nhưng cùng native scheduler

Codex cha nhận goal và đồ thị phụ thuộc, dùng subagent native cho scope đã giao, rồi đọc kết quả/kiểm chứng theo cùng session. Harness có thể hiển thị DAG và ghi policy/evidence; không cần service tự polling một hàng đợi rồi tạo thêm agent ngoài Codex.

Giới hạn active/depth/retry phải được áp dụng vào toàn cây descendant, không chỉ danh sách trực tiếp hiển thị trong TUI. Nếu native runtime đã có giới hạn, dùng nó làm nguồn quyền lực; adapter chỉ kiểm tra capability/config và theo dõi. Nếu không có khả năng enforcing một loại giới hạn, báo rõ là soft/observed; không thay bằng account routing.

Token budget cần phân biệt usage thực và estimate output của RTK. Chỉ hứa hard token cap nếu native cung cấp công cụ thực thi tương ứng; event usage đến sau có thể tạo overshoot. Các giới hạn harness kiểm soát trực tiếp được là số tác vụ tool đang nhận, deadline/cancel được native xác nhận, số byte artifact và số retry operation có thể chứng minh an toàn.

Khi đạt terminal condition, parent dừng giao việc mới, yêu cầu cancel/close theo native protocol và reconcile tác vụ/resource chưa xác nhận dừng. Không đánh dấu toàn run hoàn tất chỉ vì đóng TUI hoặc process wrapper thoát.

### Hai luồng triển khai có giới hạn để tích hợp

**Luồng A — một task ghi mã trong native swarm.** Đây là trình tự do Codex cha thực hiện qua adapter; không có daemon dispatch hoặc hàng đợi worker thứ hai.

1. Parent chọn task có dependencies đã được kiểm chứng; chụp base commit/graph generation và scope. Kiểm tra giới hạn native active/depth cùng deadline run trước khi giao. Những giới hạn chưa được protocol bảo đảm phải ghi soft, không tự nhận hard bound.
2. Chuẩn bị worktree cho scope, ghi operation ID và candidate record; chỉ cấp quyền ghi sau khi đã nhận kết quả acquire đầy đủ. Nếu dùng lease, cấp fencing generation; tránh partial-grant ambiguity thấy ở Grit. [21][24]
3. Parent dùng native subagent, lưu native ID trả về. Nếu request giao việc mất response, reconcile bằng native session/event trước khi thử lại; không spawn lại mù vì có thể tạo agent trùng.
4. Lệnh kiểm chứng đi qua argv adapter; thu raw output vào artifact với byte quota, trả compact view có provenance. Khi queue đầy, backpressure; khi chạm quota/deadline, ghi partial/cancelled và yêu cầu dừng theo transport. Không áp dụng RAW_CAP của RTK như giới hạn bộ nhớ toàn hệ thống. [16][17][18]
5. Nhận native terminal result → kiểm tra artifact và exit/signal → tạo review-ready candidate. Integration chỉ do parent thực hiện theo scope đã được cho phép. Merge lỗi giữ candidate và worktree, độc lập với việc nhả lease; không phát success theo event AgentDone của Grit. [22][23]
6. Dừng run khi đạt mục tiêu hoặc bound; xác nhận descendants/resources đã dừng, reconcile trạng thái còn unknown. Retry chỉ cho operation có idempotency hoặc bằng chứng chưa thực hiện; số retry hữu hạn và mọi lần dùng chung deadline gốc.

**Luồng B — một tool task cần sandbox/browser.** Native agent vẫn do cùng Codex cha quản lý; sandbox không chạy account/agent manager riêng.

1. Gắn run_id/tool_operation_id vào yêu cầu resource, chọn profile CPU/RAM/PID/expiry và network policy đã được chấp nhận; không bật bwrap extension theo tên gọi. Ghi ý định create trước khi gọi để có điểm reconcile. [11]
2. Create → nhận resource ID → resolve endpoints → readiness trong phần thời gian còn lại. Running, Ready và policy-enforced là ba quan sát khác nhau. Nếu mất response trước ID, tìm resource theo metadata và trạng thái operation; không giả định create thất bại. [8][10][11][15]
3. Chạy lệnh/extraction với cùng deadline, output quota và cancellation; ghi HTTP status, readiness và artifact_kind. Screenshot Lightpanda gắn text_raster; task đòi browser pixels mà engine không có capability phải trả unsupported hoặc dùng adapter khác đã được lựa chọn rõ ràng. [2][4][6]
4. Thu artifact trước teardown nếu thời gian cho phép → destroy remote → close local trong finally. Hết hạn cleanup mà chưa xác nhận hủy thì ghi cleanup_pending và reconcile sau; không đổi thành completed-clean. Không dựa vào client close để chứng minh container đã dừng. [8][12][13]

Giá trị N, depth, deadline, retry và quota cần được cấu hình tập trung cho run rồi truyền xuống; các repo tham khảo không cung cấp sẵn bảo đảm tổng hợp đó. Acceptance I-02/I-03 phải đối chiếu với audit Codex do nhóm chính sở hữu, không suy ra protocol native từ codex-multi-auth.

## 11. Công việc đề xuất cho PLAN và acceptance tests

Đây là danh sách để agent tích hợp cập nhật kế hoạch; audit **không sửa PLAN.md**. Test dưới đây là việc cần làm trong implementation sau này, chưa được chạy.

| ID / ưu tiên | Công việc cụ thể | Điều kiện nghiệm thu |
|---|---|---|
| I-01 / P0 | Chốt ADR một account + native subagents; gỡ yêu cầu multi-account/model router | Dependency/config mặc định không chứa codex-multi-auth proxy, account pool hoặc broker scheduler; demo chạy không đọc/ghi credential ngoài cơ chế native |
| I-02 / P0 | Xác minh native protocol và cây lifecycle bằng source Codex thuộc scope khác | Fixture thật của protocol cho spawn/result/cancel/reconnect; IDs xuyên parent/descendant; nhận lại event không sinh duplicate task; unknown capability được báo rõ |
| I-03 / P0 | Áp dụng bound theo native capability và phân loại hard/soft | Với giới hạn N, thử N+1 descendant và nested spawn; không vượt hard bound; deadline không tạo công việc mới; cancellation phải có ACK/reconcile; không fallback tài khoản/model |
| I-04 / P0 | Tách task result, evidence validation và merge outcome | Agent trả “done” nhưng test exit≠0 hoặc merge lỗi thì run chưa Completed; đóng UI/event disconnect không đổi state thành thành công |
| I-05 / P0 | Bổ sung evidence store independent RTK | Output thành công cũng được lưu theo policy; output >10 MiB có truncation rõ; compact view có hash/raw ref; exit 42/signal giữ nguyên; không dùng byte/4 làm token usage thực |
| I-06 / P0 | Thiết kế bounded byte stream/backpressure cho tools/TUI | Fixture consumer chậm, một dòng cực dài, stderr flood, UTF-8/CSI chia chunk; memory/output queue không vượt ngân sách; cancel không kẹt trong send/wait |
| I-07 / P0 | Worktree/scope contract tối thiểu; sửa invariant nếu port Grit locks | A-read/B-read/A-write phải blocked; nhiều connection chỉ một writer; group acquire không để lại grants ngoài kết quả; lease expired không cho worker cũ ghi với fencing cũ |
| I-08 / P0 | Merge candidate durable và retry độc lập locks | Merge conflict giữ branch/worktree/evidence; nhả lease rồi retry integration vẫn tìm candidate; AgentDone không che merge_failed; không thay đổi worktree người dùng ngoài scope |
| I-09 / P1 | Browser capability và artifact classification | Fixture CSS grid/image không được “verify” bởi text PNG; emulated media/touch ignored phải surfaced; HTTP 500 và unmet selector không thành verified success; batch URL cách ly cookie khi yêu cầu |
| I-10 / P1 | Sandbox adapter tùy chọn, lifecycle reconcile | Fault injection cancel sau create/trước endpoint; mất response trước biết ID; destroy lỗi vẫn close local và giữ cleanup_pending; stale resource tìm được theo run metadata |
| I-11 / P1 | Profile sandbox explicit và kiểm chứng enforce | Không key/tenant provider phải được nhận diện; profile bwrap widening không bật mặc định; policy intent chưa enforced không được hiển thị “protected”; test này cần môi trường sandbox được cho phép ở giai đoạn sau |
| I-12 / P1 | Optional libghostty-vt pane với wrapper an toàn | new/write/update/read/free; split Unicode/escape, resize, dirty clean; view không sống qua update; allocation/error paths không double-free; ABI/header cùng revision |
| I-13 / P0 | Error/health completion hợp đồng thật | Health dependency chết → degraded/unavailable; HTTP error dùng status thật; unknown handler → Failed; message/event sau lỗi không làm mất predecessor chưa xử lý |
| I-14 / P1 | Durable event/usage ledger và ngân sách pending | Duplicate/replayed event không double count; malformed tail được nhận diện; unknown usage không thành 0; concurrent admission không vượt reservation ceiling thuộc lớp harness kiểm soát |
| I-15 / P1 | Reconcile khi restart và tài nguyên có owner | Kill harness giữa acquire/create/stream/merge; restart nối lại native IDs và candidate, không spawn trùng; không trộm lease còn owner; dọn đúng resource, giữ evidence lỗi |
| I-16 / P2 | Pin upstream và kiểm tra giấy phép trước reuse mã | Manifest ghi commit/feature/ABI/filter revision; license inventory theo file/package; adapter contract tests chạy lại khi bump commit |

Thứ tự triển khai nên là I-01→I-02→I-03/I-04/I-05, sau đó scope/integration recovery. Browser/sandbox/embedded terminal là adapter bổ sung sau khi vòng đời native và evidence đã đúng. Sản phẩm terminal đầy đủ không bắt buộc phải sao chép cloud storage Grit, Kafka template, GUI fork hoặc localhost Responses proxy.

## 12. Danh mục nguồn đánh số

Mỗi số dưới đây chỉ bằng chứng đã đọc. Các test source là tài liệu hành vi dự kiến; không đồng nghĩa test thực thi thành công.

Kiểm tra bàn giao: 99 liên kết source local đều tồn tại và số dòng nằm trong file; 44 nhóm nguồn. HEAD và git status của cả bảy repo được kiểm tra lại sau khi viết báo cáo, không thay đổi và vẫn sạch. Chỉ báo cáo được tạo; không sửa reference repo hoặc PLAN.md, không chạy build/test.

1. [browser/src/main.zig:358](/home/minh/projects/outsource/browser/src/main.zig:358) — fetchThread init/deinit Browser và gọi lp.fetch.
2. [browser/src/lightpanda.zig:186](/home/minh/projects/outsource/browser/src/lightpanda.zig:186) — FetchOpts; :211–255 session/pages, :299–378 waits/status/results, :383 JSON output.
3. [browser/src/browser/Frame.zig:878](/home/minh/projects/outsource/browser/src/browser/Frame.zig:878) — request callback wiring; [Frame.zig:1641](/home/minh/projects/outsource/browser/src/browser/Frame.zig:1641) — data callback MIME/sniff/HTML parse.
4. [browser/src/rust/render/lib.rs:19](/home/minh/projects/outsource/browser/src/rust/render/lib.rs:19) — text-only rasterizer contract; [lib.rs:587](/home/minh/projects/outsource/browser/src/rust/render/lib.rs:587) — FFI blocks; [lib.rs:1190](/home/minh/projects/outsource/browser/src/rust/render/lib.rs:1190) — block layout, raster clamp và vẽ.
5. [browser/src/server/cdp/domains/page.zig:1034](/home/minh/projects/outsource/browser/src/server/cdp/domains/page.zig:1034) — captureScreenshot; [page.zig:1691](/home/minh/projects/outsource/browser/src/server/cdp/domains/page.zig:1691) — PNG/format tests; [screenshot.zig:133](/home/minh/projects/outsource/browser/src/browser/screenshot.zig:133) — collect DOM blocks.
6. [browser/src/server/cdp/domains/emulation.zig:62](/home/minh/projects/outsource/browser/src/server/cdp/domains/emulation.zig:62) — media/focus no-op; [emulation.zig:148](/home/minh/projects/outsource/browser/src/server/cdp/domains/emulation.zig:148) — touch no-op.
7. [browser/src/lightpanda.zig:688](/home/minh/projects/outsource/browser/src/lightpanda.zig:688) — resolveWaitUntil unit test.
8. [OpenSandbox/.../sandbox.py:364](/home/minh/projects/outsource/OpenSandbox/sdks/sandbox/python/src/opensandbox/sandbox.py:364) — kill/close/destroy; [sandbox.py:432](/home/minh/projects/outsource/OpenSandbox/sdks/sandbox/python/src/opensandbox/sandbox.py:432) — readiness; [sandbox.py:537](/home/minh/projects/outsource/OpenSandbox/sdks/sandbox/python/src/opensandbox/sandbox.py:537) — create/endpoints/cancel cleanup.
9. [OpenSandbox/server/.../middleware/auth.py:68](/home/minh/projects/outsource/OpenSandbox/server/opensandbox_server/middleware/auth.py:68) — key configuration và bypass condition.
10. [OpenSandbox/.../sandboxes_adapter.py:129](/home/minh/projects/outsource/OpenSandbox/sdks/sandbox/python/src/opensandbox/adapters/sandboxes_adapter.py:129) — generated lifecycle call; [api/lifecycle.py:67](/home/minh/projects/outsource/OpenSandbox/server/opensandbox_server/api/lifecycle.py:67) — POST route/service.
11. [OpenSandbox/.../docker_service.py:630](/home/minh/projects/outsource/OpenSandbox/server/opensandbox_server/services/docker/docker_service.py:630) — validate/provisioning thread; [docker_service.py:914](/home/minh/projects/outsource/OpenSandbox/server/opensandbox_server/services/docker/docker_service.py:914) — isolation capabilities; :934–1026 create, cleanup, expiration.
12. [OpenSandbox/.../container_ops.py:410](/home/minh/projects/outsource/OpenSandbox/server/opensandbox_server/services/docker/container_ops.py:410) — create container; [container_ops.py:495](/home/minh/projects/outsource/OpenSandbox/server/opensandbox_server/services/docker/container_ops.py:495) — runtime/start/cleanup.
13. [OpenSandbox/.../test_sandbox_business_logic.py:604](/home/minh/projects/outsource/OpenSandbox/sdks/sandbox/python/tests/test_sandbox_business_logic.py:604) — cancellation stub test; [test_sandbox_destroy.py:101](/home/minh/projects/outsource/OpenSandbox/sdks/sandbox/python/tests/test_sandbox_destroy.py:101) — destroy order/error tests; [test_sandbox_close_and_connect_validation.py:41](/home/minh/projects/outsource/OpenSandbox/sdks/sandbox/python/tests/test_sandbox_close_and_connect_validation.py:41) — caller-owned transport.
14. [OpenSandbox/server/tests/test_docker_service.py:160](/home/minh/projects/outsource/OpenSandbox/server/tests/test_docker_service.py:160) — mock security defaults; [test_docker_service.py:411](/home/minh/projects/outsource/OpenSandbox/server/tests/test_docker_service.py:411) — runtime setup failure cleanup.
15. [OpenSandbox/specs/sandbox-lifecycle.yml:54](/home/minh/projects/outsource/OpenSandbox/specs/sandbox-lifecycle.yml:54) — FSB policy intent/enforcement distinction.
16. [rtk/src/main.rs:2518](/home/minh/projects/outsource/rtk/src/main.rs:2518) — Cargo dispatch; [cargo_cmd.rs:125](/home/minh/projects/outsource/rtk/src/cmds/rust/cargo_cmd.rs:125) — handler; [cargo_cmd.rs:331](/home/minh/projects/outsource/rtk/src/cmds/rust/cargo_cmd.rs:331) — argv/stream path; [cargo_cmd.rs:2763](/home/minh/projects/outsource/rtk/src/cmds/rust/cargo_cmd.rs:2763) — failure test; [cmds/rust/runner.rs:7](/home/minh/projects/outsource/rtk/src/cmds/rust/runner.rs:7) — shell-string wrapper.
17. [rtk/src/core/runner.rs:162](/home/minh/projects/outsource/rtk/src/core/runner.rs:162) — shared execution; [stream.rs:456](/home/minh/projects/outsource/rtk/src/core/stream.rs:456) — child guard/spawn; [stream.rs:504](/home/minh/projects/outsource/rtk/src/core/stream.rs:504) — channel/capture cap; [stream.rs:942](/home/minh/projects/outsource/rtk/src/core/stream.rs:942) — exit test; [stream.rs:1028](/home/minh/projects/outsource/rtk/src/core/stream.rs:1028) — 10 MiB test.
18. [rtk/src/core/tee.rs:31](/home/minh/projects/outsource/rtk/src/core/tee.rs:31) — recovery mode; [tee.rs:128](/home/minh/projects/outsource/rtk/src/core/tee.rs:128) — successful-run exclusion test; [guard.rs:17](/home/minh/projects/outsource/rtk/src/core/guard.rs:17) — never_worse; [tracking.rs:1657](/home/minh/projects/outsource/rtk/src/core/tracking.rs:1657) — byte/4 estimate.
19. [ghostty/src/lib_vt.zig:293](/home/minh/projects/outsource/ghostty/src/lib_vt.zig:293) — C exports; [terminal/c/terminal.zig:799](/home/minh/projects/outsource/ghostty/src/terminal/c/terminal.zig:799) — new/write; [terminal.zig:2927](/home/minh/projects/outsource/ghostty/src/terminal/c/terminal.zig:2927) — split sequence tests.
20. [ghostty/src/terminal/c/render.zig:214](/home/minh/projects/outsource/ghostty/src/terminal/c/render.zig:214) — update phases; [render.zig:313](/home/minh/projects/outsource/ghostty/src/terminal/c/render.zig:313) — iterator borrowed data; [render.zig:575](/home/minh/projects/outsource/ghostty/src/terminal/c/render.zig:575) — dirty rows; [render.zig:677](/home/minh/projects/outsource/ghostty/src/terminal/c/render.zig:677) — cells; [render.zig:1100](/home/minh/projects/outsource/ghostty/src/terminal/c/render.zig:1100) — update test; [render.h:30](/home/minh/projects/outsource/ghostty/include/ghostty/vt/render.h:30) — locking/clean contract.
21. [grit/src/db/sqlite_store.rs:48](/home/minh/projects/outsource/grit/src/db/sqlite_store.rs:48) — BEGIN IMMEDIATE và same-agent UPDATE trước conflict checks; [lock_store.rs:28](/home/minh/projects/outsource/grit/src/db/lock_store.rs:28) — advisory storage interface.
22. [grit/src/cli/mod.rs:890](/home/minh/projects/outsource/grit/src/cli/mod.rs:890) — empty-lock early return; :913 merge; :951 release; :966 AgentDone; :974 failure.
23. [grit/src/git/mod.rs:182](/home/minh/projects/outsource/grit/src/git/mod.rs:182) — merge lock, dirty guard, rebase fallback; [git/mod.rs:278](/home/minh/projects/outsource/grit/src/git/mod.rs:278) — lock liveness/time heuristic.
24. [grit/src/cli/mod.rs:460](/home/minh/projects/outsource/grit/src/cli/mod.rs:460) — claim flow; :523 per-symbol locks; :546 finalization; :612 exit semantics; :624 retry release.
25. [grit/src/parser/mod.rs:100](/home/minh/projects/outsource/grit/src/parser/mod.rs:100) — name-based call edges; [parser/mod.rs:357](/home/minh/projects/outsource/grit/src/parser/mod.rs:357) — file::name; [room/mod.rs:33](/home/minh/projects/outsource/grit/src/room/mod.rs:33) — best-effort notify.
26. [grit/src/db/sqlite_store.rs:451](/home/minh/projects/outsource/grit/src/db/sqlite_store.rs:451) — separate connections race test; [sqlite_store.rs:516](/home/minh/projects/outsource/grit/src/db/sqlite_store.rs:516) — read/write tests; [sqlite_store.rs:378](/home/minh/projects/outsource/grit/src/db/sqlite_store.rs:378) — TTL test.
27. [ghostty/src/termio/mailbox.zig:11](/home/minh/projects/outsource/ghostty/src/termio/mailbox.zig:11) — capacity 64; [mailbox.zig:63](/home/minh/projects/outsource/ghostty/src/termio/mailbox.zig:63) — unlock, wakeup, drop và blocking push.
28. [temp-rs-ddd/.../health_handle.rs:15](/home/minh/projects/outsource/temp-rs-ddd/crates/interface/src/http/handle/health_handle.rs:15) — fixed ok/TODO; [pg_healthy_repo.rs:19](/home/minh/projects/outsource/temp-rs-ddd/crates/infrastructure/src/repository/pg_healthy_repo.rs:19) — SELECT 1.
29. [temp-rs-ddd/.../mq/handler.rs:41](/home/minh/projects/outsource/temp-rs-ddd/crates/interface/src/mq/handler.rs:41) — unknown handler Ok; [topic01_handler.rs:7](/home/minh/projects/outsource/temp-rs-ddd/crates/interface/src/mq/handler/topic01_handler.rs:7) — deserialize/log; [commit.rs:22](/home/minh/projects/outsource/temp-rs-ddd/crates/cli/src/server/run/kafka_consumer/commit.rs:22) — replace/drain/async commit.
30. [temp-rs-ddd/.../kafka_consumer/runtime.rs:31](/home/minh/projects/outsource/temp-rs-ddd/crates/cli/src/server/run/kafka_consumer/runtime.rs:31) — ack/select/shutdown; [pipeline.rs:42](/home/minh/projects/outsource/temp-rs-ddd/crates/cli/src/server/run/kafka_consumer/pipeline.rs:42) — bounded queues/dispatch/ack/partition routing.
31. [temp-rs-ddd/crates/application/src/lib.rs:1](/home/minh/projects/outsource/temp-rs-ddd/crates/application/src/lib.rs:1) — add/test placeholder.
32. [temp-rs-ddd/.../run/http.rs:9](/home/minh/projects/outsource/temp-rs-ddd/crates/cli/src/server/run/http.rs:9) — server start; [http/router.rs:18](/home/minh/projects/outsource/temp-rs-ddd/crates/interface/src/http/router.rs:18) — route composition; [health_router.rs:7](/home/minh/projects/outsource/temp-rs-ddd/crates/interface/src/http/router/health_router.rs:7) — health route; [response.rs:28](/home/minh/projects/outsource/temp-rs-ddd/crates/interface/src/http/response.rs:28) — body status/IntoResponse.
33. [temp-rs-ddd/.../server/run.rs:22](/home/minh/projects/outsource/temp-rs-ddd/crates/cli/src/server/run.rs:22) — abort/wait handles; [shutdown.rs:15](/home/minh/projects/outsource/temp-rs-ddd/crates/cli/src/server/shutdown.rs:15) — resource stop order; [connections.rs:12](/home/minh/projects/outsource/temp-rs-ddd/crates/infrastructure/src/connection/connections.rs:12) — backend coupling.
34. [temp-rs-ddd/.../trace/propagator.rs:6](/home/minh/projects/outsource/temp-rs-ddd/crates/telemetry/src/trace/propagator.rs:6) — install; :40–56 smoke tests.
35. [codex-multi-auth/scripts/codex.js:5288](/home/minh/projects/outsource/codex-multi-auth/scripts/codex.js:5288) — canonical-home cases; :5324–5395 proxy/shadow/fallback/cleanup.
36. [codex-multi-auth/lib/runtime-rotation-proxy.ts:853](/home/minh/projects/outsource/codex-multi-auth/lib/runtime-rotation-proxy.ts:853) — load account manager, loopback; [proxy.ts:1067](/home/minh/projects/outsource/codex-multi-auth/lib/runtime-rotation-proxy.ts:1067) — auth before routing; [proxy.ts:1431](/home/minh/projects/outsource/codex-multi-auth/lib/runtime-rotation-proxy.ts:1431) — chooseAccount.
37. [codex-multi-auth/lib/runtime-rotation-proxy.ts:1638](/home/minh/projects/outsource/codex-multi-auth/lib/runtime-rotation-proxy.ts:1638) — outbound headers/fetch deadline; [proxy.ts:2027](/home/minh/projects/outsource/codex-multi-auth/lib/runtime-rotation-proxy.ts:2027) — usage scanner/forward; [proxy.ts:2087](/home/minh/projects/outsource/codex-multi-auth/lib/runtime-rotation-proxy.ts:2087) — post-stream ledger outcome.
38. [codex-multi-auth/lib/request/stream-failover-runtime.ts:143](/home/minh/projects/outsource/codex-multi-auth/lib/request/stream-failover-runtime.ts:143) — reader cancellation, timeout, res.write backpressure.
39. [codex-multi-auth/lib/usage/ledger.ts:103](/home/minh/projects/outsource/codex-multi-auth/lib/usage/ledger.ts:103) — stale lock mtime; :136 append lock; [ledger.ts:261](/home/minh/projects/outsource/codex-multi-auth/lib/usage/ledger.ts:261) — normalization/append; [redaction.ts:86](/home/minh/projects/outsource/codex-multi-auth/lib/usage/redaction.ts:86) — hashed identifiers.
40. [codex-multi-auth/test/usage-ledger.test.ts:27](/home/minh/projects/outsource/codex-multi-auth/test/usage-ledger.test.ts:27) — raw storage redaction test.
41. [codex-multi-auth/lib/budget-guard.ts:216](/home/minh/projects/outsource/codex-multi-auth/lib/budget-guard.ts:216) — guard; [test/budget-guard.test.ts:82](/home/minh/projects/outsource/codex-multi-auth/test/budget-guard.test.ts:82) — unknown-price refusal.
42. [codex-multi-auth/test/runtime-rotation-proxy.test.ts:511](/home/minh/projects/outsource/codex-multi-auth/test/runtime-rotation-proxy.test.ts:511) — policy unreadable, 503, no fetch.
43. [codex-multi-auth/lib/config.ts:192](/home/minh/projects/outsource/codex-multi-auth/lib/config.ts:192) — rotation proxy default true.
44. [codex-multi-auth/lib/policy/runtime-policy.ts:121](/home/minh/projects/outsource/codex-multi-auth/lib/policy/runtime-policy.ts:121) — historical ledger budget evaluation, không reserve in-flight tại bước này.
