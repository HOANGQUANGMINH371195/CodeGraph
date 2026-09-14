# Kiểm toán runtime tham chiếu cho Rust terminal-native graph harness

Ngày khảo sát: **2026-09-11**. Phạm vi được giao: `ruflo`, `opendev`, `orca`, `t3code` trong `/home/minh/projects/outsource`.

## 1. Kết luận thiết kế

Hướng phù hợp là **một tài khoản Codex, một phiên điều phối chính dùng native subagents; Rust giữ đồ thị công việc, bằng chứng, giới hạn và giao diện terminal**. Không đưa account pool, model router, Queen/Raft, hay một ReAct executor thứ hai vào kiến trúc bắt buộc.

Trong bốn repo, T3 Code cung cấp tham chiếu trực tiếp nhất cho việc bọc Codex app-server và quan sát child thread native. Orca bổ sung bài học về mất sự kiện, trạng thái chưa xác minh và vòng đời terminal. Ruflo có contract ownership/source receipt đáng học nhưng lệnh đăng ký swarm không tạo executor. OpenDev có mã Rust/TUI hữu ích, song runtime subagent của nó tự gọi HTTP và tự chạy ReAct; dùng nguyên khối sẽ đi ngược thay đổi thiết kế.

Năm phát hiện quan trọng nhất:

1. **T3 phân biệt child thread với turn và với parent timeline.** Child hoàn tất một turn trở thành `idle`, có thể tiếp tục; đọc kết quả không đồng nghĩa child bắt đầu chạy lại. Đây là mô hình nên đưa vào graph. [24][25]
2. **OpenDev có chênh lệch giữa contract hiển thị và thực thi:** `task_id` được quảng bá là resume nhưng đường đã truy chỉ tái dùng ID, dựng messages mới; background Kill dùng token khác token truyền vào runner. Có bằng chứng tĩnh mạnh, chưa tái hiện bằng chạy test. [9][10][13]
3. **Orca transcript observer là kênh có thể mất dữ liệu.** Nó đọc tối đa phần đuôi 1 MiB, bỏ dòng quá 256 KiB, và loại child sau hơn 60 giây không đọc được rollout. Không dùng việc child biến mất khỏi roster làm bằng chứng task thành công. [17][18]
4. **Ruflo phân biệt đăng ký với thực thi, nhưng `agent_execute.success` chỉ chứng minh API trả lời.** Đường này không gửi tool schema, không có vòng thực thi công cụ, không tạo patch; không thể làm oracle hoàn tất node. [2][3][4]
5. **Giới hạn phải bao trùm vòng đời thật.** Semaphore của một batch spawn không giới hạn background task đã tách ra; gửi interrupt không chứng minh worker đã dừng; queue event unbounded không bảo đảm RAM hữu hạn. Cần contract native-capability rõ và kiểm thử fault injection trước khi gọi sản phẩm là bounded swarm. [14][23][26]

## 2. Phương pháp, revision và giới hạn bằng chứng

Đọc source bằng `rg`, `nl -ba`, `sed`; kiểm tra revision bằng `git rev-parse HEAD`, trạng thái bằng `git status --short`. Không cài dependency, không build, không chạy binary agent, không dùng tài khoản thật, không truy cập mạng, không gọi subagent khác. Không sửa repo tham chiếu hoặc `PLAN.md`; artifact duy nhất được ghi là báo cáo này.

| Repo | HEAD tại thời điểm khảo sát | Local dirty status |
|---|---|---|
| ruflo | `a64f8b1ad89035c8b204f8b6e0893a2288551e06` | Sạch; `git status --short` rỗng |
| opendev | `d32c660e4eed1a8e988d1fd58da88e41ba641d08` | Sạch; `git status --short` rỗng |
| orca | `26f9fd8ea152ad6126c005e5ae602c7d201f4a99` | Sạch; `git status --short` rỗng |
| t3code | `3836890e4484406813997259efc69065b25ce698` | Sạch; `git status --short` rỗng |

“Sạch” ở đây là kết quả Git thông thường, không phải chứng nhận mọi ignored/build input. Các liên kết dưới đây trỏ đến checkout cục bộ; revision trong bảng là mốc để tái lập line evidence.

Đã đọc hướng dẫn root của cả bốn repo, hướng dẫn `ruflo/v3/@claude-flow/codex`, `orca/src/main/daemon`, `orca/tests`, `orca/tests/e2e`. Không tìm thấy `AGENTS.md` tại các thư mục cha workspace và thư mục đích đã kiểm tra. Yêu cầu user về khảo sát chỉ đọc được ưu tiên: không thực hiện workflow Ruflo ghi memory/swarm, hoặc workflow OpenDev build release/gọi LLM dành cho sửa feature. [1]

Phạm vi báo cáo nhánh này là mã nguồn cục bộ; đối chiếu tài liệu Codex chính thức nằm trong báo cáo Codex riêng. Báo cáo chỉ kết luận về implementation tại các revision trên, **không xác nhận hỗ trợ của Codex đang cài, quota tài khoản, hay API chính thức hiện hành**.

### Coverage và kiểm thử

| Repo | Source thực tế đã kiểm tra | Test source đã đọc | Test thực thi |
|---|---|---|---|
| ruflo | MCP swarm/agent handlers, HTTP execute core, fenced lease reference, exact repository-state capture | Lease conflict/CAS/expiry, dirty/untracked identity, model alias tests | **0** |
| opendev | Spawn tool, manager/runner, ReAct HTTP/parallel phases, progress channel, TUI lifecycle/background Kill, InterruptToken | Spawn validation/recursive denial, bridge events, runner name, background Kill state | **0** |
| orca | Hook ingress → normalization → transcript/roster → status store/poll, endpoint publication, terminal create/attach guard, producer flow control | Rollout lifecycle/unreadable timeout, hook-server integration, endpoint ownership | **0** |
| t3code | Codex runtime start/resume/turn/child routing/interrupt, adapter task mapping, ingestion, transactional engine | Mock app-server integration, interrupt/queued-turn cases, transaction rollback và retry receipt | **0** |

