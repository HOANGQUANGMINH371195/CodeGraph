# Project Graph Harness — kế hoạch triển khai

> Quyết định hiện hành: **một account, native Codex subagents, autonomous bounded
> swarm, worker mặc định Luna (`gpt-5.6-luna`), root giữ model hiện tại**. Các nghiên cứu multi-auth cũ chỉ là reference;
> không còn yêu cầu Pro/Plus routing. W0–W13 vẫn là full product scope.

> **Quy tắc triển khai:** đến phần việc nào, quay lại đọc source và tests của
> repo liên quan trong `outsource`, ghi cách học hỏi/điều chỉnh, rồi mới code.
> Áp dụng cho cả main agent và subagent; checklist bắt buộc ở mục 0.4.1.
> **Khi chuyển task hoặc resume:** đặt lại source-gate về `pending`, đọc lại
> luồng và tests liên quan trong outsource; chỉ code sau khi ghi quyết định
> học gì, điều chỉnh gì và vì sao cần tự viết phần còn thiếu.

> **Trạng thái:** đang triển khai W1; W0 chưa qua baseline gate
>
> **Ngày lập:** 2026-09-10
>
> **Audit thiết kế gần nhất:** 2026-09-12 — đã đối chiếu với snapshot live
> của 23 repository, bao gồm Ripwire. Những gì ghi là *target* chưa được xem là capability đã
> tồn tại cho đến khi có receipt/baseline tương ứng.
>
> **Cập nhật kế hoạch:** 2026-09-12 — chốt quy trình học từ outsource trước
> mỗi phần việc; không code từ suy đoán rồi mới tìm nguồn đối chiếu.
> Bằng chứng test đã ghi nhận nằm ở mục 0.1; cập nhật tài liệu không có nghĩa
> là vừa chạy lại tests hoặc đã hoàn thành tính năng. P0.T01 đã được kiểm chứng
> bằng release manifest mới; các compatibility/license gate vẫn mở. P0.T05 đã
> đọc lại evaluator của CodeGraph và ghi receipt, nhưng chưa chạy được agent
> baseline vì môi trường thiếu `claude`, `jq` và binary `codegraph`; W0 vẫn mở.
> Ripwire đã được đọc lại sâu ở các luồng task-pack, handoff, partition/lane,
> cache, trace, quality và MCP; P0.T06 đã có compatibility matrix riêng, còn
> adapter Ripwire vẫn chưa được tích hợp hay đánh dấu runtime-ready.
> W1 hiện đã có CLI snapshot-authority wiring dạng opt-in cho
> `verify-evidence`; đây chưa phải toàn bộ CLI/runtime authority. Follow-up mới
> dùng một global wall-clock budget cho cả stable observation, Git probes và
> file hashing; đây chỉ là liveness/accounting hardening, chưa phải atomic
> filesystem snapshot.
>
> **Repo tham khảo:** `/home/minh/projects/outsource`
>
> **Repo sản phẩm:** `/home/minh/projects/project-graph-agent`
>
> **Nguồn kế hoạch chính:** `/home/minh/projects/outsource/PLAN.md`.
> Bản trong repo sản phẩm là bản đồng bộ; cập nhật cả hai trong cùng thay đổi.
>
> **Mục tiêu sản phẩm:** giúp Codex CLI hiểu cấu trúc và quan hệ của một
> repository mà không phải đọc tuần tự những file dài hàng nghìn dòng; khi cần
> vẽ kiến trúc hoặc sửa code, agent lấy một context nhỏ, có quan hệ, có nguồn
> chứng minh và có giới hạn độ tin cậy.


## 0. Checklist tiến độ và bằng chứng

Quy ước: `[x]` = đã làm và đã kiểm tra đúng phạm vi mô tả; `[ ]` = chưa
xong, đang làm hoặc chưa có bằng chứng. Có struct/trait hoặc test xanh chưa đủ
để tích cả tính năng. Chỉ tích package khi tất cả gate của package đã đạt.
Các checkbox ở phase và Definition of Done là gate, không dùng để tính phần
trăm sản phẩm bằng số dòng đã tích.

**Đọc nhanh:** mục 0.2 xem package đã đến đâu; mục 0.3 xem việc chặn tiếp theo;
mục 6 tích từng bước và acceptance; mục 11 xem thứ tự/phụ thuộc; mục 12 kiểm
đủ điều kiện bàn giao. Hiện **0/14 package qua gate hoàn thành**, không có nghĩa
là chưa viết code. W0 và W1 đang triển khai; các track còn lại chưa qua gate.

### 0.1. Những phần đã kiểm chứng

- [x] **W2 required sandbox gate:** `sh scripts/validate-sandbox.sh` bật
  GRAPH_REQUIRE_SANDBOX, thiếu backend hoặc non-Linux thì fail. Shared
  discovery policy có required-failure/optional-skip tests; gate chạy thật
  5 sandbox +16 RPC tests pass, foundation/fmt pass. Không đóng containment
  hoặc full W2 acceptance.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-sandbox-live-capability-2026-09-13.md).

- [x] **W2 sandbox live revalidation 2026-09-13:** chạy `--nocapture` để
  phân biệt skip-on-unavailable với runtime coverage: 3 sandbox tests và
  2 sandboxed RPC tests pass, không có skip messages. Mount visibility và RPC
  fixture đã chạy; full containment/production authority vẫn chưa chứng minh.
  [Evidence](/home/minh/projects/project-graph-agent/reports/w2-sandbox-live-capability-2026-09-13.md).

- [x] **W1/W2 joint negative — real zero-exit receipt:** owned runner exit 0,
  complete streams, CAS output và durable receipt kết hợp matching Git target;
  cleanup Unverifiable vẫn khiến integration bị từ chối trước/sau DB reopen,
  events giữ nguyên. Foundation/fmt pass. Không sửa completion để tạo Passed;
  positive production acceptance và full W1/W2 vẫn mở.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-target-head-git-adapter-source-review-2026-09-12.md).

- [x] **W1 canonical root retarget refusal:** authority kiểm canonical root
  hiện tại bằng root đã lưu trước Git probes; rename repo rồi đặt symlink
  ở path cũ bị từ chối. Authority mới cho path đã chuyển vẫn đọc được cùng
  snapshot. Regression/foundation/fmt pass; chưa pin inode hoặc chống mọi
  same-UID filesystem race.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-target-head-git-adapter-source-review-2026-09-12.md).

- [x] **W1-I partial — Git target-head adapter:** `GitTargetHeadVerifier` dùng
  host-owned `GitSnapshotAuthority`, so sánh full observed `ProjectRef` trước
  khi tạo `TargetHeadVerification`; worker không chọn root/policy. graph-source
  tests pass. Đây chưa phải host authentication, atomic worktree snapshot hay
  real W2 execution attestation.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-target-head-git-adapter-source-review-2026-09-12.md).
  Follow-up 2026-09-13 sửa test sai TaskSpec constructor; real Git matching,
  claimed HEAD mismatch và tracked-file drift regression pass. Foundation/fmt
  exit 0. Timestamp vẫn do constructor cung cấp, chưa phải fresh host clock;
  production authority và real W2 integration vẫn mở.
  Follow-up clock: bỏ timestamp constructor, lấy SystemTime sau Git comparison
  mỗi lần gọi; regression hai observations và foundation/fmt exit 0. Giới hạn
  timestamp constructor phía trên đã được sửa; wall-clock rollback và khoảng
  hở giữa observation/commit vẫn chưa giải quyết.
  Identity regression kiểm riêng cả sáu ProjectRef fields, file mutation/restore
  và empty commit đổi HEAD trong khi bytes giữ nguyên; foundation/fmt exit 0.

- [x] **W1-I query separation:** chuyển ba SQL production trong
  `integrate_verified` sang query files; giữ parameters, transaction/replay
  và CAS semantics. Foundation gate và fmt check exit 0; migrations không đổi.
  Test-only fixture SQL vẫn là cleanup gap riêng.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-integration-query-source-review-2026-09-12.md).

- [x] **W1-I integration output budget:** `verify_integration` dùng một
  `output_max_bytes` chung cho mọi receipt, checked subtraction trước khi mở
  streams; không cấp lại budget theo check. Regression hai receipts/4 bytes:
  budget 3 từ chối, task giữ Submitted; exact 4 cho phép atomic integration.
  Foundation và fmt check exit 0. Chưa đóng production host/target authority
  hoặc full W1-I/W2 gates.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-integration-output-budget-source-review-2026-09-12.md).
  Follow-up dùng shared policy evaluator trước target I/O; missing check và
  duplicate check-name/distinct run IDs bị chặn, panic-on-access target không
  được gọi, task giữ Submitted. Foundation/fmt exit 0; exact-set positive giữ
  nguyên. Chưa thay thế production authority hoặc full W1-I acceptance.
  Regression tiếp theo kiểm nonzero exit, timeout, cancellation và cleanup
  Unverifiable với đủ check names/output bytes: đều từ chối trước target I/O,
  giữ Submitted; foundation và fmt exit 0. Đây là synthetic receipt coverage.

- [x] **W13.01 partial — domain enum parsing:** bảy enum có `FromStr`, giữ
  API parser cũ và token exact; bảy integration tests kiểm roundtrip toàn bộ
  canonical tokens, legacy parity và từ chối case/whitespace/control inputs.
  Foundation gate và fmt check exit 0. Local lint allows vẫn là compatibility
  exceptions, không coi giảm diagnostics bằng allow là strict gate hoàn thành.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w13-strict-clippy-audit-2026-09-12.md).

- [x] **W1 materialization deadline:** dùng chung một deadline từ trước root
  resolution qua hai stable observation pairs, copy chunks/sync và freeze.
  Regression expired budget từ chối copy/freeze trước side effects; cleanup
  vẫn chạy độc lập. Foundation gate kết thúc exit 0 và fmt check pass.
  Đây là cooperative timeout, không ngắt blocking OS calls hoặc tạo atomic
  snapshot/analyzer attestation; W1/W2 vẫn mở.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-observation-deadline-source-review-2026-09-12.md).

- [x] **W1/W2 snapshot execution fixture:** child đọc bytes materialized sau
  khi live source bị sửa/xóa; test kiểm ProjectRef/graph qua public API,
  exit 0, stdout/stderr EOF, reap và cleanup snapshot. Sửa lỗi E0624 trong
  test đang dang dở; foundation gate và fmt pass. Đây là owned fixture,
  chưa phải analyzer attestation hay full W1/W2 acceptance.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-w2-snapshot-execution-source-review.md).

- [x] **W1 symlink materialization identity:** resolve link bằng directory
  capability, yêu cầu đích regular file thuộc tập Git-visible đã quan sát,
  kiểm length/hash đích khi copy sang alias. Link đến file ignored bị từ chối
  vì bytes chưa thuộc fingerprint; regression chứng minh thay bytes ignored
  giữ ProjectRef nhưng không được materialize. Internal-link positive vẫn pass.
  Foundation workspace và fmt pass; graph-source hiện 27 tests.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-materialized-snapshot-source-review-2026-09-12.md).

- [x] **W1 CLI admission trước materialization:** `verify-evidence
  --snapshot-config` kiểm exact registered AnalysisRun trước source-root
  access/Git/hash/copy; bound verifier kiểm lại sau acquisition. Regression
  missing run + unavailable root chứng minh admission error được trả trước
  filesystem error và stdout rỗng. Foundation workspace và local fmt pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-materialized-snapshot-source-review-2026-09-12.md).
  Registration vẫn chưa chứng minh analyzer đã chạy hoặc quan hệ graph đúng.

- [x] **W1 materializer cleanup follow-up:** owner thư mục tự thaw trước
  cleanup, kể cả lỗi sau freeze và trước khi trả MaterializedSource.
  Regression kiểm cây frozen toàn phần/một phần trước publication; foundation
  gate và fmt qua `scripts/with-local-tools` pass. Source gate/receipt:
  [materialized snapshot](/home/minh/projects/project-graph-agent/reports/w1-materialized-snapshot-source-review-2026-09-12.md).
  Chưa đóng W1 hoặc các gate analyzer/semantic verification.

- [x] **W4/W9 partial — symbolic string argument evidence:** lexical pass có
  opt-in literal/const alias/parameter/template, giữ identity tham số và unknown;
  không evaluate JS hoặc tạo file/table edge. Mặc định output cũ giữ nguyên.
  Ban đầu 23 tests mới và **191 pass/5 files**, tsc emit/noEmit pass.
  Follow-up chặn tham số chưa khởi tạo: 27 tests chuỗi, focused **135 pass/3
  files**, tsc emit pass. Giữ positive cho tham số khai báo trước và body use.
  Probe Orders giữ đủ 11 query keys và URL template; binding chưa xác minh.
  Follow-up local substitution dùng exact callee/parameter/argument positions:
  12 tests mới, focused **161 pass/5 files**, tsc emit pass. Orders tính được
  bốn chuỗi schema/begin/commit/rollback; `sql(name)` chưa giải. Chưa mở target
  file hay xác minh fs/URL/receiver binding; helper chưa nối CLI/MCP/graph.
  [Substitution receipt](/home/minh/projects/project-graph-agent/reports/w4-string-substitution-source-review-2026-09-12.md).
  Full suite snapshot trước follow-up: **5380 pass/1 fail/9 skip**, fingerprint
  khớp; fail UI timing 127,9ms so với <100ms. Rerun riêng UI 61 pass/1 skip,
  không đổi threshold. Full regression source mới vẫn mở, không coi là green.
  Full native-required snapshot substitution: **5393 pass/4 fail/9 skip**,
  fingerprint khớp; ba timeout 5s và highlight 443ms/<400ms. Chạy riêng ba
  files **43 pass**, giữ nguyên assertions; chưa đạt full-suite gate.
  Follow-up `this.query → sql(name) → template` đã có same-file chain helper,
  exact start/end và opt-in callee extent, kiểm receiver/mutation và giữ runtime
  assumptions. **201 focused tests pass/8 files**, tsc emit pass; Luna viết
  6 integration tests, parent review/rerun cùng boundary tests. Orders tính được
  đủ **11 candidate query paths**, chưa xác minh fs/URL hay file đích.
  Helper chưa nối CLI/MCP/persistence/diagram; full suite snapshot chain chưa chạy.
  File-read syntax adapter đã nối exact `readFileSync` import + `new URL`
  + `import.meta.url` với chain; reject fake/shadowed/written bindings và shape
  chưa hỗ trợ. **218 focused tests pass/9 files**, tsc emit pass; Orders đủ
  11 candidates. URL global chỉ là assumption, builtin/module URL runtime chưa
  xác minh. Chưa mở file đích/kiểm containment hoặc ghi graph; full gate vẫn mở.
  [File-read binding receipt](/home/minh/projects/project-graph-agent/reports/w4-file-read-binding-source-review-2026-09-12.md).
  Target capture đã kiểm required source hash, strict containment/no-symlink,
  bounded regular-file reads, target hash/LF lines và source recheck. Orders
  capture đủ **11 targets**, hash khớp; **233 focused tests pass/11 files**,
  tsc emit pass. Luna viết6 capture tests, parent review/rerun. Không trả bodies,
  không thực thi SQL; race-free containment/atomic snapshot vẫn chưa chứng minh.
  Full suite capture đã kết thúc: **5449 pass/1 fail/9 skip**, fingerprint
  trước/sau khớp. Fail UI busiest-symbol 127,579ms/<100ms; chạy riêng UI
  **61 pass/1 skip**, không sửa threshold; không coi full gate là green.
  Chưa persist SourceEvidence/SQL graph; discovery follow-up bên dưới.
  [Target capture receipt](/home/minh/projects/project-graph-agent/reports/w4-file-target-capture-source-review-2026-09-12.md).
  [Chain source gate](/home/minh/projects/project-graph-agent/reports/w4-this-query-chain-source-review-2026-09-12.md).
  [Source/tests](/home/minh/projects/project-graph-agent/reports/w4-string-argument-source-review-2026-09-12.md).

- [x] **W4/W9 partial — tự discovery local file-read chains:** từ source JS,
  nhận diện imported fs/readFileSync, reverse callers với exact lexical/this
  gates, giữ từng invocation và gap/cycle/depth/state/candidate limits. Literal
  top-level không cần caller. Shared method resolver tránh hai bộ luật lệch nhau.
  Probe `orders-file-discovery-smoke.mjs` tự tìm **11 candidates/13 states**,
  capture đủ **11 SQL files** và hash khớp; ground truth chỉ đọc sau discovery,
  không cung cấp offset/chuỗi gọi. 65 focused tests/7 files pass; mở rộng
  native-required **207 pass/11 files**, tsc emit pass.
  Receipt cũ được bảo toàn; thiếu build bị từ chối. Đây là same-file recognized
  imports only, không bao phủ mọi file API/dynamic import/cross-file flow.
  Helper chưa tích hợp CLI/MCP/indexer/persistence; runtime/atomic snapshot và
  full regression gate vẫn mở. Không nâng 0/14 package thành completed.
  [Discovery source/verification](/home/minh/projects/project-graph-agent/reports/w4-file-read-discovery-source-review-2026-09-12.md).

- [x] **W4/W9 partial — CLI `file-reads` không cần index:**
  `codegraph file-reads SOURCE --path ROOT` phát JSON bounded cho JavaScript/
  TypeScript, đọc đúng explicit root, không tạo/open `.codegraph`, không thực
  thi SQL hay ghi graph. Mỗi target trỏ `candidateIndex` vào binding canonical
  trong discovery thay vì lặp cả chain; Orders giảm JSON **44161→25136 bytes**
  nhưng vẫn 11 candidates/11 captures. Hash precondition, source/target byte
  cap, symlink/UTF-8/path refusal, state/depth/candidate/output cap đều có
  coverage; unavailable trả exit1/stdout rỗng, incomplete trả exit2 + JSON.
  **60 tests/7 files** pass ngoài sandbox (child CLI cần spawn) và tsc emit
  pass. Probe emitted CLI Orders pass với exact/one-under output cap và input
  hashes giữ nguyên. Đây chỉ là recognized same-file candidate observation;
  chưa có index/MCP/persistence, runtime hoặc atomic snapshot proof.
  [CLI source/verification](/home/minh/projects/project-graph-agent/reports/w4-file-read-cli-source-review-2026-09-12.md).

- [x] **W4/W9 partial — coordinate contract cho evidence:** discovery/CLI khai
  báo UTF-16 code units; extents zero-based, end exclusive. Test Unicode/CRLF
  kiểm read, URL base và caller hops qua emitted CLI. Sửa read locator từng
  copy cả FunctionCallEvidence: giờ chỉ `{start,end}`. **62 tests/7 files**
  pass, tsc emit pass; Orders giữ11 candidates/11 captures, output19742 bytes,
  exact/one-under cap và input hashes pass. Đây là prerequisite cho Rust
  ingestion; chưa có durable code→SQL graph. Luna quota error nên parent làm
  tiếp; không ghi nhận worker hoàn thành.
  [Coordinate evidence](/home/minh/projects/project-graph-agent/reports/w4-file-read-coordinate-source-review-2026-09-12.md).

- [x] **W0/W9 partial — SQL CLI probe chạy lại được:** thêm script
  `scripts/orders-sql-cli-smoke.mjs BINARY NEW_RECEIPT`, kiểm đủ 11 file/
  13 statements, citation/flags và exact/one-under output cap từng report.
  Chạy binary thật pass; thiếu binary fail, receipt cũ không bị ghi đè;
  fixture/binary hash giữ nguyên. Foundation và fmt chạy lại exit 0.
  Đây là syntax regression, không phải code→query binding hay gate W9.
  [Source/verification](/home/minh/projects/project-graph-agent/reports/w0-sql-cli-repro-source-review-2026-09-12.md).

- [x] **W9 partial — SQL CLI có evidence và output budget:** `analyze-sql`
  nối citation đã lưu → kiểm hash/full-file lines → SQLite syntax → JSON có
  giới hạn byte (tính cả LF), không xuất SQL body hay ghi graph. Bốn CLI tests
  pass: giới hạn chính xác, drift/missing/wrong scope, partial citation và lỗi
  được redaction. Foundation **394 Rust pass/0 fail/3 ignored +16 Node pass**;
  receipt fingerprints của năm file thay đổi vẫn khớp khi kiểm lại; fmt pass.
  Probe CLI Orders lưu **11 reports/13 statements**. Đây là cú pháp, không
  chứng minh physical table, runtime hoặc snapshot Git do caller cung cấp.
  Có foundation `SqlLinkGraph` + `SqlLinkRepository` port: scope theo adapter/
  ProjectRef/graph/code path, code+target citation full-file riêng, UTF-16
  extent và syntax ordinal được validate. V15 lưu header/link/statement/relation
  bằng strict generation-CAS, tombstone khác empty graph và reconstruct fail
  closed; store regression 55 pass/1 ignored, roundtrip/reopen/CAS test pass.
  `publish-sql-links` nhận report capped, re-read code/target dưới host root,
  kiểm path/hash/bytes/lines, index one-to-one và UTF-16 boundary, parse offline
  rồi pre-budget receipt trước CAS; black-box test exact/one-under cap pass.
  Foundation 2026-09-12 exit 0. `sql-links` historical read và
  `invalidate-sql-links` CAS tombstone đã có black-box publish→read→invalidate
  coverage; historical output không re-verify source. `sql-link-context --link`
  trả một candidate lịch sử với code/target citations và syntax observations,
  không đọc/lộ SQL body; unknown seed/invalidated owner và exact/one-under cap
  có black-box coverage. `sql-link-diagram --link` xuất Archify-compatible IR
  gồm hai neutral external components và một dashed `static file-read candidate`
  edge, manifest bằng đúng context, cards nêu không phải runtime/table/DB fact;
  no-body, unknown/invalidated và exact/one-under cap có black-box coverage.
  Local integration gate tạo fixture owned, gọi Archify validate/deliver, kiểm
  HTML không có SQL body và invalid IR không ghi đè last-good artifact. Đây
  không phải BFS/runtime data-flow; broader adapters còn mở nên **W9 chưa xong**.
  [Source và verification](/home/minh/projects/project-graph-agent/reports/w9-sql-cli-source-review-2026-09-12.md).

- [x] **W9 partial — SQLite syntax adapter trong Rust:** `analyze_sqlite`
  dùng sqlparser 0.62.0 tại system adapter, không đưa parser vào domain hoặc
  thực thi SQL. Có hash, ordinal, operation, tên relation cú pháp và parser
  span tùy chọn; chưa suy physical table/access role, span không phải full
  statement citation. Giới hạn byte/token/statement/recursion, lỗi không lộ
  source, Other giữ statement chưa phân loại. Sáu tests kiểm cả 11 SQL files
  Orders; foundation **390 Rust pass/3 ignored +16 Node pass**, fmt pass.
  Luna làm source receipt; root tiếp nhận và viết tests sau khi worker dừng.
  CLI đã nối ở follow-up phía trên; persistence, code→query binding và
  diagram SQL còn mở; **W9 chưa xong**.
  [Source/test receipt](/home/minh/projects/project-graph-agent/reports/w9-sql-parser-source-review-2026-09-12.md).

- [x] **W4 partial — SQL file inventory:** `.sql` đi qua file-level pipeline
  hiện có, giữ hash và sync/reopen/update/delete, không tạo symbol/cạnh giả.
  3 focused tests pass, tsc emit pass; Orders tăng 7→18 file records, đủ
  11 SQL hashes, node/cạnh giữ 63/143. Full suite với native bắt buộc:
  **5358 pass/0 fail/9 skip**, 306 files, fingerprint trước/sau khớp.
  Linux native watcher nhận ba thay đổi
  và đợi ba sync-complete callbacks; lượt probe đóng DB sớm trước đó được
  giữ trong receipt, không coi là teardown pass. Chưa có SQL statement/table
  parser hoặc code→query binding; **W4/W9 chưa hoàn thành**.
  [Source và verification](/home/minh/projects/project-graph-agent/reports/w4-sql-source-review-2026-09-12.md).

- [x] **W4 partial — route-conditioned call-flow:** API `getRouteFlow` và
  `context --route ID --to ID --format json` loại nhánh trái với route trong
  callback chung; hash-check route source, giới hạn traversal/output, giữ
  unknown/incomplete và raw graph riêng. 54 focused tests pass, tsc pass;
  full suite 5353 pass/0 fail/11 skip trên 305 files, fingerprint khớp
  (conditional skips được liệt kê trong receipt, không phải mọi gate đã chạy).
  Orders emitted probe xác nhận ba candidate paths và loại
  `/dispatch → createOrder`, watcher pass. Generic diagram prompt vẫn chưa
  tìm thấy code; chưa có SQL mapping hay runtime proof. **W4/W11 chưa xong**.
  [Source/test receipt](/home/minh/projects/project-graph-agent/reports/w4-route-flow-source-review-2026-09-12.md).

- [x] **W11 partial — graph Compose → diagram thật:** `deployment-diagram`
  tạo typed Archify architecture IR, evidence manifest và mapping từng node/cạnh;
  không giả nguồn Git verified, database/cloud hoặc runtime traffic. Orders
  stored graph đã qua Archify validate/deliver, malformed candidate giữ nguyên
  last-good HTML; browser containment/readability/capture pass và parent đã xem
  ảnh light/dark. Luna viết 7 CLI tests; foundation 384 Rust pass/3 ignored +
  16 Node pass, fmt pass. Chỉ một declared neighborhood, không phải CodeGraph
  request→database diagram hoặc năm-view acceptance; **W11 chưa hoàn thành**.
  [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w11-deployment-diagram-source-review-2026-09-12.md).

- [x] **W4 returned callable — node riêng và đúng chủ thể thực thi:** JS/TS
  returned arrow/function/generator có extent và identity riêng; giữ deferred
  calls trong closure, không tạo cạnh gọi chỉ vì chứa/trả về hàm. Native/wasm
  parity, lexical-this qua nested scopes, static/instance và class trùng tên
  đã kiểm; event/HTTP/link dùng dòng+cột, dead-code không đếm tên tổng hợp như
  binding. Full suite **5055 pass, 9 skip, 0 TODO / 289 files**, fingerprint
  khớp; build và sync/removal regression pass. Orders vẫn 51 nodes/131 edges,
  một receiver issue và chưa có diagram; **W4 chưa hoàn thành**.
  [Source/review/test receipt](/home/minh/projects/project-graph-agent/reports/w4-returned-callables-source-review-2026-09-12.md).

- [x] **W4 symbol ID — phân biệt vị trí và chặn trộn index cũ/mới:** thêm
  cột UTF-16 vào identity ở TypeScript/native; kiểm cùng tên/cùng dòng, Unicode,
  CRLF và parity. Writer từ chối format cũ trước khi ghi; CLI index rebuild
  tường minh, sync giữ đúng class đích khi vị trí thay đổi. Full suite **5009
  pass, 9 skip, 1 TODO / 284 files**, native bắt buộc, fingerprint trước/sau khớp;
  tsc/native build pass. Orders vẫn 51 nodes/131 edges, watcher tốt nhưng còn
  một receiver issue và chưa tạo được diagram; **W4 chưa hoàn thành**.
  [Source/test receipt](/home/minh/projects/project-graph-agent/reports/w4-node-identity-source-review-2026-09-12.md).

- [x] **W4 impact traversal — batch và đúng độ sâu:** dùng batch theo tầng,
  đóng containment cùng độ sâu rồi mới duyệt dependents; không bỏ sót caller
  khi gặp đường dài trước, giữ các call-site và containment edges thực tế.
  Regression SQLite kiểm 600 callers và cold-cache 600 classes/600 methods;
  focused 102 pass/1 skip, tsc pass. Full suite **4986 pass, 11 skip, 2 TODO /
  281 files**, source fingerprint khớp; bài API <100 ms pass, không nới ngưỡng.
  Probe riêng cho impact median 4,196 → 2,822 ms, không là SLA mọi workload.
  [Source/measurement receipt](/home/minh/projects/project-graph-agent/reports/w4-ui-node-performance-source-review-2026-09-12.md).

- [x] **W4 injected-call lifecycle — component đã kích hoạt:** indexAll/sync
  retract cạnh cũ theo producer và commit cạnh/receipt cùng transaction;
  kiểm wiring-only/removal/retry/rollback/source/config freshness. Orders smoke
  có 51 nodes/131 edges, bốn field-call edges thật và watcher hoạt động;
  index vẫn incomplete với một issue, diagram chưa có kết quả. Full suite
  ở mục trên kiểm snapshot này; **không đồng nghĩa W4 hoàn thành**.
  [Lifecycle receipt](/home/minh/projects/project-graph-agent/reports/w4-injected-reconciliation-source-review-2026-09-12.md).

- [x] **W4 injected field call mapping — candidate, chưa ghi graph:** ánh xạ
  exact class/method/call-site, kiểm source hash và inventory, giữ nhiều target
  cùng các vị trí gọi khác nhau; không đoán theo tên. Orders map được bốn cặp
  service/relay → OrderStore. 100 tests pass / 1 TODO, tsc và smoke pass;
  returned arrow chưa có indexed node được giữ caller-unmapped (TODO extraction).
  Còn parameter dispatch, freshness, reconciliation trên indexAll/sync và
  activation/end-to-end verification; **W4 chưa hoàn thành**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-injected-call-mapping-source-review-2026-09-12.md).

- [x] **W4 composed receiver decision — read-only, có giới hạn bằng chứng:**
  nối class/member/prototype effects với allocation và function handoffs;
  trả blocked/incomplete/observed-source-candidate, giữ runtime assumptions.
  Đã bổ sung regression cho alias-depth, eval, escaped keys, arguments và
  prototype boundaries từ adversarial review. Full suite **4920 pass, 11 skip /
  277 files**, fingerprint nguồn khớp; Orders có ba store candidate, endpoint
  incomplete. Graph vẫn 51 nodes/127 edges, chưa bật synthesis hay chứng minh
  câu trả lời end-to-end. Còn mapping, activation và affected-source replacement;
  **W4 chưa hoàn thành**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-receiver-decision-source-review-2026-09-12.md).

- [x] **W4 member-this effects — class-local evidence:** lượt duyệt receiver
  hiện có ghi instance/static this, property writes, method calls và escapes;
  phân biệt arrow với hàm thường, static block/initializer, computed member
  evaluation và constructor return. Join map member evidence tới class index.
  272 tests/11 suites, typecheck/tsc và orders smoke pass; graph chưa thêm cạnh.
  Còn composed receiver gate và synthesis activation/scoped replacement;
  **W4 chưa hoàn thành**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-member-this-effects-source-review-2026-09-12.md).

- [x] **W4 constructor-object/prototype evidence — chưa phải safety gate:**
  ghi prototype writes qua alias, reflection handoffs và exports; đọc cả file
  patch không có allocation. Join map đúng class qua strict import identity,
  giữ classId null khi chưa map được; test refresh loại evidence patch đã bỏ.
  252 tests/10 suites, typecheck/tsc và orders smoke pass; graph vẫn 51 nodes/
  127 edges. Còn instance/static-this effects và composed receiver validation
  trước synthesis activation; **W4 vẫn mở**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-constructor-object-uses-source-review-2026-09-12.md).

- [x] **W4 call identity và parameter handoffs — read-only evidence:**
  local function/const alias/import được nối tới function extent và tham số;
  dùng chung strict ES export walk, giữ unknown/missing/ambiguous và source
  proof. Sửa lỗi gán tên function declaration trong chính body không invalidate
  binding ngoài; regression tái hiện rồi pass. 232 tests/9 suites và tsc pass.
  Orders nối đúng ba argument vào ordersServer/notificationsServer; graph vẫn
  51 nodes/127 edges. Còn receiver gate, prototype/method effects và synthesis
  activation/reconciliation; **W4 vẫn mở**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-call-handoffs-source-review-2026-09-12.md).

- [x] **W4 function-parameter evidence — chưa nối interprocedural handoff:**
  cùng lexical collector ghi tham số, const alias, callback capture, binding/
  property writes, unsupported shape và arguments/eval hazards. Join trả
  functionUses theo file/hash; call effects có vị trí và parameter index.
  212 tests/9 suites pass; typecheck/tsc pass. Orders nhận diện create/dispatch/
  consume trong HTTP callbacks nhưng graph vẫn 51 nodes/127 edges. Còn nối
  call/import identity, kiểm receiver và kích hoạt scoped replacement;
  **W4 chưa hoàn thành**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-function-parameters-source-review-2026-09-12.md).

- [x] **W4 scoped edge replacement — storage seam, chưa nối synthesis:**
  thay heuristic edges theo đúng producer và source scope trong transaction;
  xóa được nguồn không còn candidate, giữ static/SCIP/producer khác và báo
  collision. Reject endpoint thiếu; rollback thật khi SQLite lỗi sau deletion,
  kể cả nhiều source và hơn một parameter chunk. 16 tests mới; full CodeGraph
  suite **4797 pass, 11 skip / 273 files**, typecheck pass. Chưa giải quyết
  affected-source discovery, snapshot freshness hoặc kích hoạt constructor
  inference; **W4 vẫn mở**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-edge-reconciliation-source-review-2026-09-12.md).

- [x] **W4 allocation-use evidence — chưa phải receiver validation:** collector
  dùng lại lexical binding để ghi property writes qua const alias, method calls,
  constructor handoffs và unmodeled escapes; candidate join gắn effects đúng
  allocation của argument. Có source spans, unknown computed keys và spread
  positions; parenthesized eval giữ origin unknown. 163 tests/7 suites pass,
  typecheck và tsc pass; orders smoke giữ 51 nodes/127 edges, watcher hoạt động.
  Orders chỉ ra service được truyền tiếp vào HTTP factory: còn phải theo luồng
  đó, kiểm prototype/method replacement và stale-edge reconciliation trước khi
  bật inference. **requiresReceiverValidation vẫn true; W4 chưa hoàn thành.**
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-allocation-effects-source-review-2026-09-12.md).

- [x] **W4 class-local receiver review — chưa phải whole-program validation:**
  candidate join có hazard codes/vị trí nguồn cho field write, parameter use/write,
  this escape, constructor return, inheritance và trường hợp chưa mô hình hóa.
  Phân biệt arrow/static/nested-function this; chặn effects trong parameter defaults
  và nested-class computed/heritage. 135 tests pass, tsc pass; orders có ba field
  store no-local-hazard nhưng **requiresReceiverValidation vẫn true**. Graph chưa
  thêm cạnh; còn external allocation effects, class-expression parity và stale-edge
  replacement. Không diễn giải no-local-hazard là kiểu runtime đã chứng minh.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-local-receiver-source-review-2026-09-12.md).

- [x] **W4 AST class-export gate — đã nối vào candidate join:** không còn dùng
  first-exported-class/default fallback trong luồng này. Kiểm explicit export,
  rename/re-export, wildcard conflict theo binding identity, diamond và default
  exclusion; giữ missing/unknown/ambiguous cùng evidence. 95 tests pass; tsc pass.
  Probe orders giữ 10 candidate store sites, endpoint unknown; graph chưa thêm cạnh.
  Phát hiện class expression chưa có class node trong index: giữ unresolved và
  còn phải làm extraction parity. Receiver mutation/escape và stale-edge replacement
  vẫn mở; **chưa hoàn thành W4**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-strict-exports-source-review-2026-09-12.md).

- [x] **W4 constructor evidence join — read-only candidate, chưa tạo cạnh:**
  đã ghép assignment/actual arguments với class index và import resolver;
  giữ unknown, nhiều nơi khởi tạo và source hashes. 71 tests pass; build pass,
  engine được compile lại sau sửa lazy grammar loading. Probe orders nguyên trạng
  nhận diện ba field `store` cùng trỏ tới candidate `OrderStore`, endpoint unknown.
  Graph vẫn 51 nodes/127 edges. Còn kiểm chứng strict export identity (upstream
  default-import fallback chưa đủ), receiver mutation/escape và thay thế cạnh cũ
  khi chỉ wiring thay đổi. **W0/W4 chưa qua gate**.
  [Source receipt và hai lần probe](/home/minh/projects/project-graph-agent/reports/w4-constructor-join-source-review-2026-09-12.md).

- [x] **W4 constructor assignment facts — component, chưa nối resolver:**
  đã thêm bộ đọc AST cho `this.field = parameter`, giữ vị trí tham số,
  phạm vi class và source offsets; không coi facts là kiểu receiver hay cạnh
  gọi đã chứng minh. 21 tests mới + 7 regression pass; TypeScript check pass.
  Còn nối actual arguments, import/alias/shadowing, mutation/escape và kiểm
  lại corpus orders; **W4 chưa hoàn thành**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-constructor-facts-source-review-2026-09-12.md).

- [x] **W4 construction-site origins — component, chưa tạo graph edges:**
  AST ghi actual arguments, nguồn khởi tạo qua const alias và runtime import;
  phân biệt scope, shadowing, gán lại, for-in/of và vị trí không xác định sau
  spread. Giữ cả nguồn unknown, không chọn tên class trùng trong toàn repo.
  65 tests pass qua 4 suites; TypeScript check pass. Còn ghép với assignment
  facts, resolve import liên file, mutation/escape và incremental invalidation;
  nguồn khởi tạo không đồng nghĩa kiểu runtime. **W4 vẫn mở**.
  [Source receipt](/home/minh/projects/project-graph-agent/reports/w4-construction-sites-source-review-2026-09-12.md).

- [x] **W4 diagnosis — untyped constructor injection:** probe đối chứng trong
  cùng file xác nhận `this.store = new Store()` có cạnh; truyền `new Store()`
  qua constructor parameter thì chưa có cạnh. Unknown receiver không bị gán
  sang method trùng tên. [Receipt](/home/minh/projects/project-graph-agent/reports/w4-injected-receiver-source-review-2026-09-12.md).
  Đã chốt nguồn thiếu argument→parameter→field inference, chưa triển khai fix;
  không thay fixture orders hoặc nới name guessing để làm xanh benchmark.

- [x] **W0 partial — orders graph smoke:** isolated copy index được 51 nodes,
  127 edges/7 files; chạy năm upstream explore probes, native watcher thấy edit
  sau callback sync, source/dist/fixture không đổi. [Receipt](/home/minh/projects/project-graph-agent/reports/w0-orders-graph-smoke-source-review-2026-09-12.md).
  Chưa đạt flow end-to-end: injected store/relay thiếu callees, SQL không được
  index và câu hỏi diagram không có kết quả. Đây không phải 5 correctness passes,
  MCP transport hay agent benchmark; W0/W4 vẫn mở.

