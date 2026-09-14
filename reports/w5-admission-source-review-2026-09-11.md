# W5 admission: tách no-dispatch khỏi native terminal

## Source-study trước thay đổi

- T3 Code HEAD `3836890e4484406813997259efc69065b25ce698`, worktree sạch.
- [Runtime interception](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexSessionRuntime.ts:1775)
  chuyển child notifications trước parent suppression; pre-registration events
  không được biến thành parent lifecycle. Unknown child cần reconcile riêng.
- [Adapter](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexAdapter.ts:1107)
  bỏ qua interacted thông thường, turnStarted→running, turnCompleted→idle hoặc
  interrupted/failed. Turn completion không chứng minh thread đã kết thúc.
- [Test assertion](/home/minh/projects/outsource/t3code/apps/server/src/provider/Layers/CodexAdapter.test.ts:1132)
  phát running→completed→interacted rồi child khác running; assert đúng ba
  task.updated, không reactivation từ interacted. Đây là test runtime giả đã
  đọc, không chạy trong đợt này, không phải live Codex verification.

## Adopt / avoid và acceptance trước code

### Bổ sung trước triển khai root/parent binding

Đọc lại T3 Code cùng HEAD/worktree sạch: CodexSessionRuntime.ts:1457 giữ
parentThreadId trong registration và :1520 loại root khỏi child roster;
CodexCollabRuntime.integration.test.ts:403–465 chạy scripted peer, assert root
turn completion vẫn đi qua và child lifecycle không lẫn parent stream. Test
nguồn chỉ đọc, không chạy. Học root exclusion và lineage tường minh; không lấy
metadata observer làm authentication. Harness sẽ giữ root_thread cùng parent
attempt trong gate, kiểm parent thread lúc bind (parent chưa bind → lỗi để
reconcile), chặn root-as-child và parent mismatch kể cả replay. Chưa chứng minh
account identity; đó vẫn là trách nhiệm adapter. Unit thêm các fixture đó.

Học sự tách biệt turn activity, thread lifecycle và task acceptance. Không copy
Effect adapter hay coi trạng thái UI là bằng chứng dừng process. Trong harness,
đổi API observe_terminal(attempt) thành confirm_not_dispatched(attempt), chỉ cho
reservation chưa bind; attempt đã bind phải dùng observe_native_terminal(thread).
Kiểm thử bound attempt không thể dùng no-dispatch để giải phóng slot, lặp vẫn
idempotent cho unbound, và native terminal vẫn giữ mapping/dedup/total budget.

Đây là review hiện tại của admission đã viết trước quy tắc source-first; không
ghi nhận hồi tố rằng phiên bản trước đã có source-study receipt. Adapter, root
session authentication, event authentication, durable replay và live runtime vẫn chưa nối.

## Kết quả root/parent binding

- Source-gate: ready — đã đọc lại cùng revision sạch và các source/test nêu
  trên trước khi bổ sung regression; không chạy test của repo tham khảo.
- Gate giữ root ID và parent attempt trong state private. Bind từ chối root
  làm child, parent mismatch cả replay và parent chưa có native binding.
- Regression giữ nguyên mục đích kiểm thread reuse bằng cách truyền đúng
  parent; thêm root exclusion, nested parent mismatch, replay sau terminal,
  invalid root/parent IDs và late child binding sau parent terminal/stop.
  Không có dispatch, account authentication hay durable recovery trong test.
- `sh scripts/validate-foundation.sh`: exit 0, 67 Rust + 10 Node tests pass;
  architecture check 6 crates / 29 direct dependencies pass.
- `scripts/with-local-tools cargo fmt --all -- --check` và `cmp PLAN.md
  /home/minh/projects/outsource/PLAN.md`: exit 0. Source fingerprint của
  `crates/application/src/admission.rs` (SHA-256):
  `d996d65d5a4494c0822e8e6dfef06ec8fc18f07db39916c30eb31ccd6a21fbc5`.
- Review tại main: parent terminal chặn reserve mới nhưng không xóa lineage
  cần cho reconcile child đã reserve; lỗi bind không đổi slot hoặc mapping.
  Reviewer độc lập/native adapter và live acceptance vẫn chưa có.