Đây là khảo sát chọn đường chạy trọng yếu, không phải audit toàn bộ dòng mã. Tên test được tìm bằng search chỉ dùng để định vị; các kết luận về test chính dựa trên body đã đọc. Không nói test “pass”. Đặc biệt test T3 collab ghi `.collab-script.json` và `.requests` ngay trong thư mục fixture của repo, nên không chạy trực tiếp trong checkout tham chiếu. [27]

## 3. Ruflo: lấy contract bằng chứng, không lấy executor/router

### 3.1. Luồng đầu-cuối đã truy

Luồng phối hợp: `swarm_init.handler` → validate topology/clamp `maxAgents` → tạo `SwarmState(status=running, agents=[])` → `saveSwarmStore` ghi JSON qua temporary file + rename → trả `persisted: true`. `agent_spawn` ghi `AgentRecord(status=idle)` vào agent store, bổ sung ID vào swarm store, trả `status: registered` cùng ghi chú cần executor riêng. Không có native Codex child được tạo trong chuỗi này. [2][3]

Luồng thực thi bổ sung: `agent_execute.handler` → validate input/pheromone eligibility → `executeAgentTask` → đọc agent record → đặt busy/tăng taskCount → gọi provider HTTP → idle/lastResult/save → trả output/usage. Với nhánh Anthropic, payload chỉ có system và user message; response text được ghép lại. Không có tool-dispatch loop trong đường này. Thực thi HTTP là mã thật, nhưng “agent” ở đây chưa tương đương coding worker native. [3][4]

### 3.2. Cơ chế nên học

- **Exact source identity:** clean state gắn commit/tree/repository ID; dirty state gắn tracked patch, manifest untracked và submodule state. Capture dirty hai lần rồi so sánh để phát hiện checkout đổi trong lúc đọc. Test thay từng byte và mode của untracked file cho ra identity khác. Đây là cơ sở tốt cho receipt của node và handoff. [5][7]
- **Lease có epoch/version và scope:** đường dẫn tương đối được chuẩn hóa, bắt xung đột cha/con và portable case-fold; renew tăng version, từ chối fence cũ, expiry và lease không tồn tại. Khi port sang Rust, giữ invariant này cho quyền ghi tài nguyên cục bộ do harness quản lý. [6][8]
- **Ghi rõ mức bảo đảm:** `referenceOnly=true`, `advisory=true`; class lease tự mô tả là unsigned, non-durable, single-process. Học cách công khai giới hạn thay vì gọi reference adapter là authority production. [6]

### 3.3. Rủi ro và hành vi dễ bị hiểu sai

**R1 — success yếu.** Execute core tự ghi “success = model returned without API error”; transcript tùy chọn đóng dấu `resolvedSource: api-success`. Không suy luận đã sửa đúng lỗi hoặc test đã qua từ trường `success` này. Các quality/reward downstream cũng cần phân biệt evidence API và validation. [4]

**R2 — atomic rename chưa phải atomic transaction nhiều writer.** `swarm_init` gọi load-modify-save không qua `withSwarmStoreLock`; `agent_spawn` cập nhật agent store rồi swarm store riêng. Hai process có thể đọc cùng snapshot và ghi đè cập nhật nhau; crash giữa hai store để lại registry lệch. Đây là suy luận từ chuỗi gọi, chưa race-test. Lock helper có tồn tại nhưng không bao trùm các đường trên. [2][3]

**R3 — read có side effect và fallback im lặng.** `loadSwarmStore` bắt lỗi JSON/read và dùng store rỗng; reconcile PID/TTL có thể đổi trạng thái rồi save. “Status” không thuần read-only. Không gọi CLI này chỉ để quan sát checkout trong audit. Khi áp dụng, corruption phải thành lỗi recoverable/needs-repair, không lặng lẽ thành graph rỗng. [2]

**R4 — source fingerprint chưa là snapshot bất biến.** `scope: git-visible` cố ý loại ignored inputs; đọc hai lần chỉ phát hiện một lớp race, không khóa filesystem xuyên suốt execution. Task acceptance cần thêm manifest input/toolchain hoặc snapshot/worktree thích hợp; không coi cùng HEAD là cùng trạng thái dirty. [5][7]

**R5 — test “CP authority” chỉ chứng minh reference single-process.** Test 100 acquisition dùng `Array.from` đồng bộ, không 100 process cạnh tranh; test CAS có giá trị về semantics nhưng không chứng minh durability hay distributed linearizability. [8]

### 3.4. Adoption và avoid

Adopt: source-state receipt, scope overlap, stale-fence rejection, phân biệt registered/running/validated. Avoid: runtime `agent_execute`, model/provider fallback routing, pheromone/consensus stack, file-JSON registry làm nguồn sự thật cho concurrent writer. Không thêm Ruflo daemon như dependency bắt buộc.

## 4. OpenDev: Rust/TUI hữu ích; runtime song song và resume cần cảnh giác

### 4.1. Luồng đầu-cuối đã truy

`SpawnSubagentTool::execute` đọc agent_type/task, chặn `ctx.is_subagent`, kiểm tra type/working dir, tạo session ID và instance UUID, lấy child cancellation token. Nhánh synchronous gọi `SubagentManager::spawn`; manager chọn spec/model, lọc tools, chèn working directory/instruction files, dựng `LlmCaller`, đặt `ToolContext.is_subagent=true`, chọn runner và tạo messages system + user. [9][10]