- [x] **W0 P0.T03/T04 — orders corpus v1:** fixture Node với HTTP/service,
  SQLite/outbox, HTTP notification consumer và Docker/Compose description;
  năm câu hỏi/evidence/unknowns nằm ngoài source được index. Test loopback hai
  listener/DB kiểm flow, retry/dedup/rollback; foundation 283 Rust + 13 Node đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w0-orders-fixture-source-review-2026-09-12.md).
  Chưa Docker validation, graph/MCP/watch hay agent baseline/token measurements;
  corpus files chưa commit, không tích toàn bộ W0/W4.

- [x] **W2 partial — CLI verify-rpc-outputs:** exact launch JSON + scoped CAS,
  input/content/output byte caps, optional stream SHA/length và completion giữ
  nguyên; không in raw bytes/methods hay cấp quyền retry. Test cả stream absent,
  present và các lỗi; 283 Rust + 12 Node, fmt/architecture đạt, không thêm deps/migration.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-output-cli-source-review-2026-09-12.md).
  Chưa repair/reconstruct output, orphan recovery/containment/full W2; DB open vẫn
  có thể initialize/migrate, byte cap không thay I/O deadline.

- [x] **W2 partial — RPC output re-verification:** API kiểm exact launch,
  total byte budget, SHA/length từng output và recheck ledger snapshot; kết quả
  chỉ là fresh byte observation, không cấp retry/upgrade completion. Test actual
  fixture/CAS sau reopen, lỗi bytes/budget/expectation/snapshot; 282 Rust + 12 Node,
  fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-output-reverification-source-review-2026-09-12.md).
  Chưa CLI cho verifier, khôi phục bytes/descriptor bị mất hay full W2/containment.

- [x] **W2 partial — RPC ledger host SIGKILL:** parent kill/reap owned helper
  tại bốn mốc claim, spawn, terminal chưa commit/đã commit; hai lần reopen kiểm
  exact ledger/events/outbox, transaction dở không để lại receipt/event và claim
  không reset. Foundation 281 Rust + 12 Node; fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-ledger-kill-source-review-2026-09-12.md).
  Chỉ ledger fixture, PID/output metadata giả lập; chưa actual RPC orphan recovery,
  power-loss, CAS reconstruction hay production containment/full W2.

- [x] **W2 partial — inspection khi terminal writer commit:** hai tests mới
  kiểm pinned read transaction và tám reader độc lập (100 snapshot/reader),
  exact spawn/terminal linkage, không regression, không ghi thêm event, giữ claim
  và receipt sau reopen. Foundation 280 Rust + 12 Node; fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-inspection-concurrency-source-review-2026-09-12.md).
  Không đổi production code/deps/migration; chưa OS-crash/CAS reconstruction hay
  production containment/full W2. Stress test không bao phủ mọi lịch xen kẽ.

- [x] **W2 partial — terminal receipt inspection/CLI:** launch/spawn/terminal
  và references đọc trong một deferred transaction; corrupt linkage báo lỗi trước
  task filter. CLI thêm nullable terminal brief, không lộ methods/raw output hay
  cấp retry authority. Hai tests mới + mở rộng 12 corruption cases; foundation
  đạt 278 Rust + 12 Node, fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-terminal-inspection-source-review-2026-09-12.md).
  Chưa OS-crash/reconstruction, terminal-writer interleaving test hay production
  authority/containment/full W2. Output v1 chưa release có thêm field.

- [x] **W2 partial — RPC publication failure tests:** ba tests dùng owned
  processes, SQLite/CAS thật; stderr lỗi giữ stdout đã xác minh, final ledger
  lỗi trước/sau commit retry cùng receipt sau reopen không spawn lại; output
  malformed/truncated giữ completion và pending count. Foundation đạt 276 Rust
  + 12 Node; fmt/architecture đạt, không đổi production code/deps/migration.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-publication-failures-source-review-2026-09-12.md).
  Chưa OS-crash/reconstruction, terminal CLI, production authority hay full W2.

- [x] **W2 partial — bound RPC output → CAS → terminal ledger:** prepare receipt
  từ bytes/pending/completion thật; publish kiểm stored spawn, giữ descriptor cố
  định, ingest/readback cả hai output rồi ghi receipt. Mở rộng actual normal/cancel
  test với lying writer, retry và reopen; 273 Rust + 12 Node, fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-output-publication-source-review-2026-09-12.md).
  Failure injection/publication được bổ sung ở entry phía trên; vẫn chưa có
  terminal CLI/crash recovery hay production authority/full W2; đây là staged writes.

- [x] **W2 partial — trusted fixture launch binding:** `launch_bound` giữ exact
  spawn observation cùng supervisor và terminal result qua opaque wrappers;
  premature finish trả lại ownership, legacy launch downgrade tường minh.
  Một test mới với normal/cancel actual fixtures; 273 Rust + 12 Node,
  fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-bound-launch-source-review-2026-09-12.md).
  Chưa production identity/approval/containment, receipt/CAS publication hoặc
  crash recovery/full W2/W1-I; chỉ host-owned trusted fixture provenance.

- [x] **W2 partial — terminal ledger boundary tests:** tám connection tranh
  cùng/khác report chỉ một insert; late receipt không undo cancel/claim; 12
  corrupt index/reference cases lỗi trước scope filter và không ghi thêm.
  V12 upgrade so sánh nguyên artifact rows. Ba tests mới; 272 Rust + 12 Node,
  fmt/architecture đạt, không đổi production code/migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-terminal-ledger-boundaries-source-review-2026-09-12.md).
  Còn actual host/epoch binding, CAS output publication, CLI và crash recovery;
  kiểm thử tranh chấp thread không thay bằng chứng OS-crash/full W2/W1-I.

- [x] **W2 partial — terminal RPC ledger V13:** một immutable receipt/launch,
  exact stored spawn/run/artifact linkage, read transaction, atomic event/outbox;
  replay no-op và changed report conflict. Năm tests mới gồm missing prerequisites,
  rollback, malformed receipt và V12 upgrade. 269 Rust + 12 Node, fmt/architecture
  đạt, không thêm dependency hoặc sửa migration cũ.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-terminal-ledger-source-review-2026-09-12.md).
  Còn concurrent writers/late cancel/corrupt reference tests, actual host binding,
  CAS output publication, CLI và crash recovery; chưa full W2/W1-I.

- [x] **W2 partial — terminal RPC wire v1:** converters tái dùng nested contracts,
  required nullable outputs/progress, pending uncertain=true, reject missing/
  duplicate/unknown fields của structs mới và kiểm domain linkage. Bốn tests
  mới; 264 Rust + 12 Node, fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-terminal-wire-source-review-2026-09-12.md).
  Nested completion giữ semantics cũ (exit_code thiếu là unknown); không phải
  canonical JSON. Chưa durable ledger/host binding/CAS/crash recovery/full W2.

- [x] **W2 partial — domain terminal RPC receipt:** private checked contract
  gắn spawned launch với output run/artifacts, timing, completion, pending
  uncertain và last-frame counters. Năm tests mới kiểm linkage/caps/order,
  trạng thái mâu thuẫn và Debug redaction; 260 Rust + 12 Node đạt,
  fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-terminal-domain-source-review-2026-09-12.md).
  Chưa wire/ledger, xác minh host/epoch/bytes hay crash recovery; contract chỉ
  kiểm cấu trúc khai báo, không chứng minh RPC thành công hoặc cấp quyền retry.

- [x] **W2 partial — supervisor → terminal accounting bridge:** consume input,
  chốt pending epoch và giữ output/completion/elapsed, last-frame counters,
  errors cùng ownership Child chưa reap. Mở rộng bốn process tests hiện có;
  255 Rust + 12 Node, fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-supervised-terminal-source-review-2026-09-12.md).
  Chưa validated launch binding/durable receipt/crash recovery; chưa fault-inject
  nhánh unreaped Child của bridge. Đây là raw host result, chưa full W2/W1-I.

- [x] **W2 partial — immutable terminal RPC accounting:** consume connection/
  pending table để chốt epoch + ID/method unresolved theo thứ tự, tất cả uncertain;
  không nhận thêm reply sau chốt và Debug không lộ epoch/method. Ba tests mới;
  foundation 255 Rust + 12 Node, fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-terminal-accounting-source-review-2026-09-12.md).
  Chưa bind supervisor/output hoặc durable terminal receipt/crash recovery;
  snapshot không chứng minh process exit hay cấp quyền retry. Full W2 còn mở.

- [x] **W2 partial — spawn outcome trong inspection/CLI:** một SELECT đọc
  task/launch/claim/observation nhất quán; dữ liệu hỏng báo lỗi trước scope filter.
  `rpc-launch` thêm brief observation (disposition/time/PID), vẫn unknown liveness
  và không cấp quyền retry/attach/kill. Ba store + một CLI tests mới, gồm readers
  đồng thời; mở rộng tests scope, corruption và exact output cap.
  Foundation 252 Rust + 12 Node, fmt/architecture đạt, không thêm migration/deps.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-spawn-inspection-source-review-2026-09-12.md).
  Chưa terminal receipt/crash recovery/full W2; output v1 chưa release có field
  mới, strict consumer cũ cần cập nhật, DB open vẫn có thể initialize/migrate.

- [x] **W2 partial — durable spawn observation V12 + host recording:** ghi
  kết quả gọi spawn gắn exact launch đã claim; replay bất biến, event/outbox atomic,
  V11 upgrade giữ rows/history và không bịa outcome. Host fixture ghi PID thật
  hoặc lý do không spawn; lỗi ghi DB giữ Child và lỗi OS để caller reap/reconcile.
  Năm wire/domain + chín ledger + một migration + hai fixture tests mới;
  foundation 248 Rust + 12 Node, fmt/architecture đạt, không thêm dependency.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-spawn-observation-source-review-2026-09-12.md).
  Chưa terminal RPC receipt, outcome trong CLI, crash recovery hoặc full W2;
  PID lịch sử không chứng minh liveness/cleanup và không cấp quyền attach/kill.

- [x] **W2 partial — RPC ledger inspection trước recovery:** API snapshot đọc
  descriptor/task/claim trong một SELECT; CLI `rpc-launch` trả JSON giới hạn byte,
  không ghi claim/event và không lộ argv/env. Năm store + năm CLI tests mới kiểm
  reopen/cancel/scope, corruption, concurrent readers, output cap và lỗi parse.
  Foundation 231 Rust + 12 Node, fmt/architecture đạt; không thêm migration/deps.
  [Receipt + source review](/home/minh/projects/project-graph-agent/reports/w2-rpc-query-source-review-2026-09-12.md).
  Liveness/retry authority vẫn unknown/false; DB open thông thường vẫn có thể
  initialize/migrate. Chưa outcome persistence hoặc crash recovery/full W2.

- [x] **W2 partial — trusted RPC fixture launch:** nối exact host-selected spec,
  kiểm executable/env/cwd, prepare initialize → durable claim → spawn → supervisor.
  Năm integration tests kiểm context/RPC/reap, reopen không spawn lặp, rejection
  trước ledger, lỗi permission và cancel sau claim không reset claim.
  Foundation 221 Rust + 12 Node, fmt/architecture đạt; không thêm dependency.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-fixture-launch-source-review-2026-09-12.md).
  Chỉ owned Linux fixture, chưa production approval/containment/crash recovery;
  không tích full W2/W1-I hoặc mở native dispatch.

- [x] **W2 partial — durable RPC ledger V11:** register toàn bộ spec, claim
  một lần với current task/owner/fence/expiry; unique origin-attempt và host/epoch
  chặn alias ID. Event/outbox atomic; replay lịch sử không cấp quyền hoặc reset.
  12 integration tests, gồm tám connection tranh một claim, lỗi event/outbox,
  expiry/cancel/reacquire và corrupt linkage; upgrade V10 giữ history/rows.
  Foundation 216 Rust + 12 Node, fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-ledger-source-review-2026-09-12.md).
  Chưa host approval, spawn hoặc crash giữa claim/spawn; không tích full W2/W1-I.

- [x] **W2 partial — RPC launch descriptor:** domain private/validated và wire
  schema v1 `stdio_jsonl`; tách process/connection, bind origin task/lease,
  host/approval reference và execution snapshot. Không đổi CheckCommand stdin
  đóng. Bốn domain + 11 wire tests mới pass, gồm duplicate/missing fields,
  sai transport/schema, mismatch và Debug redaction. Foundation 203 Rust +
  12 Node, fmt/architecture đạt; không thêm dependency.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-rpc-launch-contract-source-review-2026-09-12.md).
  Đây là description, chưa approval authority, durable RPC claim/spawn hoặc
  full W2/W1-I; frame fit và host identity vẫn phải kiểm khi nối runtime.

- [x] **W2 partial — prepare/attach giữ ownership:** ConnectionSetup chuẩn bị
  initialize trước spawn, lấy pipes từ cùng Child; thiếu pipe trả lại Child và
  giữ handles còn lại, caller kill/reap. Tám cấu hình lỗi và ba missing-pipe
  cases đã kiểm; bốn supervisor tests dùng đường mới, attach chưa gửi byte nào.
  Foundation 188 Rust + 12 Node, fmt/architecture đạt.
  [Receipt + review song song](/home/minh/projects/project-graph-agent/reports/w2-connection-attachment-source-review-2026-09-12.md).
  Chưa fault-inject fcntl, approved spawn/containment hay W1-I/full W2.

- [x] **W2 partial — cancel RPC khi stdin nghẽn sau handshake:** fixture Linux
  ngừng đọc, request lớn ghi dở rồi tiến độ đứng yên; cancel không cần event
  consumer, reap child và giữ nguyên ID/method/counters cùng trạng thái uncertain.
  Test mới pass 10 lần liên tiếp; foundation 186 Rust + 12 Node, fmt/architecture
  đạt. [Receipt](/home/minh/projects/project-graph-agent/reports/w2-blocked-rpc-source-review-2026-09-12.md).
  Chỉ đóng blocked-input fixture; vẫn cần poll đều, chưa containment/approved
  spawn hoặc full W2/W1-I/native dispatch.

- [x] **W2 partial — direct-child connection supervisor:** kết hợp child + I/O,
  timeout/cancel trước data, một consuming waiter, shutdown stdin rồi drain;
  pause event consumer không chặn control. Ba process tests pass; foundation
  185 Rust + 12 Node, fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-connection-supervisor-source-review-2026-09-12.md).
  Deadline từ attach và cần poll đều; chưa approved spawn/registry/containment,
  cleanup vẫn Unverifiable, chưa full W2/W1-I hoặc native dispatch.

- [x] **W2 partial — actual pipe frame boundaries:** nhận đúng frame 4096 byte,
  vượt cap báo TooLarge, JSON lỗi báo Decode, EOF giữa frame báo Truncated;
  giữ raw evidence/pending initialize uncertain. Foundation 182 Rust + 12 Node
  pass, fmt/architecture đạt. [Receipt](/home/minh/projects/project-graph-agent/reports/w2-pipe-frame-boundaries-source-review-2026-09-11.md).
  Chỉ hoàn tất fixture framing Linux, chưa full supervisor/cleanup hoặc W2/W3.

- [x] **W2 partial — bounded connection I/O:** poll stdin/stdout/stderr, tối đa
  một event mỗi lượt, giữ raw bytes/pending metadata khi stop/error, stderr drain
  riêng sau stdout EOF. Ba process tests: JSONL + 512 KiB stderr không newline,
  overflow và stop sớm. Foundation 180 Rust + 12 Node pass; fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-connection-io-source-review-2026-09-11.md).
  Chưa tích hợp process supervisor/deadline/reap/cleanup hay full W2/W3.

- [x] **W2 partial — connection/input bridge:** JSONL handshake qua pipe fixture,
  ACK chỉ mở ready sau ghi đủ bytes; busy không reserve ID mới, close/write error
  giữ pending metadata uncertain. Ba actual-pipe tests pass; foundation 177 Rust
  + 12 Node, fmt/architecture đạt (7 crates, 41 direct declarations).
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-connection-input-source-review-2026-09-11.md).
  Chưa combined read/write/control supervisor, durable RPC checkpoint, cleanup
  scope hoặc native dispatch; full W2/W3 vẫn chưa hoàn tất.

- [x] **W2 partial — nonblocking stdin:** một frame có byte cap, ghi từng chunk,
  close giữ partial-delivery counters, BrokenPipe sticky và không retry ngầm.
  Ba actual-process tests pass; foundation 174 Rust + 12 Node, fmt/architecture
  đạt. [Receipt](/home/minh/projects/project-graph-agent/reports/w2-nonblocking-input-source-review-2026-09-11.md).
  Chưa nối JSONL/correlation/supervisor, không đổi CheckCommand closed-stdin.
- [x] Rà soát cleanup Linux với native reviewer và main đối chiếu source:
  [kết luận có giới hạn](/home/minh/projects/project-graph-agent/reports/w2-cleanup-review-2026-09-11.md).
  Chốt exclusive reaping/ownership constraints; cgroup/pidfd còn cần xác minh
  capability/API trước triển khai. Reviewer đã đóng; full W2 vẫn chưa đạt.

- [x] **W2 partial — receipt stop matrix:** composition thật bao phủ exit 23,
  timeout, stdout truncation và spawn permission failure; receipt sau reopen giữ
  outcome/stream state, claim không được cấp lại, empty streams dedup đúng hash.
  Foundation 171 Rust + 12 Node pass; fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-receipt-stop-matrix-source-review-2026-09-11.md).
  Chưa production dispatch/cleanup hoặc W1-I acceptance; kết quả reviewer cleanup
  và giới hạn chấp nhận được ghi ở receipt review riêng phía trên.

- [x] **W2 partial — descendant-held pipes (Linux):** tiến trình cha thoát,
  hậu duệ giữ stdout/stderr; bản đầu trả Incomplete/Unverifiable theo drain deadline.
  Sau process-scope teardown, fixture kiểm cả hai stream Complete, hậu duệ bị
  terminate/reap trước 900 ms; cleanup vẫn Unverifiable. Wrapper chạy lại đạt
  ngày 2026-09-13 và đã gọi ignored helper trong foundation; không chạy helper
  trực tiếp khi thiếu setup environment.
  Test subreaper riêng reap đúng PID hậu duệ trước kết thúc. Foundation 168 Rust
  + 12 Node pass, 2 standalone helpers được parent gọi; fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-descendant-pipe-source-review-2026-09-11.md).
  Chỉ hoàn tất fixture Linux này, chưa production process-tree cleanup/sandbox,
  đa nền tảng hoặc full W2; không nâng cleanup thành Complete.

- [x] **W2 partial — registered-only host death:** thêm SIGKILL boundary trước
  claim; hai lần reopen giữ descriptor/event/outbox và không có claim/spawn/terminal.
  Lease hết hạn bị từ chối; lease còn hạn claim đúng một lần và không reset sau
  reopen. Đây là ledger fixture, chưa live-worker recovery hoặc full W2.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-preclaim-crash-source-review-2026-09-13.md).

- [x] **W2 partial — real-adapter composition:** test nối owned Linux process
  với DirectoryArtifacts và SQLite plan/claim/receipt; đọc lại sau reopen,
  không cấp lại claim, blob sai cùng độ dài bị HashMismatch. Foundation 167 Rust
  + 12 Node pass; fmt/architecture đạt (7 crates, 38 direct declarations).
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-real-adapter-composition-source-review-2026-09-11.md).
  Đây là test composition, chưa production dispatch/candidate application,
  crash reconciliation hay process-tree cleanup; outcome vẫn Unknown.

- [x] **W2 partial — retained output publication:** bridge kiểm cả hai
  descriptors/hash/length/budget trước ghi, dùng lại ingestion + readback;
  stderr lỗi vẫn trả bằng chứng stdout đã lưu để retry, không chạy lại process.
  Ba tests port in-memory pass; foundation 166 Rust + 11 Node, fmt/architecture
  đạt. [Receipt](/home/minh/projects/project-graph-agent/reports/w2-output-publication-source-review-2026-09-11.md).
  Chưa composition process + SQLite/filesystem + execution receipt; content
  observation không chứng minh launch/cleanup hoặc cấp integration authority.

- [x] **W2 partial — drain lifecycle:** sửa runtime timeout không được đổi
  kết quả sau khi child đã reap; cancellation và lỗi đã ghi nhận vẫn giữ đúng
  semantics. Thêm test UnixStream còn writer thì chưa EOF, thiếu reader là lỗi,
  cùng fixture stdout/stderr đồng thời 512 KiB mỗi luồng giữ byte chính xác.
  Foundation 163 Rust + 11 Node pass, fmt/architecture đạt.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w2-drain-lifecycle-source-review-2026-09-11.md).
  Held-writer unit test chưa là descendant-process cleanup test; không tích
  full W2, không đổi cleanup Unverifiable hoặc mở native/untrusted jobs.

- [x] **W2 partial — owned Linux process fixtures:** graph-execution kiểm
  executable/env digests, exact argv, canonical cwd trong root và closed stdin;
  đọc stdout/stderr nhị phân nonblocking có byte cap, timeout/cancel, reap riêng
  với EOF. Tám test executor, tổng foundation 159 Rust + 11 Node pass; fmt và
  architecture gate đạt (7 crates, 36 direct dependency declarations).
  [Source review và giới hạn](/home/minh/projects/project-graph-agent/reports/w2-fixture-executor-source-review-2026-09-11.md).
  Cleanup vẫn Unverifiable; chưa sandbox/process-tree ownership, launch claim
  wiring, artifact/receipt publication, transport backpressure hay đa nền tảng.
  Không mở native dispatch/untrusted jobs; full package count vẫn 0/14.

- [x] **W1-C development gate:** audit từng điều kiện contract/binding/status/
  ledger và rerun foundation 151 Rust + 11 Node pass; reviewer native cùng model
  không tìm thấy blocker trong scoped trust-boundary review.
  [Ma trận bằng chứng và giới hạn](/home/minh/projects/project-graph-agent/reports/w1-c-development-gate-audit-2026-09-11.md).
  Chỉ mở W2 owned trusted fixtures; không hoàn tất W1/W2, không native dispatch,
  untrusted repo jobs, merge hoặc graph writes. Full package count vẫn 0/14.

- [x] W1 partial: V10 one-shot launch claim yêu cầu exact registered plan,
  revalidate submitted/policy/candidate/analysis và chưa có receipt. Claim +
  execution_launch_claimed event/outbox commit cùng transaction; chỉ lần đầu
  trả true, replay/restart/cancellation không tái cấp. Không TTL/reset API.
  Tests 8 contenders, restart, preconditions, event/outbox rollback và V9 upgrade
  không fabricate claim; 151 Rust + 11 Node pass, 1 subprocess helper như trước.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-launch-claim-source-review-2026-09-11.md).
  Đây là một ledger claim, chưa exactly-once OS execution/approval; crash sau
  claim còn cần runtime reconciliation. Consumer phải hỗ trợ event kind mới.

- [x] W1 partial: registry kiểm thêm analysis-run identity/snapshot của candidate
  trong transaction; regression tái hiện orphan trên DB fixture đã bỏ delete
  trigger/FK, rồi pass sau fix (không là API bypass bình thường). Plan corruption
  chặn read/replay/receipt. 16 race fixtures kiểm plan với receipt khác host hoặc
  cancellation qua hai connections; reopen chỉ có trạng thái tuần tự hợp lệ.
  Foundation: 146 Rust + 11 Node pass; 1 subprocess-only helper như trên.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-plan-registry-adversarial-source-review-2026-09-11.md).
  Không chứng minh mọi dạng DB tampering, launch-once hay host authentication.

- [x] W1-C partial: ExecutionPlan domain/wire và V9 registry cố định binding/
  host/execution snapshot. First insert đối chiếu submitted task/owner/fence/
  sequence/candidate, policy và candidate descriptor trong IMMEDIATE transaction;
  đã có receipt thì không đăng ký hồi tố. Exact replay sau cancellation chỉ
  lịch sử, không cho relaunch. Receipt có plan phải match target trong transaction.
  Tests strict wire, restart/replay/conflict, missing/mismatch, cancellation,
  V8 upgrade không fabricate plan; 143 Rust + 11 Node pass (1 subprocess helper
  ignored khi discovery, được parent test chạy). Future fixture hiện V10.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-execution-plan-source-review-2026-09-11.md).
  Chưa original-expiry attestation, launch lifecycle/approval/host authentication;
  registry race/corruption coverage và audit W1-C còn mở.

- [x] W1 partial: 8 Store connections tranh receipt ID cho đúng 1 insert và
  7 replay (identical) hoặc 7 conflict (khác host); reopen giữ exact winner,
  events không đổi. Subprocess fixture thoát không chạy Drop trước/sau commit:
  trước commit không có receipt và retry được, sau commit giữ receipt/replay.
  Foundation: 138 Rust pass + 11 Node pass; 1 helper ignored khi chạy trực tiếp,
  nhưng parent test đã chạy helper trong 2 subprocess và kiểm exit code.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-receipt-race-crash-source-review-2026-09-11.md).
  Chưa power-loss, concurrent writer processes hay pre-launch admission.

- [x] W1 partial: V8 lưu ExecutionReceipt claims bất biến theo run ID, FK task,
  exact full TaskSpec; replay giống hệt trả false, xung đột không overwrite.
  Scoped read kiểm persisted identity/version/task linkage; receipt muộn sau
  cancellation vẫn là diagnostic, không emit event hay đổi task state.
  Tests restart/conflict/scope/corruption và upgrade V7 không fabricate receipts;
  136 Rust + 11 Node pass. Future-version fixture hiện V9 cho binary V8.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-receipt-store-source-review-2026-09-11.md).
  Chưa pre-launch registry/host admission, race/crash receipt tests hoặc authority.

- [x] W1 partial: verify_receipt_outputs kiểm exact host-selected binding/host/
  execution snapshot, tổng byte budget trước blob I/O, exact registered outputs,
  hash/length từng stream và recheck submission sau đọc. Ba tests mới kiểm hai
  outputs/no events, mismatch/budget/missing registration trước open, hash sai và
  cancellation qua kết nối SQLite khác. Foundation: 132 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-receipt-output-source-review-2026-09-11.md).
  Kết quả chỉ là output content observation; không xác minh candidate bytes,
  host/process/snapshot claims, chống replay hay cấp integration authority.

- [x] W1-C partial: ExecutionReceipt domain/wire nối binding, host claim,
  execution snapshot, wall timestamps/monotonic elapsed, output descriptors và
  completion. Complete stream bắt buộc có artifact; kiểm scope/run/kind/budget,
  strict fields/schema và roundtrip. Foundation: 129 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-execution-receipt-source-review-2026-09-11.md).
  Chưa host attestation, output verification hay registry/replay persistence;
  cần audit toàn bộ W1-C trước khi mở executor fixture, chưa tick gate.

- [x] W1-C partial: application đối chiếu CheckRunBinding với submitted ledger,
  registered policy và exact artifact descriptor; đọc lại submission để từ chối
  thay đổi/mất candidate. Tests SQLite kiểm không ghi event, mismatch và thiếu
  registration; scripted recheck kiểm sáu trường thay đổi, mất record và lỗi đọc.
  Foundation: 126 Rust + 11 Node pass. Đây chỉ là metadata observation, không
  chứng minh bytes, original lease expiry, command approval hay quyền execution.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-check-reconciliation-source-review-2026-09-11.md).
  W1-C còn thiếu full execution receipt và host/runtime verification.

- [x] W1-C partial: CheckRunBinding nối run ID/full task/origin lease/submission
  sequence/candidate/policy/check name/command; validation reject cross-snapshot,
  wrong task lease, invalid sequence và check ngoài policy. Strict nested wire
  roundtrip; 123 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-check-binding-source-review-2026-09-11.md).
  Origin lease chỉ provenance; chưa ledger attestation/execution snapshot sau
  apply candidate/output receipt/host authorization. W1-C vẫn chưa qua gate.

- [x] W1-C partial: command/environment fingerprints dùng full SHA-256, versioned
  domain separator và length-prefix; golden vectors đối chiếu Node crypto.
  Tests phân biệt argv boundaries/empty/Unicode, bind đủ command fields, env
  ordering và invalid entries; 120 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-command-fingerprint-source-review-2026-09-11.md).
  Không đọc ambient env; hash không là encryption/approval/host attestation.

- [x] W1-C partial: CheckCommand domain + strict versioned wire giữ program/argv,
  relative cwd, executable/environment digests và time/output limits; reject NUL,
  path/digest/limit không hợp lệ. Argv roundtrip nguyên vẹn, Debug không echo args.
  118 Rust + 11 Node pass; [receipt](/home/minh/projects/project-graph-agent/reports/w1-check-command-source-review-2026-09-11.md).
  Chưa resolve/approve executable, verify env/hash thật hay bind receipt với task;
  contract dành noninteractive checks (stdin đóng), không thay interactive jobs.

- [x] W1-C partial: domain + versioned wire ExecutionCompletion tách stop reason,
  child reap, stdout/stderr completeness và scope cleanup; exit 0 đơn lẻ không
  đủ reported Passed, unknown exit không default 0. Wire reject unknown fields/
  variants/future schema. 115 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-execution-completion-source-review-2026-09-11.md).
  Đây là reported observations, chưa task/command binding, host authentication,
  execution receipt đầy đủ hay integration authority; W1-C vẫn mở.

- [x] W1 partial: 8 kết nối tranh guarded lease chỉ có một winner/fence=1;
  registration đấu guarded/legacy lease qua 16 fixture rounds giữ legal outcomes
  và thứ tự policy→lease khi policy tồn tại. Reopen/outbox khớp event history.
  112 Rust + 11 Node pass; [receipt](/home/minh/projects/project-graph-agent/reports/w1-policy-races-source-review-2026-09-11.md).
  Đây là in-process/separate-connection tests, chưa multiprocess/crash proof.

- [x] W1 partial: PolicyLeaseRepository kiểm full task + registered policy trong
  cùng transaction cấp lease; thiếu/sai không fallback. Shared lease helper giữ
  dependency/fence/event/outbox; tests reclaim, stale owner, cancel, invalid time
  và rollback event/outbox. 110 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-policy-lease-source-review-2026-09-11.md).
  Chưa nối native dispatcher; CLI lease cũ còn host-only legacy, không có policy gate.

- [x] W1 partial: đăng ký policy mới ghi CheckPolicyRegistered + outbox cùng
  transaction; event/outbox failure rollback policy, replay giữ event/time đầu.
  Legacy policy không được backfill lịch sử giả. 108 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-policy-outbox-source-review-2026-09-11.md).
  Đây là registration history, chưa dispatch/verified execution/integration.

- [x] W1 partial: V7 lưu immutable required-check policy theo full task contract;
  đăng ký lần đầu chỉ queued/fence=0, replay cùng policy sau lease được phép,
  đổi policy bị conflict. V6 upgrade không fabricate policy; future V8 và policy
  schema không hỗ trợ bị từ chối. 106 Rust + 11 Node pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w1-policy-ledger-source-review-2026-09-11.md).
  Chưa có policy events/dispatch gate, execution verification hay integration
  authority; legacy lease vẫn chưa yêu cầu policy.

- [x] W1 partial: RequiredChecks policy thuần domain từ chối cấu hình rỗng/trùng,
  báo missing/duplicate/unexpected, wrong candidate, missing evidence và mọi
  outcome khác Passed. Kết quả deterministic theo tên check; 102 Rust + 11 Node
  pass. [Source receipt](/home/minh/projects/project-graph-agent/reports/w1-check-policy-source-review-2026-09-11.md).
  Observations vẫn là claims; chưa resolve/authenticate execution receipts,
  bind target head hay cấp integration authority.

- [x] W1 partial: test adapter trả sai artifact ID/graph và đủ sáu trường project
  đều bị chặn trước I/O; binding sai bị chặn trước lookup. Recheck phát hiện đổi
  spec/owner/fence/sequence/time/artifact và không nuốt repository error.
  99 Rust + 11 Node pass; đây là scripted adapter tests, không chứng minh atomic
  integration hay live runtime. [Receipt](/home/minh/projects/project-graph-agent/reports/w1-submission-adapter-source-review-2026-09-11.md).

- [x] W1 partial: `verify_submitted_content` bind toàn bộ task contract và artifact
  registry/snapshot, kiểm byte budget/SHA-256 rồi đọc lại submission; cancel trong
  lúc đọc làm kết quả thất bại. 95 Rust + 11 Node pass; không ghi task event.
  [Source-first receipt](/home/minh/projects/project-graph-agent/reports/w1-submission-content-source-review-2026-09-11.md).
  Đây là content observation, chưa phải integration authority; checks, target
  head và atomic commit-time revalidation còn mở.

- [x] W1 partial: SubmittedCandidate query đọc task spec/owner/fence và submission
  event/artifact reference trong cùng SQL snapshot; reject missing/duplicate hoặc
  malformed binding. Reopen giữ metadata, đọc không emit event, cancel ẩn candidate.
  92 Rust + 11 Node pass; [source-first receipt](/home/minh/projects/project-graph-agent/reports/w1-submission-source-review-2026-09-11.md).
  Chưa kiểm artifact/content/snapshot/check/target; đây là host-only observation,
  không phải lease/integration authority. W1 và integrate(string) vẫn chưa mở.

- [x] W3 partial: Connection core ghép framing/RPC/pending/handshake private;
  chặn prepare_request trước ack-write, repeat initialize; epoch sai không
  consume frame. Composed-flow tests kiểm request/reply, partial frame và
  failure giữ unresolved metadata; 90 Rust + 11 Node pass.
  [Source-first receipt](/home/minh/projects/project-graph-agent/reports/w3-connection-source-review-2026-09-11.md).
  Core chưa I/O: trả bytes chưa gửi; process ownership, drain stderr, bounded
  queue, permissions/auth và live native acceptance chưa hoàn thành.

- [x] W3 partial: handshake state chờ initialize reply đúng typed ID/metadata,
  chuẩn bị initialized rồi chờ host xác nhận write; lỗi/malformed/fail không
  thành ready. Không dùng codexHome từ response, experimentalApi chỉ là opt-in.
  87 Rust + 11 Node pass; [source-first receipt](/home/minh/projects/project-graph-agent/reports/w3-handshake-source-review-2026-09-11.md).
  Đây là sequencing cục bộ chưa enforce mọi send; connection host, authenticated
  epoch, transport thật và effective capability/live acceptance vẫn còn mở.

- [x] W3 partial: RPC encode_line validate labels, cap tính cả LF và serialize
  trực tiếp; CLI output dùng chung protocol LimitedBuffer, bỏ implementation
  trùng. Roundtrip cả bốn message families, optional omission/null result,
  Unicode escaping, exact/one-under cap; 84 Rust + 11 Node pass.
  [Source-first receipt](/home/minh/projects/project-graph-agent/reports/w3-outbound-source-review-2026-09-11.md).
  Encode chưa gửi/flush pipe; handshake, delivery/recovery và native gate còn mở.

- [x] W3 partial: PendingRequests bounded theo connection epoch, monotonic IDs,
  ghép out-of-order replies, unknown/duplicate trả riêng; server request hoặc
  notification không consume pending. Uncertain/close giữ metadata đối soát,
  late reply còn match; 82 Rust + 11 Node pass.
  [Source-first receipt](/home/minh/projects/project-graph-agent/reports/w3-correlation-source-review-2026-09-11.md).
  Epoch do host cấp, chưa authentication/unique-epoch persistence; chưa
  handshake, outbound dispatch, auto-retry hoặc native runtime acceptance.

- [x] W3 partial: bounded RPC envelope decoder phân biệt request/notification/
  response/error, giữ numeric/string IDs, reject duplicate/mixed envelope;
  unknown method không bị đổi thành completion hoặc auto-approval.
  Framing→RPC interleaving tests pass; tổng 78 Rust + 11 Node.
  [Source-first receipt](/home/minh/projects/project-graph-agent/reports/w3-rpc-source-review-2026-09-11.md).
  serde_json đã khóa được thêm vào protocol, architecture 6 crates/30 direct
  deps; regression cấm JSON dependency trong domain/application. Chưa có
  pending-map, dispatcher/approval, handshake/auth hay native transport.

- [x] W3 partial: incremental LineDecoder cap tính cả CR/LF, trả một frame cùng
  consumed count; oversize/EOF giữa dòng đóng decoder, không tự resync.
  Test Unicode/CRLF qua các chunk sizes, exact cap, EOF và nối lifecycle parser;
  74 Rust + 10 Node pass. [Source-first receipt](/home/minh/projects/project-graph-agent/reports/w3-framing-source-review-2026-09-11.md).
  Chưa nối process/transport, drain stderr, bounded queue hay timeout; không
  coi transport EOF là native terminal. P9.T11 và W3 vẫn mở.

- [x] W3 partial: Deserialize-only native lifecycle projection cho thread/closed,
  thread/status/changed, turn/started và turn/completed; typed IDs/statuses,
  reject request-shaped envelope, unknown/malformed method/status. Không lưu
  transcript trong kết quả và không mutate admission/task khi parse.
  70 Rust + 10 Node pass; [source-first receipt](/home/minh/projects/project-graph-agent/reports/w3-native-events-source-review-2026-09-11.md).
  Chưa nối transport/authentication, frame cap, capability negotiation hoặc
  generation reconcile (closed thread có thể resume cùng ID); P9.T11 vẫn mở.

- [x] Admission bind attempt↔native thread một-một, replay idempotent/conflict,
  late bind sau stop và unknown terminal không release slot. Source-first review
  T3 Code hiện tại đã tách confirm_not_dispatched (unbound-only) khỏi native
  thread terminal. Gate pin root ID, chặn root-as-child và parent mismatch kể cả
  replay; late child binding dùng lineage giữ lại sau parent terminal/stop.
  Invalid root/parent IDs không đổi mapping; 67 Rust + 10 Node pass.
  Source/decision trước sửa ở
  [W5 receipt](/home/minh/projects/project-graph-agent/reports/w5-admission-source-review-2026-09-11.md).
  Chưa có authenticated event adapter, persistence/recovery hoặc live Codex gate.

