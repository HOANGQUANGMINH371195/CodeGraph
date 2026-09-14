# Codex native subagents cho Project Graph Harness

## Phạm vi và kết luận

Codex local ở revision `818f1cca8ccf8899f0f4d59336baebaccf358eed`, worktree
clean tại lúc kiểm tra. Native agent control đủ giàu để làm chủ vòng đời
subagent; không cần account rotation để tạo đội agents. Đây là audit source
và đối chiếu tài liệu, không phải chứng nhận public app-server đã expose mọi
private core method hoặc runtime sản phẩm đã tích hợp thành công.

Quyết định thiết kế là một root account, mặc định inherited model, role-specific
instructions và context. Harness giữ graph/domain admission và integration
policy, Codex giữ agent lifecycle. Các giới hạn quota, permission và capability
vẫn phải được xác minh trên runtime được phân phối. Không có kết luận rằng
single-account swarm luôn rẻ hơn hoặc chính xác hơn single-agent.

## Luồng spawn thực tế

MAv1 handler `handle_spawn_agent` parse message/items/role, tính child depth và
chặn vượt config limit trước launch. Nó tạo spawn config từ parent, xử lý model
override/role, rồi áp dụng runtime overrides. `SpawnAgentOptions` mang parent
thread/turn/root-turn và environment selections; kết quả trả native thread ID,
nickname, effective model và trạng thái qua collaboration events.^1

Tiếp theo `AgentControl` chọn MAv1/MAv2, kiểm execution capacity, reserve
residency/spawn slot và lấy environment/exec-policy inheritance. Nó chọn fresh
thread hoặc fork history, truyền cùng control handle sang child và chỉ commit
reservation sau khi thread được tạo. Không coi việc cấp nickname là một job
đã chạy thành công.^2

`ThreadManagerState::spawn_new_thread_with_source` dùng shared AuthManager,
parent ID, inherited environments và exec policy để tạo ThreadSpawnRequest.
Đây là đường nguồn hỗ trợ mô hình cùng runtime/auth manager, không phải bằng
chứng auth credential bất biến suốt phiên hoặc mọi role config đều an toàn.^3

## Registry, quota và lifecycle

AgentControl được scope theo root tree và chia sẻ registry, rollout budget,
service tier cùng execution limiter. Phân biệt registry identity, resident
runtime và đang chạy turn là cần thiết: số thread lịch sử không bằng concurrency
và không bằng số process terminal đang sống.^4

Execution limiter có counter/guard; drop guard giảm active count. MAv2 có
capacity/residency logic khác MAv1. Vì vậy harness cần versioned adapter tests,
không hardcode một ý nghĩa chung cho mọi field “max threads”. Domain admission
vẫn cần tổng budget, child depth, task dedup, file ownership và per-tool quotas;
native turn limit không thực hiện các invariant graph này.^5

Wait dùng status subscriptions và timeout deadline; một wait timeout không
chứng minh agent chết. Resume kiểm trạng thái và chỉ thử load closed agent ở
đường thích hợp. Control có thao tác tìm live descendants; đóng một nhánh agent
không phải receipt rằng external process tree đã sạch. Harness phải reconcile
thread, job, process, worktree và graph projection thành các state riêng.^6

## Context và permission inheritance

Fork history có filtering: giữ context items cần thiết nhưng bỏ các tool-call
records nhất định, parent cumulative usage, inter-agent records và các hints
không thuộc child. Full-history fork và truncated history có cách phục hồi
context baseline khác nhau. Không nên diễn giải “fork” là sao chép mọi thứ hoặc
là worktree clone. Fresh scoped task là default phù hợp cho graph-first harness;
fork chỉ khi phụ thuộc lịch sử thực sự và budget cho phép.^7

Tài liệu official mô tả project-scoped custom agents, inheritance và các nút
cấu hình concurrency/model. Local Codex hiện yêu cầu direct request hoặc project/
skill instruction để spawn. Do đó autonomous delegation cần một project policy
ủy quyền rõ, không chỉ lời hứa “AI sẽ tự biết chia việc”. Tên config và schemas
có thể thay đổi theo version; phải probe runtime trước áp dụng.^8

Source có test sandbox runtime override nhưng một test đang bị điều kiện
platform hạn chế. Đọc test không bằng đã chạy test: audit này không build hoặc
chạy suite Codex. Không dùng test-name làm chứng cứ permission isolation đã đạt
trên Linux runtime sản phẩm.^9

## Tích hợp vào harness

Điểm tích hợp cụ thể hơn private spawn API: protocol local có
`MultiAgentMode::{ExplicitRequestOnly, Proactive, Custom}` và trường
`multiAgentMode` ở thread/start, turn/start. Hai roundtrip tests xác nhận giá trị
`proactive` và experimental reason tương ứng. Đây là source evidence cho một
seam native có sẵn, không phải kết quả chạy live. Public method registry có
thread/start/fork, không thấy một generic agent/spawn RPC trong file registry
đã đọc. Tích hợp nên khởi tạo root với proactive mode khi capability cho phép,
rồi dùng native model tool/events; không tự đặt tên RPC chưa có.^10