`StandardReactRunner::run` → `ReactLoop::run/run_inner` → `execute_llm_call` → `AdaptedClient.post_json_streaming/post_json`; kết quả đi qua response/tool phases, registry thực thi tool; progress qua `SubagentEventBridge` → `ChannelProgressCallback` → mpsc → TUI cập nhật active subagent. Khi return, spawn tool chuyển messages sang session history, save và trả output cho parent. Đây là **runtime tự gọi LLM thật**, không phải wrapper native Codex. [10][11][12][15]

Nhánh background tạo `tokio::spawn`, trả task ID ngay, chạy cùng manager và gửi `BackgroundCompleted`. Chính việc tách task này làm thời gian sống worker khác thời gian sống lời gọi spawn tool. [9][14]

### 4.2. Cơ chế nên học

- Dùng instance ID riêng cho nhiều agent cùng role; gắn tool ID và child cancellation token vào event. Tuy nhiên nên đưa luôn parent tool ID vào event đầu tiên, tránh TUI phải ghép bằng task text. [9][12][15]
- Một nguồn phát terminal result: callback `on_finished` cố ý no-op, spawn tool phát `Finished` có tool-call count và shallow warning. Có test kiểm tra không phát Finished trùng. [12][16]
- Runner theo loại workload có max steps: Explore mặc định 100 và timeout 600 giây; runner thường mặc định 25 iterations. Học ý tưởng giới hạn rõ, không mang vòng ReAct này sang harness Codex. [10]
- `ParallelPolicy::partition_with_tools` giữ barrier của operation ghi, cho nhóm read-safe liền kề chạy cùng batch. Dùng được cho công việc nội bộ harness như scan/fingerprint; không dùng để xếp lại tool calls thuộc runtime Codex. [14]

### 4.3. Phát hiện mới có bằng chứng source

**O1 — resume chưa phục hồi context trên đường đã truy (mức tin cậy cao từ đọc tĩnh).** Schema nói `task_id` tiếp tục phiên cũ; execute chỉ chọn lại `child_session_id`. Manager không nhận history/session ID và luôn dựng messages mới. Cuối cùng tạo `Session::new`, gán cùng ID rồi save. Không thấy `load_session` trong spawn tool. Vì vậy không thể gọi đây là conversation resume; còn có nguy cơ thay nội dung phiên cùng ID bằng lịch sử lượt mới, cần test riêng nếu muốn tái sử dụng. [9][10]

**O2 — Kill background không nối đến runner token trong đường này (mức tin cậy cao từ đọc tĩnh).** `spawn_background` giữ `params.cancel_token` để truyền manager; đồng thời tạo `InterruptToken::new()` độc lập, chỉ gửi clone cho TUI. `kill_task` request token TUI rồi đặt `Killed`, còn `InterruptToken::new` tự tạo CancellationToken khác. Không có cầu nối hai token ở function đã đọc. Parent cancellation vẫn có đường truyền; không đánh đồng nó với Kill riêng task. Test `test_kill_task` chỉ kiểm token của manager/TUI và enum Killed, không chạy worker để chứng minh worker dừng. [9][13][16]

**O3 — số 0 không phải phép đo chi phí.** Standard runner truyền `None` cho cost tracker; cả success/error background event điền `cost_usd: 0.0`. Harness phải biểu diễn `Unknown`/`None`, kèm nguồn token usage khi có, không dùng zero này làm điều kiện cho tiếp tục swarm. [9][11]

**O4 — giới hạn 25 chỉ ở batch dispatch.** Parallel phase tạo semaphore mới mỗi lần gọi và giữ permit quanh `tool_registry.execute`. Background spawn trả về trước khi worker xong, nên permit được nhả trong khi worker sống; nhiều batch có thể vượt giới hạn active workers được tưởng tượng từ con số 25. Chặn recursion không giải quyết vấn đề lifetime này. [14]

**O5 — nhánh backgrounding có thể tạo thông báo thành công không dựa trên receipt.** Khi cancellation thắng `join_all` và task monitor báo background requested, code chèn “Agent spawned successfully. Running independently in the background.” cho mọi tool call. Future sync/đang đợi semaphore có thể bị drop mà chưa spawn; chuỗi text đó không phải bằng chứng đã detach. Không port logic này. Chưa chạy tái hiện. [14]

**O6 — lỗi persistence bị bỏ qua và identity TUI có fallback heuristic.** Save session dùng `let _ = ...`; TUI tìm placeholder theo task text hoặc pending background task. Hai prompt giống nhau có thể tạo attribution mơ hồ. Channels là unbounded và send error bị bỏ qua. Contract mới cần durable result/error, parent-child correlation đầy đủ và queue có giới hạn. [9][12][15]

Nhánh thường còn truyền `None` cho `tool_approval_tx` khi gọi manager, dù runner có khả năng nhận channel; không nên suy luận delegation kế thừa approval đầy đủ chỉ từ việc prompt có instruction files. Đây là observation cục bộ, không kết luận mọi entry point OpenDev bypass quyền. [9][11]

### 4.4. Adoption và avoid

Adopt: event enums/correlation, cancellation tree, TUI dirty update, output truncation có cảnh báo, kiểm tra shallow result như tín hiệu phụ. Avoid: tự host ReAct/HTTP, model override/router, background task manager làm authority thực thi, gọi reuse-ID là resume, dùng UI Killed làm chứng nhận process exit. Test runner name chỉ là smoke test cấu trúc; test bridge dùng callback giả lập, không phải end-to-end LLM. [11][16]

## 5. Orca: quan sát native subagents và vòng đời terminal

### 5.1. Luồng đầu-cuối đã truy