- [x] SwarmAdmission in-memory host-owned: shared active/total/depth cap,
  dedup attempt/work-key, deadline/stop; parent terminal không release child,
  finished attempt không refund total budget. 63 Rust + 10 Node pass.
  State private/non-Clone; chưa nối native runtime, persistence/recovery,
  canonical dedup, retry, dependency readiness hoặc permission enforcement.
  P9.T12 vẫn mở; host phải serialize cùng một gate cho cả cây.

- [x] Cả 13 JSON-file read sites ở CLI dùng cap 8 MiB trước parse (tối đa
  limit+1 byte đọc). Unit exact/empty/infinite-reader và CLI oversize registrations,
  enqueue đúng ngưỡng/không event khi lỗi: 60 Rust + 10 Node pass.
  Chưa bound parser overhead, DB-resident metadata hoặc special-file deadline;
  artifact stdin giữ budget riêng, startup DB migration vẫn có thể xảy ra.

- [x] CLI task có hard serialized JSON-line cap (--max-output-bytes), mặc định
  64 KiB/trần 16 MiB. Encode trước publish, overflow không ghi partial stdout;
  UTF-8/escapes/newline/exact boundary/zero được test. 58 Rust + 10 Node pass.
  Chưa giới hạn DB input/Value allocation, chưa áp dụng cho mọi command hoặc
  bảo đảm atomic write khi OS pipe lỗi; W4 ContextEnvelope vẫn mở.

- [x] Application scoped task query và CLI task ID --scope TASK.json: so đủ
  ProjectRef + graph version, mismatch→null, malformed scope không fallback
  unscoped. 57 Rust + 10 Node pass (mở rộng CLI regression hiện có).
  Filter sau validated lookup; chưa là DB row-level security hoặc native child
  authorization. Host-bound scope và hạn chế raw DB/unscoped tools còn mở.

- [x] TaskQueryRepository + CLI task ID: spec/state từ một SQLite statement,
  validate wire/domain và row identity, missing→null, không phát event hoặc cấp
  lease. Regression queued/leased/submitted/cancelled qua process mới và corrupt
  JSON/identity; 57 Rust + 10 Node pass. Đây là coordinator DB-owner view, chưa
  phải scoped subagent endpoint, native liveness hoặc bounded fleet listing.

- [x] Tạo repo sản phẩm tại `/home/minh/projects/project-graph-agent`.
- [x] Tạo workspace gồm `domain`, `application`, `protocol`, `store`, `source`, `cli`;
  kiểm tra Cargo build qua test workspace. Đã có gate declared Cargo dependencies
  riêng (bên dưới); chưa kiểm module-level/transitive/runtime boundaries.
- [x] `scripts/check-architecture.mjs` kiểm 6 package/29 declared dependencies
  qua cargo metadata: package alias, optional/platform/dev/build edges và
  internal path identity; package/dependency mới phải có policy review.
  `sh scripts/validate-foundation.sh` pass: 10 Node tests + 21 Rust tests.
  Bằng chứng `reports/w1-architecture-gate-2026-09-11.md`; local gate, chưa CI
  hoặc cross-platform/strict-lint/whole-product acceptance.
- [x] TaskId/WorkerId được validate; TaskSpec chặn self/duplicate dependency.
  Bằng chứng: `crates/domain/src/lib.rs`, test domain.
- [x] Wire task chặn schema version không hỗ trợ trước khi chuyển sang domain.
  Bằng chứng: test protocol; chưa có bộ JSON Schema độc lập.
- [x] Enqueue idempotent với cùng spec; insert dependency lỗi rollback task/event.
  Bằng chứng: tests store.
- [x] Lease fencing tăng sau hết hạn; worker cũ không submit được sau reopen;
  hai connection cạnh tranh chỉ một lease thành công. Bằng chứng: tests store.
- [x] Submission không mở khóa task phụ thuộc trước transition integrated.
  Bằng chứng: test store; chưa chứng minh verification/merge thật.
- [x] W1 application/store cancel queued/leased/submitted, idempotent sau reopen,
  chặn submit/lease/integrate sau hủy; event/outbox atomic có fault-injection
  rollback test. Không mở khóa dependent; giữ actor/reason gốc. Foundation gate
  hiện pass 28 Rust + 10 Node tests. CLI `cancel` có subprocess tests, JSON
  không xác nhận process termination. Race cancel/cancel, cancel/submit và
  cancel/integrate qua hai connection có 8 trials mỗi loại. Chưa independent
  wire schema/authorization/process kill hoặc scheduler propagation.
  Báo cáo `reports/w1-cancellation-2026-09-11.md` trong repo sản phẩm.
- [x] Schema ban đầu nằm trong `crates/store/migrations/V1__create_task_store.sql`,
  được refinery embed; test xác nhận bảng migration history trên DB mới.
- [x] Chạy `cargo test --workspace --locked --offline` ngày 2026-09-11:
  21 tests pass, 0 fail; gồm CLI evidence và 6 integration tests graph-source.
  Application source verifier được kiểm qua integration tests; chưa có unit suite riêng.
- [x] SourceEvidence có validated constructor, immutable ledger migration V3,
  idempotent record, conflict detection và lookup theo toàn bộ ProjectRef +
  GraphVersion. SQL mới nằm trong files riêng. CLI record-evidence/evidence
  được test qua subprocess, gồm roundtrip, stale snapshot, conflict và schema.
  Citation khi ghi vẫn unverified; đọc kiểm chứng qua verify-evidence riêng.
- [x] Thêm graph-source adapter qua SourceReader port và application verifier:
  kiểm SHA-256 toàn file, UTF-8, inclusive line range và input/output byte limits;
  trả source slice giữ nguyên CRLF. CLI verify-evidence hoạt động từ ledger.
  Fixture 2.002 dòng trả đúng hai dòng trong budget 32 byte; sửa ngoài slice
  bị phát hiện. Test Linux chặn symlink escape, FIFO và file không hợp lệ.
  cap-std 4.0.3 và sha2 0.10.9 được khóa trong Cargo.lock.
- [ ] Nối source verifier với snapshot authority, AnalysisRun identity và
  semantic verification. Hiện root/snapshot binding do host truyền vào, output
  ghi caller_supplied; source vẫn untrusted. Thành công chưa tạo accepted fact
  hoặc verifier decision/outbox. Chưa xác minh portability ngoài Linux.
  **Partial 2026-09-12:** `graph-application` đã có port
  `SourceSnapshotAuthority`, typed `SourceSnapshotBinding` và
  `verify_source_identity`: authority phải bind đúng root/project/graph trước,
  store phải trả `AnalysisRun` exact theo evidence, rồi mới được đọc/hash source.
  Authority failure, binding mismatch, missing/mismatched run đều fail closed;
  test xác nhận reader không bị gọi trước identity gates. Đây chưa phải concrete
  Git/worktree authority, chưa nối CLI, chưa chứng minh toàn-tree fingerprint,
  semantic relationship hoặc accepted fact/outbox. **Follow-up partial:**
  `graph-source::GitSnapshotAuthority` đã resolve canonical worktree và
  `git-common-dir`, hash tracked/deleted/non-ignored-untracked entries, bytes,
  executable bit, symlink target, HEAD và porcelain status dưới byte/timeout
  caps; linked-worktree và fail-closed mismatch/budget/non-Git tests pass.
  Architecture gate giữ hashing ở source adapter và graph-domain không có crypto.
  Đây vẫn chỉ là point-in-time host observation; chưa atomic concurrent snapshot,
  AnalysisRun/semantic verifier wiring hoặc accepted fact/outbox. **CLI follow-up
  partial:** `verify-evidence --snapshot-config CONFIG.json` đã parse config strict,
  kiểm root trùng command, dùng `GitSnapshotAuthority`, yêu cầu exact registered
  `AnalysisRun` trước khi đọc source và chỉ ghi `snapshot_binding=git_authority`
  sau khi mọi gate pass. Legacy invocation vẫn giữ
  `snapshot_binding=caller_supplied`; các command source khác chưa được relabel.
  Test subprocess kiểm missing-run, config field/limit/root mismatch, stale Git
  bytes và successful bound read; workspace test và foundation validator pass.
  **Stability follow-up partial:** authority thực hiện hai full observation liên
  tiếp và trả `SnapshotChanged` nếu root/common-dir/HEAD/status/file identity
  khác nhau; stability unit 2/2; latest graph-source package **23/23** pass.
  Đây chỉ là bounded consistency
  check, không phải atomic filesystem snapshot; worktree vẫn có thể đổi sau
  observation thứ hai và trước source read.
  **Deadline follow-up partial:** `ObservationDeadline` được tạo một lần cho
  cả hai observation, giới hạn chung Git child polling, Git text/path parsing,
  entry inspection và file hashing; hết hạn trả `SnapshotTimeout` fail-closed.
  Focused graph-source **23 pass**, CLI snapshot **1 pass** và foundation
  validator **exit 0** với 8 packages/49 direct dependencies. Deadline không
  thể ngắt mọi blocking OS call và không làm stable pair thành atomic.
  **Materializer source-gate 2026-09-12:** đã đọc lại Ripwire cache
  (`ingest_cache.h`, `portablecachecheck.sh`, `cacheidentitycheck.sh`) và
  CodeGraph worktree resolver/tests đúng phần việc trước khi code. Quyết định
  đã áp dụng là materialize Git-visible entries vào owner-scoped temporary
  directory riêng của standard library bằng capability-relative I/O,
  no-follow regular-file open, symlink nội bộ được chuẩn hóa thành bytes tự
  chứa, freeze read-only sau copy, re-observe trước khi trả reader/binding;
  lỗi copy/unsafe link/đổi snapshot phải fail-closed.
  Đây là immutable-by-ownership snapshot cho CLI opt-in, không gọi là atomic,
  không thêm persistent cache và không mở authority cho các command khác.
  **Materializer implementation partial:** GitSnapshotAuthority đã copy
  Git-visible entries vào private owner-scoped temporary directory của
  standard library bằng capability-relative I/O, no-follow final
  regular-file open, bounded/hash-checked copy, normalize symlink nội bộ thành
  regular bytes, freeze read-only trên Unix và re-observe worktree trước khi
  trả MaterializedSource. `tempfile` chỉ còn ở `[dev-dependencies]`; runtime
  dùng `OwnedTempDir` + `Drop` để giữ dependency direction của Clean
  Architecture. verify-evidence
  --snapshot-config dùng bound materialized reader;
  verify_bound_source_identity giữ AnalysisRun gate trước source read.
  Graph-source **25 pass**, CLI snapshot **1 pass**, foundation validator và
  workspace **pass**. Chưa
  có atomic kernel snapshot, hostile same-UID isolation, analyzer attestation,
  semantic graph verification hoặc accepted fact/outbox; không tick full W1.
  Source receipt:
  [`w1-materialized-snapshot-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/w1-materialized-snapshot-source-review-2026-09-12.md).
  Receipts:
  [`w1-source-identity-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/w1-source-identity-source-review-2026-09-12.md),
  [`w1-git-snapshot-authority-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/w1-git-snapshot-authority-source-review-2026-09-12.md),
  [`w1-cli-snapshot-authority-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/w1-cli-snapshot-authority-source-review-2026-09-12.md),
  [`w1-stable-snapshot-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/w1-stable-snapshot-source-review-2026-09-12.md),
  [`w1-observation-deadline-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/w1-observation-deadline-source-review-2026-09-12.md).
- [x] Bỏ public string-based FactAssertion::accept; đường accept mới phải đi
  qua typed candidate + verifier receipt + `GraphWriter` (P1.T07).
- [x] Chặn string-based task integration trong production Store, kể cả task
  submitted: API/CLI trả lỗi, không phát event integrated hoặc đổi trạng thái.
  53 Rust + 10 Node tests pass; regression trực tiếp Store, service và CLI.
  Transition cũ chỉ còn fixture private cfg(test) để kiểm transaction/readiness;
  không được coi fixture này là verified integration đã triển khai.
- [x] Artifact content observations ledger V6: chỉ API nhận VerifiedArtifactContent,
  exact registered metadata, immutable replay/conflict và scoped lookup. V5 upgrade
  không fabricate observations; 52 Rust + 10 Node tests pass. Historical record
  không chứng minh bytes hiện tại, protection hoặc execution.
- [x] CLI verify-artifact --observation-id lưu observation sau kiểm byte thành công;
  artifact-observation ID TASK.json trả lịch sử scoped, không đọc blob và không
  nâng trust. Regression kiểm persisted record qua process mới, blob bị sửa,
  failed check không tạo observation và cả 6 trường snapshot + graph version.
  Foundation vẫn 52 Rust + 10 Node tests pass (mở rộng test CLI hiện có).
  Đây là content observation, chưa là execution/semantic verification receipt.
- [x] CLI ingest-artifact ARTIFACT.json ROOT đọc piped/redirected stdin,
  dùng application ingestion và trả metadata-only JSON với inserted flags,
  content_verified=true/protection+execution=false/receipt_persisted=false.
  50 Rust + 10 Node tests pass: missing-run/budget, hash failure rồi retry,
  replay, short/long/conflicting input, verify sau ghi và help không tạo DB.
  Chưa tự sinh descriptor, redact input, deadline hoặc durable receipt.
- [x] Application ingest_artifact + ArtifactWriter port nối ledger/blob theo
  metadata-first replay: budget/run/conflict trước consume input; lỗi bytes giữ
  metadata unverified để retry, success đọc lại blob thay vì tin writer. 49 Rust
  + 10 Node tests pass, gồm retry sau reopen và writer false-success. Không phải
  transaction SQLite/filesystem; CLI ingestion, durable receipt/reconciler vẫn mở.
- [x] DirectoryArtifacts ingestion API: exclusive staging, bounded copy,
  hash/length verification, file/directory sync, hard-link publish no-clobber;
  existing corrupt blob không bị overwrite. Test empty/binary/100k, replay,
  read/hash/length failure cleanup và 4 writers cùng blob: 47 Rust + 10 Node
  tests pass trên Linux. Chưa CLI/ledger ingestion transaction, crash recovery,
  orphan GC, deadlines hoặc portability; root phải private do host quản lý.
- [x] Artifact content verifier streaming + read-only DirectoryArtifacts:
  buffer 32 KiB, SHA-256/length, budget trước open, tối đa declared length+1
  bytes; trả receipt metadata không trả blob. Test binary/empty/truncate/append,
  hash mismatch, scope/directory rejection và symlink escape pass trên Linux.
  43 Rust + 10 Node tests; chưa ingestion, durable receipt, deadline,
  protection attestation hoặc snapshot authority; không accept semantic facts.
- [x] CLI `verify-artifact ID TASK.json ROOT --max-bytes N` trả metadata-only
  receipt, content_verified=true khi đúng bytes; protection/execution false,
  snapshot caller_supplied, receipt_persisted=false. 44 Rust + 10 Node tests:
  missing blob, hash/length/budget/snapshot errors không xuất success JSON;
  metadata ledger không được nâng trust sau lần verify.
- [x] Artifact domain/wire descriptor có full snapshot, run ID, SHA-256, byte
  length, kind, retention và declared protection enums. Reject unknown fields,
  caller reference_count/verified, invalid hashes/scope/length/schema. 36 Rust
  + 10 Node tests pass. Chưa blob store, reference tracking,
  protection verification, GC hoặc execution linkage; P1.T07 vẫn mở.
- [x] Artifact ledger V5 và application port: immutable replay/conflict,
  composite FK tới AnalysisRun đúng full snapshot, scoped lookup kiểm lại
  descriptor. Test V4 upgrade, restart, missing/mismatched run và SQL bypass
  scope pass; toàn foundation 38 Rust + 10 Node tests. Chưa blob ingestion,
  consumer reference tracking hoặc execution attestation.
- [x] CLI `record-artifact`/`artifact`: JSON versioned, missing-run rejection,
  replay/conflict và full-snapshot lookup; content/protection/execution_verified
  luôn false, không ingest bytes. 40 Rust + 10 Node tests pass; help không tạo DB.
- [x] AnalysisRun registration có domain/wire contract và immutable ledger V4:
  analyzer/version/config/input digests, full snapshot-scoped lookup, conflict
  detection và V3 upgrade giữ citation chưa đăng ký. CLI `record-analysis-run`
  và `analysis-run` có JSON versioned, replay/conflict/snapshot tests và luôn
  báo execution_verified=false. 33 Rust + 10 Node tests pass.
  Đây chỉ là registration; chưa execution receipt/lifecycle, manifest
  verification hoặc semantic acceptance. `verify-evidence` vẫn báo
  analysis_run_verified=false. Báo cáo `reports/w1-analysis-registration-2026-09-11.md`.
- [x] W0 có preflight read-only: `node scripts/preflight.mjs` xuất JSON,
  exit 1 khi thiếu công cụ; 3 tests bằng `node --test scripts/preflight.test.mjs`
  pass. Qua scripts/with-local-tools, cả 8 probes hiện pass: Node 22.22.1,
  SQLite FTS5, npm 10.9.2, Rust/Cargo 1.93.1, rustfmt, clippy và Git.
  Đây không phải build/adapter acceptance.
- [x] Pin Rust baseline/MSRV 1.93.1 và components trong rust-toolchain.toml;
  thay khai báo 1.85 chưa được kiểm chứng. Format toàn workspace và fmt check
  pass. Clippy chạy được nhưng còn warnings; strict lint gate chưa đạt.
- [x] P0.T01 — release manifest [`repo-lock-20260912.json`](/home/minh/projects/project-graph-agent/repo-lock-20260912.json)
  ghi snapshot live của 23 repo, gồm Ripwire, commit/branch, dirty status hash,
  root manifest/license hashes, declared version/license, product toolchain,
  CodeGraph CLI/kernel artifact pin và Ghostty `not-enabled`. Đây chưa phải
  license interpretation/compatibility approval; các review đó vẫn pending.
  Receipt: [`p0-repository-lock-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-repository-lock-source-review-2026-09-12.md).
- [x] P0.T02 — CodeGraph smoke đã pass: kernel build, dist build, `npm test`
  (**5.478 pass / 11 skip / 0 fail**), Orders index/status, real MCP stdio
  `initialize → tools/list → codegraph_status → codegraph_explore`, và watcher
  sau sửa file. Raw receipts giữ riêng transport với direct ToolHandler;
  không biến smoke thành W0 acceptance. Xem
  [`p0-codegraph-smoke-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-codegraph-smoke-source-review-2026-09-12.md).
- [x] CodeGraph tại 3ed73bc127323e63153bf6ec8354afa82ce36aaf đã npm ci,
  build engine/viewer/UI package và build kernel release; probe xác nhận kernel
  load. Full suite chạy xong: 4.598 pass, 16 fail, 11 skipped trên 267 files.
  Rerun 7 files lỗi không thuộc Dart với kernel tắt: 102 pass, 12 fail.
  Đây là baseline lỗi được ghi nhận, chưa đạt acceptance.
- [x] Receipt targeted `store-owner-regression-20260911.json` ghi 226 tests pass,
  0 fail trên bốn files: object-literal-methods, mcp-callers-truncation,
  ts-chained-receiver và resolution. Bản sửa local đang nằm trong CodeGraph;
  receipt ở `.harness/baselines/` của repo sản phẩm. Đây là kết quả đã lưu,
  không phải lần chạy mới trong đợt sửa tài liệu. Full-suite follow-up bên dưới
  bổ sung phạm vi kiểm tra, nhưng baseline gate vẫn chưa đạt.
- [x] Full-suite follow-up sau store-owner patch: 4.601 pass, 14 fail,
  11 skipped/267 files. So theo test identity: hai lỗi callers/store-action
  cũ hết, không có failing test mới. Build engine/viewer pass. Báo cáo:
  `reports/w0-store-owner-2026-09-11.md` trong repo sản phẩm. Suite này chưa
  có fingerprint trước launch và chạy chồng build, không là hermetic receipt.
- [x] Recorder probe schema v2 nhận diện source tracked + untracked và toàn
  bộ dist; kiểm hash trước/sau, không coi dirty checkout là clean upstream.
  Bốn Node tests pass (preflight + source fingerprint). Probe mới kernel-loaded,
  source ổn định. Đính chính phép đo: `allocatedChars` không phải budget;
  chênh 3 chars là giữa output trước/sau hoàn thiện, không phải vượt hard cap.
- [x] Tái dùng upstream factory-closure probe, lưu command/commit/input hashes:
  189/385 dòng file chính được trả; có refreshMetrics/applyFilter; envelope
  15.939 chars, pre-finalization 15.936, soft target 13.000 và hard ceiling
  19.500; vượt soft target, không vượt hard ceiling. Chưa phải agent benchmark.
  Bằng chứng và lệnh: repo sản phẩm reports/w0-codegraph-2026-09-11.md.
- [x] Outbox event được ghi cùng transaction bằng migration V2; rollback không
  để lại mục chờ. Cursor riêng từng consumer, ack theo thứ tự, replay sau reopen
  đã có test. Projector graph/vector và verifier decision vẫn chưa tích hợp.
- [x] Test checksum drift từ chối mở DB và giữ task đã lưu.
- [x] Test upgrade refinery V1 → V2 backfill event cũ vào outbox, giữ lease còn
  hiệu lực để submit, và giữ cursor sau reopen. Targeted test
  `upgrade_backfills_outbox_and_preserves_active_lease` pass.
- [x] Test 4 connection mở DB mới đồng thời pass 20 lần liên tiếp sau khi
  grouped migrations trong một transaction và retry DatabaseBusy khi đặt WAL.
  Runner đọc lại history sau race; chưa chứng minh crash/power-loss recovery.
- [x] Sửa final explore limiter: tính header/omission notice/closing fence trong
  hard ceiling và tính lại surviving files. Nhóm explore có 290 tests pass/24
  files trước bổ sung guard session; guard không checkpoint file bị cắt dở
  sau đó qua 49 targeted tests. Bổ sung boundary/Unicode/multi-block cases:
  39 tests limiter/session pass và build patch cuối pass; còn full suite mới.
  Đây chưa phải bounded ContextEnvelope của Rust harness.
- [x] Audit source ba repo mới sqlite-vector/context-mode/LLMRouter, ghi quyết
  định tích hợp ở mục 3.5 và task P4.T05/P8.T11/P9.T09. Chỉ đọc source/docs,
  chưa chạy benchmark hoặc xác nhận adapter tương thích.
- [x] Đo trực tiếp handwritten HNSW của Ruflo: synthetic 1k×128 và 10k×384,
  fixed seeds, exact cosine oracle, ef sweep tới 1600 và filter selectivity 1%.
  10k×384: ef800 đạt recall@10 98,8%, p50 15,66 ms; exact scalar JS 4,28 ms.
  Báo cáo `reports/w8-ruflo-hnsw-2026-09-11.md` có commands/raw receipts và
  giới hạn phép đo. Chưa benchmark chung Zvec/SQLite-vector hoặc real embeddings;
  không dùng kết quả này để tích W8 hay tuyên bố HNSW nói chung kém hơn.

### 0.2. Package completion

Evidence bổ sung W0: đã giữ static TS/JS member chains thành unresolved
call-site evidence ở cả WASM và Rust kernel, chặn name-guessing tại resolver.
250 tests WASM pass; nhóm Steps thêm 41 pass/2 lỗi selector Zustand còn lại.
Sau build engine/viewer và kernel: 91/91 tests native/parity pass (5 files),
không skip. Chi tiết `reports/w0-chain-evidence-2026-09-11.md` trong repo sản
phẩm. Chưa chạy lại full suite sau thay đổi này; không tích hoàn thành W0/W4.

Follow-up selector: đã bổ sung owner-verified Zustand selector bindings và
scope `var` hoisting; 234/234 targeted tests pass (3 files), gồm hai lỗi Steps
API cũ. Build pass. Full suite đã xong: **4.622 pass, 5 fail, 11 skipped /
4.638 tests, 268 files**; source fingerprint trước/sau khớp. So theo test
identity với baseline 14 lỗi: 9 lỗi cũ hết, không có lỗi mới. Còn Dart parity
(4) và React Native cross-platform pairing (1); chưa qua W0/W4 gate. Xem
`reports/w0-selector-owner-2026-09-11.md` và `reports/w0-remaining-triage-2026-09-11.md`
trong repo sản phẩm. Đây không phải hermetic/performance benchmark.

Follow-up React Native: đã mở đường bridge riêng cho runtime NativeModules
import được kiểm lexical scope (kể cả alias), không mở lại generic name guess.
249/249 tests pass (4 files), gồm pairing Android/iOS và negative shadow/type
import/missing module. Build/compiler pass. Chưa rerun full suite sau patch;
bốn lỗi Dart parity đã biết còn chờ xử lý. Báo cáo
`reports/w0-native-receiver-2026-09-11.md` trong repo sản phẩm.

Follow-up Dart: `extension type` đã có container/member ownership ở WASM và
kernel Rust; 65/65 tests parity/bridge pass, gồm bốn lỗi Dart cũ và hai test
LF/CRLF mới kiểm getter/body/containment. Compiler và kernel release build pass.
Full suite mới đã xanh: **4.629 pass, 0 fail, 11 skipped / 4.640 tests,
268 files**; source fingerprint trước/sau khớp. Các skip gồm 8 platform cases,
2 kernel-presence assertions opt-in và 1 performance test cần index chính repo.
Chưa qua toàn bộ W0/W4 gate. Xem `reports/w0-dart-extension-type-2026-09-11.md`.

Bổ sung: `CODEGRAPH_KERNEL_EXPECT=1` chạy scaffold/deep-nesting được 18/18 pass,
không skip. Factory-closure probe mới kernel-loaded, source ổn định, không
vượt hard ceiling; vẫn vượt soft target và product acceptance=false.

Đây là bảng theo dõi chính cho W0–W13. Mục 6 mô tả acceptance chi tiết,
mục 11 quản lý dependencies, mục 12 là gate toàn sản phẩm; không cộng các
checklist này với nhau để tính phần trăm tiến độ.

- [ ] **W0 — baseline:** pin toolchain/repo, fixtures và receipts tái chạy được.
- [ ] **W1 — đang làm:** protocol, task/attempt, evidence/assertion ledger, verifier,
  transactional outbox và recovery.
- [ ] **W2:** execution host, pipe/PTY, sandbox, cancel/timeout, receipts.
- [ ] **W3:** Codex app-server/native subagents, single-account identity, capability và lifecycle preflight.
- [ ] **W4:** CodeGraph adapter, incremental graph, bounded context và freshness.
- [ ] **W5:** scheduler, mailbox, worktrees, claims và merge queue.
- [ ] **W6:** TUI/headless, fleet/account view, reconnect và event parity.
- [ ] **W7:** Lightpanda browser, cited research và trust boundary.
- [ ] **W8:** memory, local embedding, Zvec và stale suppression/rebuild.
- [ ] **W9:** manifest/API/IaC/runtime adapters.
- [ ] **W10:** Joern/CPG và T3MP3ST.
- [ ] **W11:** Archify IR, diagrams và evidence navigation.
- [ ] **W12:** swarm cải thiện graph, critic và regression feedback.
- [ ] **W13:** platform/release/compatibility và benchmark sản phẩm đầy đủ.

### 0.3. Việc cần xử lý trước khi nối runtime

- [ ] **W4/W9 SQL linkage — baseline cụ thể:** fresh Orders index không có
  file record/node cho cả **11/11 file SQL** ở baseline trước admission;
  bản mới đã có đủ 11 file records/hash nhưng vẫn chưa có SQL nodes/edges.
  Rust system adapter đã có SQLite syntax observations độc lập; chưa nối
  chúng vào provider graph hoặc giải quyết relation/catalog/access semantics.
  Thêm ground truth có hash tại
  `benchmarks/orders-sql-v1.json` và inventory trong probe hiện có.
  Runtime fixture 1/1 pass không chứng minh graph đã hiểu SQL.
  Thứ tự triển khai: admission/hash + sync/removal của SQL files → parser
  theo dialect và statement ranges → binding wrapper/argument/path → nối
  code/SQL/DB-instance để vẽ. Không suy bảng từ `query('enqueue')`, không
  nhập authored expectations như extracted facts, không thực thi SQL để phân tích.
  Joern taint query/config pass và Zvec SELECT parser chưa thay thế được các
  bước này; **chưa đánh dấu SQL mapping hoàn thành**.
  [Source và baseline receipt](/home/minh/projects/project-graph-agent/reports/w4-sql-source-review-2026-09-12.md).

- [ ] **W1 snapshot authority — phần còn lại sau materializer:** hoàn thiện
  stable/atomic filesystem snapshot hoặc immutable worktree strategy mạnh hơn
  để tránh trộn bytes khi source mutate đồng thời; bind analyzer execution
  attestation vào snapshot và source evidence; sau đó mới mở semantic
  CodeGraph/Joern verification và accepted fact/outbox.
  `verify-evidence --snapshot-config` hiện đã materialize Git-visible source
  vào cây riêng, no-follow/hash-check khi copy, normalize internal symlink,
  freeze read-only và re-observe trước khi trả reader; nó vẫn chỉ chứng minh
  point-in-time Git observation + exact registered AnalysisRun, chưa làm cho
  source, quan hệ graph hay output trở thành trusted fact. Mở rộng wiring sang
  các command đọc/ghi source chỉ sau khi mỗi command có source-study receipt,
  strict config/exit/error matrix và regression riêng. Materializer không phải
  kernel atomic snapshot và read-only mode không phải hostile same-UID
  isolation; global deadline vẫn không ngắt mọi blocking OS call. Git SHA-256
  repository/non-UTF-8 path vẫn là compatibility decision chưa chốt.

- [x] **W4 prerequisite — lexical type binding:** thêm lookup class/interface
  theo scope và node identity; hỗ trợ alias/import type, default và barrel được
  grammar hiện tại hỗ trợ. Không chọn type ngoài scope khi gặp generic, merge
  hoặc namespace chưa hỗ trợ. Review đã tái hiện và sửa bảy regression;
  focused suite prerequisite 116 pass/4 files, `tsc --noEmit` pass. Binding đã
  được nối vào annotated field-call resolver bên dưới; không chứng minh runtime
  dispatch.
  [Source/review/test receipt](/home/minh/projects/project-graph-agent/reports/w4-type-binding-source-review-2026-09-12.md).
- [x] **W4 annotated field-call integration — phạm vi đã kiểm chứng:** nối field
  evidence → lexical type binding → exact member/inheritance cho các trường hợp
  được hỗ trợ; closure giữ lexical this, ordinary function không mượn receiver.
  Cạnh mang heuristic/declared-type và runtime assumptions, không giả làm runtime
  dispatch đã chứng minh. Sync xử lý barrel rebind, target shift, invalid/restore
  và thêm file đổi import precedence; lifecycle 2 pass, focused UI/resolver 109 pass.
  [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w4-typed-field-integration-source-review-2026-09-12.md).
- [ ] **W4 binding coverage còn lại:** general `typeof` value và unannotated
  constructor initializer, namespace merge, class-expression nodes, grammar
  `export type *`, generic/inheritance rộng hơn, config-driven imports và runtime
  mutation/escape đầy đủ. Single matching constructor initialization đã có test;
  không suy rộng thành toàn bộ initializer binding.
- [x] **Full-suite regression gate — callback/parameter integration:** nguồn
  `5a0098c34f5018b16340834fad4da7da255538ba203f52758912755525b674b7`
  trước/sau khớp; runner exit 0, **5335 pass / 0 fail / 9 skip / 0 TODO,
  5344 tests trong 300 files**. Đây là regression gate cho snapshot này,
  không tick coverage W4 hoặc diagram acceptance.
  [Full-suite receipt](/home/minh/projects/project-graph-agent/reports/w4-orders-parameter-consumption-2026-09-12.md).
  Lịch sử trước khi gate xanh: lượt sau callback patch
  có 5202 pass / 17 fail / 9 skip, runner exit 1, fingerprint trước/sau khớp.
  Lỗi retrieval allocation và router/screen đang được sửa; không dùng green cũ
  cho source mới. Callback/default ownership focused mới: 128 pass / 0 fail,
  sáu files, native kernel bắt buộc và runner exit 0. Chưa có full-suite green
  sau các bản sửa compatibility.
  Follow-up riêng: default/class-field/lifecycle 51 pass; parameter-handoff
  contract 42 pass; React Router AST scope 46 pass (đã thêm sibling/parameter/
  destructuring-shadow negatives). Các lượt focused không cộng thành full-suite.
  Compatibility cũ 166 pass/5 fail đã được sửa và kiểm tra focused: retrieval
  CG21/27/31 45 pass; Steps/flow 196 pass/0 fail trong bảy files. Full-suite mới
  đã kết thúc: 5292 pass / 3 fail / 9 skip, 5304 tests trong 296 files,
  runner exit 1; fingerprint trước/sau cùng
  c76ebba2275838943cbcf68a6aaa7ee26f26d707e8f866067561353505d985e1.
  Ba lỗi còn lại ở explore-oversize-member (2) và explore-reservation-invariant
  (1): file admission/context budgeting; chưa đóng gate.
  Follow-up độc lập: 295/295 explore tests trong 24 files pass, gồm body hàm
  đầy đủ với query symbol và câu hỏi tự nhiên. Đã sửa lỗi typecheck iterator
  trong bản follow-up; full-suite mới sau mapper/lifecycle vẫn phải chạy.
  P1 do review tìm ra (default RHS gán body calls cho hàm ngoài) đã sửa bằng
  execution scope riêng, giữ body dependency và không suy invocation. Lượt red
  32 pass/12 fail vẫn giữ; lượt focused sau sửa initializer/HOC 136 pass/0 fail.
  ArkTS và generic anonymous ownership ngoài phạm vi này vẫn còn mở.
  [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w4-callback-ownership-source-review-2026-09-12.md).
- [x] **Baseline lịch sử sau typed-field integration:** bản cuối 5160 pass,
  0 fail, 9 skip, 0 TODO / 5169 tests, 293 files; runner exit 0 và fingerprint
  trước/sau khớp. Watchdog test đợi initialize thật thay vì đoán 800 ms, giữ hạn
  thoát 5 giây; có slow-start, disabled control, failed-spawn và probe-error
  tests, xác nhận dọn nhóm tiến trình. Focused 44 pass; review độc lập đã xử lý.
  Chỉ sửa test ở follow-up, không đổi resolver/runtime. Lượt cũ 5154 pass/1 fail
  vẫn giữ trong receipt; không khẳng định hồi tố nguyên nhân của PID cũ.
  [Source/review/verification receipt](/home/minh/projects/project-graph-agent/reports/w4-watchdog-readiness-source-review-2026-09-12.md).
  Gate này không tích hộ W4 hoặc acceptance graph/diagram end-to-end.

- [ ] **W4 callback execution ownership:**
  **Bổ sung node:http đang triển khai:** route từ literal method/url guard nối
  đúng callback bằng heuristic/source evidence; không suy listener đang chạy.
  Index/sync nhận diện cả file HTTP mới và file cũ vừa thêm import. Parent
  186/186 focused pass trong 7 files, build pass; file thường giữ raw-native
  path qua admission control. Full-suite source mới đã kết thúc: 5345 pass,
  1 fail, 9 skip; lỗi API node vượt ngưỡng 100 ms (112,9065 ms). Không nới
  threshold và không dùng green snapshot parameter integration để đóng gate này.
  [Source và route receipts](/home/minh/projects/project-graph-agent/reports/w4-node-http-routes-source-review-2026-09-12.md).
  Emitted Orders: 63 nodes/143 edges, ba route POST /orders, /dispatch, /events
  có cạnh tới đúng callback; mỗi cạnh có predicate/source hash và heuristic
  caveat. Sáu parameter observations và watch refresh vẫn đạt. Diagram query
  tổng quan vẫn rỗng; chưa có Compose/SQL/system projection hoàn chỉnh.
  Callback và default function có
  callable node riêng; không gán body calls cho factory hoặc suy contains là
  invocation. Initializer-only traversal giữ calls/allocations/computed keys,
  native/TypeScript parity và lexical-this safeguards đã qua focused tests.
  Baseline trước node:http: Orders 60 nodes/140 edges, 5 probe exit 0 và watch pass; diagram
  vẫn “No relevant code found”, injected receiver còn incomplete (4 edges/1 issue).
  HTTP parameter dispatch, registration/invocation và diagram end-to-end còn mở.

  - [x] Bổ sung inventory lời gọi quan sát được, giữ cả missing/unknown/spread
    argument; không dùng tập tracked handoff hiện tại như tập đầy đủ. Giữ API
    handoff cũ và kiểm thử lời gọi thứ hai không có argument đã biết.
    Parent chạy lại 90/90 pass trong hai files; đây là inventory lexical,
    chưa phải runtime dispatch hoặc kiểm chứng toàn bộ call graph.
  - [ ] Tái sử dụng bounded receiver analysis với root tham số riêng, ánh xạ
    exact callback execution span, provenance theo call site và producer lifecycle
    riêng; không giả field injection hoặc suy contains là calls.
    Shared reviewer đã có: kiểm tra root/span/hazards, bảng tra cứu dùng lại
    trong một input, state/budget riêng mỗi review; parent 158/158 pass và
    tsc --noEmit pass. Mapper/producer và diagram end-to-end chưa hoàn thành.
  - [ ] Khi nhiều invocation cùng tạo một cạnh callback → method, tổng hợp
    provenance trước khi ghi: DB dedupe theo endpoints/kind/coordinates và giữ
    metadata đầu tiên, không tự giữ các nguồn bằng chứng của bản trùng.
  - [x] Read-only parameter-call mapper: exact allocation → factory parameter
    → executing callback → instance method, kiểm tra receiver và source snapshot;
    evidence gộp theo đúng tọa độ cạnh. Parent 64/64 pass (14 mapper tests).
    Đây là candidate mapping; chưa đóng producer lifecycle hoặc Orders acceptance.
    Probe Orders source-loaded mới: sáu observations xác nhận ba cặp
    create/dispatch/consume; DB có ba collision với cạnh instance-method cũ,
    nên không ghi đè hoặc báo thành ba cạnh mới. Emitted-build Orders đã trả
    đủ sáu observations qua API và năm caller/callee responses, có vị trí nguồn
    và runtime-assumption caveat; reader/MCP focused 5/5 pass, build/typecheck
    pass. Diagram vẫn trả `No relevant code found`; không tick end-to-end.
    Lifecycle đã siết assertion rebind, extraction-error độc lập và abort recovery;
    full required-native suite mới đã xanh 5335/0/9, fingerprint trước/sau khớp.
    [Consumption receipt](/home/minh/projects/project-graph-agent/reports/w4-orders-parameter-consumption-2026-09-12.md).

  Baseline prerequisite mới: 147/147 pass trong năm files, chưa có producer mới.
  [Source-study dispatch](/home/minh/projects/project-graph-agent/reports/w4-call-handoffs-source-review-2026-09-12.md).

