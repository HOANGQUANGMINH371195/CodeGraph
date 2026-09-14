use serde_json::json;
use std::{fs, path::PathBuf, process::Command};

const CODE: &str = "const x = \"😀\";\n";
const CODE_HASH: &str = "ad2575687a8314045f8ca33806508afb1e2439b84dabaee2a33b7cdf6aa8ca12";
const SQL: &str = "SELECT 1;\n";
const SQL_HASH: &str = "b4e0497804e46e0a0b0b8c31975b062152d551bac49c3c2e80932567b4085dcd";

struct Fixture {
    _dir: tempfile::TempDir,
    db: PathBuf,
    root: PathBuf,
    task: PathBuf,
    report: PathBuf,
}
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("sql")).unwrap();
    fs::write(root.join("code.mjs"), CODE).unwrap();
    fs::write(root.join("sql/q.sql"), SQL).unwrap();
    let project = json!({"repository_id":"r","worktree_id":"w","git_head":"h","working_tree_fingerprint":"f","config_hash":"c","ignore_policy_version":"i"});
    let task = json!({"schema_version":1,"id":"t","project":project,"graph_version":"g","role":"reader","account_lane":"local","scope":["code.mjs"],"dependencies":[],"context_ref":"ctx","expected_artifacts":["report"],"token_budget":1});
    let citation = json!({"schema_version":1,"id":"code","project":task["project"],"graph_version":"g","path":"code.mjs","content_sha256":CODE_HASH,"start_line":1,"end_line":1,"analysis_run":"old"});
    let report = json!({"schemaVersion":1,"kind":"file_reads","status":"observed","sourceCoordinateEncoding":"utf16-code-unit","source":{"path":"code.mjs","sha256":CODE_HASH,"byteLength":CODE.len(),"lineCount":1},"discovery":{"coordinateEncoding":"utf16-code-unit","sourceSha256":CODE_HASH,"candidates":[{"status":"candidate","sourceSha256":CODE_HASH,"read":{"invocation":{"start":0,"end":5}}}]},"targets":[{"status":"captured-candidate","runtimeVerified":false,"atomicSnapshotVerified":false,"raceFreeContainmentVerified":false,"containmentChecksPassed":true,"requiresStableFilesystem":true,"source":{"path":"code.mjs","sha256":CODE_HASH,"byteLength":CODE.len(),"lineCount":1},"target":{"path":"sql/q.sql","sha256":SQL_HASH,"byteLength":SQL.len(),"lineCount":1},"candidateIndex":0}],"runtimeVerified":false,"atomicSnapshotVerified":false,"raceFreeContainmentVerified":false,"requiresStableFilesystem":true,"persisted":false});
    let task_path = dir.path().join("task.json");
    let citation_path = dir.path().join("citation.json");
    let report_path = dir.path().join("report.json");
    fs::write(&task_path, serde_json::to_vec(&task).unwrap()).unwrap();
    fs::write(&citation_path, serde_json::to_vec(&citation).unwrap()).unwrap();
    fs::write(&report_path, serde_json::to_vec(&report).unwrap()).unwrap();
    let db = dir.path().join("state.db");
    let output = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .args([
            "--database",
            db.to_str().unwrap(),
            "record-evidence",
            citation_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    Fixture {
        _dir: dir,
        db,
        root,
        task: task_path,
        report: report_path,
    }
}
#[test]
fn publishes_only_host_reverified_candidate_links_and_prebudgets_output() {
    let f = fixture();
    let run = |fixture: &Fixture, cap: Option<usize>| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"));
        c.args([
            "--database",
            fixture.db.to_str().unwrap(),
            "publish-sql-links",
            "code",
            fixture.task.to_str().unwrap(),
            fixture.report.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
            "--analysis-run",
            "capture",
            "--expected-generation",
            "0",
        ]);
        if let Some(cap) = cap {
            c.args(["--max-output-bytes", &cap.to_string()]);
        }
        c.output().unwrap()
    };
    let base = run(&f, None);
    assert!(
        base.status.success(),
        "{}",
        String::from_utf8_lossy(&base.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&base.stdout).unwrap();
    assert_eq!(value["generation"], 1);
    assert_eq!(value["candidate_only"], true);
    assert_eq!(value["runtime_verified"], false);
    let read = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .args([
            "--database",
            f.db.to_str().unwrap(),
            "sql-links",
            "code",
            f.task.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(read.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&read.stdout).unwrap()["generation"],
        1
    );
    let context = |fixture: &Fixture, link: &str, cap: Option<usize>| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"));
        c.args([
            "--database",
            fixture.db.to_str().unwrap(),
            "sql-link-context",
            "code",
            fixture.task.to_str().unwrap(),
            "--link",
            link,
        ]);
        if let Some(cap) = cap {
            c.args(["--max-output-bytes", &cap.to_string()]);
        }
        c.output().unwrap()
    };
    let context_base = context(&f, "file-read:0:sql/q.sql", None);
    assert!(
        context_base.status.success(),
        "{}",
        String::from_utf8_lossy(&context_base.stderr)
    );
    let context_value: serde_json::Value = serde_json::from_slice(&context_base.stdout).unwrap();
    assert_eq!(context_value["kind"], "sql_link_context");
    assert_eq!(context_value["target_evidence"]["path"], "sql/q.sql");
    assert_eq!(context_value["statements"][0]["operation"], "query");
    assert_eq!(context_value["runtime_verified"], false);
    assert_eq!(context_value["semantic_verified"], false);
    assert!(!String::from_utf8_lossy(&context_base.stdout).contains(SQL));
    let exact_context = context(&f, "file-read:0:sql/q.sql", Some(context_base.stdout.len()));
    assert!(exact_context.status.success());
    let under_context = context(
        &f,
        "file-read:0:sql/q.sql",
        Some(context_base.stdout.len() - 1),
    );
    assert!(!under_context.status.success());
    assert!(under_context.stdout.is_empty());
    let unknown = context(&f, "file-read:99:sql/nope.sql", None);
    assert!(!unknown.status.success());
    assert!(unknown.stdout.is_empty());
    let diagram = |fixture: &Fixture, link: &str, cap: Option<usize>| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"));
        c.args([
            "--database",
            fixture.db.to_str().unwrap(),
            "sql-link-diagram",
            "code",
            fixture.task.to_str().unwrap(),
            "--link",
            link,
        ]);
        if let Some(cap) = cap {
            c.args(["--max-output-bytes", &cap.to_string()]);
        }
        c.output().unwrap()
    };
    let diagram_base = diagram(&f, "file-read:0:sql/q.sql", None);
    assert!(
        diagram_base.status.success(),
        "{}",
        String::from_utf8_lossy(&diagram_base.stderr)
    );
    let diagram_value: serde_json::Value = serde_json::from_slice(&diagram_base.stdout).unwrap();
    assert_eq!(diagram_value["kind"], "sql_link_diagram");
    assert_eq!(diagram_value["evidence_manifest"], context_value);
    let components = diagram_value["diagram"]["components"].as_array().unwrap();
    assert_eq!(components.len(), 2);
    assert!(components.iter().all(|item| item["type"] == "external"));
    assert_eq!(components[0]["tag"], "candidate_only");
    let connections = diagram_value["diagram"]["connections"].as_array().unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0]["label"], "static file-read candidate");
    assert_eq!(connections[0]["variant"], "dashed");
    let cards = diagram_value["diagram"]["cards"]
        .to_string()
        .to_ascii_lowercase();
    assert!(cards.contains("not runtime traffic"));
    assert!(cards.contains("physical table"));
    assert!(!String::from_utf8_lossy(&diagram_base.stdout).contains(SQL));
    let exact_diagram = diagram(&f, "file-read:0:sql/q.sql", Some(diagram_base.stdout.len()));
    assert!(exact_diagram.status.success());
    let under_diagram = diagram(
        &f,
        "file-read:0:sql/q.sql",
        Some(diagram_base.stdout.len() - 1),
    );
    assert!(!under_diagram.status.success());
    assert!(under_diagram.stdout.is_empty());
    let unknown_diagram = diagram(&f, "file-read:99:sql/nope.sql", None);
    assert!(!unknown_diagram.status.success());
    assert!(unknown_diagram.stdout.is_empty());
    let invalidate = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .args([
            "--database",
            f.db.to_str().unwrap(),
            "invalidate-sql-links",
            "code",
            f.task.to_str().unwrap(),
            "--expected-generation",
            "1",
        ])
        .output()
        .unwrap();
    assert!(invalidate.status.success());
    let after = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .args([
            "--database",
            f.db.to_str().unwrap(),
            "sql-links",
            "code",
            f.task.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(after.status.success());
    assert!(serde_json::from_slice::<serde_json::Value>(&after.stdout).unwrap()["graph"].is_null());
    let invalidated_context = context(&f, "file-read:0:sql/q.sql", None);
    assert!(!invalidated_context.status.success());
    assert!(invalidated_context.stdout.is_empty());
    let invalidated_diagram = diagram(&f, "file-read:0:sql/q.sql", None);
    assert!(!invalidated_diagram.status.success());
    assert!(invalidated_diagram.stdout.is_empty());
    let exact_fixture = fixture();
    let exact = run(&exact_fixture, Some(base.stdout.len()));
    assert!(exact.status.success());
    let under_fixture = fixture();
    let under = run(&under_fixture, Some(base.stdout.len() - 1));
    assert!(!under.status.success());
    assert!(under.stdout.is_empty());
}