HTTP hook đi vào `server-lifecycle`: đọc body, resolve source, normalize pane alias/body → `normalizeLocalHookPayload` → Codex normalizer. Parent hook có transcript_path gọi `reconcileCodexSubagentTranscript`: đọc event_msg/sub_agent_activity, resolve child JSONL trong thư mục ngày của parent hoặc ngày child khởi chạy, đọc child task_started/task_complete, cập nhật roster. `codexRosterEffectiveState` giữ aggregate working nếu lead done mà vẫn có child; child waiting được ưu tiên. [17][18][19][20]

Kết quả → `applyNormalizedStatus` → state theo pane, schedule persistence, listener fanout → `scheduleCodexSubagentPoll`. Poll kiểm tra identity object hiện tại, normalize lại, chỉ apply khi subagents đổi, rồi re-arm bằng snapshot mới. Nhờ vậy child vẫn được theo dõi khi không phát lifecycle hook riêng. [20][21]

Test integration dựng JSONL tạm, khởi động hook server thực trên loopback, POST hook, kiểm snapshot có child, append task_complete rồi chờ roster hết child. Đây là integration của observer với file/HTTP thật và dữ liệu giả lập, không gọi Codex account. Không chạy trong audit. [22]

### 5.2. Cơ chế nên học

- **Observer native không cần scheduler agent:** roster phát hiện child từ runtime evidence; model metadata lấy từ child rollout, không lấy model parent để bịa cho child. Metadata update không tạo lại child đã kết thúc. [17][18]
- **Một nguồn trạng thái authoritative trên host thực thi:** normalize rồi fanout từ status store; reader không tự tạo precedence khác nhau. Có timing/observation stamp và nhận diện restored-unconfirmed. Dùng graph projection trong Rust theo cùng nguyên tắc. [21]
- **Coalesce poll bằng một timer:** deadline từng pane dùng monotonic clock; generation guard bỏ callback stale; callback có thể cancel sibling trước khi sibling chạy. Đây là scheduling cho I/O quan sát, không phải scheduler cạnh tranh với native subagents. [19]
- **Endpoint ownership:** bind tên riêng, thử hard-link exclusive, chỉ thay incumbent khi probe chứng minh dead; kiểm identity lại và probe lần hai trước rename; confirm ownership sau publish. Timeout/unknown không được gọi là dead. Khi không hỗ trợ hard-link có fallback vẫn đòi death proof và confirm. Windows named pipe là nhánh riêng. [23]
- **Terminal backpressure:** producer controller có high/low watermark 256 Ki/32 Ki ký tự, hysteresis, reassert pause sau 5 giây, release khi teardown. Ý tưởng phù hợp output terminal nhiều worker nhưng channel control/approval cần độc lập với output flood. [23]

### 5.3. Gaps và fallback

**C1 — roster không phải durable task ledger.** `finishCodexSubagent` xóa entry; task_complete, interrupted, đổi parent transcript và unreadable quá grace đều có thể khiến child biến mất. Không còn phân biệt lý do tại roster snapshot. Graph cần giữ terminal reason/evidence thay vì xóa lịch sử node. [18]

**C2 — transcript bounded read nhưng lossy.** Nếu backlog hơn 1 MiB, cursor nhảy đến phần đuôi; có thể mất spawn/complete nằm ở phần bị bỏ. Dòng dài hơn 256 KiB bị bỏ; directory listing bị cắt còn 4096 tên. Đây là giới hạn resource của observer, không phải lossless replay. `carry` cho dòng chưa có newline chưa bị chặn tại chỗ gán, nên không khẳng định tổng bộ nhớ parser đã bounded hoàn toàn. [17]

**C3 — không đọc được file không chứng minh worker exit.** Test cố ý advance 61 giây và mong roster trống. Với graph harness, chuyển observation thành `unverifiable`/stale, giữ task chưa xác minh, cho phép reconcile lại. Không dùng timeout này để mở dependency hoặc trao lại quyền ghi cho worker khác. [18][22]

**C4 — bootstrap vẫn cần tín hiệu.** Poll chỉ re-arm khi đã có tracked transcript children; observer cần parent hook có transcript path để đọc lần đầu. Mất tất cả parent hooks/path có thể không phát hiện child. App-server event feed nên là đường chính; transcript là fallback có provenance và gap indicator. [20][21]

**C5 — optimization không là hard bound.** Controller pause/resume nuốt lỗi transport theo thiết kế; backpressure best-effort cần metric nếu thất bại. Endpoint publication còn cửa sổ giữa probe cuối và rename, đúng như hướng dẫn daemon công khai; không quảng bá như atomic compare-and-swap filesystem. [23][1]

### 5.4. Adoption và avoid

Adopt: observer có provenance, ID/session generation, coalesced polling, unknown liveness, endpoint ownership và flow-control semantics nếu cần PTY. Avoid: account selection/restart machinery, dùng transcript làm nguồn duy nhất, xóa child tương đương done, port toàn bộ Electron/relay/daemon. Trong sản phẩm đầy đủ, chỉ thêm daemon độc lập khi yêu cầu detach/reconnect đòi hỏi.

## 6. T3 Code: adapter native và event/receipt là tham chiếu chính

### 6.1. Luồng đầu-cuối đã truy

`CodexAdapter.startSession` tạo runtime và consumer event nằm trong session scope. `makeCodexSessionRuntime` spawn binary với app-server arguments/cwd/env, nối child-process client. `runtime.start` gọi initialize/initialized rồi thread/start hoặc thread/resume, lưu provider thread ID vào resume cursor. `sendTurn` xây params và gọi turn/start; queued follow-up ID không đè activeTurnId đang chạy. [24][28]