- [x] **W4 prerequisite — AST field evidence:** giữ nguyên annotation và
  initializer, exact class/member spans, static/instance và parameter-property;
  không nhầm local/accessor/static block thành field. Focused checks: 43 pass
  trong bốn files, `tsc --noEmit` pass ở bước prerequisite. Collector hiện đã
  nối annotated field resolver và có closure/lifecycle tests; chưa đóng W4.
  [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w4-class-field-evidence-source-review-2026-09-12.md).

- [ ] **W4 còn extraction và end-to-end coverage:** lifecycle đã kích hoạt và
  full suite sau impact batching pass 4986 tests/11 skip/2 TODO; giữ nguyên
  bài API <100 ms. Hai full suite trước thất bại latency vẫn lưu trong receipt.
  Orders smoke hiện 51 nodes/131 edges, bốn quan hệ service/relay → OrderStore
  có trong getCallees; index incomplete với một issue, diagram chưa có kết quả.
  Same-line method-ID, parity native/TypeScript và explicit legacy rebuild đã
  kiểm chứng ở mục 0.1. Returned-callable extraction cũng đã qua full suite
  5055 pass/9 skip/0 TODO; zero test TODO không có nghĩa hết yêu cầu sản phẩm.
  Orders smoke sau typed-field integration vẫn 51 nodes/131 edges, watch pass,
  5 probe exit 0 nhưng diagram trả “No relevant code found”; không coi exit 0
  là acceptance. Tiếp theo đọc lại source rồi xử lý binding coverage còn thiếu,
  truyền giá trị factory-return tới nơi gọi, HTTP parameter dispatch và
  flow/diagram end-to-end. Không thay khoảng trống invocation bằng cạnh containment.
  [Performance receipt](/home/minh/projects/project-graph-agent/reports/w4-ui-node-performance-source-review-2026-09-12.md).
  Chưa đạt end-to-end acceptance hoặc đóng W4.
  [Lượt thực thi hiện tại](/home/minh/projects/project-graph-agent/reports/w4-injected-reconciliation-source-review-2026-09-12.md).

- [x] Đã xác định và sửa vòng phụ thuộc phát triển W1-verifier ↔ W2-executor:
  mục 11.0 tách W1-C contracts, W2 owned fixtures và W1-I joint verification.
  Source OpenDev được đọc lại; chưa chạy executor, không thay package acceptance.

- [x] Đối chiếu lại sequencing W1→W2→W3 và source transport OpenDev/Codex:
  [W2 source review + acceptance fixtures](/home/minh/projects/project-graph-agent/reports/w2-transport-source-review-2026-09-11.md).
  Không copy MCP Content-Length sang Codex JSONL, unbounded queues, detached
  stderr pump hoặc coi gọi kill là proof teardown. Đây là source review,
  chưa chạy fixture/process; không tích W2/P10.T05–T06.
- [x] W1-C development contracts đã qua audit giới hạn; xem mục 11.0 và
  [bằng chứng](/home/minh/projects/project-graph-agent/reports/w1-c-development-gate-audit-2026-09-11.md).
- [ ] Tiếp theo mở rộng W2 từ owned Linux runner: kiểm process-tree cleanup,
  transport/backpressure và nối launch claim + artifact/receipt. Runner hiện chỉ
  trả raw bytes/completion observations, chưa tạo receipt thật; W1-I dùng
  receipts đó kiểm verifier và atomic decision/outbox. Không yêu cầu bằng chứng
  execution trước khi có executor. Native dispatch/integration vẫn chờ gate đầy
  đủ W1/W2; xem mục 11.0, không coi các core sans-I/O là acceptance.
- [ ] RPC descriptor, one-shot ledger và trusted fixture preflight → spawn →
  ConnectionSetup đã nối (§0.1), dùng clock host. Đã có API/CLI inspection không
  thử claim lại; đọc thấy có/không có claim đều chưa chứng minh process state.
  Đã lưu spawn-stage observation V12 và nối host fixture, giữ Child khi ghi DB lỗi.
  Observation đã có trong inspection/CLI nhất quán (§0.1). Terminal RPC/output
  receipt và staged publication đã có; còn durable prepared-output journal và
  reconciliation khi crash giữa claim/spawn/attach, giữ pending uncertain, không
  tự retry/reset claim. Production vẫn cần resolve approval do host chọn và bind
  toàn bộ spec với quyền hiện tại; claim=true hoặc string approval_id không cấp
  quyền. Chưa đóng race final cancellation/expiry check → spawn, filesystem TOCTOU
  hoặc containment. Phạm vi trước gate vẫn là owned fixtures, không native dispatch.
  [Current restart gap review](/home/minh/projects/project-graph-agent/reports/w2-restart-gap-review-2026-09-13.md)
  phân biệt replay publication không spawn với orphan-process recovery; chưa
  triển khai journal hoặc nhận lại process từ PID.
  Source-gate cho thứ tự ghi CAS → manifest, bounded reopen và replay không
  spawn đã ghi tại [journal source study](/home/minh/projects/project-graph-agent/reports/w2-output-journal-source-gate-2026-09-13.md).
  Code/kill-test journal vẫn mở; không dùng rename-only snapshot làm bằng chứng
  durability hoặc chuyển corruption thành missing.
  Protocol `rpc_journal` đã có manifest v1 với exact-launch binding, byte cap
  trước decode và cumulative output cap; chưa có storage/reopen journal hoặc
  crash recovery. Validation ghi trong journal source-study receipt.
  `decode_reader` giới hạn đọc manifest ở cap+1 trước parse và giữ lỗi I/O;
  caller vẫn phải chọn root, kiểm regular file và áp deadline.
  Encode manifest dùng capped writer, không tăng độ dài buffer vượt cap trong
  lúc serialize; không phải giới hạn tổng allocation của converter/serializer.
  PreparedRpcReceipt::stage_journal đã ghi/verify stdout, stderr rồi manifest
  qua CAS port, không ghi ledger/spawn; caller phải giữ manifest descriptor.
  Durable discovery/reopen, replay ledger và kill-test vẫn mở.
  `reopen_rpc_journal` đọc manifest có cap, kiểm hash trên cùng buffer được
  parse, đối chiếu scope/run và verify output; trả metadata lịch sử. Durable
  handle discovery, replay ledger và SIGKILL acceptance vẫn mở.
  Replay ledger từ journal đã có `replay_rpc_journal`: kiểm bytes và exact
  spawn/terminal, ghi lại run/artifact/receipt idempotent; không có claim/spawn
  port. Durable discovery và SIGKILL acceptance vẫn mở.
  Test replay từ chối manifest hỏng, stderr thiếu và terminal hợp lệ nhưng
  khác receipt đã commit; kiểm event history và receipt gốc không đổi.
  `stage_registered_journal` lưu manifest handle vào artifact ledger sau CAS;
  lookup theo ID host chọn + snapshot sau reopen rồi replay đã có fixture.
  Chưa auto-discovery hoặc crash acceptance tại từng boundary.
  Owned publisher SIGKILL fixture đã bao phủ handle-committed và
  terminal-committed: reopen/replay không spawn thêm, giữ exact receipt/event
  replay. Bổ sung ordered outbox acknowledgement, cursor sau reopen và consumer
  độc lập; focused test/fmt đạt. Chưa chứng minh toàn bộ outbox row integrity.
  Chưa kill giữa từng CAS/ledger write, auto-discovery hoặc power-loss.

- [ ] W0: full suite CodeGraph local patch đã hết lỗi (4.629 pass/0 fail),
  không xoá baseline gốc 16 fail. Tiếp tục kiểm ContextEnvelope budget
  của harness (khác soft target upstream), platform skips và build provenance.
  Bổ sung multi-service
  fixture, MCP/watcher receipts và agent A/B benchmark.
- [ ] Đưa pinned Rust 1.93.1 vào CI/platform matrix và xử lý Clippy warnings.
  Constructor docs của Artifact/CheckRunBinding/RequiredChecks/CheckCommand đã
  mô tả các nhánh lỗi hiện có; không đổi runtime authority. Strict Clippy vẫn mở.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w13-constructor-docs-2026-09-13.md).
  ContextOperation parser và ScopeSelector constructor cũng đã có Errors docs;
  domain test/fmt pass ngày 2026-09-13.
  RPC process/connection/spawn/terminal constructors and input markers likewise
  document their validation errors; domain tests/fmt pass ngày 2026-09-13.
  RPC process accessors now carry `must_use`; domain tests/fmt pass ngày
  2026-09-13.
  RpcSpawnDisposition/Observation accessors now carry `must_use`; domain
  tests/fmt pass ngày 2026-09-13.
  UncertainRpc and RpcInputProgress accessors now carry `must_use`; domain
  tests/fmt pass ngày 2026-09-13.
  ExecutionPlan constructor/accessors now document their existing
  binding contract; domain tests/fmt pass ngày 2026-09-13.
  ExecutionReceipt constructor/accessors now document existing linkage and
  budget errors and carry `must_use`; domain tests/fmt pass ngày 2026-09-13.
  GraphVersion constructor/accessors and worktree comparison now carry the
  corresponding documentation/`must_use`; domain tests/fmt pass ngày 2026-09-13.
  AssertionKind/EvidenceKind parsers now document unknown-value errors and
  string conversions carry `must_use`; domain tests/fmt pass ngày 2026-09-13.
  ClaimState parser mô tả exact spelling và giới hạn metadata; source-gate:
  [receipt](/home/minh/projects/project-graph-agent/reports/w13-claim-parser-docs-2026-09-13.md).
  TargetHeadVerification constructor/accessors đã document validation và giữ
  authentication ở application boundary; domain tests/fmt pass.
  [Receipt](/home/minh/projects/project-graph-agent/reports/w13-target-verification-docs-2026-09-13.md).
  Các checks local đã chạy; chưa có strict lint/portable release gate.
  Các batch docs/accessor sau receipt bốn constructor thiếu source-gate trước
  patch nhất quán; không đánh dấu tuân thủ hồi tố.
  [Reconciliation](/home/minh/projects/project-graph-agent/reports/w13-lint-reconciliation-2026-09-13.md).
  Audit strict `--workspace --all-targets -- -D warnings` mới exit 101 tại
  graph-domain (304 lib / 404 lib-test diagnostics, có overlap); chưa kiểm
  xanh các crate phía sau. Đã giữ log và phân loại lỗi để xử lý tiếp:
  [Strict lint audit](/home/minh/projects/project-graph-agent/reports/w13-strict-clippy-audit-2026-09-12.md).
- [ ] Chuẩn hóa task/attempt lifecycle; admission, running, verification,
  rejection, cancellation, timeout và retry có transitions được test.
- [ ] Triển khai authority thay thế `integrate(task, verification_string)` đã bị chặn bằng receipt
  đã kiểm tra artifact, source snapshot, scope, checks và target head.
- [ ] Tạo acceptance transition có verifier/GraphWriter authority; hàm accept
  cũ dùng chuỗi đã bị bỏ. Phải kiểm evidence tồn tại, hợp lệ và đúng snapshot.
- [ ] Tạo Evidence/Artifact/AnalysisRun/GraphVersion contracts và ledger; tách
  static, runtime, documented, human và security claims.
- [ ] Ghi decision + outbox trong cùng transaction; consumer idempotent,
  restart/replay không làm mất hoặc nhân đôi materialized facts.
- [x] Migration: fixture DB có V7 tương lai bị binary V6 từ chối qua refinery,
  history/schema/payload giữ nguyên; grouped V1→V6 lỗi tại V3 rollback cả V2,
  giữ task/event và bảng xung đột, retry sau xử lý fixture thành công.
  Missing/divergent migration policy được bật tường minh; 55 Rust + 10 Node pass.
- [ ] Migration crash/power-loss recovery và platform compatibility. Các tests
  rollback SQL, checksum drift, version mới hơn và concurrent initialization
  không thay thế crash injection; mở DB vẫn có thể cấu hình journal mode.
- [x] Preflight từ chối user_version khác 0 bằng lỗi typed trước WAL/migration;
  không clear marker hoặc fabricate history. Fixture 1/99/-1, có/không refinery
  history giữ nguyên bytes, journal DELETE và payload; 56 Rust + 10 Node pass.
  Đây không phải xác nhận mọi DB có marker đều là prototype của harness.
- [ ] Chuyển đổi DB prototype dùng `PRAGMA user_version` trước refinery: hiện chưa có
  đường nâng cấp; không xoá DB hoặc giả ghi migration history để bỏ qua.
- [ ] Thêm schema roundtrip/compatibility fixtures và independent validator.

Bước thực hiện: hoàn thiện W0 song song phần W1 không cần graph; W2/W3/W4 chỉ
được tích gate khi các dependency tương ứng có bằng chứng. Tất cả W0–W13 vẫn
nằm trong phạm vi sản phẩm hoàn chỉnh.

### 0.4. Quy tắc cập nhật checklist và giao việc cho swarm

- Mỗi task triển khai có ID dạng W4.01, owner, phạm vi file/worktree, dependency,
  đầu ra và acceptance test. Chỉ tách task khi có thể kiểm chứng độc lập.
- Ghi trạng thái todo / doing / blocked / review / done bên cạnh task.
  Checkbox chỉ tích khi done; task bị chặn phải ghi nguyên nhân.
- Receipt tối thiểu: revision hoặc source fingerprint, command, kết quả,
  đường dẫn log/artifact và reviewer. Không ghi secret vào receipt.
- Khi source thay đổi làm evidence cũ mất hiệu lực, bỏ tích gate liên quan
  và chạy lại checks; không giữ trạng thái xanh chỉ vì từng pass.
- Coordinator chia scope ghi độc lập; worker dùng worktree riêng và nhận
  graph IDs, snapshot, evidence ranges cùng budget, không nhận dump cả repo.
- Worker nộp patch hoặc candidate graph delta + evidence; critic kiểm độc lập;
  integrator/GraphWriter áp dụng kết quả đã chấp nhận. Worker không tự xác nhận
  claim của mình thành authoritative fact.
- Fan-out bị giới hạn bởi tài nguyên, quota và budget đo được. Coordinator và
  worker cùng account; worker mới dùng Luna, root giữ model hiện tại. Role/model
  không phải quyền mở rộng capability.
- Kết thúc mỗi đợt: cập nhật mục 0 và acceptance tương ứng, ghi blocker cùng
  bước tiếp theo; đồng bộ hai bản kế hoạch. Không tích cả package vì có skeleton.

Mẫu ghi dưới một task khi bắt đầu làm (không tạo thêm sổ trạng thái song song):

```text
status: doing | owner: <worker hoặc chưa phân công>
scope: <repo/worktree + files> | depends_on: <task IDs>
source-study: <repo + revision/fingerprint + file:symbol/line + flow/tests>
source-gate: pending | ready | gap-documented
adopt/avoid: <cơ chế học, điều chỉnh và gap trước khi code>
pre-code-checklist:
  - [ ] Đã đặt lại source-gate về pending khi bắt đầu/chuyển task hoặc resume.
  - [ ] Đã chọn nguồn theo cơ chế cần làm, không chỉ theo tên repo/package.
  - [ ] Đã đọc lại source + tests outsource đúng phần việc đang làm.
  - [ ] Đã ghi luồng, bài học áp dụng/không áp dụng và phần còn thiếu.
  - [ ] Đã chốt cách triển khai + test; source-gate không còn pending.
receipt: <revision/fingerprint + command + exit + log/artifact>
review: <reviewer + kết luận> | blocker: <nếu có> | next: <bước tiếp>
```

ID `P<phase>.T<nn>` là bước thực thi trong mục 6; `W<nn>` là package/gate tích
hợp, không phải cùng hệ đánh số. Bảng mục 11 ánh xạ hai hệ để tránh làm Phase 6
trước execution chỉ vì số phase nhỏ hơn. Giữ ID ổn định khi sắp xếp lại task;
bước mới dùng ID mới. Các checkbox acceptance kiểm kết quả, không thay task.

### 0.4.1. Source-first: đọc outsource đúng phần việc rồi mới code

**Quy tắc bắt buộc cho mọi task W0–W13, main agent và subagent: đến phần
việc nào, quay lại đọc outsource để học phần đó rồi mới code.** Áp dụng khi
thêm tính năng, sửa bug, refactor và tiếp tục việc dang dở. Không tự nghĩ
implementation trước rồi mới tìm nguồn để hợp thức hóa.

Trước patch đầu tiên của từng phần việc:

- Xác định repo/file liên quan và câu hỏi về cơ chế cần học.
- Đọc source hiện tại cùng tests: luồng chính, nhánh lỗi và giới hạn.
- Ưu tiên học và điều chỉnh cơ chế đã có: đối chiếu các nguồn liên quan trước
  khi chọn thiết kế; không mặc định viết lại từ đầu khi outsource đã giải quyết
  cùng bài toán. Ghi rõ phần tái sử dụng ý tưởng, phần cần thay đổi và lý do.
- Ghi bài học, cách áp dụng/điều chỉnh và tests dự kiến vào receipt của task.
- Chỉ tự thiết kế phần nguồn chưa đáp ứng, sau khi ghi phạm vi đã tìm và lý do.

Phải truy được `yêu cầu → source/test vừa đọc → quyết định áp dụng → patch
và test sản phẩm`; tên repo, README, trí nhớ hoặc audit cũ không thay bằng
chứng đọc luồng thực thi. Chỉ đọc phạm vi liên quan, không dump toàn bộ repo.
Học cơ chế không đồng nghĩa sao chép code hoặc áp dụng máy móc.

Nếu phát sinh phần việc mới, quay lại gate đọc nguồn cho phần đó. Nếu code cũ
chưa có source-study, đối chiếu lại trước patch tiếp theo, sửa phần không phù
hợp và kiểm thử; không ghi lùi bằng chứng hay tích checklist hồi tố.
Reviewer kiểm liên kết nguồn–quyết định–patch trước khi chấp nhận tích hợp.

**Điểm dừng ngay trong task:** nếu đang code mà phát sinh cơ chế chưa được
source-study bao phủ, dừng patch phần đó, quay lại đọc implementation và tests
tương ứng trong outsource, bổ sung quyết định vào receipt rồi mới tiếp tục.
Không chờ sang task mới hoặc đến bước review mới đối chiếu nguồn.

- [ ] Trước từng phần implementation, receipt chỉ rõ cơ chế nguồn → phần code
  dự kiến áp dụng → điều chỉnh cần thiết → test kiểm chứng; phần phát sinh
  chưa có đối chiếu phải quay lại source-gate, không tự triển khai trước.

**Gate theo phần việc, không chỉ theo package:** một lần đọc cho W2, W4 hay
W8 không cấp quyền code mọi hạng mục của package đó. Trước mỗi phần việc mới
(ví dụ terminal cancellation, graph invalidation, HNSW update), quay lại nguồn
tương ứng và ghi quyết định riêng. Khi resume hoặc mất context, mở lại receipt
và source liên quan trước khi tiếp tục; không coi bản tóm tắt là đã đọc nguồn.
Ưu tiên cơ chế đã được đối chiếu với yêu cầu và test; chỉ tự thiết kế phần còn
thiếu hoặc không phù hợp, kèm lý do cụ thể. Không ép dùng pattern upstream
nếu nó vi phạm trust boundary, license hoặc yêu cầu của sản phẩm.

Checklist ghi ngay trong receipt của từng task:

**Ba câu phải trả lời trước khi code:**

1. Phần việc này học từ repo nào, file/symbol và test nào vừa đọc lại?
2. Nguồn xử lý luồng chính và nhánh lỗi ra sao; cơ chế nào sẽ được áp dụng
   hoặc điều chỉnh cho sản phẩm?
3. Phần nào vẫn phải tự thiết kế, vì sao nguồn hiện có chưa đáp ứng và test
   nào sẽ kiểm chứng phần bổ sung đó?

Ghi câu trả lời trong `source-study` và `adopt/avoid` của receipt hiện có,
không tạo thêm tài liệu trùng lặp. Chưa trả lời được thì tiếp tục tìm/đọc nguồn,
không chuyển thẳng sang viết code theo suy đoán. Quy tắc này là điều kiện bắt
đầu từng phần việc, không phải một checkbox đọc outsource một lần cho cả dự án.

**Khi giao việc cho swarm:** task brief phải chỉ rõ repo/file hoặc symbol cần
quay lại đọc, câu hỏi về luồng cần trả lời và yêu cầu nộp source-study trước
patch đầu tiên. Không giao yêu cầu chung chung “tham khảo outsource” rồi để
worker tự code từ trí nhớ. Nếu chưa xác định được nguồn, bước đầu của task là
tìm nguồn, chưa phải implementation. Coordinator kiểm gate này khi giao việc
và khi nhận kết quả; việc một agent khác đã đọc không miễn bước đọc của worker.

Luồng bắt buộc: **chọn task → đọc outsource đúng phần → ghi bài học/gap →
chốt cách áp dụng và test → code → kiểm chứng**. Task mới bắt đầu với
`source-gate: pending`; chỉ chuyển sang `ready` khi đã có bằng chứng đọc nguồn
và quyết định áp dụng. Nếu không có implementation phù hợp sau khi tìm nguồn,
dùng `gap-documented` kèm phạm vi tìm, thiết kế bổ sung và test dự kiến. Không
viết implementation khi gate còn `pending`; gate này không thay acceptance
test và không đồng nghĩa task đã hoàn thành.

**Cách thực hiện hằng ngày:** khi chuyển sang đầu việc tiếp theo, tạo lại các
ô `pre-code-checklist` trong receipt của đầu việc đó ở trạng thái chưa tích.
Thông báo ngắn cho người dùng phần đang học và repo liên quan trước khi đọc;
sau khi đối chiếu, ghi rõ sẽ áp dụng cơ chế nào rồi mới bắt đầu patch code.
Không mang trạng thái đã tích từ task trước sang task mới, kể cả cùng repo
hoặc cùng package. Các ô dưới đây là mẫu quy trình, không phải bằng chứng rằng
toàn dự án đã đọc nguồn xong.

Khi resume cùng task, giữ receipt cũ làm lịch sử nhưng đặt lại
`source-gate: pending` và các ô pre-code; ghi lần đọc lại nguồn hiện tại trước
patch tiếp theo. Không cần đọc lại toàn repo hay chạy lại toàn bộ tests chỉ để
mở gate: phải đọc đúng luồng và assertions liên quan, rồi chọn checks phù hợp
với thay đổi. Đọc source là điều kiện bắt đầu, không thay cho kiểm thử sau code.

- [ ] Chọn repo tham khảo phù hợp; đọc hướng dẫn áp dụng và skill liên quan.
- [ ] Truy luồng thực thi entry point → state/xử lý → storage/side effect → kết
  quả; đọc nhánh lỗi, cancellation/recovery nếu liên quan.
- [ ] Đọc test và assertion tương ứng; phân biệt mock, stub, fallback với đường
  chạy thật. README/comment/tên hàm không đủ làm bằng chứng.
- [ ] Ghi revision/dirty fingerprint, file:symbol hoặc dòng; cơ chế sẽ học,
  phần không lấy và lý do, thay đổi cần thiết cho native single-account và graph.
- [ ] Chốt implementation cùng regression/acceptance từ nguồn vừa đọc, rồi
  mới sửa code. Reuse code phải qua license gate, không copy nguyên repo.
- [ ] Coordinator/reviewer kiểm source-study đúng phần việc và có trước patch;
  thiếu bằng chứng thì trả task về bước đọc nguồn, chưa chấp nhận tích hợp.
- [ ] Sau code: kiểm thử, review khác biệt với nguồn và cập nhật bằng chứng.
  Không tích bước đọc-source hồi tố cho phần đã code mà chưa đối chiếu.

| Phần việc | Repo quay lại đọc trước khi code |
|---|---|
| Native agents, admission, lifecycle, fleet | codex, t3code, orca, ruflo; đối chiếu opendev |
| Terminal tools, parallel commands, stop/backpressure | opendev, t3code, orca, rtk |
| Worktree, lease, merge/recovery | grit, ruflo, orca |
| Graph/index/sync/context | codegraph; joern/codepropertygraph cho phân tích sâu |
| Architecture/evidence/red-team | archify, T3MP3ST, codegraph |
| SQLite/migrations/Rust boundaries | icm, temp-rs-ddd; chọn pattern phù hợp local |
| Retrieval/embedding/vector/memory | context-mode, icm, zvec, sqlite-vector, HNSW trong ruflo |
| Browser/sandbox/terminal rendering | browser, OpenSandbox, ghostty |

Nếu nguồn không có cơ chế phù hợp: tìm các repo còn lại trong outsource, ghi
phạm vi đã tìm và khoảng trống, nêu thiết kế bổ sung cùng lý do trước khi code.
Không bịa attribution; nếu cần quyền hoặc mở rộng scope thì hỏi người dùng.
Không sửa repo tham khảo hay tự chạy dịch vụ chỉ để học source.

Mỗi subagent phải nộp source-study receipt; coordinator kiểm nguồn trước khi
tích hợp. Main vẫn tự đọc skill bắt buộc và nguồn liên quan đến quyết định tích
hợp. Khi đổi phần việc hoặc upstream thay đổi, đọc lại phần tương ứng. Với code
đã viết chưa đối chiếu, review lại khi tiếp tục phần đó, không tự đánh dấu đã
tuân thủ. Đây là quy trình thực thi bắt buộc, không thu hẹp phạm vi sản phẩm.

### 0.5. Quyết định giữ phạm vi và giảm phần trùng

- Giữ nguyên đích sản phẩm đầy đủ W0–W13; không đổi thành MVP, không coi
  parser, source-slice CLI hay một demo swarm là sản phẩm hoàn thành.
- Một scheduler, một event stream và một đường commit có verifier; các repo
  tham khảo đóng góp adapter/pattern, không cùng sở hữu trạng thái task.
- Có **local embedding** trong W8. Phân biệt embedding model tạo vector và
  embedded database chạy local: SQLite không tự tạo embedding, Zvec không
  thay graph hay evidence ledger. Zvec adapter thuộc phạm vi phải kiểm chứng;
  graph/FTS vẫn hoạt động khi semantic backend bị tắt hoặc hỏng.
- Chưa thay DB, chọn model hoặc bổ sung runtime chỉ dựa vào README. Các lựa
  chọn cần benchmark giữ nguyên trạng thái chưa chốt cho tới gate tương ứng.
- Đợt cập nhật này chỉ sửa kế hoạch và đồng bộ bản mirror; không triển khai
  runtime, đăng nhập tài khoản hoặc tự khởi chạy swarm/red-team.

## 1. Quyết định kiến trúc cấp cao

### 1.0. Kết luận audit bổ sung: graph, swarm và worktree

Ba lớp phải được tách ngay từ đầu:

```text
Project Graph       = sự thật về code/architecture/evidence
Harness Scheduler   = task DAG, scope, lease, dedupe, retry, merge policy
Agent Thread        = thực thi một nhiệm vụ trong context/worktree được cấp
```

Một Codex thread/subagent không tự động là một graph task và cũng không tự động
có một Git worktree. T3Code hiện chứng minh mô hình `thread → worktreePath →
environment cwd`: người dùng chọn **New worktree**, server gọi `git worktree
add -b` và lưu path trên thread; background submission với chế độ worktree tạo
worktree riêng. T3Code cũng cho phép nhiều thread dùng chung một worktree, nên
worktree là isolation cho filesystem/branch, không phải cơ chế chống duplicate
task. Sản phẩm phải thêm `SubagentTask.scope_selector`, ownership lock,
`graph_version`, lease/fencing và overlap detector.

Mỗi implementer task mặc định được cấp một linked worktree riêng từ cùng
`base_revision`; mỗi read-only analyzer có thể dùng snapshot dùng chung. Không
được dùng chung `.codegraph`, `.t3`, port, cache mutable hoặc working directory
giữa các worktree. Worktree manager phải ghi `task_id`, path, branch, base SHA,
dirty fingerprint, setup receipt và cleanup state; merge chỉ được phép sau khi
verify base/head, path ownership, tests và graph delta. Dirty user worktree
không được tự động stash/reset.

Đây là cơ chế của sản phẩm mục tiêu, không phải claim rằng T3Code đã có
subagent scheduler hoàn chỉnh. Các nguồn đối chiếu hiện tại là
`t3code/apps/server/src/vcs/GitVcsDriverCore.ts` (create/remove worktree),
`t3code/apps/server/src/persistence/Services/ProjectionThreads.ts`
(`worktreePath` persistence), `t3code/packages/shared/src/projectScripts.ts`
(cwd/env binding), và `t3code/docs/user/thread-sidebar.md` (background
worktree behavior).

### 1.1. Xây trên nền tảng nào

`codegraph` là nền tảng fast index; Rust harness là sản phẩm chính. Repository này đã có phần lớn các
khối cần thiết:

- native Rust kernel cho parsing/extraction;
- Tree-sitter grammars và hỗ trợ nhiều ngôn ngữ;
- symbol graph, call/import/type relationships;
- framework-aware routes và một số cross-language bridges;
- SQLite + FTS5 làm local graph store;
- incremental indexing, file watcher, daemon và worktree handling;
- impact/blast-radius và context builder;
- MCP server cho Codex CLI cùng installer, hook và instructions;
- UI để xem node, source, route và flow.

Không viết lại các phần này bằng Zig ở giai đoạn đầu. Rust đã xuất hiện trong
kernel của `codegraph`, và ecosystem hiện tại của workspace cũng có các thành
phần Rust phù hợp cho MCP, file watcher, async và typed JSON.

### 1.2. Vai trò của Joern và Code Property Graph

`joern` và `codepropertygraph` là deep-analysis track bắt buộc của hệ thống hoàn
chỉnh, nhưng không phải mọi query đều phải chạy CPG. Fast graph và deep graph
phục vụ hai latency tier khác nhau.

- `codepropertygraph` là schema/serialization/tooling để trao đổi CPG.
- `joern` là backend phân tích sâu, đặc biệt hữu ích cho control-flow,
  data-flow, taint-flow và các truy vấn interprocedural.
- `codegraph` là lớp truy xuất nhanh, luôn cập nhật, tối ưu cho context của AI.

`codegraph` là fast graph authority. Joern/CPG là deep-analysis authority cho
CFG/DFG/PDG/taint khi query yêu cầu hoặc fast graph báo coverage thấp. Không đẩy
toàn bộ CPG chi tiết vào prompt; deep analyzer phải trả về graph slice có mục
đích, evidence và unknowns.

### 1.3. Phân biệt ba loại dữ liệu

Hệ thống phải tách rõ:

1. **Graph facts** — quan hệ được parser/resolver chứng minh, là nguồn sự thật
   cấu trúc.
2. **Semantic memory** — summary, docs, ADR, domain terms và lịch sử task,
   dùng để tìm kiếm và định hướng.
3. **Runtime evidence** — trace/log/telemetry, dùng để xác nhận đường chạy
   thực tế.

Không dùng embedding để thay thế edge của code graph. Không dùng LLM summary
để ghi đè static fact. Mỗi edge phải cho biết provenance và coverage.

### 1.4. Một tài khoản, native Codex subagents

**Quyết định thay thế:** root và subagents dùng cùng một tài khoản Codex do host
chọn; theo yêu cầu ngày 2026-09-12, worker mới mặc định dùng Luna (`gpt-5.6-luna`),
root giữ model hiện tại để điều phối và kiểm chứng. Bỏ Pro→Plus dispatch,
auth rotation và shadow homes khỏi kiến trúc bắt buộc. Không sửa credential
hoặc cấu hình global. Role/model khác không đồng nghĩa tài khoản khác.

Policy này chỉ áp dụng cho project và các lần spawn tiếp theo; worker đang chạy
hoàn tất task hiện tại. Kiểm tra runtime hỗ trợ model và ghi effective model vào
receipt. Nếu Luna không khả dụng, báo rõ, để root xử lý hoặc queue task; không
âm thầm fallback sang model khác. Worker gặp bài khó gửi evidence/gap về root,
không tự nâng model/quyền. Đo tổng token, retries và chất lượng toàn swarm trước
khi kết luận tiết kiệm; không suy quota subscription từ tên model.

**Chính sách vận hành Luna:** Luna là mặc định cho scout/explorer, source-study,
search, phân loại, test triage, extractor chạy trên partition độc lập và critic
đầu tiên có output schema hẹp. Planner được phép fan-out các việc này ngay khi
scope không chồng lấn, nhưng baseline là tối đa **4** worker đồng thời và
`max_depth = 2`; mỗi child phải có budget riêng, scope claim và điều kiện
fan-in. Không spawn Luna cho quyết định kiến trúc, thay đổi cross-cutting,
merge/GraphWriter, security verdict hay khi evidence của task trước còn thiếu:
root thực hiện hoặc duyệt các phần đó. Khi acceptance thấp, duplicate scope,
hoặc cost thực tế vượt ngưỡng, circuit breaker đóng fan-out và gom việc về
root. Đây là "swarm nhiều nhưng bounded", không phải tạo agent vô hạn.

Sản phẩm vẫn là Rust CLI/TUI như OpenDev, kết nối Codex qua app-server/process
boundary. Codex sở hữu native agent lifecycle (spawn, steer, wait, resume,
close và cây parent/child). Harness sở hữu durable domain jobs, graph context,
admission/budgets, write ownership và merge gates; ánh xạ attempt↔thread ID,
không xây thêm agent scheduler cạnh tranh.

Autonomous swarm phải được cho phép bằng project-scoped policy: planner tự
phân rã các task độc lập trong mục tiêu đã cấp; deterministic admission chặn
vượt budget/depth/write scope và duplicate. Child không tự nâng quyền. Root
tiếp tục critical path thay vì chỉ giao việc rồi chờ. Context riêng + graph
slice là mặc định; full transcript fork chỉ khi cần và được cấp budget.

Source local có MAv1/MAv2: adapter probe tool/schema và event capabilities của
revision đã pin, không suy ra public RPC từ private core handler. Nếu thiếu
admission/event hook, thêm fork seam nhỏ; không fallback ngầm sang external
threads mất native parent-child semantics.

Nguồn: [Codex subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents),
`codex/codex-rs/core/src/agent/control.rs` và
`core/src/tools/handlers/multi_agents/spawn.rs`. Permission/model inheritance
phải có contract tests trên runtime đích; không suy entitlement từ tên gói.
MCP/CLI vẫn tương thích Codex upstream; giữ codegraph_explore hiện có và
ContextEnvelope tool version riêng project_graph_context_v1.

### 1.5. Quyết định hợp nhất sau browser/Orca/multi-auth

| Thành phần | Trách nhiệm duy nhất |
|---|---|
| Harness Rust | session, task DAG, admission, budgets, event log, merge policy |
| Orca patterns | fleet dispatch, mailbox, execution-host identity, reconnect |
| Codex app-server/native subagents | một root account; reasoning/tools và agent lifecycle |
| codex-multi-auth reference only | học quota telemetry/refresh locking; không triển khai cross-account rotation |
| GRIT + T3Code patterns | symbol claims, worktree allocation, serialized integration |
| OpenDev patterns | terminal concurrency, workflow/context và TUI |
| GraphWriter + ContextGateway | commit assertions và truy xuất evidence theo version |
| Lightpanda browser | search/navigation/DOM extraction qua MCP/CDP |
| ICM adapter | durable decisions/handoffs; Zvec là retrieval index thay thế được |
| Ruflo adapter | các policy/router hữu ích; không thêm scheduler hoặc memory authority |

Loại phần dư thừa khỏi đường chạy bắt buộc: nhiều scheduler, nhiều kho memory
cùng authoritative, hai Codex runtime, ba bộ terminal UI, polling fleet bằng
LLM và broadcast nguyên transcript. T3Code/Orca desktop, mobile, full Ghostty
windowing stack là nguồn tham khảo/client mở rộng; sản phẩm terminal vẫn có
đủ fleet, web, graph, deep analysis, memory, remote execution và review.

### 1.6. Những quyết định được bổ sung sau audit

1. **Một harness sở hữu protocol, không phải hai framework sở hữu task.**
   `Project Graph Harness` định nghĩa scheduler, job contract, graph writer và
   policy của chính nó. Ruflo/OpenDev là implementation hoặc adapter có thể
   thay thế; chúng không được trở thành nguồn truth cho lease, memory hay graph
   facts. Ruflo hiện hướng mạnh tới Claude Code, còn OpenDev là một coding
   agent hoàn chỉnh, nên cả hai phải đi qua capability adapter đã test thay vì
   bị import sâu vào Codex ngay từ đầu.
2. **Đích là một sản phẩm hoàn chỉnh.** Các track fast graph,
   deep analysis, swarm, runtime, memory, sandbox, terminal UI, Codex integration,
   diagrams và security đều nằm trong Definition of Done. Phase chỉ biểu diễn
   dependency và thứ tự delivery; feature không bị loại khỏi target vì chưa cần
   ở release đầu.
3. **Fact không đồng nghĩa với một row mutable trong `edges`.** Mọi producer
   gửi assertion có revision/evidence; Graph Gateway materialize edge hiệu lực
   sau verifier. Candidate, static inference, runtime observation, document và
   human assertion không được đè lẫn nhau.
4. **Graph lịch sử dùng snapshot/delta, không cố bitemporal toàn bộ từ ngày
   đầu.** Index hiện hành là materialized view cho một `ProjectRef`; query một
   commit cũ dùng cache snapshot read-only và `GraphDelta`, không đổi database
   của worktree đang làm việc.