Tuy nhiên field serialization chưa chứng minh effective behavior: đường MAv2
`effective_multi_agent_mode` còn xét configured/catalog hint và reasoning effort;
`MultiAgentModeInstructions` render policy thành context message. World-state
diff có cơ chế reset về explicit khi mất proactive state. Đây là lý do cần
end-to-end test requested mode→effective model-visible mode→native spawn,
kể cả compaction/resume; không dùng config flag như enforcement ngân sách.^11

NativeSessionBinding nên chứa runtime/host/root/native-child IDs, effective model,
opaque account alias và capability version. TaskAttempt giữ immutable scope,
graph version, evidence/artifact requirements và write ownership. Không giữ
credentials trong ContextPack, task receipt hoặc graph. Legacy account_lane trong
prototype cần migration tương thích, không được xem là auth enforcement.

DelegationProposal là dữ liệu: mục tiêu, dependencies, lý do parallel, output,
scope, budget và terminal condition. Một admission gate kiểm proposal trước
spawn, kể cả child delegation. Nếu native tools chưa có hook bắt buộc đi qua
gate, cần chứng minh seam bổ sung; instruction-only policy không tương đương
security enforcement. Không dùng nhiều external worker processes để giả lập
native subagents chỉ vì dễ nối API hơn.

Parent nên giữ critical path; side tasks có write sets riêng hoặc worktrees.
Graph context cấp dần orientation/module/symbol/source evidence; cả parent và
child đều truy xuất artifact bằng refs, không broadcast transcripts. Critic có
scope riêng, không tự accept output mình tạo. Merge cần checks trên candidate
đã hợp nhất và target-head compare-and-swap.

## Acceptance còn thiếu

| Contract | Bằng chứng cần trước acceptance |
|---|---|
| Same-account native tree | Runtime smoke xác nhận parent/root IDs và identity, không secret logging |
| Model inheritance | No override giữ effective root model; role config không đổi ngầm |
| Admission | Spawn storm/depth/global-budget tests qua actual tool seam, không chỉ prompt |
| Context | Cold/fork/compaction tests; freshness và bounded graph/source slices |
| Worktree | Hai writers conflict, shared interface, dirty base; native fork không được tính là isolation |
| Recovery | Spawn-before-binding crash, lost event, closed/resumed descendants, no duplicate effects |
| Permission | Parent live restrictions không bị role config/tool data nới rộng |
| Quality/cost | Same model/account single vs 2/4/8 agents, tổng tokens và semantic success |

## Sources

1. OpenAI Codex, [MAv1 spawn handler](/home/minh/projects/outsource/codex/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs:49), local pinned source.
2. OpenAI Codex, [agent spawn reservation/dispatch](/home/minh/projects/outsource/codex/codex-rs/core/src/agent/control/spawn.rs:610), local pinned source.
3. OpenAI Codex, [shared auth thread creation](/home/minh/projects/outsource/codex/codex-rs/core/src/thread_manager.rs:1786), local pinned source.
4. OpenAI Codex, [root-scoped AgentControl](/home/minh/projects/outsource/codex/codex-rs/core/src/agent/control.rs:116), local pinned source.
5. OpenAI Codex, [execution limiter](/home/minh/projects/outsource/codex/codex-rs/core/src/agent/control/execution.rs:14), local pinned source.
6. OpenAI Codex, [wait](/home/minh/projects/outsource/codex/codex-rs/core/src/tools/handlers/multi_agents/wait.rs:140), [resume](/home/minh/projects/outsource/codex/codex-rs/core/src/tools/handlers/multi_agents/resume_agent.rs:39), [descendant traversal](/home/minh/projects/outsource/codex/codex-rs/core/src/agent/control.rs:899).
7. OpenAI Codex, [fork filtering](/home/minh/projects/outsource/codex/codex-rs/core/src/agent/control/spawn.rs:53), local pinned source; control_tests.rs includes fork/compaction cases, not executed here.
8. OpenAI, [Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents), accessed 2026-09-11; source and current docs may differ by version.
9. OpenAI Codex, [sandbox override test/platform note](/home/minh/projects/outsource/codex/codex-rs/core/src/tools/handlers/multi_agents_tests.rs:2146), local pinned source, not executed here.
10. OpenAI Codex, [MultiAgentMode](/home/minh/projects/outsource/codex/codex-rs/protocol/src/config_types.rs:337), [thread field](/home/minh/projects/outsource/codex/codex-rs/app-server-protocol/src/protocol/v2/thread.rs:112), [turn field](/home/minh/projects/outsource/codex/codex-rs/app-server-protocol/src/protocol/v2/turn.rs:256), [experimental roundtrip tests](/home/minh/projects/outsource/codex/codex-rs/app-server-protocol/src/protocol/v2/tests.rs:4871), [RPC registry](/home/minh/projects/outsource/codex/codex-rs/app-server-protocol/src/protocol/common.rs:559).
11. OpenAI Codex, [effective mode resolution](/home/minh/projects/outsource/codex/codex-rs/core/src/session/multi_agents.rs:156), [mode context rendering](/home/minh/projects/outsource/codex/codex-rs/core/src/context/multi_agent_mode_instructions.rs:7), [world-state mode reset](/home/minh/projects/outsource/codex/codex-rs/core/src/context/world_state/multi_agent_mode.rs:61).