Runtime nhận notification: nhớ receiver-turn relation → intercept child v2 trước legacy suppressor → đăng ký từ thread_spawn hoặc subAgentActivity, cập nhật lifecycle/live child turn → emit sự kiện nội bộ `collabAgent/*`. Adapter đổi chúng thành task.started/updated/progress/completed có agent linkage, dùng child thread ID làm task ID và timelineBypass cho parent chat. [24][25]

`ProviderRuntimeIngestion` cập nhật background liveness/read context, biến runtime event thành activity và dispatch `thread.activity.append`. Engine kiểm command receipt, quyết định event, ghi event + projection + accepted receipt trong một transaction, chỉ publish sau commit. Đây là chuỗi từ subprocess event đến read model/receipt, không có yêu cầu T3 tự spawn một executor thay Codex child. [29][30]

### 6.2. Cơ chế nên áp dụng

- **Identity tách rõ:** canonical application thread khác provider thread; child identity không phải nickname. Giữ parent ID, spawn turn ID, runtime turn ID; metadata đến muộn không đổi batch/spawn attribution. [24][25]
- **Idle khác completed:** child turn completed thành idle/failed/interrupted theo status; interacted đơn thuần không đổi idle thành running. Điều này ngăn swarm “sống lại” chỉ vì parent lấy kết quả. [25]
- **Control plane không chìm trong child chatter:** route phân biệt agent-event, parent-owned và drop. Approval resolution phải xuyên qua dù request mang child thread ID; root không được bị đăng ký thành child của chính nó. [24][27]
- **Structured lifetime:** consumer dùng `forkIn(sessionScope)` để không chết khi startSession return. Khi đóng, settle pending approval/input và đóng scope/queues. Rust cần task supervision theo session lifetime tương đương, không thả JoinHandle không có chủ. [28]
- **Stop có fanout hữu hạn:** interrupt các child live turn trước, concurrency 8, timeout 3 giây mỗi child và 10 giây cả nhóm; sau đó gửi parent interrupt. Có test child chưa đăng ký, một child không trả lời, memory-consolidation thread và queued follow-up. [26][27]
- **Atomic receipt/projection:** transaction bảo đảm nhiều event của cùng command không commit nửa chừng khi projection lỗi. Test inject failure rồi retry cho ra đúng hai event dự kiến. Tốt cho graph state machine, không cần copy Effect reactor framework. [30][31]

### 6.3. Điều cần sửa khi chuyển ý tưởng

**T1 — fallback resume có thể thành phiên mới.** `openCodexThread` bắt nhóm recoverable errors, log warning rồi thread/start. Hợp lý cho UI muốn tiếp tục chat, nhưng autonomous task cần thấy `ResumeFailed`/`SessionReplaced`, giữ lineage và chưa tự chạy lại side effect chưa xác minh. Test acceptance phải xác nhận provider ID đổi được phản ánh vào graph. [24]

**T2 — source comment có phần lạc hậu.** Comment tại 917–932 nói notification đến trước registration pass-through; implementation tại 1786–1820 đã suppress lifecycle của foreign conversation khi biết root và nhớ live turn để stop. Đánh giá dựa trên branch thật và fixture, không sao chép comment thành spec. Vẫn có tradeoff: suppress chưa đồng nghĩa có đủ identity để đưa child lên graph. [24]

**T3 — “bounded stop” hiện chỉ chặn thời gian chờ child group.** Parent `client.request(turn/interrupt)` phía sau không có timeout cục bộ tại đoạn này. Không kết luận toàn bộ Stop có upper bound; Rust cần deadline cả control command và trạng thái `StopUnconfirmed` khi chưa có acknowledgement/lifecycle evidence. [26]

**T4 — unbounded queues.** Runtime tạo event queue và server notification queue bằng `Queue.unbounded`. Đây chưa là thiết kế RAM bounded cho terminal nhận nhiều delta. Tách stream dữ liệu lớn khỏi lifecycle/approval, coalesce progress và lưu artifact thay inline output vô hạn. [24]

**T5 — metadata lookup không nên điều khiển lifecycle.** Child lookup dùng thread/resume excludeTurns, timeout 5 giây, chạy trong runtime scope, nuốt lỗi; metadata thiếu phải vẫn là unknown. Test kiểm request đúng một lần và failure không trì hoãn parent. Đây là lookup theo implementation này, không suy ra mọi phiên bản Codex bảo đảm request đó thuần read-only. [28][27]

**T6 — receipt dedup còn cần payload binding.** Engine đọc receipt theo command ID và so aggregate kind/ID; trong nhánh retry đã đọc không so digest nội dung command. Cần thêm hash/schema version khi áp dụng: cùng command ID + cùng aggregate nhưng nội dung khác phải conflict. Accepted receipt chỉ chứng minh command đã được xử lý/persist, không chứng minh side effect native hoàn tất exactly-once. [30][31]

**T7 — mock integration không phải live native certification.** Test boots runtime thật với scripted peer; một phần notifications đến từ captured fixture, phần còn lại tổng hợp cho failure/approval. `it.live` ở đây dùng clock thật cho subprocess test, không phải tài khoản LLM thật. Nó đáng dùng làm contract regression nhưng không chứng minh compatibility với binary đang cài. [27]

### 6.4. Adoption và avoid

Adopt: adapter boundary, native child routing, correlation IDs, state reducer, replay/receipt, stop semantics, session-scoped consumer. Avoid: multi-provider/instance registry làm requirement, account/home routing, toàn bộ WS/mobile/Electron stack, full reactor scheduler, gọi event normalization nội bộ `collabAgent/*` là RPC native. Các model field quan sát được chỉ là metadata; không kéo theo yêu cầu multi-account.

## 7. Integration contracts đề xuất cho harness Rust