5. **Codex CLI được nâng cấp theo mô hình OpenDev/Orca.** Một executable/TUI riêng
   (`project-graph-agent`) sở hữu session, fleet, terminal multiplexer, router,
   worktree, approvals và evidence stream; Codex `app-server` cung
   cấp thread runtime, subagent spawning, roles, hooks, memory, context
   fragments, MCP và event protocol. Không ép Codex CLI nguyên bản phải hiểu
   toàn bộ graph bằng prompt; tích hợp sâu ở app-server/protocol/extension khi
   cần, giữ upstream boundary rõ để rebase được.
6. **Mọi text đến từ repo là dữ liệu không tin cậy.** Source, README, logs,
   test output, diagram label và memory retrieval không thể đưa chỉ dẫn cho
   control plane hay tự tăng quyền tool.


### 1.7. Chuẩn code Rust: SOLID, Clean Architecture và DDD

Học cách chia module, workspace config và bootstrap từ `temp-rs-ddd`; áp dụng
DDD cho task/attempt, lease, assertion/evidence, account binding và worktree.
Template đó chưa có application use case hoàn chỉnh để sao chép nguyên trạng.

Dependency hướng vào core: domain không phụ thuộc SQLite, serde wire, CLI,
Codex hay native vector engine; application sở hữu use cases/ports, adapter
thực thi I/O; CLI/TUI composition root lắp adapter. Hiện `store` còn dùng
protocol DTO để serialize, cần tách persistence mapping khi ổn định contracts.

- [ ] Chia module theo trách nhiệm và bounded context; tránh gom domain/store
  vào một `lib.rs` lớn. Không thêm abstraction khi chưa có trách nhiệm rõ.
- [ ] Value objects có constructor validate; domain errors phân loại được.
- [ ] Domain giữ invariants/transitions; repository đảm bảo transaction/CAS.
- [ ] Kiểm dependency direction trong CI và test use case qua ports.
- [ ] Public API có docs; rustfmt/clippy chạy được; lint workspace được kế thừa.
- [ ] Dùng các skill Rust đã cài theo công việc: domain/type-driven cho model,
  ecosystem/lifecycle cho adapter, concurrency/error cho runtime,
  refactor/coding-guidelines cho review. Skill cần LSP dùng fallback có kiểm
  references và compiler khi môi trường chưa có LSP.

### 1.8. Storage, migration và embedding

Quyết định triển khai hiện tại là SQLite cho durable task/event/evidence ledger;
CodeGraph giữ SQLite/FTS5 dưới Graph Gateway. Zvec phục vụ semantic retrieval.
Chưa có benchmark dự án chứng minh cần thay SQLite bằng DB khác.
`sqlite-vector` mới bổ sung là ứng viên adapter vector khác trong W8 để so với
Zvec, không phải lý do đổi ledger hay chạy hai vector engine cùng mặc định.

- [ ] Mỗi store có owner, namespace, schema và policy backup/recovery rõ ràng.
- [ ] DDL đặt ở migration files; refinery đang được dùng cho `rusqlite`.
  Alembic thuộc hệ sinh thái Python; tương đương chức năng migration không
  đồng nghĩa refinery có autogenerate/downgrade giống Alembic.
- [ ] Migration đã phát hành bất biến; thay đổi mới có version mới và upgrade
  test. Tách protocol version khỏi storage schema version.
- [ ] SQL queries phức tạp đặt trong file có tên dưới adapter; bind parameters,
  lỗi có operation/query identity và source error để debug, không log secrets.
- [ ] Đo write contention, traversal/context latency, RSS/disk và recovery trên
  workload thật trước khi cân nhắc redb/RocksDB/CozoDB hay engine khác.
- [ ] Local embeddings đi qua `EmbeddingProvider`; FastEmbed là ứng viên cần
  benchmark chất lượng tiếng Việt/Anh và code, chưa được tích hợp/chốt model.
- [ ] `VectorIndex` có Zvec adapter; pin Rust SDK/native library và kiểm packaging
  từng platform. Một process sở hữu writer collection, fleet gửi yêu cầu qua owner.
- [ ] Embedding chạy local sau khi model được provision/cache; remote provider
  là tùy chọn cấu hình riêng. Không suy ra embedding API entitlement từ Plus/Pro.
- [ ] Cache key gồm source/content hash, model revision, tokenizer/preprocessing,
  dimensions và normalization; đổi model cần rebuild collection tương ứng.
- [ ] Vector hit chỉ trả candidate IDs; gateway kiểm namespace/freshness rồi mở
  rộng graph và chọn evidence ranges trước khi tạo ContextPack.