Đây là **đề xuất thiết kế**, chưa phải schema native đã xác minh. Task graph là sổ công việc/dependency; native runtime vẫn sở hữu inference, tool execution và child lifecycle. Parent Codex lập và thực hiện delegation trong envelope được cấp; Rust quan sát/kiểm chứng, áp giới hạn tại những entry point thực sự kiểm soát và phát control request. Không thêm planner/worker queue executor cạnh tranh.

| Contract | Nội dung tối thiểu | Quy tắc quan trọng |
|---|---|---|
| `RuntimeCapabilities` | runtime revision, protocol version, observe-child, resume, interrupt-child, native depth/concurrency limits, sandbox/tool restrictions | Phân biệt supported / unsupported / unverified; không tự bịa RPC spawn mới |
| `RunIdentity` | run ID, native root thread ID, runtime instance/generation, account scope opaque duy nhất | Không lưu credentials trong graph; không chọn account dựa trên quota |
| `AgentIdentity` | native child thread ID, parent ID, spawn turn ID, role/display name | Identity ổn định qua nhiều turn; nickname không làm khóa |
| `TaskNode` | task ID, dependency IDs, acceptance criteria, scope, artifact refs, assignment attempts | Task khác agent; một agent có thể làm nhiều node/attempt, graph dependency khác ancestry |
| `RuntimeObservation` | event ID/sequence nếu có, receive sequence, source runtime/transcript, generation, raw kind, typed payload, gap/quality | Không có native sequence thì không giả lập thành chứng nhận ordering native; giữ local receive order riêng |
| `BoundedRunEnvelope` | max live children/depth, wall deadline, max attempts/turns, output bytes, allowed write scopes, token budget khi đo được | Child envelope chỉ thu hẹp; limit native được dùng nếu đã verify; thiếu telemetry không thành zero |
| `StopReceipt` | request ID, target thread+active turn, requested/acknowledged/observed-idle-or-exited/unconfirmed | Request accepted không là stopped; stop có deadline chung, không giải phóng writer scope vì timeout |
| `ExecutionReceipt` | task/attempt/source-state ID, result artifact, validation command+exit, executor identity | LLM text/API success và UI done không mở dependency; test receipt phải gắn đúng source |
| `LocalWriteClaim` | repository/worktree, exact scopes, owner, epoch/version, expires, assurance level | Lease advisory không thay sandbox; ngăn stale write tại boundary có khả năng enforce |
| `Journal` | command ID+payload digest, append event, projection sequence, receipt trong transaction | Replay idempotent; crash sau dispatch native cần reconcile, không blind retry |

Không thể bảo đảm hard bound bằng prompt đơn thuần. Nếu native runtime ở phiên bản đích chưa có cách enforce giới hạn child/depth hoặc không thể chặn delegation vượt budget trước side effect, sản phẩm phải công khai mức `observed/best-effort` và giới hạn chế độ autonomy tương ứng. Mục việc đầu tiên dưới đây giải quyết năng lực này, không dùng một executor thứ hai để che khoảng trống.

Graph cần giữ hai trục: lifecycle agent (`running/waiting/idle/closed/failed/unverifiable`) và trạng thái task (`pending/assigned/result-reported/validated/failed/blocked`). `idle` không tự động là task validated; parent done chưa làm run complete khi child/task bắt buộc còn chưa xác minh.

## 8. Các task đề xuất đưa vào kế hoạch và acceptance tests

Không sửa `PLAN.md`. Các mục này là handoff cụ thể cho người tích hợp.

| Ưu tiên / task | Deliverable | Acceptance test cụ thể |
|---|---|---|
| P0-A: xác minh contract native một tài khoản | Adapter capability matrix gắn binary/protocol revision; account scope cố định | Fixture/sandbox chỉ đọc xác nhận root-child IDs, observe, resume, active turn interrupt và native limit knobs. Unsupported phải trả lỗi typed; không fallback sang provider/account khác. Không cần tài khoản live trong unit suite |
| P0-B: child reducer và graph linkage | Reducer thuần Rust; task graph tách agent ancestry | Replay child trước registration, registration trùng, root-as-child, late metadata, interacted-after-idle, parent done trước child. Không đổi parent state từ child lifecycle; không mất approval resolution |
| P0-C: stop có bằng chứng | Control path riêng, deadline tổng, stop receipts từng target | Child A treo RPC, B hoàn tất, C chưa đăng ký nhưng đã có live turn, parent có queued follow-up. Stop vẫn target active turn; timeout tổng hữu hạn; target chưa xác minh giữ unconfirmed. Kiểm worker thực sự ngừng phát side effect, không chỉ enum đổi |
| P0-D: resume đúng nghĩa | Resume giữ native lineage; session replacement là event riêng | Seed context có nonce, resume cùng native ID đọc lại nonce; unknown/deleted session không âm thầm tạo child thay thế rồi tuyên bố resume. Restart giữa result và save không mất history đã ghi |
| P0-E: envelope xuyên suốt run | Giới hạn active lifetime/depth/attempt/deadline bằng capability native đã verify; giám sát vi phạm | N+1 children qua nhiều batch, background spawn trả sớm, child resume, cancellation trong lúc chờ slot. Không nhả admission accounting trước terminal evidence; nếu chỉ observer thì test phải báo enforcement unavailable, không pass giả |
| P0-F: journal và receipt transactional | Event + projection + command receipt, payload digest | Crash/failure tại từng điểm trước/sau append/project/commit/dispatch native. Exact retry không lặp mutation; cùng command ID khác payload bị conflict; uncertain native dispatch chuyển reconcile-needed |
| P1-A: source-bound validation | Fingerprint tracked/untracked, manifest build inputs khi cần, validation receipts | Cùng HEAD nhưng đổi tracked byte/untracked mode làm receipt cũ invalid. Thay input sau test trước handoff bị phát hiện. API 200 hoặc “done” trong text không đủ mở dependency |
| P1-B: quyền ghi và handoff | Exact scope claims; writer ownership hợp tác hoặc enforce theo năng lực đã chứng minh | Xung đột `src/a` với `src/a/b`, case alias, stale epoch sau restart, child hoàn tất muộn. Không trao lại cùng quyền ghi chỉ vì heartbeat timeout. Shared lockfile được tích hợp bởi một owner |
| P1-C: observer fallback có mất dữ liệu hiển thị rõ | Incremental parser bounded bytes/carry, provenance/gap, reconcile state | Backlog >1 MiB, dòng >256 KiB, thiếu newline, file truncate/rotate, child qua nửa đêm, file unreadable >60 giây. Không chuyển thành success; carry/RAM có bound đo được; child xuất hiện lại không nhân đôi |
| P1-D: terminal không nghẽn control | TUI render từ projection; bounded/coalesced output; artifact spool | Flood output từ N worker nhưng Stop/approval còn phản hồi trong SLO đã định; progress coalesce không mất lifecycle. Consumer còn sống sau start return; session close drain/cancel có receipt |
| P2-A: detach/reconnect nếu sản phẩm yêu cầu | Host owner/generation, endpoint publication và reconnect | Hai host tranh socket, incumbent timeout/EPERM, old host shutdown sau replacement, attach-only vào session đã exit. Không xóa endpoint người khác, không spawn replacement ngoài claim |