Nguồn tham khảo: [Codex migrations](https://github.com/openai/codex/tree/main/codex-rs/state),
[Refinery](https://docs.rs/refinery/0.9.2/refinery/),
[Zvec Rust SDK](https://github.com/zvec-ai/zvec-rust),
[FastEmbed](https://github.com/Anush008/fastembed-rs).
Đây là nguồn thiết kế; acceptance vẫn cần test adapter của sản phẩm.

## 2. Tầm nhìn sản phẩm

Tên tạm thời: **Project Graph Harness**.

### 2.1. Trải nghiệm mong muốn

Người dùng hỏi:

> Sửa luồng đăng nhập và cho biết những service/test nào bị ảnh hưởng.

Agent không bắt đầu bằng việc mở toàn bộ `AuthService.java`. Harness làm như
sau:

```text
intent
  ↓
graph query planner
  ↓
entry points + callers/callees + implementations + tests + config
  ↓
evidence-linked context pack
  ↓
Codex đọc vài code slice cần thiết
  ↓
plan/patch/test
```

Context trả về có dạng khái niệm:

```json
{
  "schema_version": "project-graph/context/v1",
  "project": "example",
  "revision": "git-sha",
  "view": "flow",
  "nodes": [
    {"id": "route.login", "kind": "route", "name": "POST /login"},
    {"id": "auth.controller.login", "kind": "method", "name": "login"},
    {"id": "auth.service.authenticate", "kind": "method", "name": "authenticate"}
  ],
  "edges": [
    {"from": "route.login", "to": "auth.controller.login", "kind": "dispatch"},
    {"from": "auth.controller.login", "to": "auth.service.authenticate", "kind": "calls"}
  ],
  "code_slices": [
    {"path": "src/auth/controller.ts", "start": 42, "end": 78}
  ],
  "evidence": [
    {"path": "src/auth/controller.ts", "start": 42, "end": 78,
     "source": "codegraph", "revision": "git-sha", "confidence": 0.98}
  ],
  "coverage": {"status": "partial", "unknowns": ["runtime DI target"]}
}
```

### 2.2. Các view phải hỗ trợ

| View | Câu hỏi trả lời | Nguồn chính |
|---|---|---|
| `orientation` | Repo gồm những module/service nào? Bắt đầu tìm ở đâu? | file/module graph, hubs, manifests |
| `symbol` | Symbol này định nghĩa ở đâu và làm gì? | AST/symbol/source slice |
| `flow` | A đi tới B theo đường nào? | calls, routes, messages, Joern khi cần |
| `impact` | Sửa A có thể ảnh hưởng ai? | reverse edges, tests, public APIs |
| `architecture` | Component/boundary/dependency cấp cao là gì? | module + IaC + API + summaries |
| `dataflow` | Dữ liệu đi từ source tới sink ra sao? | reads/writes, taint/data-flow |
| `deployment` | Component được deploy và nối với resource nào? | Terraform/Kubernetes/Docker/CI |
| `runtime` | Đường chạy thực tế nào đã được quan sát? | OpenTelemetry/log/trace |
| `change` | Commit/working tree hiện tại thay đổi gì? | git diff, graph delta, coverage |

## 3. Kiểm kê các repository trong workspace

Snapshot lịch sử ngày 2026-09-11 trong `repo-lock.json` và
`repo-lock-20260911-expanded.json` được giữ nguyên để truy vết, nhưng không
còn là inventory hiện hành. Snapshot live mới ngày 2026-09-12 nằm trong
[`repo-lock-20260912.json`](/home/minh/projects/project-graph-agent/repo-lock-20260912.json), gồm 23 repo và đã thêm
Ripwire. CodeGraph được ghi là dirty bằng status hash; không dùng SHA gốc để
nhận diện nhầm cả patch local. Phase 0 phải pin đầy đủ commit SHA, branch,
status, manifest/license, version, toolchain và artifact thay vì chỉ pin tên
branch hoặc suy compatibility từ README.

Danh sách commit của snapshot live lúc audit là:

```text
OpenSandbox       eed301cca02b261256b2c5e5a23bb1a570c90160
T3MP3ST           29824d5625ede419ac8cdae418c8f4c72c6270f7
archify           18911058008f17dc065af23a2cdc9bfeff6d3f7a
browser           d693f49872fc72676baecfb3626b1876eadaf57d
codegraph         3ed73bc127323e63153bf6ec8354afa82ce36aaf
codepropertygraph e7b6e8da670e4b58a64ba153d197041c25fd798a
codex             818f1cca8ccf8899f0f4d59336baebaccf358eed
codex-multi-auth  f71768223f8cc5d9375b115046b624ce49e7db55
ghostty           44f2a44df7e8c4a0c6df3f7d872ef3d7ead88e51
grit              0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe
icm               2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42
joern             7c1163d96705d354d7c1957531487a63af34dda6
opendev           d32c660e4eed1a8e988d1fd58da88e41ba641d08
orca              26f9fd8ea152ad6126c005e5ae602c7d201f4a99
rtk               79347d5e0e20a61002b8dffe190d377846cb1e59
ruflo             a64f8b1ad89035c8b204f8b6e0893a2288551e06
t3code            3836890e4484406813997259efc69065b25ce698
zvec              67ea1fa65ff99ee4c3a5bf4f2c0a6799c0390ead
ripwire            48222d62f41c6e15f60855127c1d9ee06b3aed4c
```

### 3.1. Thành phần lõi

| Repository | Vai trò trong kế hoạch | Quyết định |
|---|---|---|
| [`codegraph`](/home/minh/projects/outsource/codegraph) | indexer, symbol/dependency graph, context compiler, MCP, watcher, UI | **Fast graph authority và source of truth materialized** |
| [`codepropertygraph`](/home/minh/projects/outsource/codepropertygraph) | CPG schema, domain classes, protobuf, graph interchange | **Deep graph interchange/schema và compatibility layer** |
| [`joern`](/home/minh/projects/outsource/joern) | deep static analysis, CPG/data-flow/taint queries | **Deep-analysis track, chạy theo query/coverage** |
| [`codex`](/home/minh/projects/outsource/codex) | agent/client đích; MCP, plugin, skill, hook và context extension | **Một runtime/native subagents; fork seam khi cần** |
| [`archify`](/home/minh/projects/outsource/archify) | typed JSON IR → validated interactive architecture/sequence/data-flow HTML | **Tầng trình bày cuối** |

### 3.1.1. Ba repository bổ sung: RTK, ICM và GRIT

| Repository | Capability đã xác nhận ở snapshot hiện tại | Cách tích hợp vào harness |
|---|---|---|
| [`rtk`](/home/minh/projects/outsource/rtk) | Rust CLI proxy lọc/compact output của shell; README hiện ghi 100+ command filters và hook tự rewrite, nhưng hook không đi qua các tool Read/Grep/Glob native | **Output projection**: lưu raw stdout/stderr, exit code, argv và hash trước; RTK chỉ tạo view sau đó. Hook Codex là opt-in và phải có passthrough khi parse thất bại. |
| [`icm`](/home/minh/projects/outsource/icm) | Rust single-binary memory MCP với episodic Memories, permanent Memoirs, feedback, decay/dedup; trạng thái upstream hiện vẫn experimental và cloud sync đã bị bỏ | **Memory adapter** theo namespace `project/worktree/revision`; lưu source hash, TTL và provenance. Không đưa ICM memory vào graph authority, không bật PostToolUse toàn cục mặc định vì có thể tạo bloat. |
| [`grit`](/home/minh/projects/outsource/grit) | Rust coordination layer khóa symbol/function bằng AST, cấp worktree cho agent và serialize merge bằng file lock | **Worktree/claim adapter** ưu tiên: map `SubagentTask` sang function claims, giữ `.grit/worktrees/<agent>`, rebase/merge tuần tự và import claim receipt. Grit chỉ xử lý patch coordination; Harness vẫn sở hữu lease, evidence và graph commit. |

Ba repo này không phải ba scheduler mới. RTK nằm ở lớp trình bày/transport,
ICM ở semantic memory, còn GRIT là claim/worktree coordination worker;
`Project Graph Harness` vẫn là chủ sở hữu duy nhất của task DAG, lease,
evidence ledger và GraphWriter. Phase 0 phải khóa version/license/API thực tế
của chúng và tạo adapter contract test trước khi đưa vào critical path.

### 3.2. Capability sản phẩm và adapter tham khảo

| Repository | Vai trò | Điều kiện đưa vào |
|---|---|---|
| [`zvec`](/home/minh/projects/outsource/zvec) | local vector/full-text/hybrid retrieval cho docs, summaries, task memory | Khi FTS + graph ranking chưa đủ cho prose/semantic search |
| [`ruflo`](/home/minh/projects/outsource/ruflo) | router, planner, swarm, persistent memory, Graph RAG, ADR/DDD plugins | Policy/router adapter tùy chọn; phải có contract tests |
| [`opendev`](/home/minh/projects/outsource/opendev) | terminal concurrency, typed workflows, agent fleet, MCP/LSP tools | Reference/adapter cho execution fabric; sản phẩm có runtime riêng sở hữu JobProtocol |
| [`T3MP3ST`](/home/minh/projects/outsource/T3MP3ST) | authorized security-analysis adapter, không phải generic graph extractor | Deep-security worker bắt buộc cho security track, sau threat model, sandbox, scope approval, verifier và license boundary |
| [`OpenSandbox`](/home/minh/projects/outsource/OpenSandbox) | sandbox cho indexing, build/test, Joern và untrusted repository | Execution isolation bắt buộc cho untrusted/deep/runtime jobs; local runner chỉ là trusted fallback |
| [`t3code`](/home/minh/projects/outsource/t3code) | control surface/UI cho Codex và các agent, remote session | UI/reference client; phải hiển thị live swarm/evidence và không sở hữu graph semantics |
| [`ghostty`](/home/minh/projects/outsource/ghostty) | native terminal/VT presentation cho CLI/TUI riêng | VT/performance reference; một TUI chính và headless parity |
| [`ripwire`](/home/minh/projects/outsource/ripwire) | deterministic ranked structural context, task packs, impact/test/quality probes | Context scout/projection tùy chọn qua process boundary; evidence authority vẫn thuộc Harness |

### 3.3. Ranh giới triển khai và dependency policy

| Repository | Giá trị tham khảo | Quy tắc không được vi phạm |
|---|---|---|
| [`ruflo`](/home/minh/projects/outsource/ruflo) | orchestration/memory patterns, policy and witness ideas | Không cài toàn bộ 323 MCP tools/hooks vào Codex, và không dùng Claude-specific setup như protocol core; chỉ dùng adapter được test |
| [`opendev`](/home/minh/projects/outsource/opendev) | Rust agent fleet, typed workflows, context/compaction, MCP và LSP patterns | Không để OpenDev và Codex cùng điều phối một task nếu chưa có lease/idempotency/fencing |
| [`T3MP3ST`](/home/minh/projects/outsource/T3MP3ST) | security gates, reproducible benchmark/receipt và red-team workflow | Không link/embed AGPL code vào core MIT/Apache; không chạy ngoài scope được phê duyệt; finding security không tự thành architecture fact |
| [`ghostty`](/home/minh/projects/outsource/ghostty) | terminal host/native UI, VT parser, PTY và performance architecture | Không đưa UI/terminal dependency vào graph data plane; không phụ thuộc ABI chưa versioned mà không pin commit/test |
| [`t3code`](/home/minh/projects/outsource/t3code) | hiện đã có browser adapter dùng `libghostty-vt` pinned + ABI test | Tái sử dụng protocol/adapter lessons; không để web renderer và native TUI có hai semantics khác nhau |

`T3MP3ST` mang giấy phép AGPL-3.0. Process/API boundary giảm coupling kỹ thuật
nhưng không tự động giải quyết mọi nghĩa vụ copyleft khi phân phối/cung cấp
network service. Không copy code vào binary/library phân phối của sản phẩm;
trước khi ship adapter phải có license decision record và legal review. Nếu
không qua gate này, chỉ giữ security contract/reimplementation độc lập.


### 3.4. Audit ba repository mới — capability và phần phải xây

| Repo / license | Luồng source đã đối chiếu | Học/tái sử dụng | Gap cần giải quyết |
|---|---|---|---|
| browser = Lightpanda / AGPL-3.0-or-later | `src/mcp/Server.zig` → `src/mcp/tools.zig` → `src/browser/tools.zig`; CDP tại `src/server/cdp/` | search provider, semantic tree, scoped markdown/extract, bounded dump | BrowserEvidence gắn URL/time/hash; compatibility fallback; isolation và license packaging |
| orca / MIT | `src/cli/handlers/orchestration/worker-launch-handler.ts` → RPC `orchestration.workerStart`; inbox/reply trong `message-inbox-handlers.ts` | task/spec/deps/parent, model/effort capability check, dispatch receipt, residual resources | graph scope và native thread/attempt binding là contract mới của harness |
| codex-multi-auth / MIT | `scripts/codex.js` → shadow home/proxy → `lib/runtime/rotation-account-selection.ts` → account/affinity/policy | account pin, health/quota, refresh lease, usage ledger, capability matrix | chỉ reference: refresh locks/quota telemetry; không adopt rotation/pool/role-account binding |

Bằng chứng quan trọng:

- Orca `structured-agent-session-status-feed.ts` có publish/revokeLive và
  status sink. `docs/reference/agent-status-store.md` nói rõ mới PR 1a landed;
  các bước xóa duplicate store và hợp nhất mọi reader còn là kế hoạch upstream.
  Harness học quy tắc một authority trên execution host, không claim Orca đã
  hoàn tất toàn bộ migration.
- Orca `docs/reference/ssh-execution-boundary.md`: mất SSH là
  `unverifiable`, không phải `exited`. Host chạy job giữ process identity,
  tools và artifacts; không fallback chạy nhầm repo trên client.
- Multi-auth `lib/routing-profiles.ts` có `projectKey`, preferred/avoid tags,
  model allow/deny và weights. Các worktree có thể resolve về cùng repository
  identity; project profile không phải security boundary. Kiến trúc mới không dùng cross-account roles.
- Multi-auth `lib/runtime/rotation-account-selection.ts`: manual pin ưu tiên
  trước affinity/rotation. `scripts/codex.js` có shadow-home sync-back; cần
  không đưa cơ chế shadow-home/rotation này vào runtime một account; chỉ học các failure modes.
- Lightpanda `src/browser/tools.zig` đã import search providers và có hướng dẫn
  tree → scoped markdown/extract. `src/browser/dump.zig` có max_bytes và marker
  truncation. PNG/PDF text rendering không chứng minh pixel fidelity như Chromium.

Audit này chỉ đọc source/docs; không đăng nhập, đổi account pin, gọi quota của
tài khoản thật hoặc cài các runtime mới.

### 3.5. Audit ba repo bổ sung — output retrieval, learned routing và SQLite vector

Audit local ngày 2026-09-11; ba checkout sạch tại lúc đọc. Manifest bổ sung
`repo-lock-20260911-expanded.json` trong repo sản phẩm ghi 22 repositories;
CodeGraph có local patch nên không tuyên bố cả workspace sạch. Giữ repo-lock
19 repo ban đầu làm baseline lịch sử, không ghi đè evidence cũ.

| Repo / commit / root license | Source đã đối chiếu | Học gì và đặt ở đâu |
|---|---|---|
| `context-mode` / `ad7ef27106ee9ebbe9a75d0b75106c9187506e09` / ELv2 | `src/server.ts` (intentSearch/batch), `src/store.ts` (FTS5/BM25), `src/session/snapshot.ts`, `src/search/flood-guard.ts`, `src/executor.ts`; tests flood-guard/bytecap/cross-session | W2/W4: raw output → artifact → bounded searchable projection; W5/W8: compact resume theo references, quota per-agent bên dưới fleet cap |
| `LLMRouter` / `d1490a37202b1bea799ab3a601cab349d2291b08` / MIT | `llmrouter/models/knnrouter/router.py`, `models/racerrouter/{router,policy,objective}.py`, `evaluation/batch_evaluator.py` | W3/W5/W12: học quality/cost routing từ task receipts; dùng offline evaluator/trainer và một policy seam của harness |
| `sqlite-vector` / `0c2223ada9dce1fa33248c8835a15f51d9a0f655` / Apache-2.0 | `src/sqlite-vector.c`, `src/distance-*.c`, README và `LICENSE.md` | W8: benchmark vector BLOB trong bảng SQLite thường, exact scan làm oracle và quantized scan như lựa chọn giảm RAM/latency |

**Context-mode:** giá trị bổ sung so với RTK là tìm lại từng phần output theo
intent, không chỉ compact chuỗi. Giữ artifact gốc có hash/exit/argv trước khi
index; trả section/line refs và byte budget. FTS/BM25 không cần embedding để
bắt đầu tìm log/test errors. Khi cần semantic search, dùng chính W8 chứ không
tạo memory service thứ hai. Resume chứa task/graph/artifact IDs và gaps, không
dump event history. Rate-limit key do host xác nhận gồm project/session/agent;
có quota riêng cho từng actor và aggregate fleet cap để tránh đổi ID lách budget.

Không sao chép các giới hạn chưa phù hợp: `BuildSnapshotOpts.maxBytes` trong
snapshot hiện bị bỏ qua; harness phải enforce output cuối. `executor.ts` dùng
subprocess/temp cwd/safe env không tự chứng minh OS/filesystem/network isolation;
vẫn phải qua W2/OpenSandbox. ELv2 khác MIT/Apache: chỉ học pattern lúc này,
chưa vendor/copy code hoặc bật plugin; license/distribution review còn pending.
Không lấy tỷ lệ tiết kiệm từ README làm acceptance của harness.

**LLMRouter:** KNN học mapping query embedding → model; RACER có policy hai
candidate và objective reward/cost. Đây là model routing, không phải graph
partition, worktree scheduler hoặc multi-account entitlement manager. GraphRouter
của upstream cũng không thay project code graph. Harness bắt buộc lọc
role/tool eligibility bằng capability/quota/privacy trước khi chấm điểm task.
Học offline evaluator cho delegation/context policy; model routing không phải
runtime default, chỉ nghiên cứu tùy chọn nếu người dùng cho phép sau này.

Dataset lấy từ task thật: intent, graph coverage/unknowns, scope/risk, context
size, model revision, verifier outcome, actual usage, retry và latency. Split
theo repo/thời gian để tránh leakage; đánh giá compile/tests và claim correctness,
không dùng answer similarity làm bằng chứng patch đúng. So deterministic role
policy với learned policy trên held-out tasks; cold-start/out-of-distribution
hoặc policy artifact lỗi phải abstain/fallback rõ, không random-route. Python/
Torch có thể dùng offline nghiên cứu, không thêm server bắt buộc vào fast path.
Quota subscription và giá API vẫn được báo riêng, không suy ra một loại từ loại kia.

**SQLite-vector:** extension tìm vector, không tự sinh embedding. Có storage
trong bảng thường và scan operators; không hiểu “không cần virtual table để lưu”
thành “không có SQLite module”: source có đăng ký `vector_full_scan` và
`vector_quantize_scan`. Không mặc định coi quantization là HNSW/ANN index.
Giữ embedding provider độc lập; dùng cùng vectors/model/filter/k cho benchmark
Zvec và SQLite-vector, kiểm update/delete, quantized cache/preload invalidation,
namespace, crash/reopen, RSS/disk và recall so với exact oracle. Native build,
extension loading policy và compatibility với bundled SQLite phải có fixture.
Mặc định một vector backend theo cấu hình; Zvec track và degraded graph/FTS
vẫn giữ trong scope, không chuyển ledger vì benchmark vector.

Kết luận: nhận cả ba nguồn tham khảo nhưng không thêm scheduler, memory authority
hay runtime model thứ hai. Mọi capability trên là source-audit, chưa phải kết
quả chạy của sản phẩm. Task áp dụng dưới đây giữ chưa tích tới khi có receipt.

### 3.6. Ruflo handwritten HNSW — đưa vào benchmark W8

Học implementation `ruflo/v3/@claude-flow/memory/src/hnsw-index.ts`: heap cho
candidate/top-k, normalized-vector cache, multilayer search, serialization và
post-filter API. Đây là graph ANN cho vector similarity, không phải code graph.
Normalization tránh tính lại norm; dot product vẫn phụ thuộc dimensions, không
coi lời mô tả O(1) trong comment là complexity đã chứng minh.

Phải tách ba backend khi đo: handwritten TypeScript HNSW, native/WASM bridge,
và CLI memory bridge. Source test `issue-2922-hnsw-status-honesty.test.ts` xác
nhận đường bridge được test là brute-force-cosine; plugin `ruvector-upstream`
có fallback mock khi module WASM không nạp được. Tên “HNSW” hoặc trạng thái
ready không đủ để gán speedup cho request path. Không tự cài cả Ruflo vào core
chỉ để gọi một index.

Benchmark sản phẩm chạy trực tiếp lớp TypeScript được transpile nguyên source,
không thay thuật toán; receipt pin source/compiler/runner/dataset/graph seed.
Đo exact cosine scalar JS làm correctness oracle và baseline cùng process;
không coi đó là baseline native tối ưu cho SQLite-vector/Zvec. Random vectors
chỉ kiểm geometry/algorithm, chưa chứng minh semantic quality cho code Việt/Anh.

`searchWithFilters` upstream chỉ overfetch 3× rồi lọc; cần đo recall và underfill
với project/snapshot/model filters trước khi làm retrieval adapter. Namespace
phải được enforce, nhưng trả ít kết quả hợp lệ không đồng nghĩa đã tìm đủ:
adapter cần filter-aware partition/search hoặc fallback có budget và coverage
gap rõ ràng. `getStats().memoryUsage` là estimate, phải báo riêng process RSS.
Acceptance chưa đạt chỉ vì latency thấp khi recall giảm.

### 3.7. Audit 22 repo nền + Ripwire bổ sung — native swarm và các cổng chất lượng mới

- [x] Hoàn tất source-flow audit liên quan cho 22 repo nền, gồm bốn nhánh
  subagent song song và Codex đọc trực tiếp; có revision, phạm vi và nguồn.
  Ripwire được audit bổ sung riêng ở §3.8. Đây không phải đọc mọi file hoặc
  chạy toàn bộ test.
- [x] Hợp nhất quyết định một account/native subagents; worker mới dùng Luna
  (`gpt-5.6-luna`), root giữ model hiện tại theo mục 1.4.
  Không giữ multi-auth/model router làm dependency hay executor bắt buộc.
- [x] Có [ma trận adoption và tổng hợp](/home/minh/projects/project-graph-agent/reports/outsource-synthesis-2026-09-11.md)
  cùng năm báo cáo chuyên đề. Deep-research định hướng truy luồng/source và
  tách bằng chứng tĩnh khỏi runtime verification; không tick implementation từ audit.
- [ ] License/package inventory, các fixture tái hiện mới và implementation
  acceptance vẫn mở; P9.T16 chỉ hoàn thành khi các gate của nó có bằng chứng.

| Nhóm báo cáo / task nguồn | Ghép vào package và task hiện có | Gate bổ sung bắt buộc |
|---|---|---|
| Codex + runtime P0-A/B/D/E | W3/W5, P9.T11/T12/T15 | Effective proactive mode, child reducer, resume lineage, lifetime admission, crash reconcile |
| Runtime P0-C/F, P1-A/B/C/D | W1/W5/W6/W7 | Stop deadline tổng cả parent, payload-digest receipt, source fingerprint, observer gaps, bounded control/data queues |
| Graph G02–G08 | W1/W2/W4 | Dirty snapshot identity, extraction receipts, sync/rebuild oracle độc lập golden, hard serialized budget, partial/unresolved |
| Graph G09–G12 | W10/W11/W12 | Joern limits, CPG loss mapping, artifact publication, stub/not_executed và native-required parity |
| Retrieval R02–R04/R06–R09/R12/R14 | W1/W4/W8 | Scope fail-closed, refresh attribution, artifact/chunk IDs, filtered recall, generation, usage unknown, actor+global budget |
| Retrieval R10/R11/R13 | W8/W12 | Exact nearest oracle, quantization invalidation, C ABI ownership, partial batch và crash durability |
| Infrastructure I-05–I-08/I-13–I-15 | W1/W5/W6/W7 | Raw evidence độc lập RTK, backpressure, reader→writer contention, durable merge candidate, health/error semantics |
| Infrastructure I-09–I-12/I-16 | W6/W7/W9/W12 | Browser capability, sandbox destroy/reconcile, explicit isolation profile, terminal ABI/lifetime, license pin |

Chi tiết fixture và nguồn nằm ở:
[runtime](/home/minh/projects/project-graph-agent/reports/outsource-runtime-audit-2026-09-11.md),
[graph](/home/minh/projects/project-graph-agent/reports/outsource-graph-audit-2026-09-11.md),
[retrieval](/home/minh/projects/project-graph-agent/reports/outsource-retrieval-audit-2026-09-11.md),
[infrastructure](/home/minh/projects/project-graph-agent/reports/outsource-infrastructure-audit-2026-09-11.md)
và [Codex](/home/minh/projects/project-graph-agent/reports/outsource-codex-audit-2026-09-11.md).
Các task nguồn là acceptance bổ sung của package, không phải năm runtime khác nhau.

### 3.8. Audit Ripwire — context structure-first, không thay graph authority

Audit local ngày 2026-09-12: `ripwire` ở commit
`48222d62f41c6e15f60855127c1d9ee06b3aed4c`, license Apache-2.0, C++23,
one-process/offline. Source đã đọc: `README.md`, các skill
`ripwire-before-you-build`, `ripwire-graph-query`, `ripwire-handoff`,
`ripwire-quality-bar`, `src/graph.h`, `src/ingest.cpp`, `src/ingest_cache.h`,
`src/serialize.h`, `src/mcpserver.h` và các fixture/test liên quan. Chưa build,
chạy benchmark hay coi các con số README là acceptance của Harness.

| Điều Ripwire thực hiện | Bài học nhận vào Harness | Ranh giới bắt buộc |
|---|---|---|
| Call/reference graph được xếp hạng; `--for`/`--pack-task` chọn signatures, bodies, docs, callers, test candidates | W4/P9.T13: ContextBroker cần route theo intent trước, rồi trả context pack nhỏ có symbol/range/evidence và test plan | Result là **candidate projection**; name-based resolution, dynamic dispatch, callback, macro và external call có thể thiếu; không tự accept edge hay runtime flow |
| Resolver có `ambiguous`, `unresolved`, `declined`, count floor và refusal selector | W1/W4: mỗi query response phải biểu diễn ambiguity/unresolved/truncation/refusal theo từng edge/response, không trả zero hay one target như chắc chắn | Không chuyển score, PageRank, churn, co-change, hotspot hay `tested` thành fact/quality verdict; chúng là ranking signal có snapshot/coverage |
| Cache per-file content hash, parser/version/config split, checksum/corrupt-cache rejection và deterministic re-stamp IDs | W1/W4: cache key của projection phải gồm file/source hash, extractor version, parser config, root/ignore policy và graph version; cache hit không thay evidence verification | Không lấy native cache blob làm durable authority hay assume whole-tree snapshot khớp chỉ vì cache warm |
| Serialization pre-price bytes, hard budget, explicit truncation/over-ceiling markers và JSON/XML parity tests | W4/P9.T05: ContextEnvelope pre-serialize trước mutation; budget gồm metadata/escaping; disclosed `shown/total`, omitted reason và encoding; semantic parity CLI/MCP phải có contract test | Không dùng estimated tokens như hard security quota; actual account usage và tool output receipt ghi riêng |
| `--handoff` tách disk-verified packet khỏi heuristic co-change/notes/doc retrieval; chấm `HEAD+dirty` | W5/P9.T05: ClaimBundle/Handoff phải tách `verified` và `heuristic`, pin ProjectRef/GraphVersion/dirty fingerprint, không handoff raw transcript | Docs/notes và git heuristics không thành accepted assertion; handoff không cấp capability, lease hay quyền merge |
| `--partition` tạo common core + context slices theo call-graph communities; `--plan-lanes` dự báo collision/landing order từ structural claims | W5/P9.T12: dùng được làm **advisory brief** để giảm orienting lặp lại, nhưng Harness tự tạo task DAG, WorkerTree, lease, write-scope, dependency và fan-in từ authoritative state | Community/rank-median split không phải semantic boundary; `overlap_*` đo bề mặt candidate trước budget trim, `conflict`/`contract_touch` là dự báo structural-only, không phải lock, authorization, conflict detector hay approval |

**Quyết định:** tích hợp sau cùng qua một optional `RipwireContextScout` process
adapter (CLI trước, MCP chỉ khi schema/overhead được chứng minh). Adapter đọc
output có schema/version, giới hạn input/output, record argv/binary/source hash,
root và snapshot receipt; sau đó Harness rebind mọi path/symbol/range vào
`ProjectRef` + accepted evidence trước khi đưa vào ContextEnvelope. Không embed
MCP schema/skills global, không dùng Ripwire làm daemon, graph database,
GraphWriter hoặc swarm scheduler.

`--for`/`--pack-task` của Ripwire có thể tự kèm full body để tối đa terminality;
đó là posture của Ripwire, **không** là default của Harness. `RipwireContextScout`
chỉ nhận metadata/rank/path/symbol/range và disclosure; Harness tự lấy lại range
đã được evidence-bind dưới final byte/token budget. Không chuyển body CDATA,
doc/notes hay quality signals thô vào ContextEnvelope, và không cho candidate
vượt qua provenance/freshness gate chỉ vì ranking cao. `--partition`/`--plan-lanes`
có thể là advisory input để cắt task brief cho swarm, nhưng WorkerTree/lease,
write-scope, approval và fan-in vẫn do Harness kiểm soát. Đặc biệt, không map
Ripwire `partition` index, `execution` recommendation, model/effort hint,
collision/landing-order hay `overlap_*` vào worker identity, capability, quota,
dependency readiness hoặc merge decision; Harness phải revalidate mọi path/range
và tạo admission plan độc lập. Raw scout output là untrusted tool text: parse
theo schema/version + size/time limit, escape như dữ liệu có thể prompt-inject,
và chỉ persist digest/disclosure/candidate references cần audit — không replay nó
như instruction khi resume.

Gate trước khi bật adapter: (1) compatibility/exit/error matrix và license
receipt; (2) golden corpus Orders có dynamic/ambiguous/missing/capped cases;
(3) cache-corruption, parser/config/version change, dirty snapshot và symlink
root tests; (4) CLI/MCP semantic parity nếu MCP được bật; (5) benchmark cùng
query/model với exact source-reading baseline, đo correctness, coverage,
latency và context bytes, không dùng claim README. Nếu tool unavailable/parse
fail/cap hit, ContextBroker trả degradation/coverage gap và quay về CodeGraph +
FTS/source ranges; không silent fallback thành câu trả lời đầy đủ.

#### Cập nhật sau khi bổ sung Ripwire — kế hoạch được nâng cấp (2026-09-12)

Đã đọc lại source và test của Ripwire ở commit
`48222d62f41c6e15f60855127c1d9ee06b3aed4c`, tập trung vào `src/graph.h`,
`src/ingest*.{cpp,h}`, `src/serialize.h`, `src/mcp*.h` và các test
`callformcheck`, `declinecheck`, `cache*check`, `bundleidcheck`,
`partitioncheck`, `planlanescheck`, `handoffcheck`, `quality*check`,
`testgate*`, `editcheck`, `runtrace` và `situdiff`. Kết luận này là source
review; chưa coi benchmark/README claim là bằng chứng của sản phẩm.

Lượt đọc sâu này bổ sung các điểm vận hành phải giữ trong adapter: `pack-task` có
fixed section quotas và carry-forward dưới một ngân sách cuối cùng; handoff giữ
`HEAD+dirty` và chia verified/heuristic; cache là một superset per-tree với
content/parser/config/checksum rejection; MCP/CLI phải đi cùng computation và
renderer; quality/trace/situational output là observations có disclosure, không
phải graph truth. Ripwire cũng chứng minh một gate phải đo precondition của chính
nó và phải chạy cả plain build (degrade diagnostics còn thấy được) lẫn Release
(optimizer/invariant path). Các điểm này được ghi thành receipt
[`p0-compatibility-matrix-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-compatibility-matrix-source-review-2026-09-12.md)
và áp dụng cho P4/P9/W13.

Ripwire bổ sung một lớp **context projection có mục đích**, chứ không giải
quyết thay Harness việc tạo graph authority:

1. **Một lần gọi phải có terminality:** `--for`, `--pack-task`, `--callers`,
   `--impact`, `--from-trace`, `--situ`, `--handoff`, `--test-gate` và
   `--edit-check` là các operation profile khác nhau. ContextBroker sẽ định
   tuyến intent vào profile, có ngân sách chung và một response đủ dùng; không
   bắt agent gọi một chuỗi grep/read không có ledger. Đây là cách giảm việc
   nhiều subagent phải orient lại cùng một cây source.
2. **Honesty là một phần của API:** `ambiguous`, `unresolved`, `declined`,
   `counts_floor`, `truncated`, `over_ceiling` và lý do refusal phải giữ ở
   cấp response/edge. Zero chỉ có nghĩa là chưa tìm thấy trong phạm vi đã
   quét, không có nghĩa là không tồn tại. Mọi projection của Harness phải
   mang coverage/unknown tương ứng.
3. **Cache không phải authority:** cache per-file phải bị vô hiệu khi sai
   content hash, parser/extractor version, config/ignore policy, architecture
   hoặc checksum/bounds. Cache key Harness mở rộng thêm ProjectRef, GraphVersion
   và revision; cache hit chỉ tối ưu tốc độ, không bỏ qua rebind evidence,
   snapshot hoặc freshness verification.
4. **Retrieval theo vai trò, không theo text giống nhau:** trước khi thêm
   symbol/helper/adapter, chạy role query và tìm exemplar có test/fan-in tốt;
   ContextBroker ưu tiên signature, caller/callee, source range, test candidate
   và rationale liên quan. Không mặc định chuyển full body hoặc similarity snippet
   vào prompt; body chỉ được re-capture khi evidence-bound và còn budget.
5. **Handoff và partition chỉ là advisory:** Ripwire tách disk-verified
   `HEAD+dirty` khỏi co-change/docs/notes/quality heuristic; `partition` và
   `plan-lanes` chỉ dự báo community, collision và landing order. Harness vẫn
   tự quyết định WorkerTree, lease, capability, write scope, dependency,
   approval, fan-in và merge; không biến rank/quality thành accepted fact.
6. **Feedback sau chỉnh sửa quay lại graph qua verifier:** quality delta,
   test-gate, edit-check và trace mapping tạo candidate observation/coverage
   gap có command, source snapshot và artifact receipt. Chúng không tự phán
   graph đúng/sai, không cấp retry/merge authority và không thay runtime trace.
7. **CLI trước, MCP sau:** adapter đầu tiên là process-boundary CLI với schema,
   version, timeout, byte cap, exit/error matrix và digest. MCP chỉ bật sau khi
   chứng minh semantic parity, auth/scope, cache/dirty snapshot và fallback;
   không embed Ripwire daemon/schema global vào core.

Các thay đổi vào roadmap:

- [ ] **P4.T07 — Context operation profiles:** thiết kế registry cho
  `orient/navigate/impact/test/handoff/trace/change-review`, route intent
  deterministically, chia budget cho metadata/ranges/citations và ghi một
  disclosure ledger (`shown/total/omitted/why`) trong cùng envelope. Học
  `pack-task` fixed quotas/carry-forward nhưng tối ưu terminality dưới ceiling;
  terminality metric phải đo post-call native reads, tách policy-mandated reads
  khỏi năng lực projection.
- [ ] **P4.T08 — Ripwire evidence rebind:** implement `RipwireContextScout`
  CLI adapter theo process boundary; kiểm schema/version/argv/binary hash,
  ProjectRef/GraphVersion/dirty fingerprint, path/range containment, output
  cap, prompt-injection escaping và replay digest. Candidate chỉ được đưa vào
  ContextEnvelope sau source re-capture; thêm golden cases ambiguous,
  unresolved, declined, malformed, stale và unavailable.
- [ ] **P4.T09 — Cache/fallback parity:** thêm cache-corruption/version/config/
  architecture invalidation, deterministic cold-vs-warm output, CLI fallback
  CodeGraph+FTS, và contract parity CLI/MCP nếu MCP được chấp thuận. Đo
  correctness/coverage/unknowns/bytes/latency trên cùng fixture và baseline
  đọc source; không dùng token estimate làm security quota. Cache phải giữ
  superset records, offset/checksum/trailer và tự reparse khi corrupt; không
  key sai theo một flag làm mất tính đúng hoặc biến cache hit thành evidence.
- [ ] **P4.T10 — One-computation/front-door parity:** mọi CLI/MCP/native query
  dùng cùng operation result, renderer và disclosure vocabulary; pipeline một
  chiều `ingest → graph → projection → context → delivery`, không để từng
  front door tự dựng graph/ranking riêng. Gate so semantic JSON/Markdown/CLI/MCP
  và kiểm exact output budget trước mutation.
- [ ] **P5.T13 — Swarm orientation/handoff:** mỗi worker nhận context pack theo
  operation profile; có verified/heuristic sections, snapshot pin và negative
  evidence. Ripwire `partition/plan-lanes` được lưu như advisory input, không
  được sửa lease, WorkerTree, quota, scope hay merge decision.
- [ ] **P9.T17 — Quality feedback adapter:** chuẩn hóa `quality-delta`,
  `test-gate`, `edit-check`, `affected` và `from-trace` thành candidate
  observations; gắn tool version/command/source snapshot, phân biệt floor với
  total và gửi qua verifier/GraphWriter trước projection.

Thứ tự bắt buộc khi triển khai Ripwire: (a) operation contract + fallback,
(b) CLI adapter và hostile-output tests, (c) re-capture/rebind vào evidence,
(d) cache/freshness và semantic parity, (e) swarm handoff/partition advisory,
(f) quality feedback và benchmark. Chưa được tick P4/P5/P9 chỉ vì đã đọc
source hoặc có adapter stub.

## 4. Kiến trúc mục tiêu

```text
                       ┌─────────────────────────────┐
                       │ Codex CLI / T3Code / agents │
                       └──────────────┬──────────────┘
                                      │ MCP / CLI / HTTP
                       ┌──────────────▼──────────────┐
                       │ Context Gateway              │
                       │ query planner + budget      │
                       │ evidence + coverage        │
                       └──────────────┬──────────────┘
                                      │
               ┌──────────────────────▼──────────────────────┐
               │ Unified Project Graph                       │
               │ code + module + API + config + infra + docs │
               │ static / inferred / observed / authored    │
               └──────┬───────────────┬───────────────┬──────┘
                      │               │               │
          ┌───────────▼──────┐ ┌──────▼────────┐ ┌────▼───────────┐
          │ CodeGraph        │ │ Joern/CPG     │ │ Runtime/IaC    │
          │ fast local index  │ │ deep analysis │ │ adapters       │
          └───────────┬──────┘ └──────┬────────┘ └────┬───────────┘
                      │               │               │
          ┌───────────▼───────────────▼───────────────▼───────────┐
          │ Local stores: SQLite graph + FTS; optional Zvec index │
          └──────────────────────────┬────────────────────────────┘
                                     │
                       ┌─────────────▼─────────────┐
                       │ Archify JSON IR / viewer  │
                       └───────────────────────────┘
```

### 4.1. Nguyên tắc data plane

- Graph Gateway là **writer duy nhất** của graph authority; SQLite dưới
  `.codegraph/` là materialized local store, không phải API ghi tự do cho
  worker/plugin.
- Graph query ưu tiên quan hệ hơn vector similarity.
- Zvec chỉ là read model cho summary/docs/semantic search, có thể xóa và xây lại.
- Joern là deep-analysis read model/adapter; không bắt buộc mọi repo phải có CPG.
- Archify chỉ nhận facts đã được context compiler chọn; không tự suy diễn topology.
- Mọi output có `revision`, `freshness`, `coverage` và `unknowns`.
- Không đưa cả container/file dài vào context nếu chỉ cần outline hoặc symbol range.
- Nếu source vừa sửa nhưng graph chưa đồng bộ, phải cảnh báo và yêu cầu đọc trực tiếp
  đúng range thay vì trả fact cũ như sự thật.
- Graph code, architecture projection, runtime observation và collaborative
  knowledge là các layer riêng; query chỉ được crossing layer theo policy rõ,
  không tạo một "edge tổng quát" không biết nó đến từ đâu.

### 4.2. Native subagent swarm topology

```text
User ↔ Rust CLI/TUI ↔ Codex root session / một account
                          │ Native AgentControl
                   planner tự phân rã task
                          │ TaskSpec + graph refs
           ┌──────────────┼───────────────┐
           ▼              ▼               ▼
       explorer      implementer     independent critic
       read scope    owned worktree  evidence/review scope
           └──────────────┼───────────────┘
                   Evidence/PatchBundle
                          ▼
           deterministic checks → review → integrate
                          ▼
                GraphWriter → ContextGateway

Harness: domain-job ledger · admission · budgets · thread mapping · write gates
Codex: native spawn · message/steer · wait/events · resume · close
Execution host: terminal/PTY · sandbox · browser · analyzer processes
```

Budget account dùng chung toàn cây; child không tạo thêm quota. Event-driven
reconciliation thay LLM polling. Fan-out chỉ khi task độc lập và có lợi ích đo
được. Close agent không chứng minh task integrated hay terminal process đã
dừng; cần execution receipt và independent merge verification.

### 4.3. Một protocol điều phối duy nhất

Không để Ruflo, OpenDev, Codex và sandbox cùng có khái niệm lease riêng. Harness
sở hữu `JobProtocol`; từng repo chỉ implement một trong các interface sau:

```text
Router          classify intent + propose a plan (Ruflo adapter optional)
Scheduler       admit/lease/cancel/retry jobs; owns fencing token
Executor        runs an approved argv/workflow (local runner/OpenDev adapter)
Analyzer        emits assertions + artifacts (CodeGraph/Joern/T3MP3ST/adapters)
Verifier        validates evidence and policy
GraphWriter     single transactional committer
ContextGateway  reads only accepted materialized facts
```

State machine tối thiểu:

```text
queued → admitted → leased → running → collecting → verifying
                                              ├→ committed
                                              ├→ rejected
                                              ├→ cancelled
                                              └→ timed_out / incomplete
```

Mỗi lease có `fencing_token`, expiry và owner. GraphWriter từ chối receipt cũ
hoặc receipt của lease bị thay thế. Delivery của worker là **at-least-once**;
idempotency ở GraphWriter/outbox mới là điều bảo đảm, không hứa "exactly once"
cho terminal process hay network. Một project/revision có một serialized graph
writer; readers dùng snapshot nhất quán.

### 4.4. Snapshot, delta và artifact lifecycle

`ProjectRef` phải định danh chính xác nội dung được phân tích:

```text
repository_id + worktree_id + git_head + working_tree_fingerprint
+ manifest/config hash + ignore-policy version
```

`GraphVersion` thêm extractor/resolver/adapter version và tạo một manifest
content-addressed. Index hiện hành giữ latency nhanh trong worktree; khi cần
`revision=git-sha` hoặc `change`, runner tạo/reuse snapshot read-only trong
cache riêng, rồi `GraphDelta(base, head)` so sánh hai version. Không đổi index
live để trả lời lịch sử.

Artifact (stdout/stderr đã redact, CPG export, trace, diagram receipt) phải có
content hash, retention class, encryption/redaction state và reference count.
Garbage collection chỉ xóa artifact/snapshot khi không còn GraphVersion,
accepted assertion hoặc user-pinned report tham chiếu; xóa không được làm mất
khả năng giải thích một edge còn hiển thị.

### 4.5. Graph-aware swarm: subagent là một graph workload, không phải chat room

Swarm là một phần quan trọng của sản phẩm, nhưng chỉ tạo giá trị khi graph quyết
định **điểm chia**, **context tối thiểu** và **điểm hội tụ**. Với một flow đơn
giản, một agent + ContextEnvelope thường tốt hơn fan-out. Với một task vượt qua
nhiều service/boundary, planner tạo task DAG từ graph cut, connected component,
entry point và dependency order — không chỉ tách prompt theo tên thư mục.

```text
                     graph-aware planner
                              │
              ┌───────────────┼────────────────┐
              ▼               ▼                ▼
          scout/locator   flow tracer      config/IaC mapper
          read-only       read-only         read-only
              └───────────────┼────────────────┘
                              ▼
                    evidence verifier
                              ▼
                    integrator/context pack
```

Mỗi `SubagentTask` bắt buộc có:

```text
task_id, parent_job_id, role, graph_version, scope_selector/node_ids,
entry/exit boundary, allowed capabilities, ContextPack reference,
expected assertion kinds, output schema, token/time/tool/cost budget,
lease/fencing token, dependency ids, redaction/egress policy
```

Subagent mặc định nhận graph slice + evidence IDs + source ranges cần thiết,
không nhận toàn repo hoặc raw transcript của swarm. Nó chỉ trả `ClaimBundle`
(assertions, citations, unknowns, alternatives, receipt), không tự merge vào
memory/graph. Handoff là object versioned, có owner/revision/expiry; integrator
đọc structured handoff chứ không tin lời kể tự do.

Topology được chọn theo task:

| Tình huống | Topology | Quy tắc merge |
|---|---|---|
| symbol/flow hẹp, một graph component | single agent | không spawn chỉ để "có swarm" |
| nhiều branch độc lập từ một entry point | fan-out/fan-in | verifier kiểm từng ClaimBundle |
| source → API → worker → infra theo thứ tự | pipeline/DAG | downstream chỉ thấy accepted upstream assertions |
| dynamic hoặc security-sensitive boundary | tracer + adversarial verifier | evidence thắng majority vote |
| patch nhiều file có overlap | ownership-aware worktree tasks | một writer/region, review trước merge |

Ruflo có thể implement router/topology/memory; OpenDev có thể chạy worker
song song. Nhưng graph scheduler mới quyết định partition, ownership và merge.
Không dùng voting để biến suy đoán thành fact: ba agent cùng sai vẫn là sai.

Circuit breaker của swarm phải chặn fan-out vô hạn: `max_depth`, `max_children`,
`max_parallel`, cumulative token/tool/cost budgets, duplicate-scope detection,
cooldown khi evidence acceptance thấp và explicit escalation khi task đòi thêm
capability. Event log giữ causal DAG `job → subtask → claim → verifier decision`
để agent sau biết vì sao một kết luận đã bị reject.

### 4.6. Product shape: terminal-native như OpenDev, graph-native ở bên trong

Sản phẩm cuối là một binary Rust/TUI có thể chạy headless, không phải một MCP
server đứng sau Codex. Tên tạm thời `project-graph-agent`:

```text
project-graph-agent
├── CLI: run / inspect / swarm / graph / memory / worktree / review / replay
├── TUI: chat · worker tree · terminal panes · graph route · evidence · approvals
├── Session runtime: một Codex root account + native subagent tree
├── Swarm runtime: role registry · task DAG · leases · handoffs · fan-in
├── Tool runtime: shell · MCP · LSP · CodeGraph · Joern · sandbox
├── Context runtime: graph query · context compiler · compaction · memory
├── Evidence runtime: receipts · ledger · verifier · graph writer
└── Storage: project graph · snapshots · artifacts · session/event log
```

Các chế độ vận hành:

| Chế độ | Hành vi |
|---|---|
| `chat` | một agent chính trả lời/sửa code, tự query graph trước discovery |
| `swarm` | planner tạo task DAG, spawn role-specific subagents và fan-in claims |
| `analyze` | chạy toàn bộ graph/index/deep-analysis pipeline không cần chat |
| `review` | swarm security/test/architecture review trên revision hoặc diff |
| `run` | thực thi workflow terminal/build/test/runtime đã khai báo |
| `replay` | phát lại session/event/receipt để debug hoặc benchmark |
| `serve` | app-server/MCP/HTTP gateway cho T3Code và client bên ngoài |

Codex integration dùng app-server làm đường chính; MCP/skills/roles là lớp tương
thích. Hooks chỉ bổ sung context, audit và invalidation theo schema hỗ trợ.
Scheduler cấp lease trước launch; không dựa vào SubagentStart để đổi auth hoặc
cấp quyền. `codex-agent-graph-store` lưu topology thread, khác với task DAG và
project code graph. Harness TaskAttempt liên kết native parent/root/child IDs;
không dựng external account workers thay native delegation.

Compaction lưu ContextPack references và context epoch; source từng được gửi
nhưng đã bị compact phải fetch lại khi cần. Source/README/web text được chèn
như dữ liệu, không nâng thành developer policy.

TUI không đọc SQLite trực tiếp. TUI subscribe `EventStream` và gọi
`ContextGateway`; cùng một event phải render được ở terminal, JSONL, HTTP và
replay. Đây là điều kiện để học OpenDev về trải nghiệm terminal mà không làm
presentation layer trở thành một nguồn state thứ hai.

## 5. Mô hình dữ liệu hợp nhất

### 5.1. Node types

#### Tầng source/code

- `project`, `repository`, `worktree`, `commit`;
- `directory`, `module`, `package`, `file`;
- `namespace`, `class`, `interface`, `trait`, `struct`, `enum`;
- `function`, `method`, `constructor`, `parameter`, `field`, `variable`;
- `route`, `handler`, `test`, `fixture`;
- `config_file`, `config_key`, `manifest`, `dependency`.

#### Tầng hệ thống/triển khai

- `service`, `worker`, `frontend`, `library`, `cli`;
- `api`, `endpoint`, `schema`, `event`, `topic`, `queue`, `consumer`;
- `database`, `table`, `bucket`, `cache`, `external_system`;
- `container`, `image`, `pod`, `deployment`, `terraform_resource`;
- `network`, `secret_reference`, `iam_role`, `environment`.

#### Tầng ngữ nghĩa/runtime

- `document`, `adr`, `requirement`, `domain_concept`, `bounded_context`;
- `runtime_span`, `log_pattern`, `metric`, `trace_route`;
- `summary`, `decision`, `task`, `test_result`;
- `analysis_run`, `artifact`, `claim`, `graph_version`, `subagent_task`.

Toàn bộ node model là target của sản phẩm; implementation có thể materialize
theo dependency graph của các phase. Source/code, manifest, route, test và
evidence là nền móng; system/runtime, swarm và knowledge nodes có adapter riêng,
không bị loại khỏi Definition of Done.

### 5.2. Edge types

#### Edges có thể suy ra từ source

- `contains`, `defines`, `imports`, `reexports`;
- `calls`, `called_by`, `references`, `instantiates`;
- `extends`, `implements`, `overrides`, `satisfies_type`;
- `reads`, `writes`, `returns`, `passes_argument`;
- `dispatches_to`, `route_handles`, `navigates_to`;
- `tests`, `fixtures`, `generated_from`.

#### Edges từ hệ thống/config

- `belongs_to_service`, `exposes`, `publishes`, `consumes`;
- `configures`, `deploys`, `mounts`, `connects_to`, `persists_to`;
- `depends_on_package`, `depends_on_resource`, `uses_secret`;
- `module_input`, `module_output`, `terraform_reference`.

#### Edges runtime/knowledge

- `observed_calls`, `observed_reads`, `observed_writes`;
- `observed_message`, `documented_by`, `owned_by`, `decided_by`;
- `supersedes`, `affected_by_commit`, `validated_by_test`.

Mỗi edge phải có:

```text
source_id
target_id
kind
layer              code | architecture | deployment | runtime | knowledge
provenance         extractor/resolver/trace/doc/human
confidence         0..1, nếu có thể tính
evidence[]         path + line range + revision hoặc trace id
resolution_status  resolved | partial | unresolved | stale
observed_at        nếu là runtime evidence
```

### 5.3. Provenance contract

Không cho phép output trả lời kiểu “A gọi B” nếu không biết nguồn. Các trạng
thái cần được hiển thị:

| Trạng thái | Ý nghĩa |
|---|---|
| `authoritative` | resolver/compiler/package metadata xác nhận |
| `static_inferred` | suy ra từ AST/CPG/framework convention |
| `runtime_observed` | có trace/log/telemetry xác nhận |
| `documented` | chỉ có trong docs/ADR |
| `human_asserted` | người dùng/kiến trúc sư xác nhận |
| `partial` | có một phần path hoặc còn unresolved nodes |
| `unknown` | chưa đủ evidence; không được tự điền |

### 5.4. Fact assertion ledger và trust resolution

`nodes`/`edges` là view tối ưu truy vấn; assertion ledger là nơi giữ lý do một
quan hệ được phép xuất hiện. Mỗi producer gửi một `FactAssertion`, không gửi
SQL trực tiếp:

```json
{
  "assertion_id": "sha256-or-uuid",
  "graph_version": "gv_...",
  "subject": "symbol:...",
  "predicate": "calls",
  "object": "symbol:...",
  "claim_state": "candidate|accepted|rejected|superseded|expired",
  "assertion_kind": "parser|resolver|cpg|runtime_trace|document|human|security_finding",
  "evidence_ids": ["ev_..."],
  "producer": {"analyzer": "...", "version": "...", "run_id": "..."},
  "scope": {"project": "...", "revision": "..."}
}
```

`Evidence` phải pin `GraphVersion`, source path đã canonicalize, byte/line
span, content hash, extractor/query version và artifact receipt. `edge_key` là
identity canonical của relation; `called_by` là reverse query/index của
`calls`, không cần trở thành một assertion cạnh tranh riêng. Identity symbol
không chỉ dựa trên path: dùng project/worktree, language, qualified name,
signature/kind, declaration span và semantic/content fingerprint; rename/move
matching là một relation có evidence, không phải đoán im lặng.

Trust resolution không trộn loại evidence:

- parser/compiler/resolver exact-revision có thể materialize `static` fact;
- Joern/CPG chỉ materialize `validated_static` khi source mapping và query
  receipt hợp lệ;
- trace hợp lệ chỉ materialize `runtime_observed`, không biến nó thành static
  `calls` hay complete topology;
- docs/human tạo knowledge/architecture overlay riêng;
- finding của T3MP3ST là security claim/candidate cho tới khi verifier xác
  nhận bằng source/test/trace phù hợp.

Nếu assertions mâu thuẫn, ContextGateway trả conflict/coverage gap và các nguồn
liên quan; không lấy confidence trung bình để giả vờ có một fact. Confidence
phải được calibrate trên fixture và không thay thế evidence.

### 5.5. Graph projection và architecture overlay

Code graph không tự suy ra "service thật" chỉ từ cây thư mục. Hệ thống giữ bốn
projection tách biệt:

1. **Structural code graph:** parser/resolver/CPG facts.
2. **System projection:** API, manifest, IaC, build/deploy mapping.
3. **Observed projection:** traces/tests/runtime evidence theo environment.
4. **Knowledge/architecture overlay:** ADR, ownership và map do người dùng
   xác nhận.

Thêm một `architecture.yaml`/JSON versioned (hoặc metadata tương đương) cho
những điều code không thể chứng minh: service boundary, ownership, intended
deployment, aliases và allowed cross-boundary dependencies. Mọi collapse thành
component trên diagram phải ghi rule ID + input assertion IDs. LLM có thể đề
xuất overlay nhưng chỉ human/validator mới accept; diagram phải phân biệt
`observed`, `declared`, `inferred` và `unknown`.

## 6. Kế hoạch triển khai theo phase

Các bước có ID là checklist thực thi; chỉ tích sau khi có receipt đúng scope.
Các danh sách về loại node, thứ tự ranking hoặc ví dụ protocol là đặc tả,
không phải đầu việc nên không chuyển thành checkbox. Khi một bước lớn cần
nhiều worker, tách thành task con dưới ID đó, giữ gate cha chưa tích.

### Phase 0 — Chốt baseline và hợp đồng sản phẩm

#### Mục tiêu

Đảm bảo team hiểu phần nào đã tồn tại trong `codegraph`, phần nào chỉ là ý
tưởng, và không xây lại các tính năng đã có.

#### Việc cần làm

- [x] **P0.T01** — Ghi snapshot version/commit/branch/license của 23 repo vào
   [`repo-lock-20260912.json`](/home/minh/projects/project-graph-agent/repo-lock-20260912.json); pin cả analyzer
   source/status và local CLI/kernel artifact. Ghostty ABI được ghi
   `not-enabled` vì chưa được dùng.
- [x] **P0.T02** — Chạy smoke test hiện tại của `codegraph`: build kernel,
   dist, `npm test`, index/status fixture, `codegraph_explore` qua MCP stdio
   thực tế và watcher sau khi sửa một file. **5.478 pass / 11 skip / 0 fail**;
   Orders raw receipts và source-study ở
   [`p0-codegraph-smoke-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-codegraph-smoke-source-review-2026-09-12.md).
- [x] **P0.T03** — Đã chọn/tạo `fixtures/orders` (Node corpus v1; Docker chưa chạy), gồm:
   - HTTP entry point;
   - controller/handler;
   - service;
   - database;
   - queue/event;
   - test;
   - Docker hoặc Terraform.
- [x] **P0.T04** — Đã viết `benchmarks/orders-v1.json`, câu hỏi benchmark cố định:
   - “Hệ thống khởi đầu request ở đâu?”
   - “Luồng `POST /orders` tới database thế nào?”
   - “Nếu sửa `OrderService`, những gì bị ảnh hưởng?”
   - “Service nào publish/consume event này?”
   - “Tạo architecture/sequence/data-flow diagram có evidence.”
- [ ] **P0.T05** — Tạo baseline không có graph: agent chỉ được dùng `find`, `grep`, `rg`, `Read`.
   Recheck deterministic factory probe 2026-09-12 exit 0: 16/32 inner
   definitions, 15.796 chars (soft 13.000 / hard 19.500), native loaded và
   source fingerprint khớp. Chưa chứng minh build/source correspondence hoặc
   agent A/B; thiếu PATH tools vẫn còn.
   [Current diagnostic](/home/minh/projects/project-graph-agent/reports/w0-codegraph-2026-09-11.md).
   Reuse trước các deterministic probe và `scripts/agent-eval/` đã có trong
   `codegraph`; không viết một evaluator song song rồi so số liệu không cùng
   cách đo. **Source gate đã ready nhưng execution bị block:** đã đọc
   `run-all.sh`, `no-cli-shim.sh`, `parse-run.mjs`, `bench-readme.sh`,
   `parse-bench-readme.mjs`, `offload-eval.md` và parser self-test; receipt
   [`p0-no-graph-baseline-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-no-graph-baseline-source-review-2026-09-12.md).
   `claude`, `jq`, `codegraph` không có trên PATH (dist launcher cục bộ của
   CodeGraph vẫn tồn tại nhưng không thay thế được agent harness); npm hiện là
   12.0.2 thay vì pin 10.9.2. Không giả lập stream-json, token/cost/latency hay correctness;
   chỉ parser self-test 68/68 pass. Chưa tick cho đến khi chạy được đúng
   baseline trên fixture Orders và lưu raw logs/ground-truth result.
- [x] **P0.T06** — Viết compatibility matrix cho từng integration: current public API,
   required capability, version pin, license, failure mode, và adapter test.
   Đặc biệt không giả định Ruflo hook/Claude surface hoạt động nguyên vẹn với
   Codex, hoặc OpenDev là một embedded generic job API chỉ vì có agent fleet.
   Matrix 23 repo, boundary và test catalog nằm ở
   [`p0-compatibility-matrix-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-compatibility-matrix-source-review-2026-09-12.md);
   các adapter execution/license closure vẫn mở.
- [x] **P0.T07** — Chọn target stack đầu tiên và một owner cho từng workstream; ghi threat model
  cho source untrusted, prompt injection, dependency install, terminal output,
  secret/egress và worker write permission. Receipt source-first:
  [`p0-threat-model-ownership-dag-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-threat-model-ownership-dag-source-review-2026-09-12.md).
  Đây là policy/design gate; sandbox, native runtime và license enforcement
  vẫn cần acceptance riêng.
- [x] **P0.T08** — Lập dependency DAG đầy đủ cho graph, native swarm/session binding,
  browser, runtime, deep analysis, memory và terminal; ghi rõ critical path,
  parallel tracks và Ripwire sub-DAG. T3Code/Orca frontend và Codex fork seam
  chỉ triển khai khi cần; bảng mục 11 là thứ tự dependency chính thức.
  Receipt và owner map nằm trong
  [`p0-threat-model-ownership-dag-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p0-threat-model-ownership-dag-source-review-2026-09-12.md).

#### Đầu ra

- [x] `PLAN.md` có kiến trúc mục tiêu, dependency DAG và checklist tiến độ.
- [ ] fixture và câu hỏi benchmark versioned.
- [ ] report baseline: tool calls, file reads, tokens, latency, correctness.
- [x] P0.T05 source-study: evaluator upstream, metric caveats và environment
  blocker đã được ghi receipt; execution baseline vẫn chưa đạt.
- [ ] bảng capability hiện tại của `codegraph`.
- [ ] compatibility matrix cho từng integration.
- [x] repository lock/release manifest của toàn bộ 23 repo và product pins.
- [x] compatibility matrix 23 repo với API/boundary/license/failure/adoption gate.
- [x] threat model/policy baseline và owner/DAG receipt; ADR về graph
  authority/snapshot strategy đã được chốt trong mục 1.0–1.6 và được receipt
  source-first ở P0.T07/P0.T08 dẫn chiếu.

#### Điều kiện hoàn thành

- [ ] Có thể tái chạy baseline trên cùng commit.
- [ ] Mọi con số benchmark có command và artifact chứng minh.
- [ ] Không đưa claim của README upstream vào acceptance nếu chưa chạy lại.
- [ ] Có một stack target được chọn; không bắt đầu đồng thời 20 integration.
- [ ] Capability chưa có test adapter bị đánh dấu `assumption`, không được đưa vào
  critical path.

### Phase 1 — Context contract và graph semantics

#### Mục tiêu

Định nghĩa format ổn định giữa indexer, query layer, Codex và Archify trước khi
thêm nhiều analyzer.

#### Đề xuất thư mục

```text
schemas/
  project-graph-node.schema.json
  project-graph-edge.schema.json
  fact-assertion.schema.json
  evidence.schema.json
  graph-version.schema.json
  job-receipt.schema.json
  subagent-task.schema.json
  context-envelope.schema.json
  architecture-evidence.schema.json
  graph-delta.schema.json
fixtures/
  multi-service/
  dynamic-boundary/
  infra-and-runtime/
```

Repo sản phẩm đã tồn tại: đặt schema công khai và compatibility fixtures tại
repo sản phẩm, protocol adapter tham chiếu bản versioned đó. Không tạo một
bản schema authoritative thứ hai trong `codegraph/docs`.

#### Việc cần làm

- [x] **P1.T01** — Định nghĩa node/edge IDs ổn định qua content/path/symbol
  identity. `graph-domain` hiện có `NodeId`/`EdgeId` value objects với format
  versioned và namespace repository/worktree; `graph-application` tạo digest
  bằng canonical length-prefix + SHA-256 để giữ domain thuần,
  kind/language/qualified-name/signature/path/byte-anchor; line/column là
  evidence, không phải identity duy nhất. Content fingerprint được giữ riêng
  để phát hiện body change mà không đổi logical symbol ID; edge key gồm hai
  endpoint IDs, kind và discriminator, không dùng auto-increment/call-site
  row. Có strict parse/path/span/fingerprint/collision tests.
  Rename/move matching vẫn là relation có evidence ở task sau; chưa đổi
  extractor/database hoặc GraphWriter. [Source/implementation receipt](/home/minh/projects/project-graph-agent/reports/p1-graph-identity-source-review-2026-09-12.md).
- [x] **P1.T02 — done** — Định nghĩa `schema_version` và forward-compatible
  fields; `graph-protocol` có `SchemaCompatibility`, từ chối legacy/future
  không downgrade, giữ strict unknown top-level fields và chỉ nhận
  namespaced/bounded `extensions`; độc lập với SQLite migration, extractor,
  `GraphVersion` và cache format. Đã chạy protocol tests và foundation
  validation. [Implementation/source receipt](/home/minh/projects/project-graph-agent/reports/p1-schema-version-source-review-2026-09-12.md).
- [x] **P1.T03** — Định nghĩa `ContextEnvelope` như ở mục 2.1: wire schema
  version riêng, view/query, project+revision+graph snapshot, bounded nodes,
  edges, code slices, evidence manifest, budget và `source_is_untrusted`.
- [x] **P1.T04** — Định nghĩa line evidence bắt buộc cho mọi source-derived
  node/edge/slice; kiểm tra citation cùng project, graph version, path và range.
- [x] **P1.T05** — Định nghĩa `coverage` và `unknowns`, giữ rõ
  `complete|partial|unknown` cùng lý do unresolved thay vì fabricate target.
  Implementation receipt:
  [`p1-context-envelope-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p1-context-envelope-source-review-2026-09-12.md).
- [x] **P1.T06** — Định nghĩa `ProjectRef`, `GraphVersion`, snapshot cache và `GraphDelta`;
   nêu rõ working-tree fingerprint, source hash, resolver/analyzer version và
   khi nào result bị stale/expired. Domain invariants và strict wire DTO đã có
   trong `crates/{domain,protocol}/src/graph_version.rs`; cache không được coi
   là atomic filesystem snapshot, delta chỉ forward cùng repository/worktree.
   [Source/implementation receipt](/home/minh/projects/project-graph-agent/reports/p1-graph-version-source-review-2026-09-12.md).
- [x] **P1.T07 — typed fact trust boundary hoàn tất trong phạm vi đã kiểm chứng**
  — domain state machine + GraphWriter, candidate-only wire DTO, application
  accept port, V17 immutable assertion/decision ledger, same-snapshot source/
  artifact/analysis evidence checks, replay/conflict/corruption handling và
  accepted-only projection. Runtime/document/human evidence registry, CLI/MCP
  surface, generic terminal-decision persistence và full graph projector vẫn
  là phần mở rộng của các task sau; không coi P1.T07 là hoàn tất W1.
  [`p1-fact-trust-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p1-fact-trust-source-review-2026-09-12.md).
- [x] **P1.T08 — hoàn tất trong phạm vi scoped; source-gate ready** — Định nghĩa
   context pack cho subagent: scope selector, capability grant,
   handoff/claim bundle và untrusted-content label. Đã đọc lại
   Ripwire/context-mode/Orca/OpenDev đúng các luồng pack, scope, capability và
   handoff; triển khai typed domain + strict wire contract
   `project-graph/context-pack/v1`, claim candidate-only, handoff không mở
   scope/quyền và mọi text là data không tin cậy. Tests domain/protocol và
   `cargo test --workspace --locked --offline` đã pass. `cargo fmt` chưa chạy
   được vì môi trường thiếu component `rustfmt`; đây là giới hạn môi trường,
   không phải gate chức năng. P1.T09/P1.T10 vẫn là các gate output budget và
   schema/golden độc lập. [Source/implementation receipt](/home/minh/projects/project-graph-agent/reports/p1-context-pack-source-review-2026-09-12.md).
- [x] **P1.T09 — hoàn tất contract budget trong phạm vi scoped** —
   `ContextBudget` hiện khai báo và validate max nodes/edges, source ranges và
   bytes, traversal depth, serialized bytes (hard transport bound), characters
   và optional tokenizer-specific tokens; emitted counters không vượt ceiling,
   token metadata phải nhất quán. Deterministic serialization được giữ bằng
   struct field order/BTreeMap extensions và regression test; final compiler
   enforcement/selection vẫn thuộc P4. [Budget/schema receipt](/home/minh/projects/project-graph-agent/reports/p1-context-budget-schema-2026-09-12.md).
- [x] **P1.T10 — hoàn tất schema/golden gate trong phạm vi scoped** — thêm strict
   Draft 2020-12 `ContextEnvelope`/`ContextPack` schemas, valid golden fixtures,
   invalid compatibility fixtures và Python `jsonschema` validator độc lập có
   semantic checks evidence/snapshot/scope. Candidate-only claims và
   untrusted-content markers được ràng buộc ở schema; independent JSON Schema
   gate không thay P4 output compiler. [Budget/schema receipt](/home/minh/projects/project-graph-agent/reports/p1-context-budget-schema-2026-09-12.md).
- [x] **P1.T11 — đóng acceptance contract trong phạm vi scoped** — siết cả
   schema độc lập và Rust protocol: item `resolved` không được thiếu evidence,
   item không có evidence phải mang trạng thái `partial|unresolved|stale`; thêm
   `ContextEnvelope::canonical_json` để ổn định bytes bất kể thứ tự discovery;
   validator Python chạy semantic checks cho fixture schema-valid nhưng invalid.
   Candidate claim vẫn nằm ngoài normal flow và ContextPack giữ candidate-only.
   Receipt source-first và raw verification ở
   [`p1-acceptance-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p1-acceptance-source-review-2026-09-12.md).

#### Điều kiện hoàn thành

- [x] Context envelope validate được bằng một validator độc lập.
- [x] Cùng graph + cùng query + cùng revision tạo output byte-stable sau khi bỏ
  timestamp runtime.
- [x] Một edge không có evidence hoặc provenance bị reject, hoặc được đánh dấu
  `unknown` rõ ràng.
- [x] Một `candidate`/security finding không thể xuất hiện trong normal flow view
  như accepted fact.
- [x] Context/handoff không trộn project, worktree hoặc GraphVersion khác nhau.
- [x] Source content được label as untrusted data trong mọi agent-facing contract.

P1 acceptance receipt: [`p1-acceptance-source-review-2026-09-12.md`](/home/minh/projects/project-graph-agent/reports/p1-acceptance-source-review-2026-09-12.md).
Đây chỉ là gate của context contract; W1-I, snapshot authority nối runtime,
ContextGateway/compiler và package completion vẫn mở.

### Phase 2 — Chuẩn hóa và mở rộng `codegraph` làm core indexer

#### Mục tiêu

Biến hệ thống hiện tại thành một graph/context engine chính thức, nhưng giữ
đường đi tương thích với CLI/MCP đang dùng.

#### Files/areas cần ưu tiên trong `codegraph`

- `src/db/schema.sql`, `src/db/migrations.ts`: graph metadata, evidence, source
  revision, adapter results và schema migrations.
- `src/types.ts`: type model cho provenance, coverage, system nodes và context.
- `src/extraction/`: extraction layer và language-specific facts.
- `src/resolution/`: cross-file/framework/package resolution.
- `src/graph/`: traversal, type hierarchy, dynamic boundary, named-symbol flow.
- `src/context/`: ranking, formatting và context envelope.
- `src/mcp/`: tool schema, dispatch, output cap, staleness handling.
- `src/sync/`: watcher, incremental update, worktree/revision reconciliation.
- `src/ui-server/api/`: API cho graph/evidence/flow/coverage.
- `src/installer/targets/codex.ts`: Codex config/integration.

#### Thay đổi dữ liệu

Thêm từng bước, không sửa schema thủ công mà không có migration:

```sql
CREATE TABLE graph_revisions (...);
CREATE TABLE evidence (...);
CREATE TABLE edge_evidence (...);
CREATE TABLE adapter_runs (...);
CREATE TABLE coverage_reports (...);
CREATE TABLE summaries (...);
CREATE TABLE external_entities (...);
```

Các bảng này phải có foreign key tới `nodes`, `edges`, `files` khi phù hợp.

#### Indexer behavior

- [ ] **P2.T01** — Giữ extraction hiện tại làm fast path.
- [ ] **P2.T02** — Khi file thay đổi, re-extract file và các dependent bị ảnh hưởng; không
   re-scan toàn repo trừ khi manifest/config thay đổi.
- [ ] **P2.T03** — Gắn `file_hash`, `git_revision`, `extractor_version` và `resolver_version`.
- [ ] **P2.T04** — Giữ unresolved references thay vì đoán.
- [ ] **P2.T05** — Với dynamic dispatch, tạo boundary node/edge có trạng thái `partial`.
- [ ] **P2.T06** — Với container/class/module lớn, mặc định trả structural outline thay vì cả
   body.
- [ ] **P2.T07** — Tách source slice từ graph node bằng line/column và hash để tránh trả nhầm
   nội dung sau khi file đã thay đổi.

#### Query surface

Giữ legacy `codegraph_explore` tương thích; expose target
`project_graph_context_v1` cho các view sau và đo tool selection bằng A/B:

```json
{
  "query": "How does POST /orders reach the database?",
  "projectPath": "/repo",
  "view": "flow",
  "depth": 5,
  "budget": {"maxChars": 18000, "maxNodes": 40},
  "includeCode": "targeted",
  "evidence": "required",
  "freshness": "catch-up"
}
```

Các intent nội bộ:

- `orient`: map/hubs/entry points;
- `locate`: symbol/file/route lookup;
- `trace`: source-to-target route;
- `impact`: reverse dependency and test impact;
- `architecture`: collapse symbols thành module/service boundaries;
- `dataflow`: read/write/source/sink path;
- `delta`: compare revision/working tree;
- `explain`: combine graph + selected source slices.

Các tool nhỏ hiện có (`node`, `search`, `callers`, `callees`, `impact`, `files`,
`status`) tiếp tục tồn tại cho CLI/debug nhưng không cần expose mặc định cho
agent nếu `explore` đã bao phủ output.

#### Acceptance tests

- [ ] Full suite CodeGraph sau patch qua gate: xử lý hoặc có quyết định kỹ thuật
  kèm regression fixture cho từng lỗi baseline; không sửa kỳ vọng chỉ để xanh,
  không dùng targeted pass thay full suite hay bỏ qua regression mới.
- [ ] Query container 2.000 dòng trả outline trước, không trả full body mặc định.
- [ ] Query flow trả node/edge/evidence và không mở file ngoài range cần thiết.
- [ ] Query trên file đang pending sync trả staleness warning.
- [ ] Query unresolved dynamic call không tự biến `unknown` thành edge chắc chắn.
- [ ] Index/sync/serve concurrent không corrupt SQLite/WAL.
- [ ] Cross-worktree projectPath không đọc nhầm `.codegraph` của worktree khác.

### Phase 3 — Codex compatibility integration

#### Mục tiêu

Codex tự biết khi nào cần dùng graph, không cần người dùng gõ tên tool, nhưng
không bị ép dùng graph khi graph chưa index hoặc coverage không đủ.

#### Việc cần làm

- [ ] **P3.T01** — Dùng installer hiện tại để cấu hình MCP cho Codex.
- [ ] **P3.T02** — Đưa instructions ngắn, ổn định vào plugin/skill:
   - dùng graph cho orientation/flow/impact;
   - query graph trước khi grep/read diện rộng;
   - đọc source trực tiếp khi staleness hoặc evidence thiếu;
   - không mô tả `unknown` như fact.
- [ ] **P3.T03** — Bật auto-load cho tool nếu host hỗ trợ, tránh agent không thấy tool ở
   prompt đầu tiên.
- [ ] **P3.T04** — Đảm bảo subagent/non-MCP path có CLI fallback:
   `codegraph explore`, `codegraph context`, `codegraph affected`.
- [ ] **P3.T05** — Giữ input/output schema JSON ổn định để Codex không phải parse prose.
- [ ] **P3.T06** — Ghi session markers/handoff chỉ chứa project/revision/query/IDs, không dump
   toàn bộ source vào lịch sử.
- [ ] **P3.T07** — Xây Codex-native extension theo contract, feature flag và fallback MCP/CLI;
   không để native path trở thành điểm duy nhất có thể vận hành.

#### Codex-native extension track

Nếu cần tích hợp sâu vào thread context, nghiên cứu các vùng hiện có trong
`codex/codex-rs`:

- `ext/mcp` cho server/plugin contribution;
- `ext/skills` cho skill lifecycle;
- `ext/history-notes` cho persistent thread context;
- `file-watcher` cho invalidation;
- core context builder/compaction nếu cần budget-aware injection.

Không inject một graph dump vào system prompt. Nếu có native contributor, nó chỉ
đưa một compact orientation card và link/reference tới MCP query.

#### Acceptance tests

- [ ] Fresh Codex session thấy và gọi được `codegraph_explore`.
- [ ] Query “how does X reach Y?” không cần người dùng chỉ file trước.
- [ ] MCP unavailable/index absent có fallback rõ ràng, không làm session chết.
- [ ] Hook không cấp thêm quyền filesystem ngoài project root.
- [ ] Subagent nhận được instructions/fallback nhưng không tự động dump graph lớn.

### Phase 4 — Context compiler và token budget

#### Mục tiêu

Tối ưu đúng mục tiêu ban đầu: ít file đọc, nhưng không hy sinh khả năng xác minh.

#### Checklist thực thi — W4

- [ ] **P4.T01** — Implement query planner và graph expansion có depth/node/edge
  caps; trả unknown khi không tìm được đường có evidence.
- [ ] **P4.T02** — Implement ranking, outline và source-range selection cho từng
  context mode; fixture file trên 2.000 dòng không bị dump toàn body.
- [ ] **P4.T03** — Enforce budget trên envelope đã serialize, gồm metadata và
  truncation markers; khai báo riêng byte/character/token limits và tokenizer.
- [ ] **P4.T04** — Nối freshness/snapshot verifier; benchmark cùng query/model
  giữa không graph, fast graph và full retrieval, có quality regression gate.
- [ ] **P4.T05** — Học context-mode: index raw command/browser/test artifacts
  theo section/line với provenance; batch query và intent retrieval trả đúng
  ranges dưới final byte/token cap. Test Unicode, stderr/failure retention,
  zero-result fallback, cross-project/session isolation và prompt injection;
  RTK chỉ là projection, không làm mất artifact gốc.
- [ ] **P4.T06** — Spike `RipwireContextScout` qua CLI: bind output ranked vào
  ProjectRef/GraphVersion/evidence, giữ ambiguity/unresolved/refusal/cap như
  coverage gaps, và so output bytes/correctness với CodeGraph + FTS/source
  baseline. Scout chỉ transport candidate metadata/range; Harness re-capture
  evidence và tự compose bounded range, không forward default full body,
  docs/notes/quality score như fact. `partition`/`plan-lanes` chỉ advisory cho
  task brief, không được điều khiển swarm authority: test phải chứng minh
  `partition`/execution/model hint/collision/landing order không đổi WorkerTree,
  lease, write scope, quota, approval hay fan-in nếu Harness admission không
  accept riêng. Parse output như hostile tool data (schema/version, max
  stdin/stdout/time, escaping/prompt-injection fixture, digest-only resume), và
  không bật MCP/daemon nếu chưa có schema parity, version/license,
  corrupt-cache/dirty-snapshot và fallback contract tests.

#### Retrieval pipeline

```text
1. Parse intent/query
2. Identify anchors: symbol, route, file, module, domain term
3. Resolve exact nodes with FTS/name segments
4. Expand only allowed edge kinds and depth
5. Rank by path relevance, provenance, recency, boundary, tests
6. Collapse repetitive nodes into summaries
7. Select line ranges, not whole files
8. Attach evidence/coverage/unknowns
9. Enforce output budget
10. Return structured JSON + concise Markdown
```

#### Ranking policy

Ưu tiên:

1. exact symbol/route named by user;
2. entry point and target path;
3. resolved calls/dispatch edges;
4. public interfaces and implementations;
5. tests covering selected nodes;
6. config/IaC that changes the path;
7. recent changed files;
8. hubs only when relevant to blast radius;
9. summaries/docs;
10. vector similarity as tie-breaker, không phải authority.

Loại hoặc giảm hạng:

- generated/vendor/build files;
- unrelated test fixtures;
- duplicate reexports;
- long containers khi chỉ cần members;
- stale/unresolved edges nếu không liên quan trực tiếp.

#### Context modes

- `compact`: project map + 5–12 nodes + unknowns;
- `normal`: map + subgraph + selected signatures/line ranges;
- `deep`: thêm Joern slices, data-flow và config evidence;
- `diagram`: context compiler trả Archify IR, không trả source body mặc định;
- `patch`: target symbols + callers + tests + write/read paths.

#### Acceptance metrics

Trên cùng câu hỏi và cùng model:

- [ ] giảm file reads so với baseline;
- [ ] giảm tool calls dùng cho discovery;
- [ ] context phải giữ dưới budget;
- [ ] route answer có evidence đúng line/commit;
- [ ] không tăng hallucinated edge rate;
- [ ] unknown coverage được hiển thị thay vì bị che;
- [ ] terminality không tệ hơn baseline ở cùng budget, và mọi cut có
  `shown/total/capped/next` hoặc coverage gap có lý do.

### Phase 5 — Adapter cho architecture/config/API/deployment

#### Mục tiêu

Vượt giới hạn của code-only graph để tiến gần system architecture thật.

#### Checklist thực thi — W9

- [x] **W9 partial — Compose declared-fact adapter:** crate Rust `graph-system`
  nhận full-file SourceSlice đã xác minh; tạo Service/Volume/Network và cạnh
  Mounts/DependsOn/AttachedTo kèm citation/hash. Named mount ngắn/dài được hỗ trợ;
  interpolation, tham chiếu chưa biết và option chưa mô hình hóa giữ unknown,
  không suy runtime/traffic. Parser chặn YAML lỗi/trùng key/anchor/tag/multi-doc,
  giới hạn 1 MiB/64 tầng/100.000 events; citation CR/CRLF đọc lại đúng nguồn.
  Parent kiểm 18 test adapter; foundation 301 Rust pass/3 ignored và 14 Node
  pass; Clippy library adapter với `-D warnings` và fmt pass.
  Đây chưa phải tích hợp graph/query/diagram hay hoàn thành P5.T01–P5.T04.
  [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w9-compose-source-review-2026-09-12.md).
- [x] **W9 partial — Compose protocol/CLI:** `analyze-compose ID TASK_SPEC ROOT`
  xác minh citation full-file theo snapshot rồi trả JSON candidate có phiên bản,
  không trả YAML thô hay tự ghi derived evidence. Nguồn tối đa 1 MiB; output mặc
  định 256 KiB, tính cả newline, lỗi budget không in JSON dở dang. Sáu black-box
  tests kiểm Orders (4 nodes/2 mounts), exact byte limits, stale/sai snapshot,
  YAML lỗi/partial citation, symlink escape và secret values. Foundation đạt
  308 Rust pass/3 ignored, 15 Node pass; fmt pass. Clippy strict toàn protocol
  vẫn fail (61 diagnostics hiện có), không đánh dấu lint gate toàn workspace đạt.
  [CLI/protocol source và verification receipt](/home/minh/projects/project-graph-agent/reports/w9-compose-cli-source-review-2026-09-12.md).
- [x] **W9 partial — validated deployment aggregate:** domain `DeploymentGraph`
  bất biến kiểm full-source scope/hash/run/range, citation ID, node/edge trùng,
  endpoint đúng loại và mount metadata. Giữ adapter/version cùng graph rỗng cho
  replacement; chưa phải quyền publication hay bằng chứng runtime. Adapter
  `analyze_compose_graph` và CLI đã đi qua boundary này. Parent kiểm 16 domain,
  20 system và 6 CLI tests; foundation 326 Rust pass/3 ignored + 15 Node pass,
  fmt và Clippy scoped system library pass. Không thêm migration hoặc SQL.
  [Domain/source/verification receipt](/home/minh/projects/project-graph-agent/reports/w9-deployment-domain-source-review-2026-09-12.md).
- [x] **W9 partial — relational deployment persistence:** refinery V14 thêm
  header/node/edge/unknown rows; port `DeploymentRepository` thay thế/invalidate
  nguyên tử theo full snapshot + source path + adapter với expected generation.
  Adapter version không đổi owner; empty success khác tombstone; citations cũ
  bất biến. Read transaction tái dựng validated aggregate, kiểm count/ordinal,
  citation scope và endpoint; không coi dữ liệu lịch sử là freshness/runtime proof.
  Luna viết 7 tests, parent bổ sung thành 8 và thêm 3 corruption/rollback tests;
  V13 upgrade giữ history/data, hai writer chỉ một thắng, xóa rows cũ không xóa
  foreign owner. Foundation 338 Rust pass/3 ignored + 15 Node pass; fmt pass.
  Chưa chứng minh arbitrary coherent DB tampering hoặc live publication.
  [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w9-deployment-persistence-source-review-2026-09-12.md).
- [x] **W9 partial — CLI publish/query/invalidate:** `publish-compose` xác minh
  full-file rồi ghi bằng expected-generation CAS; `deployment` chỉ đọc historical
  snapshot; `invalidate-compose` retract owner ngay cả khi source đã xóa.
  Receipt budget kiểm trước mutation; lỗi stdout sau commit không rollback DB,
  phải query generation trước retry. Luna viết 9 black-box tests, parent bổ sung
  exact node/edge source lines và một Linux transport test; giữ nguyên candidate
  analysis. Đã sửa regression thông báo input 8 MiB, mở test cho ba lệnh mới.
  Foundation 348 Rust pass/3 ignored + 16 Node pass; fmt pass. Thêm dependency
  nội bộ CLI→domain có architecture test, không thêm external crate/migration.
  [CLI source và verification receipt](/home/minh/projects/project-graph-agent/reports/w9-deployment-cli-source-review-2026-09-12.md).
- [x] **W9 partial — source capture và single-owner reindex/watch:** tạo citation
  mới từ một buffer, ID scoped/hash/run có length-prefix và golden độc lập.
  `reindex-compose` capture → parse → kiểm hash lại → CAS publish/invalidate;
  graph/tombstone không đổi không tăng generation. `watch-compose` polling tuần
  tự một owner, lỗi source cho phép recovery, lỗi CAS/DB/output dừng. Không phải
  snapshot nguyên tử filesystem+DB hoặc whole-worktree verification.
  Hai worker Luna viết 8 capture + 8 CLI tests; parent thêm golden (9 capture),
  test thay source giữa capture/verify và mở transport/input-cap tests.
  Foundation 366 Rust pass/3 ignored + 16 Node pass; fmt pass, không thêm
  dependency/migration. [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w9-compose-reindex-source-review-2026-09-12.md).
- [x] **W9/W11 partial — bounded deployment context:** `deployment-context`
  duyệt graph đã lưu theo seed ID, incoming/outgoing/both, giới hạn depth/node/edge
  và JSON bytes; giữ chiều cạnh, không dangling edge, deduplicate citation.
  Không đọc source body; trả historical/untrusted, omitted counts và unknown count
  toàn owner. Chưa phải context router đa provider hoặc Archify IR; Store vẫn
  load toàn owner trước traversal, nên output cap không phải DB allocation cap.
  Luna viết 8 traversal tests; parent thêm 2 CLI tests và input-limit coverage.
  Fixture hơn 4000 dòng vẫn query được khi xóa source fixture; đây không phải
  benchmark tiết kiệm token tổng quát. Foundation 376 Rust pass/3 ignored +
  16 Node pass; fmt pass, không dependency/migration mới.
  [Source và verification receipt](/home/minh/projects/project-graph-agent/reports/w9-deployment-context-source-review-2026-09-12.md).
- [ ] **W9 Compose integration còn lại:** nối reindex vào watcher/invalidation
  multi-owner của graph authority, discovery và snapshot/config reconciliation;
  nối build/module/API/SQL và context query,
  rồi kiểm compiler/diagram bằng evidence thực tế. Không coi DTO candidate là
  quyền ghi graph hoặc kết quả đã chứng minh deployment đang chạy.

- [ ] **P5.T01** — Chuẩn hóa adapter contract và manifest/package/build mapping;
  mỗi ecosystem trong phạm vi có positive/negative fixture.
- [ ] **P5.T02** — Nối API/message contract tới handler/publisher/consumer bằng
  source evidence; tên tính động giữ unresolved.
- [ ] **P5.T03** — Nối IaC/deployment tới service/module qua declared references;
  test resource đổi tên, config đổi và invalidation.
- [ ] **P5.T04** — Thêm docs/ADR/ownership overlay, giữ riêng documented claims;
  chạy fixture cross-service từ request qua persistence/event/deployment.

#### Adapter ưu tiên

#### 5.1. Manifest và package/build

Đọc và tạo edges cho:

- `package.json`/workspaces/exports/imports;
- `go.mod`, `cargo metadata`, `pyproject`, `pom.xml`, `build.gradle`;
- `build.sbt`, Bazel/MODULE files;
- lockfiles chỉ làm dependency evidence, không index vendor code mặc định.

`codegraph` đã có resolution cho nhiều ecosystem; mở rộng bằng adapter/test
fixture thay vì tạo resolver song song.

#### 5.2. API và message contracts

Parse:

- OpenAPI/Swagger;
- GraphQL schema;
- protobuf/JSON schema;
- Kafka/RabbitMQ/NATS topic declarations;
- framework route definitions.

Nối endpoint/event tới handler/publisher/consumer khi có reference tĩnh; dynamic
mapping phải `partial`.

#### 5.3. IaC/deployment

Ưu tiên:

- Terraform/OpenTofu;
- Kubernetes YAML/Helm;
- Dockerfile/Compose;
- serverless/CDK/CloudFormation khi có fixture thực tế;
- CI workflow.

`codegraph` đã có Terraform/OpenTofu extraction; phần còn thiếu nên vào adapter
layer với provenance `iac_declared`, không làm CPG schema phải chứa mọi provider
property.

#### 5.4. Docs/ADR/ownership

Index markdown/ADR/CODEOWNERS thành semantic memory. Chỉ tạo domain/ownership
edge nếu có heading/metadata rõ; LLM không được biến câu văn suy đoán thành
authoritative code edge.

#### Acceptance tests

- [ ] Một fixture Terraform + app code tạo được edge resource → module/service.
- [ ] Một OpenAPI route map được tới handler với line evidence.
- [ ] Một event publisher/consumer map được khi tên/topic là literal.
- [ ] Dynamic/computed reference được báo unresolved.
- [ ] Xóa/rebuild adapter read model không làm mất code graph facts.

### Phase 6 — Joern/CPG và T3MP3ST deep-analysis adapter

> **Dependency gate:** trước khi bật bất kỳ deep job nào phải hoàn thành
> execution policy của Phase 10/OpenSandbox (hoặc local trusted runner với
> cùng contract), scope approval, artifact redaction, timeout/cancellation và
> audit receipt. Số phase mô tả workstream; thứ tự runtime không được chạy
> Joern/T3MP3ST trước sandbox gate.

#### Mục tiêu

Thêm chiều sâu khi fast graph không đủ, không làm mọi query chậm và nặng.
Joern cung cấp static CPG/data-flow; T3MP3ST cung cấp một workflow red-team có
kiểm soát để khám phá và kiểm chứng các flow mà static resolver không chắc chắn.

#### Trigger chạy Joern

Gọi Joern khi:

- user chọn `view=dataflow` hoặc `depth=deep`;
- query source/sink, taint, security finding;
- `codegraph` gặp dynamic boundary cần resolve;
- coverage của static fast path dưới threshold;
- cần đối chiếu call path phức tạp/interprocedural.

Không gọi Joern cho:

- orientation ban đầu;
- symbol lookup;
- import/dependency đơn giản;
- file watcher mỗi lần save;
- các query không cần data-flow.

#### Thiết kế adapter

- [ ] **P6.T01** — `joern` chạy sidecar/subprocess với project-specific cache.
- [ ] **P6.T02** — Adapter chuyển output CPG/queries thành normalized graph facts.
- [ ] **P6.T03** — Giữ `joern_version`, frontend, query id, revision và execution receipt.
- [ ] **P6.T04** — Import edge vào read model hoặc lưu result theo `analysis_run_id`.
- [ ] **P6.T05** — Không cho Joern ghi đè edge nhanh; nếu conflict, lưu cả hai provenance.
- [ ] **P6.T06** — Map CPG node tới source path/line/symbol ID của `codegraph` khi có thể.
- [ ] **P6.T07** — Timeout, memory cap và cancellation bắt buộc.
- [ ] **P6.T08** — Nếu Joern không chạy được, trả fast graph + coverage gap, không fail cả MCP.

#### `codepropertygraph` sử dụng thế nào

- Dùng schema/protobuf để trao đổi deep-analysis result.
- Chỉ đề xuất extension schema khi cần node/edge đã ổn định qua nhiều adapter.
- Không fork CPG cho mỗi loại config/runtime entity.
- Upstream đã có `Finding`, key/value và evidence nodes trong `schema/Finding.scala`;
  tái sử dụng trước khi đề xuất extension. System graph và analysis ledger nằm ngoài CPG.

#### Acceptance tests

- [ ] Cùng một fixture, fast graph và Joern adapter cho cùng symbol identity.
- [ ] Data-flow query có path/evidence hoặc explicit `no_path_found`.
- [ ] Dynamic dispatch được ghi là partial nếu frontend không resolve.
- [ ] Timeout không làm daemon/watcher/MCP chết.
- [ ] Joern cache invalidates theo source revision và query version.

#### T3MP3ST red-team analysis loop

T3MP3ST không được coi là một parser khác ghi thẳng vào graph. Nó là một
consumer của graph và producer của candidate evidence:

```text
CodeGraph/Joern phát hiện boundary chưa rõ
        ↓
Harness Scheduler tạo scoped analysis task
        ↓
OpenDev chạy worker commands/agent steps
        ↓
T3MP3ST kiểm tra flow trong target được cho phép
        ↓
receipt + finding + path + artifact
        ↓
verifier đối chiếu source/revision/test
        ↓
EvidenceEvent được commit vào graph
```

Phạm vi v1 của adapter:

- [ ] **P6.T09** — Nhận project/revision và allowlisted path/service/entry point.
- [ ] **P6.T10** — Nhận các candidate flow từ `codegraph`/Joern thay vì tự quét vô hạn.
- [ ] **P6.T11** — Chạy source-level hoặc local test analysis trước; dynamic/network action
   phải có explicit approval và OpenSandbox policy.
- [ ] **P6.T12** — Trả `analysis_run`, `candidate_path`, `finding`, `observed_edge` và
   `unknowns`, không trả một graph đã được coi là đúng.
- [ ] **P6.T13** — Cho verifier kiểm tra line/source hash, test receipt, scope và tool version.
- [ ] **P6.T14** — Chỉ edge có verifier status phù hợp mới được nâng từ `candidate` thành
   `runtime_observed` hoặc `validated_static`.
- [ ] **P6.T15** — Ghi cả negative result/coverage gap để agent biết flow nào chưa kiểm tra.

Không cho phép:

- tự mở rộng target/network scope;
- chạy exploit hoặc destructive action mặc định;
- gửi source/evidence ra ngoài local policy;
- biến finding của model thành graph fact không có receipt;
- để T3MP3ST bypass authorization, sandbox, timeout hoặc audit log.

Do T3MP3ST là AGPL-3.0, adapter v1 phải gọi nó như executable/service tách
process hoặc tái triển khai contract độc lập; không link AGPL library vào core
binary nếu chưa có quyết định license.

#### Acceptance tests cho red-team loop

- [ ] Scope rỗng hoặc path ngoài allowlist bị từ chối trước khi worker chạy.
- [ ] Cùng `job_id` retry không tạo duplicate evidence/edge.
- [ ] Worker bị kill/timeout vẫn tạo receipt trạng thái `incomplete`, không tạo fact.
- [ ] Finding không có source/revision evidence chỉ hiện ở candidate report.
- [ ] Một path được verifier chấp nhận xuất hiện trong `runtime`/`deep-flow` view
  với provenance đúng.
- [ ] Audit log thể hiện ai yêu cầu, worker nào chạy, command nào đã chạy và
  artifact nào được dùng.

### Phase 7 — Archify integration

#### Mục tiêu

Vẽ diagram từ facts đã truy xuất, không dùng renderer để bù cho thiếu hiểu biết.

#### Checklist thực thi — W11

- [x] **W11 partial — declared deployment architecture projection:** CLI export
  bundle gồm typed IR và context/citations, snapshot-local component/edge bindings;
  max 12 nodes/24 edges, byte cap, dashed declared edges, coverage/unknown cards.
  Unicode labels giữ tên đầy đủ trong manifest; không collapse hoặc đoán traffic.
  Actual Archify validate/deliver + last-good failure test đạt trên Orders;
  visual-check đạt 4 desktop viewports/4 light-dark captures, parent đã xem ảnh.
  [HTML fixture](/home/minh/projects/project-graph-agent/.harness/w11-deployment-diagram-2026-09-12/architecture.html),
  [source/test receipt](/home/minh/projects/project-graph-agent/reports/w11-deployment-diagram-source-review-2026-09-12.md).
  Chưa có verified Git navigation, stable delta IDs, graph-aware auto-layout,
  cross-provider collapse hay các compiler view còn lại; không tích P7.T01–03.

- [ ] **P7.T01** — Implement graph projection → typed Archify IR; mỗi collapse
  rule giữ input assertion IDs, legend phân biệt observed/declared/unknown.
- [ ] **P7.T02** — Validate/render năm loại view trong bảng mapping; giữ last-good
  artifact khi fail, gắn evidence manifest và revision vào output.
- [ ] **P7.T03** — Implement evidence navigation và diagram delta; kiểm schema,
  source linkage và visual readability trên fixture trước acceptance.

#### Pipeline

```text
project graph query
  ↓
diagram context compiler
  ↓
Archify typed JSON IR
  ↓
schema validation
  ↓
deliver/visual-check
  ↓
HTML/SVG/PNG/WebM + evidence manifest
```

#### Mapping

| Graph view | Archify type |
|---|---|
| module/service/resource map | `architecture` |
| CI/CD/agent/tool chain | `workflow` |
| request/call/message path | `sequence` |
| source → transform → store/sink | `dataflow` |
| retry/state machine | `lifecycle` |

#### Quy tắc truthfulness

- Mỗi diagram edge phải trỏ về một graph edge/evidence ID.
- Edge không có evidence không được vẽ đậm như đường chắc chắn.
- Tối đa 8–12 primary nodes cho overview; phần chi tiết để trong cards/links.
- `unknown`/`partial` phải được thể hiện trong legend/card.
- Dùng commit-pinned source evidence khi người dùng yêu cầu traceable diagram.
- Archify validate/deliver là gate; không gọi diagram thành “đã đúng” nếu chỉ
  render thành công.

#### Acceptance tests

- [ ] `architecture_context` tạo Archify architecture JSON validate được.
- [ ] `flow` tạo sequence/dataflow có route/edge IDs tương ứng.
- [ ] Diagram delta giữa hai revision chỉ chứa added/removed/changed facts thật.
- [ ] Click evidence mở đúng file/line/commit.
- [ ] Artifact lỗi không ghi đè last-good artifact.

### Phase 8 — Zvec và semantic memory

#### Mục tiêu

Tìm docs/domain/task history khi keyword/graph anchor không đủ, nhưng vẫn giữ
graph là authority cho quan hệ kỹ thuật.

#### Dữ liệu đưa vào Zvec

- file/module summaries;
- doc sections/ADRs;
- API descriptions;
- generated context packs;
- accepted/rejected architecture decisions;
- task handoffs và test-failure explanations;
- embeddings của code slice chỉ khi có source hash/revision.

#### Dữ liệu không coi là authoritative

- embedding nearest-neighbor tự nó không tạo edge;
- summary cũ không được dùng nếu source hash đã thay đổi;
- memory từ repo khác không được trộn nếu thiếu project namespace;
- output của agent chưa được verify không được lưu thành fact.

#### Strategy

1. Graph exact search.
2. FTS/identifier search.
3. Semantic retrieval qua Zvec.
4. Graph expansion quanh các candidate.
5. Evidence/reranking.
6. Context budget.

Nếu Zvec integration tăng quá nhiều dependency hoặc build complexity, giữ FTS5
và SQLite/FTS5; Zvec là read model production có thể rebuild hoặc thay thế
theo cùng interface.


#### Checklist embedding và retrieval

- [ ] **P8.T01** — Tạo `EmbeddingProvider`, `VectorIndex` ports và DTO có namespace/model metadata.
- [ ] **P8.T02** — Chọn model qua tập query code/docs tiếng Việt/Anh; đo recall/nDCG, latency,
  memory và token savings so với graph + FTS.
- [ ] **P8.T03** — Provision model có version/hash và giới hạn tài nguyên; test chạy offline
  sau provision; thiếu model có trạng thái degraded rõ.
- [ ] **P8.T04** — Chunk theo symbol/section, giữ breadcrumbs và evidence ranges; không cắt
  chunk chỉ theo số ký tự làm mất boundary quan trọng.
- [ ] **P8.T05** — Batch inference, cache theo hash, bounded queue và incremental invalidation.
- [ ] **P8.T06** — Zvec create/open/upsert/delete/query hoạt động qua Rust adapter; giữ
  native engine ngoài domain và kiểm lifecycle/thread/process ownership.
- [ ] **P8.T07** — Outbox projectors đồng bộ vector index, watermark/GraphVersion và rebuild;
  crash giữa ledger commit/index update phục hồi được.
- [ ] **P8.T08** — Query/document embeddings cùng model revision/dimensions/preprocessing.
- [ ] **P8.T09** — Test cross-project isolation, stale suppression, delete/rename và model change.
- [ ] **P8.T10** — Benchmark hybrid retrieval + graph expansion + reranker; khi vector backend
  lỗi vẫn phục vụ graph/FTS với coverage đúng thực tế.
- [ ] **P8.T11** — Spike SQLite-vector sau native/license review và benchmark
  cùng Zvec và Ruflo handwritten HNSW trên workload 10k/100k/1M vectors nếu tài nguyên cho phép (thiếu tài
  nguyên ghi rõ gap, không ngoại suy). Dùng exact oracle, query corpus Việt/Anh,
  snapshot/model filters, concurrent reads/updates và recovery; chọn backend
  bằng ADR có receipts, không triển khai hai engine mandatory.
- [ ] **P8.T12** — Đo riêng Ruflo `v3/@claude-flow/memory/src/hnsw-index.ts`,
  native/WASM bridge và CLI fallback nếu đưa vào adapter. Pin backend thực chạy,
  source/compiler hash, seed/dataset/query hashes, dimensions/metric/k/M/ef,
  build time, p50/p95/p99, recall so exact oracle và process RSS (khác estimate).
  Benchmark filter selectivity 100%/10%/1%, update/delete/reopen và concurrent
  queries; không dùng latency unfiltered thay cho scoped retrieval acceptance.
  Ba engine nhận cùng embedding corpus và query set trước khi xếp hạng.

### Phase 9 — Native single-account swarm và graph improvement

Dependency: schema/job store, execution policy, graph snapshot và Codex
native-subagent adapter phải có trước launch. UI fleet không cần chờ deep analyzer.

#### Checklist thực thi theo package

- [ ] **P9.T01** — **W3:** Implement app-server handshake, account/model capability
  preflight, native spawn lifecycle và parent/child permission inheritance; mock trước live opt-in.
- [ ] **P9.T02** — **W3:** Implement quota admission, serialized auth refresh,
  checkpoint và native tree reconciliation; không đổi account giữa response stream.
- [ ] **P9.T03** — **W5:** Implement scheduler DAG, lease/fencing, mailbox dedup,
  cancellation/retry và resource budgets; test worker chết và delivery lặp.
- [ ] **P9.T04** — **W5:** Implement worktree provisioning, symbol/file ownership,
  conflict detection, merge candidate checks và target-head CAS; giữ user edits.
- [ ] **P9.T05** — **W5:** Implement role-scoped ContextPack/handoff, compaction
  recovery và fan-in; worker không tự accept claim hay patch của mình.
- [ ] **P9.T06** — **W7:** Implement browser/search gateway, BrowserEvidence,
  scoped extraction, citations và missing-provider/unsupported-page fallback.
- [ ] **P9.T07** — **W12:** Implement coverage scout → candidate resolver rule →
  isolated fixture/patch → independent critic → regression → versioned reindex.
- [ ] **P9.T08** — **W12:** Benchmark single-agent với DAG/fan-out trên cùng task;
  đo graph quality, tổng tokens, retry, latency, quota và marginal worker value.
  Chỉ tăng fan-out khi giữ chất lượng và đạt budget, không coi nhiều agent là tốt hơn.
- [ ] **P9.T09** — **W3/W5/W12:** Học LLMRouter: thu routing dataset từ receipts,
  replay/held-out evaluation và shadow-mode policy trước live routing. Test
  role/tool eligibility, shared quota changes, cold-start/OOD,
  policy rollback và reviewer independence; giữ deterministic fallback.
- [ ] **P9.T10** — **W5/W8:** Học context-mode: compact resume bằng IDs/references
  và bounded retrieval; test 10 agents tìm kiếm đồng thời không dùng chung
  counter nhầm, một actor flood không làm đói actor khác, tổng fleet budget
  vẫn được enforce và ID giả không tạo thêm quota.

#### NativeSessionBinding và bounded autonomous delegation

Đây là **schema mục tiêu của harness**, không phải config Codex copy-paste:

```yaml
fleet:
  runtime: codex-native-subagents
  account_policy: inherit-root-only
  model_policy:
    root: preserve-current
    worker_default: gpt-5.6-luna
    unavailable: report-and-root-handle-or-queue
    silent_fallback: false
  delegation:
    authorization: project-scoped
    mode: autonomous-bounded
    max_active_subagents: 4
    max_depth: 2
    max_total_spawns_per_task: 12
    duplicate_task_policy: reject
    overflow: queue
  context:
    default: fresh-scoped
    full_history_fork: explicit-only
  writes:
    policy: owned-worktree
    integration: independent-verification
  speculative_duplicates: false
```

Các cap là starting policy cần benchmark, không phải giới hạn subscription.

1. Probe native schemas/revision, effective model và quota của root; ghi opaque
   account identity, không đọc/export credential bytes.
2. Persist NativeSessionBinding(runtime_id, host_id, root_thread_id, account_alias,
   model, effort, capability_version). Child giữ identity root; mismatch fail
   closed/checkpoint, không rotate hay override model ngầm.
3. Planner xuất DelegationProposal gồm goal, dependencies, role, write/read scope,
   evidence cần trả, budget, lý do song song và terminal condition.
4. Admission reserve budget toàn cây, semantic task dedup, depth/total-spawn cap;
   bind TaskAttempt↔native_thread_id và worktree ownership. Child muốn chia tiếp
   phải đi qua cùng gate; không reset budget bằng root mới.
5. Lifecycle theo native capability phiên bản thực chạy. Agent/job/process/graph
   states tách nhau; completion text không đủ để accept code hoặc fact.
6. Reconnect reconcile tree/attempts/receipts trước resume/retry; không nhân đôi
   side effect khi chỉ mất thông báo. Unknown không tự thành dead/success.
7. Quota hết → checkpoint/queue. Model khác chỉ là tùy chọn do người dùng cấp
   trong tương lai, không default. Native concurrency không thay domain budgets.

fork_context không phải fork filesystem: execution host phải chứng minh
worktree isolation. Harness không giả vờ private core APIs là app-server RPC.

#### Autonomous swarm và deep graph improvement

- [ ] **P9.T11 — W3/W5:** Native adapter contract spawn/send/wait/close/resume
  hoặc capability-equivalent; parent/root ID, identity inheritance, default
  model, version negotiation, terminal status và no fabricated success.
  Probe experimental multiAgentMode=proactive ở thread/start hoặc turn/start
  của revision local; unsupported phải báo rõ, không bypass authorization.
  Test requested→effective mode→spawn sau compaction/resume; MAv2 source còn
  resolve mode từ config/catalog/effort. Wire roundtrip không đủ chứng minh mode
  thực thi; proactive instruction không thay deterministic admission.
  Migrate legacy account_lane sang native binding bằng versioned compatibility,
  không biến string metadata cũ thành credential authority.
- [ ] **P9.T12 — W5:** Autonomous admission: goal authority, dedup, global/depth/
  total-spawn caps, dependency readiness, monotonic child permissions; test
  recursive spawn storm, starvation và quota shared giữa mọi descendants.
- [ ] **P9.T13 — W4/W5:** ContextBroker orientation→module→symbol→evidence,
  freshness/provenance/negative evidence, bounded fallback; không gửi nguyên
  transcript hoặc file dài mặc định.
- [ ] **P9.T14 — W5/W12:** Independent critic và fan-in kiểm spans/hash/snapshot;
  unresolved disagreement tạo gap/task kiểm chứng, không majority-vote ra fact.
- [ ] **P9.T15 — W3/W5:** Reconcile tree, interrupted/closed children, process
  residues; crash giữa spawn và attempt binding không tạo duplicate worker.
- [ ] **P9.T16 — W0/W12:** Audit đủ 23 repo qua source flows/test evidence và
  adoption matrix; đề xuất có revision/license gate/fixture. Source-read research
  không được tick implementation acceptance.
  Source-flow reports/adoption matrix cho 22 repo nền đã có ở §3.7; Ripwire audit
  source-only ở §3.8. P0.T01 nay đã bổ sung live lock của 23 repo, nhưng license
  inventory interpretation, compatibility fixtures và các regression fixtures
  mới chưa hoàn tất nên task tổng vẫn để mở.

#### Tiết kiệm token theo chất lượng kết quả

- Coordinator nhận orientation, task DAG, unresolved decisions và review diff nhỏ.
  Subagent dùng Luna theo policy hiện hành, nhận task hẹp: inputs/outputs, contract, graph slice, tests, completion
  criteria và write scope; không fork toàn transcript coordinator.
- Shell/index/schema/test/status collection chạy deterministic. LLM không
  polling `ps`, quota, heartbeat hay tóm tắt lại log không đổi.
- Mailbox theo Orca: header + IDs trước, body theo yêu cầu; gộp progress events,
  chỉ đánh thức coordinator khi dependency ready, blocker hoặc review gate.
- Initial configurable budgets: worker ContextPack 6k tokens; handoff summary
  1k; coordinator planning/review pack 12k; tối đa 1 lần worker tự sửa cùng
  lỗi trước khi escalate. Budget không được cắt mất source cần để verify:
  trả truncation/gap và xin targeted slice hoặc nâng task budget.
- Context cache key gồm project/version/query/role/context epoch. Giữ prefix
  ổn định theo runtime/context epoch; đo cache thực tế, không mặc định fork giúp tiết kiệm.
- Không mặc định fan một prompt ra nhiều agent để chọn winner như demo Orca.
  Partition thành task bổ sung nhau; speculative alternatives chỉ bật khi
  task value và budget đủ, có cap và tiêu chí so sánh.
- Review thường dùng schema/compiler/tests trước. Independent critic review thay đổi
  cross-module, auth/concurrency/unsafe, conflict và claims chưa đủ bằng chứng.
  Researcher không tự phê duyệt chính kết luận của nó.
- Báo riêng actual tokens (cached/input/output/reasoning khi có), task latency,
  retry/escalation rate và quota chung của một account. Subscription quota không phải
  hóa đơn API; pricing API chỉ dùng cho API lane được cấu hình riêng.

Không lấy savings từ model nhỏ/account khác làm release target. Preflight
theo schema app-server local; native subagent tooling phải được probe riêng.
Quota subscription là shared constraint, không chuyển API USD thành quota.

#### Task dispatch, mailbox và worktree

`TaskSpec` gồm task/parent/dependencies, role, native session/thread binding, ProjectRef,
GraphVersion, scope, expected artifacts, checks, budgets, lease/fencing.
Identity tách rõ task, attempt, provider thread, terminal pane và execution host.

Lifecycle: planned → admitted → provisioning → running → submitted → verifying
→ integrated/rejected. Cancel/timeout/failure là terminal attempt state;
task có thể queue attempt mới. Provision failure giữ residual resources trong
receipt để reconcile, không tạo worktree thứ hai chỉ vì RPC timeout.

Mỗi implementer có worktree riêng; readonly workers dùng pinned snapshot.
GRIT adapter claim symbols và serialize integration. Claims giữ dependencies
của shared interface; hai function không overlap vẫn có thể conflict ngữ nghĩa.
Reviewer kiểm tra hợp đồng liên module, không chỉ text merge sạch.
Lockfiles/migrations/generated files và source không parse được dùng file lock.

Merge queue kiểm base/head/scope → tạo integration candidate → chạy checks trên
candidate hợp nhất → compare-and-swap target ref → re-index/invalidate memory.
Nếu target đổi, retest candidate mới. GraphWriter chỉ publish graph sau accepted
head; worker không commit thẳng graph facts. Dirty/untracked user files được
bảo toàn, cleanup chỉ xử lý worktree owned đã lưu patch/artifacts.

Fleet status theo bài học Orca: execution host là authority; client subscribe
snapshot + events có sequence/cursor. Connectivity là live/unverifiable/exited
tách khỏi task state. Restored log không chứng minh process đang sống.
Không retry write job ở máy local vì mất liên lạc với SSH worker.

#### Role catalog và feedback loop

| Role | Work | Native execution |
|---|---|---|
| planner/integrator | chia graph task, giải quyết dependency và review hợp nhất | root giữ model hiện tại; delegated child dùng Luna |
| navigator/researcher | anchors, scoped source/web evidence | native child, Luna |
| implementer/test-mapper | scoped patch, fixture, affected tests | native child, Luna, owned worktree |
| flow-tracer/dependency-resolver | trace code/API/IaC; nêu boundary gaps | native child, Luna |
| runtime-observer/security-redteam | orchestrate approved analyzer và thu receipts | native child, Luna; tool scope được cấp riêng |
| critic | độc lập kiểm source/evidence/candidate diff | native child, Luna, tách producer; root kiểm chứng cuối |

Graph optimization: coverage scout → candidate rule → isolated fixture/patch
→ adversarial positive/negative tests → regression → versioned analyzer release
→ re-index affected snapshots. T3MP3ST tham gia security track theo scope;
không tự giải quyết mọi dynamic boundary hoặc ghi model guesses thành facts.

#### Browser research và cập nhật knowledge graph

Lightpanda là browser worker qua MCP/CDP; dùng tool đã có thay vì tự viết browser.
Luồng: query → provider search/snippets → URL shortlist → semantic tree/scoped
markdown hoặc extract → cited ResearchBundle → source/version verification.

Search providers trong upstream cần cấu hình riêng; không giả định subscription
Codex trả phí provider. Fetch public URL trực tiếp vẫn dùng được khi search
provider chưa cấu hình; báo rõ phần discovery không khả dụng.

`BrowserEvidence`: query, requested/final URL, retrieved_at, page/content hash,
title, backend/version, selector/text span, bounded excerpt, truncation,
artifact ref và auth context alias nếu cần. Dedup theo URL/content và fetch
cache TTL; URL versioned docs ưu tiên hơn trang latest khi repo pin dependency.

Web docs chỉ tạo documented claims; quan hệ trong repo phải có source hoặc
runtime evidence của đúng revision. Browser observations của localhost gắn
build/commit/environment để làm runtime evidence. Không đưa cả page vào prompt
khi chỉ cần một section. Cache cookie/session tách task/project; tài khoản
browser không phải Codex account lane.

Lightpanda capability không tương đương Chromium pixel rendering. Khi web API,
layout/visual interaction không hỗ trợ, dùng backend Chromium/Playwright với
cùng BrowserEvidence schema; không gắn label visual validation cho text PNG.
Browser pool có page/process cap, timeout, cancel và sandbox. Thiết kế packaging
giữ AGPL license/notice; process boundary không tự miễn nghĩa vụ phân phối.

#### Acceptance tests của native swarm/browser

- [ ] Mock native tree cùng account: worker mới dùng Luna, root giữ model hiện tại;
   ghi effective model và parent/child IDs đúng, Luna unavailable không fallback ngầm;
   role override không đổi auth/sandbox ngầm, quota unknown có trạng thái rõ.
- [ ] Concurrent children chia global budget; recursive spawn không vượt cap,
   instruction injection không nới quyền, không duplicate jobs hoặc thread leaks.
- [ ] Worker timeout sau dispatch/side effect không nhân đôi task/command/merge.
- [ ] Mất SSH hoặc reconnect UI không đổi running thành success/dead vô căn cứ.
- [ ] Hai patch khác function cùng file, shared API change, dirty base và target
   head đổi giữa verify/merge đều có kiểm tra và receipt.
- [ ] Compaction khôi phục context refs; mailbox duplicate/out-of-order được dedup.
- [ ] Browser JS fixture: search adapter → scoped extract → URL/hash citation;
   malformed/truncated/unsupported pages có fallback hoặc explicit gap.
- [ ] Trang web/memory chứa prompt injection không đổi role/account/write scope.
- [ ] Missing search credential không làm deadlock browser fetch/local graph.
- [ ] Reviewer phân biệt documented edge với accepted source fact; failed test
    hoặc unverified path không được nhập graph như đã chứng minh.
- [ ] Headless và TUI đọc cùng event stream, account alias, quota và outcomes.
- [ ] Live opt-in smoke với một account đã đăng nhập: chứng minh native delegation,
    identity inheritance, events/reconnect/close; mocks không chứng minh entitlement.

### Phase 10 — Sandbox, runtime evidence, T3Code và terminal product

#### Checklist execution, runtime và terminal

- [ ] **P10.T05** — **W2:** Implement execution-host identity, pipe/PTY process
  supervisor, stdout/stderr artifacts, exit receipts và bounded output.
- [ ] **P10.T06** — **W2:** Implement timeout/cancel/process-tree teardown,
  sandbox/egress policy và restart reconciliation; remote disconnect là unverifiable.
- [ ] **P10.T07** — **W9:** Implement trace collector và source/service mapping
  theo build/environment; runtime observation không thay static topology.
- [ ] **P10.T08** — **W6:** Implement chat/fleet/terminal/evidence/approval views,
  JSONL headless và replay từ cùng EventStream; kiểm reconnect, backpressure
  và terminal responsiveness khi nhiều worker đang xuất log.

ID T01–T04 ở lộ trình terminal bên dưới được giữ nguyên; các task này bổ sung
execution/runtime gates, không thay thế hoặc tích hộ chúng.

#### OpenSandbox

Dùng khi cần:

- index repository không tin cậy;
- chạy build/test để xác minh architecture claims;
- chạy Joern với giới hạn CPU/RAM/time;
- parse/deploy IaC trong môi trường tách biệt;
- thu runtime traces từ fixture.

MCP graph server local vẫn phải hoạt động không có OpenSandbox. Sandbox là
execution boundary, không phải graph store.

#### Runtime evidence

Bổ sung collector/adapter cho OpenTelemetry hoặc trace format tương đương:

- map service/span name tới graph service/symbol khi có stable ID;
- lưu trace ID/time window/environment;
- không coi một trace đơn lẻ là complete topology;
- giữ static route và observed route riêng.

#### T3Code

Sau khi API ổn định, T3Code có thể cung cấp:

- graph/flow browser;
- context preview trước khi gửi cho agent;
- evidence/coverage viewer;
- session/working set;
- diagram preview và delta review;
- remote agent control.

T3Code là UI/control surface, không sở hữu graph semantics. Nó gọi HTTP/MCP
gateway và không truy cập SQLite trực tiếp.

#### Ghostty

Ghostty không đưa vào graph data plane, nhưng có giá trị thực tế cho terminal
surface của harness. `libghostty`/`libghostty-vt` có thể là reference hoặc
native backend cho CLI/TUI riêng, với fallback headless:

- học cách tách terminal parser/state khỏi UI;
- giữ read/write/render loop riêng để terminal không làm nghẽn graph query;
- tái sử dụng VT parser/state qua C ABI nếu license/build policy cho phép;
  PTY và process lifecycle do executor quản lý;
- học threading, SIMD/fast path, native platform integration và graceful
  degradation từ kiến trúc Ghostty;
- hiển thị live job stream, worker tree, graph route và evidence receipt trong
  một terminal session.

Lộ trình Ghostty:

- [ ] **P10.T01** — Xây protocol stdout/stderr/artifact trước để mọi client có cùng semantics.
- [ ] **P10.T02** — TUI chính subscribe job stream của Harness; dùng OpenDev/Orca làm reference.
- [ ] **P10.T03** — Native backend: đánh giá `libghostty-vt` cho parser/state; không copy cả
   windowing stack nếu chỉ cần terminal.
- [ ] **P10.T04** — Tích hợp T3Code/desktop chỉ sau khi protocol job/evidence đã ổn định.

#### Acceptance tests

- [ ] Terminal UI không thay đổi semantics của graph hoặc sở hữu state riêng.
- [ ] Không nuốt stderr/exit receipt; log bị compact vẫn truy cập được raw artifact.
- [ ] Tắt renderer vẫn chạy analysis headless, replay cùng outcomes.
- [ ] Timeout/cancel dọn đúng process/resources do task sở hữu; không đụng user jobs.
- [ ] Mất remote connection không kích hoạt duplicate write job ở host khác.

## 7. API và tool contract chi tiết

### 7.1. MCP tool chính

Tool hiện tại `codegraph_explore` giữ schema upstream. Input/Output dưới đây
là target mới `project_graph_context_v1`, không phải capability đã có.

#### Input

```json
{
  "query": "string, required",
  "projectPath": "absolute path, optional",
  "view": "auto|orientation|symbol|flow|impact|architecture|dataflow|deployment|runtime|change",
  "depth": "integer 0..8",
  "maxNodes": "integer",
  "maxEdges": "integer",
  "maxChars": "integer",
  "includeCode": "none|outline|targeted|full",
  "evidence": "optional|required",
  "freshness": "cached|catch-up|strict",
  "revision": "working-tree|HEAD|git-sha"
}
```

#### Output

```json
{
  "schema_version": "project-graph/context/v1",
  "status": "ok|partial|unavailable|stale",
  "project": {},
  "query": {},
  "nodes": [],
  "edges": [],
  "paths": [],
  "code_slices": [],
  "evidence": [],
  "coverage": {},
  "freshness": {},
  "unknowns": [],
  "next_queries": [],
  "markdown": "bounded human-readable rendering"
}
```

### 7.2. CLI equivalents

```bash
codegraph context --format json --no-code "understand order flow"
codegraph explore "how does POST /orders reach the database"
codegraph callers "OrderService.create" --json
codegraph callees "OrderController.create" --json
codegraph impact "OrderService.create" --file src/orders/service.ts --json
codegraph affected --stdin --json
codegraph status --json
```

CLI và MCP phải gọi cùng query library; không fork logic ranking/coverage.

### 7.3. HTTP API

HTTP chỉ cần sau khi UI/remote agent cần. Các endpoint tối thiểu:

```text
GET  /health
GET  /projects
GET  /context?project=&query=&view=
GET  /nodes/:id
GET  /flow?from=&to=&depth=
GET  /impact?node=
GET  /evidence/:id
GET  /coverage?project=
POST /diagrams/archify
```

`POST /diagrams/archify` phải trả IR và validation receipt; renderer có thể chạy
ở process/service riêng.

## 8. Testing và đánh giá

### 8.1. Unit tests

- node/edge identity;
- line range/hash validation;
- provenance and confidence;
- query parser/intent router;
- traversal depth/budget;
- ranking determinism;
- stale index detection;
- migration rollback/forward compatibility;
- JSON Schema validation;
- Archify mapping;
- Zvec document invalidation;
- Joern timeout/cancellation.

### 8.2. Integration tests

1. Source edit → watcher → incremental graph update.
2. Import rename → dependent edge update.
3. Route declaration → handler route node.
4. Terraform resource → service/config edge.
5. OpenAPI endpoint → source handler evidence.
6. Joern deep query → normalized path.
7. Codex MCP session → context envelope.
8. Context envelope → Archify JSON → delivered artifact.
9. Index in OpenSandbox → result export with revision/evidence.
10. T3Code/API client → graph query without direct DB access.

### 8.3. Golden fixtures

Tạo fixture tối thiểu cho:

- TypeScript/Node web app;
- Python/FastAPI;
- Go service + worker;
- Rust workspace;
- Java/Spring dynamic DI boundary;
- Terraform + Kubernetes;
- queue publisher/consumer;
- reflection/dynamic import unresolved;
- generated/vendor code;
- monorepo nhiều project/worktree.

Mỗi fixture có expected nodes/edges/evidence và expected unknowns.

### 8.4. Agent benchmark

Chạy 3 arms trên cùng model, cùng repository và cùng commit:

1. **Baseline:** built-in shell/read/search.
2. **Fast graph:** `codegraph` only.
3. **Full graph:** fast graph + adapters/Joern/Zvec as permitted.

Đo:

- correctness của answer/route;
- evidence precision/recall;
- số file reads;
- số tool calls;
- input/output tokens;
- latency p50/p95;
- cost nếu có;
- hallucinated edge rate;
- unknown disclosure rate;
- stale-answer rate;
- context resident footprint sau nhiều lượt.

Không dùng benchmark claim nếu control arm có thể gọi graph ngầm qua PATH.
Isolate CLI/path và ghi lại tool receipt.

### 8.5. Orchestration, red-team và terminal benchmark

Đánh giá thêm toàn bộ harness, không chỉ từng analyzer:

- router chọn fast/deep/red-team backend đúng intent và coverage;
- swarm không nhân đôi cùng một analysis job;
- OpenDev chạy N command song song mà không race writer hoặc lẫn worktree;
- cancellation/timeout giải phóng process, lease và sandbox;
- T3MP3ST chỉ chạy trong scope đã phê duyệt và mọi finding có receipt;
- candidate evidence không xuất hiện như authoritative edge trước verifier;
- memory sau compaction vẫn giữ project/revision/decision references;
- terminal UI và headless mode cho cùng stdout/stderr/exit/artifact receipt;
- graph query vẫn trả lời được khi Ruflo, OpenDev, T3MP3ST hoặc Ghostty tắt;
- một worker lỗi không làm mất kết quả của các worker độc lập khác.

Các metric bổ sung:

- scheduler overhead;
- parallel speedup và queue wait;
- duplicate-job rate;
- evidence acceptance/rejection rate;
- red-team coverage theo scoped entry point;
- sandbox escape/policy violation count (mục tiêu bằng 0);
- memory contamination/cross-project leakage (mục tiêu bằng 0);
- headless/UI receipt parity.

### 8.6. Benchmark native swarm cùng account và policy Luna

So sánh cùng tasks/base revision/tool access và cùng account: single-agent;
swarm root hiện tại + Luna workers; swarm đó + graph/RTK/browser cache. Thêm
baseline cùng model cho mọi agent để tách ảnh hưởng model khỏi graph.
Production tối đa 4 worker; fan-out 8 chỉ cho benchmark được cấp budget và
runtime cho phép, không tự tăng cap khi chạy task thường.
Ghi tổng tokens của tất cả turns/agents, cached tokens riêng, latency,
task success, semantic regressions, retries và số lần escalation. Không chỉ
đo token ở coordinator hay rút output để tạo savings giả.

Release target cấu hình ban đầu: graph-assisted native swarm giảm ≥30% tổng uncached input
so với cùng model chạy single-agent trên corpus đủ task song song; tỷ lệ task thành công
không giảm quá 5 điểm phần trăm; không regression ở các invariant account,
evidence và merge. Đây là mục tiêu cần đo, chưa là kết quả. Báo confidence
interval/repeats và cả task nhỏ bị overhead; scheduler chọn single-agent khi
partition không đem lợi ích. Quota account báo theo telemetry thực nhận,
không chuyển API USD estimates thành phần trăm quota Plus/Pro.

### 8.7. Acceptance target cho sản phẩm hoàn chỉnh

Trên fixture và ít nhất ba repository thực tế:

- orientation trả trong một MCP call hoặc hai call có chủ đích;
- flow query tìm được primary path mà không đọc toàn file;
- code slices có line/hash đúng revision;
- impact query liệt kê callers/tests với coverage;
- không có edge “chắc chắn” khi resolver báo unknown;
- graph cập nhật sau edit trong giới hạn watcher đã cấu hình;
- diagram validate được và mọi edge trace về evidence;
- baseline comparison chứng minh giảm discovery file reads.

## 9. Bảo mật, quyền và riêng tư

- Graph/artifacts lưu local theo mặc định; dùng hosted Codex gửi context đã chọn
  tới provider của lane. Remote model và search-provider egress hiển thị rõ.
- Validate mọi `projectPath`, evidence path và `file://` access trong project root.
- Không cho MCP query đọc arbitrary filesystem.
- Redact secrets/config values trước khi lưu summary, vector hoặc telemetry.
- Không index `node_modules`, vendor, build artifact, credential files mặc định.
- Runtime trace phải tách environment/tenant/project namespace.
- Joern/OpenSandbox process có timeout, memory/CPU cap và cancellation.
- Tool output bounded để chống prompt/context exhaustion.
- MCP config/hook phải dùng absolute executable path và cảnh báo repo untrusted.
- `T3MP3ST` và Lightpanda có AGPL obligations; kiểm tra license/NOTICE khi ship adapter.
- Bảo toàn license/NOTICE của Apache/MIT dependencies.
- Audit mọi plugin/skill vì agent có thể đọc instructions ngoài source.

## 10. Phần hợp nhất và lược bỏ

- Một scheduler Rust thay các mô tả Ruflo/OpenDev/Orca cùng sở hữu job.
- Một memory service: ICM adapter cho record, SQLite ledger cho evidence;
  Ruflo memory APIs là facade nếu cần, Zvec chỉ index có thể rebuild.
- Một terminal CLI/TUI; tái sử dụng Ghostty VT khi có lợi đo được. PTY process
  manager thuộc executor, không thuộc libghostty-vt.
- Một Codex app-server transport chính; embedded runtime là lựa chọn tương lai
  khi có bottleneck chứng minh, không là deliverable song song.
- Một versioned context schema; legacy codegraph tool giữ nguyên input.
- Một account admission policy theo task; bỏ global account switch theo lượt,
  blanket rotation và assumption gói Plus/Pro đủ để biết model availability.
- Một browser gateway dùng Lightpanda tools; không thêm browser automation stack
  của Ruflo/Orca vào core khi đã có capability tương đương.
- Bỏ danh sách rủi ro lặp với invariant/tests; giữ tests theo failure mode
  ngay trong phase chịu trách nhiệm.

## 11. Dependency DAG và work packages

Phase ở mục 6 là nhóm thiết kế; bảng này quyết định thứ tự triển khai.
Mỗi package giao cho worker phải có scope/worktree và test receipt.

| ID | Work package | Phụ thuộc | Gate |
|---|---|---|---|
| W0 | Repo/toolchain lock, fixtures, baseline shell/graph | — | reproducible baseline; tất cả integrations có owner/adapter mode |
| W1 | Rust protocol, task/event store, ProjectRef/assertions | W0 | schema roundtrip, crash/outbox/replay |
| W2 | Execution host, sandbox, pipe/PTY, cancellation | W1-C để phát triển; W1/W2 joint acceptance | process receipt và teardown; remote unverifiable |
| W3 | Codex app-server/native subagents, một account | W1,W2 | native lifecycle/identity + capability/shared quota |
| W4 | Fast graph, migrations, source slices/context compiler | W1 | precision/freshness/hash, legacy MCP compatibility |
| W5 | Fleet scheduler/mailbox/worktrees/GRIT merge queue | W2,W3,W4 | task deps, fencing, conflict/retry/recovery |
| W6 | Rust TUI/headless native tree + shared quota display | W3,W5 | event parity, reconnect, terminal performance |
| W7 | Lightpanda/search/research evidence gateway | W2,W4 | scoped extraction, citations, fallback |
| W8 | ICM records, memory invalidation; Zvec retrieval | W1,W4 | namespace, stale suppression, rebuild |
| W9 | Manifest/API/IaC/runtime adapters | W2,W4 | cross-boundary fixtures, unresolved preserved |
| W10 | Joern/CPG và scoped T3MP3ST | W2,W4,W5 | query/source mapping, timeout, finding verification |
| W11 | Archify diagram/compiler + evidence navigation | W4,W9 | all diagram edges link evidence; visual check |
| W12 | Swarm-driven graph improvement | W5,W7,W9,W10 | candidate → fixtures → critic → regression → reindex |
| W13 | Release/platform/compatibility/benchmark | W6–W12 | complete product DoD và published receipts |

Ánh xạ tới checklist thiết kế/thực thi (không phải bảng trạng thái thứ hai):

| Package | Checklist chi tiết |
|---|---|
| W0 | Phase 0; baseline blockers ở mục 0.3 |
| W1 | Phase 1; lifecycle/verifier/migration backlog ở mục 0.3 |
| W2 | P10.T05–T06 và execution acceptance Phase 10 |
| W3 | P9.T01–T02; Phase 3 cho tương thích Codex/MCP |
| W4 | Phase 2 và Phase 4; compatibility Phase 3 |
| W5 | P9.T03–T05 và fleet acceptance Phase 9 |
| W6 | P10.T08, terminal roadmap và event parity Phase 9–10 |
| W7 | P9.T06 và browser acceptance Phase 9 |
| W8 | Checklist embedding/retrieval Phase 8 và memory contracts |
| W9 | Phase 5 và P10.T07 |
| W10 | Phase 6; chỉ launch sau execution/scope gates |
| W11 | Phase 7 |
| W12 | P9.T07–T08 và graph-improvement acceptance |
| W13 | Checklist release dưới đây và toàn bộ mục 12 |

W4 có thể chạy song song W2/W3; W7/W8/W9 sau nền móng có scope độc lập.
Dependencies trong bảng là gate tích hợp/hoàn thành: có thể chuẩn bị contract,
fixture và code độc lập trước gate, nhưng không tuyên bố package đã hoàn tất.
Ưu tiên đóng W0 và trust/persistence của W1 trước khi mở rộng live fleet.

T3Code/Orca frontend adapters dùng protocol W6 nếu triển khai; không trì hoãn
sản phẩm terminal vì phải port toàn desktop/mobile UI.

### 11.0. Mốc phát triển W1-C/W1-I, không thay package acceptance

Dependency phát triển khác dependency nghiệm thu: W1 verifier cần execution
receipts do W2 tạo. Không chờ verifier end-to-end hoàn tất trước khi xây executor.
[Đối chiếu source và vòng phụ thuộc](/home/minh/projects/project-graph-agent/reports/w1-w2-dependency-review-2026-09-11.md).
Đây không thêm package và không giảm bất kỳ gate W0–W13 nào.

- [x] **W1-C (development contract gate):** task/policy/lease/fencing contracts,
  versioned command + execution receipt, binding snapshot/candidate/check policy,
  phân biệt exit/reap/stdout/stderr/cleanup; validation và ledger tests.
  **Đã đạt trong phạm vi development:** audit contract/ledger, registry/replay
  và đối chiếu receipt kèm tests; không dựa riêng parsing hoặc lease tests.
  [Requirement-to-evidence audit](/home/minh/projects/project-graph-agent/reports/w1-c-development-gate-audit-2026-09-11.md).
  Host/runtime evidence thuộc W2/W1-I; full W1/W2 vẫn chưa đạt.
- [x] **W2 fixture development (phạm vi trusted/Linux):** sau W1-C đã có
  `graph-execution` chạy fixture do harness sở hữu trong thư mục tạm, preflight
  exact argv/env/cwd/executable, giữ bounded stdout/stderr, cancel/timeout,
  reap/stream completion và cleanup state tách nhau. Composition tests đã
  publish output artifacts, ghi durable `ExecutionReceipt`, reopen và reverify
  bytes mà không nâng unknown cleanup thành execution authority; foundation
  2026-09-12 pass. Không account/native dispatch, untrusted repo job, merge hay
  graph write. Đây chưa phải host authentication, process-tree containment,
  crash reconciliation, target-head verification, W1-I hoặc full W2.
- [x] **W2 process-scope partial (trusted/Linux):** trusted fixture và RPC
  supervisor launch child trong process-group riêng bằng `CommandExt`, giữ
  identity của đúng `Child`, teardown TERM→KILL bằng `rustix` và test descendant
  giữ pipe được dừng trước khi fixture sleep tự kết thúc. Direct-child reap,
  stream EOF, group signal/state và cleanup authority vẫn tách nhau; không dùng
  `pgrep`, shell `kill` hay persisted PID. `ScopeCleanup` vẫn là
  `Unverifiable`: process-group biến mất chưa chứng minh descendant đã
  `setsid`, escape namespace/cgroup hay crash giữa các bước. Receipt/source gate:
  [W2 process-scope study](/home/minh/projects/project-graph-agent/reports/w2-process-scope-source-study-2026-09-12.md).
  `cargo test -p graph-execution --locked --offline` pass (2026-09-12); đây
  chưa phải full process-tree containment, sandbox, crash recovery, W1-I hoặc
  full W2.
- [x] **W2 sandbox/egress source-gate (2026-09-12):** đã đọc lại OpenSandbox
  `bwrap` argv/mount/network/env builder, capability probe, fail-closed
  lifecycle gate, session cleanup/retry tests và egress policy/nftables trước
  implementation. Quyết định: tách sandbox adapter khỏi graph store; private
  network + deny-by-default là mặc định; thiếu bwrap/capability trả
  `Unsupported`/`Unverifiable`, không fallback host runner và không nâng group
  gone thành containment proof. Receipt:
  [W2 sandbox/egress source study](/home/minh/projects/project-graph-agent/reports/w2-sandbox-egress-source-study-2026-09-12.md).
- [x] **W2 bounded sandbox adapter (trusted/Linux partial):** đã thêm
  `graph-execution::SandboxRuntime` với absolute bwrap discovery/probe,
  canonical declared-root/cwd/executable admission, `/mnt` bind, mask các
  host roots thông dụng, explicit environment và private-network request;
  allow-list domain bị fail-closed thành `UnverifiableEgress`. Opt-in fixture
  chạy thật chứng minh declared mount visibility, host temp masking và
  outside-root rejection trước spawn; focused 3 pass, execution/workspace
  regression pass. Đây chỉ là raw host execution result, không tạo authority.
  Chưa có production namespace/cgroup/seccomp/nftables egress enforcement,
  crash reconciliation, cross-platform proof hay full process-tree containment;
  không tick W2/full W1-I. Receipt:
  [W2 sandbox/egress source study](/home/minh/projects/project-graph-agent/reports/w2-sandbox-egress-source-study-2026-09-12.md).
- [x] **W2 sandboxed RPC composition (trusted/Linux partial):** thêm
  `TrustedRpcFixtureHost::new_sandboxed`/`with_sandbox`; sandbox plan được tạo
  trước register/claim, allow-list egress và capability/path lỗi đều
  fail-closed trước spawn. bwrap dùng exact absolute backend/argv nhưng vẫn
  gắn stdin/stdout/stderr vào cùng `ConnectionSetup` và
  `ConnectionSupervisor`, giữ process-group, timeout/cancel, bounded drain và
  direct-child reap. Integration chứng minh RPC handshake/request/response,
  `/mnt/cwd`, explicit environment, stored spawn observation và no unreaped
  child; allow-list không ghi ledger. Focused RPC **14/14**, sandbox **3/3**,
  locked workspace suite pass, architecture gate pass. PID observation là
  host-visible bwrap leader, không phải inner workload identity; raw result vẫn
  `ScopeCleanup::Unverifiable`. Chưa có OpenSandbox native lifecycle gate,
  cgroup/seccomp/nftables egress, host authentication, crash reconciliation,
  cross-platform hay full process-tree containment; không tick W2/full W1-I.
  Receipt:
  [W2 RPC sandbox integration source-gate](/home/minh/projects/project-graph-agent/reports/w2-rpc-sandbox-integration-source-review-2026-09-12.md).
- [ ] **W1-I (joint integration evidence):** verifier dùng receipts thật từ W2,
  kiểm source/candidate/policy/target, rồi atomic acceptance + outbox/replay.
  JSON claims hay policy satisfied không tạo verified authority.
  **Partial 2026-09-12:** `IntegrationDecision` bắt buộc exact set một receipt
  `Passed` cho từng required check, `TargetHeadVerifier` là host port tường minh,
  và V16 persist decision/target record. Store re-read task/submitted
  candidate/policy/plan/receipt trong một `BEGIN IMMEDIATE`, đổi
  `submitted→integrated`, ghi event và để trigger V2 ghi outbox cùng transaction.
  Tests cover exact/missing/duplicate/extra/failed checks, idempotent replay,
  reopen, two-connection single-winner/replay, cancellation-wins và injected
  outbox rollback. Chưa chứng minh end-to-end với receipt W2 thật qua
  authenticated production target head, crash recovery/power-loss hay host
  process-tree containment; không tick W1-I/full W1/W2.
- [ ] **Full W1/W2 acceptance:** vẫn cần toàn bộ schema, lifecycle, crash/recovery,
  sandbox/platform và verifier gates. Chỉ sau đó mới mở native W3 và fleet theo
  dependency; W0 baseline vẫn bắt buộc cho nghiệm thu đầy đủ.

### 11.1. Phân luồng triển khai song song

1. Đợt nền móng: một scope xử lý W0 baseline/fixtures, một scope hoàn thiện W1
   contracts/ledger/verifier. Chốt shared contract trước khi worker sửa adapter.
2. Sau W1-C: phát triển execution W2 bằng owned fixtures; W4 theo dependency
   contracts của nó. Chỉ sau joint acceptance W1/W2 mới mở native runtime W3;
   W5 chờ các gate W2/W3/W4, không launch fleet ghi code sớm.
3. Sau nền móng: browser W7, memory/embedding W8 và system adapters W9 có thể
   tách worker; mỗi nhánh có scope, receipts và reviewer riêng.
4. W6/W10/W11/W12 mở đúng dependency trong bảng. Integrator serialize thay
   đổi schema/shared API; analyzer read-only dùng snapshot chung, implementer
   dùng worktree riêng. Chưa có runtime swarm thì thực hiện bằng workflow hiện
   có, không ghi rằng fleet sản phẩm đã vận hành.

### 11.2. Checklist release — W13

- [ ] **W13.01** — Chốt platform/capability matrix; clean-machine build/install,
  CI fmt, strict Clippy, tests, dependency direction và protocol compatibility.
- [ ] **W13.02** — Kiểm upgrade/backup/restore, migration failures, crash recovery,
  graph/vector rebuild và rollback binary theo compatibility policy đã test.
- [ ] **W13.03** — Chạy E2E các chế độ chat/swarm/analyze/review/run/replay/serve;
  gồm restart, offline/degraded backend và remote unverifiable.
- [ ] **W13.04** — Chạy benchmark mục 8 trên fixture và ít nhất ba repo thật;
  chốt thresholds trước run, báo cả quality, unknowns, reads, tokens và latency.
- [ ] **W13.05** — Review threat model, secret/egress boundaries, dependencies,
  license/NOTICE; pin binaries/models/analyzers và tạo release manifest/SBOM.
- [ ] **W13.06** — Tạo signed artifacts, hướng dẫn setup/debug/update/uninstall,
  Codex upstream rebase procedure và smoke test trên máy sạch.
- [ ] **W13.07** — Chuyển receipts cần bàn giao ra khỏi cache `.harness` bị ignore
  sang artifact store/report bundle bền vững, redact secrets, lưu checksum và
  command tái chạy; reviewer đối chiếu mọi gate mục 12 trước chốt release.

## 12. Tiêu chí “đã hoàn thành”

Mục tiêu chỉ được coi là hoàn thành khi tất cả điều sau có evidence:

- [ ] Codex session mới gọi được graph mà không cần người dùng chỉ file.
- [ ] Có binary/TUI `project-graph-agent` hoạt động như một terminal-native coding
  agent: chat, swarm, analyze, review, run, replay và serve.
- [ ] Agent trả lời được orientation/flow/impact trên fixture và repository thật.
- [ ] Agent không phải đọc tuần tự file hàng nghìn dòng cho các câu hỏi cấu trúc.
- [ ] Context chứa source slices tối thiểu đủ để kiểm chứng, không phải graph dump.
- [ ] Mọi quan hệ quan trọng có provenance, revision và coverage.
- [ ] Unknown/dynamic/runtime gaps được nêu rõ.
- [ ] File edit làm graph cập nhật hoặc cảnh báo stale trước khi dùng.
- [ ] Archify diagram được sinh từ IR validate được và edge trace về graph evidence.
- [ ] Joern chỉ được gọi khi cần deep analysis và không làm hỏng fast path.
- [ ] Harness router/swarm chọn backend theo intent/coverage và giữ namespace memory
  tách biệt theo project/revision.
- [ ] Graph-aware swarm spawn đúng role-specific subagents, partition theo graph,
  fan-in claim có verifier, resume/replay được và không có duplicate/overlap
  write ngoài lease.
- [ ] Có role catalog, model/account binding, capability grants và task lifecycle
  trên app-server runtime; hooks được test theo capability hiện có.
- [ ] Executor theo OpenDev patterns chạy được nhiều analysis job terminal với lease, cancellation,
  resource budget và receipt; không tự ghi graph authority.
- [ ] T3MP3ST chỉ chạy red-team trong scope được phê duyệt; findings chưa verify
  không trở thành graph facts.
- [ ] OpenSandbox có thể cô lập job sâu; một job lỗi không làm hỏng graph daemon.
- [ ] Terminal TUI/headless có event parity; VT backend không làm mất receipts.
- [ ] Zvec/Ruflo/OpenSandbox/T3Code có degraded mode rõ ràng; core graph vẫn phục
  vụ local operation khi một integration lỗi.
- [ ] Có BrowserEvidence pipeline và single-account native fleet đạt tests mục Phase 9.
- [ ] Có benchmark so sánh baseline/fast/full graph với receipt tái chạy được.
- [ ] Có benchmark swarm: single-agent vs fan-out/DAG, parallel speedup, token/cost
  overhead, claim precision/recall, failure recovery và graph optimization gains.
- [ ] Có release/update strategy cho fork Codex: upstream compatibility matrix,
  protocol migration, signed artifacts và rebase procedure.
- [ ] Không có license/security blocker cho artifact được phân phối.

## 13. Trình tự tiếp tục và lệnh kiểm chứng

Kế hoạch đã được triển khai một phần. Theo dõi trạng thái ở mục 0; các lệnh
bên dưới là checks cần có receipt, không phải bằng chứng đã chạy thành công.

1. Đóng baseline W0; song song hoàn thiện contracts/persistence W1 không cần
   graph. Các thiếu sót cụ thể theo mục 0.3.
2. Sau W1-C, phát triển W2 bằng owned fixtures để kiểm W1-I; W4 graph/context
   giữ dependency riêng. Sau joint acceptance W1/W2, W3 nối native subagents
   runtime sau W2, rồi W5 nối scheduler/worktree/merge.
3. Mở các nhánh W6–W12 theo dependencies ở mục 11; thứ tự phase không có
   nghĩa là mọi công việc phải làm tuần tự.
4. W13 kiểm toàn bộ Definition of Done, benchmark và release receipts.

Trạng thái package theo dõi tại mục 0.2; mục này chỉ chỉ dẫn thứ tự,
không tạo thêm checklist package lặp.

```bash
cd /home/minh/projects/project-graph-agent
cargo test --workspace --locked --offline
```

Baseline adapters:

```bash
# 1. Kiểm tra baseline repo lõi
cd /home/minh/projects/outsource/codegraph
npm test

# 2. Build và kiểm tra kernel hiện tại
npm run build:kernel

# 3. Index fixture/repo thử nghiệm
codegraph init
codegraph status --json

# 4. Kiểm tra output context và MCP-compatible path (current codegraph CLI)
codegraph context --format json --max-nodes 8 --no-code "understand the main request flow"
codegraph explore "how does the main request flow reach persistence"
codegraph status --json

# 5. Kiểm tra renderer sau khi có context compiler
cd /home/minh/projects/outsource/archify
node archify/bin/archify.mjs doctor
node archify/bin/archify.mjs validate architecture examples/maka-architecture.architecture.json --quality showcase --json
```

Nếu baseline `codegraph` chưa build được, xử lý blocker trước khi tích gate W4
hoặc nối Joern vào graph thật. Các contract/task-store W1 độc lập vẫn có thể
triển khai song song. Không dùng graph giả làm bằng chứng hoàn thành integration.