Tiêu chí hoàn tất nghiên cứu tích hợp: mọi requirement “bounded”, “resumed”, “stopped”, “validated” đều có test ở boundary có tác dụng thật; test chỉ set/get store hoặc đổi label không đủ. Unit dùng reducer/clock giả; integration dùng scripted native peer/process giả có thể kiểm side-effect counter. Chỉ khi được phép riêng mới chạy capability probe với binary/tài khoản thật; không gắn trạng thái live-verified vào kết quả từ mock.

## 9. Tham chiếu source có đánh số

Các số trên trỏ vào cụm evidence dưới đây. Mỗi liên kết ghi line bắt đầu chính xác; phạm vi đọc/ý nghĩa được mô tả trong nhãn hoặc câu kèm theo.

1. Hướng dẫn: [Ruflo root](/home/minh/projects/outsource/ruflo/AGENTS.md:25), [Ruflo Codex](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/AGENTS.md:74), [OpenDev](/home/minh/projects/outsource/opendev/AGENTS.md:1), [Orca](/home/minh/projects/outsource/orca/AGENTS.md:1), [Orca daemon](/home/minh/projects/outsource/orca/src/main/daemon/AGENTS.md:1), [Orca tests](/home/minh/projects/outsource/orca/tests/AGENTS.md:1), [Orca E2E](/home/minh/projects/outsource/orca/tests/e2e/AGENTS.md:1), [T3](/home/minh/projects/outsource/t3code/AGENTS.md:1).
2. Ruflo swarm state: [load/reconcile/save/lock](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/swarm-tools.ts:136), [init handler](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/swarm-tools.ts:261), [persist và response](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/swarm-tools.ts:317).
3. Ruflo agent: [registry rồi swarm update](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/agent-tools.ts:352), [registered response](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/agent-tools.ts:433), [execute handler](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/agent-tools.ts:492).
4. Ruflo execute: [HTTP payload và text response](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/agent-execute-core.ts:234), [state → provider call](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/agent-execute-core.ts:555), [API success proxy](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/agent-execute-core.ts:663), [transcript provenance và result](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/src/mcp-tools/agent-execute-core.ts:718).
5. Ruflo exact state: [capture clean/dirty](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/src/harness/repository-state.ts:407), [match/release eligibility](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/src/harness/repository-state.ts:487).
6. Ruflo lease: [scope normalization/overlap](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/src/harness/in-memory-fenced-lease-reference.ts:38), [reference-only acquire](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/src/harness/in-memory-fenced-lease-reference.ts:75), [renew/release/current fence](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/src/harness/in-memory-fenced-lease-reference.ts:135).
7. Ruflo source-state tests: [tracked byte và untracked mode](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/tests/harness-repository-state.test.ts:48), [ignored/build input separation test locations](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/tests/harness-repository-state.test.ts:143).
8. Ruflo tests: [single-process acquisitions/CAS/expiry](/home/minh/projects/outsource/ruflo/v3/@claude-flow/codex/tests/harness-authorities.test.ts:72), [model alias-only assertions](/home/minh/projects/outsource/ruflo/v3/@claude-flow/cli/__tests__/agent-execute-models.test.ts:1).
9. OpenDev spawn tool: [resume schema](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/spawn.rs:127), [validation/recursive guard](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/spawn.rs:169), [IDs/cancel/sync dispatch](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/spawn.rs:264), [save/output](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/spawn.rs:352), [background detached task/token](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/spawn.rs:485), [background result/cost](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/spawn.rs:558).
10. OpenDev manager: [runner bounds và spawn parameters](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/subagents/manager/spawn.rs:20), [instruction/model/tool setup](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/subagents/manager/spawn.rs:126), [new messages/runner call/result](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/subagents/manager/spawn.rs:212).
11. OpenDev runner: [StandardReactRunner không cost tracker](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/subagents/runner/standard.rs:25), [test chỉ runner name](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/subagents/runner/standard_tests.rs:1).
12. OpenDev event bridge: [event IDs và unbounded sender](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/events.rs:5), [callbacks/Finished no-op](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/events.rs:125).
13. OpenDev stop: [InterruptToken tự tạo cancel](/home/minh/projects/outsource/opendev/crates/opendev-runtime/src/interrupt.rs:47), [TUI kill_task](/home/minh/projects/outsource/opendev/crates/opendev-tui/src/managers/background_agents.rs:190).
14. OpenDev concurrency: [batch semaphore/dispatch/select/background text](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/react_loop/phases/parallel.rs:40), [ordered read-safe groups](/home/minh/projects/outsource/opendev/crates/opendev-tools-core/src/parallel.rs:62).
15. OpenDev end-to-end runtime/TUI: [ReAct gọi LLM phase](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/react_loop/execution.rs:219), [HTTP calls](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/react_loop/phases/llm_call.rs:91), [tool registry dispatch](/home/minh/projects/outsource/opendev/crates/opendev-agents/src/react_loop/phases/tool_dispatch.rs:467), [TUI identity match](/home/minh/projects/outsource/opendev/crates/opendev-tui/src/app/handle_subagent.rs:10), [TUI finish](/home/minh/projects/outsource/opendev/crates/opendev-tui/src/app/handle_subagent.rs:166).
16. OpenDev tests: [spawn validation/recursion](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/spawn_tests.rs:3), [bridge callback simulation](/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/agents/events_tests.rs:26), [Kill token/store only](/home/minh/projects/outsource/opendev/crates/opendev-tui/src/managers/background_agents_tests.rs:48), [preserve Killed label](/home/minh/projects/outsource/opendev/crates/opendev-tui/src/managers/background_agents_tests.rs:116).
17. Orca transcript parser: [bounds/cursor](/home/minh/projects/outsource/orca/src/shared/codex-subagent-transcript.ts:11), [directory/date resolution](/home/minh/projects/outsource/orca/src/shared/codex-subagent-transcript.ts:106), [activity/model/completion parsing](/home/minh/projects/outsource/orca/src/shared/codex-subagent-transcript.ts:172).
18. Orca reconciliation: [parent/child/unreadable retirement](/home/minh/projects/outsource/orca/src/shared/codex-subagent-transcript.ts:254), [roster delete và metadata no-resurrection](/home/minh/projects/outsource/orca/src/shared/codex-subagent-roster.ts:60), [aggregate state](/home/minh/projects/outsource/orca/src/shared/codex-subagent-roster.ts:126).
19. Orca poll: [monotonic deadlines/generation/callback cancellation](/home/minh/projects/outsource/orca/src/shared/codex-subagent-poll-scheduler.ts:8), [child-driven normalized snapshot](/home/minh/projects/outsource/orca/src/shared/agent-hook-listener/providers/codex-events.ts:26).
20. Orca ingress: [HTTP normalization/apply/schedule](/home/minh/projects/outsource/orca/src/main/agent-hooks/server/server-lifecycle.ts:84), [Codex hook → transcript reconcile](/home/minh/projects/outsource/orca/src/shared/agent-hook-listener/providers/codex-events.ts:102).
21. Orca status: [state/timing/observation](/home/minh/projects/outsource/orca/src/main/agent-hooks/server/server-status-update.ts:23), [persist/fanout](/home/minh/projects/outsource/orca/src/main/agent-hooks/server/server-status-update.ts:195), [identity check/re-arm poll](/home/minh/projects/outsource/orca/src/main/agent-hooks/server/server-status-retries.ts:46).
22. Orca tests: [real hook-server + synthetic rollout](/home/minh/projects/outsource/orca/src/main/agent-hooks/server-codex-subagent-transcript.test.ts:40), [unreadable 61s drops roster](/home/minh/projects/outsource/orca/src/shared/codex-subagent-transcript.test.ts:131).
23. Orca terminal: [publish/link/probe/rename](/home/minh/projects/outsource/orca/src/main/daemon/daemon-endpoint-ownership.ts:63), [replacement proof](/home/minh/projects/outsource/orca/src/main/daemon/daemon-endpoint-ownership.ts:130), [socket tests với mock subprocess](/home/minh/projects/outsource/orca/src/main/daemon/daemon-endpoint-ownership.test.ts:36), [attach/terminating guards](/home/minh/projects/outsource/orca/src/main/daemon/terminal-host-session-create.ts:32), [producer hysteresis/best-effort](/home/minh/projects/outsource/orca/src/main/ipc/pty-producer-flow-control.ts:8).
24. T3 native runtime: [resume fallback](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:711), [child registration commentary](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:917), [route table](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:1054), [spawn và unbounded queues](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:1187), [child registration implementation](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:1457), [intercept-before-suppress/pre-registration](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:1775).
25. T3 adapter lifecycle: [child identity/task mapping](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexAdapter.ts:1037), [interacted/idle/waiting](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexAdapter.ts:1107).
26. T3 stop: [queued turn không đè active turn](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:2376), [bounded child interrupt rồi parent request](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:2394).
27. T3 tests: [scripted mock peer, synthetic additions](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts:1), [fixture writes và metadata test](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts:161), [failure test location](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts:347), [fanout assertions](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts:403), [Stop test](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts:478), [hung child/parent assertions](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts:575), [queued-follow-up test location](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts:608).
28. T3 lifetime và startup: [metadata lookup bound](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:1370), [initialize/thread open](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:2266), [close và sendTurn](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:2307), [consumer lifetime rationale](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexAdapter.ts:2318), [forkIn/session start](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexAdapter.ts:2439).
29. T3 ingestion: [background liveness](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts:2020), [activity → engine dispatch](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts:2129).
30. T3 engine: [receipt aggregate conflict/retry](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/OrchestrationEngine.ts:144), [event/projection/receipt transaction và publish](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/OrchestrationEngine.ts:273).
31. T3 engine tests: [projection failure injection](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts:1594), [rollback/retry assertions](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts:1681), [genuine retry](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts:1927), [different aggregate conflict test location](/home/minh/projects/outsource/t3code/apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts:1991).
