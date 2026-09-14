use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

const SCHEMA: &[u8] = include_bytes!("../../../fixtures/orders/sql/schema.sql");
const HASH: &str = "8f7b9074e54e70979a12d8e3ea8f2381a78e6c5679f2d440e92b8734a1213215";

struct Fixture {
    _dir: tempfile::TempDir,
    db: PathBuf,
    root: PathBuf,
    task: PathBuf,
    citation: Value,
}

fn fixture(source: &[u8], hash: &str, start: u32, end: u32) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("query.sql"), source).unwrap();
    let project = json!({"repository_id":"r","worktree_id":"w","git_head":"h",
        "working_tree_fingerprint":"f","config_hash":"c","ignore_policy_version":"i"});
    let citation = json!({"schema_version":1,"id":"sql-source","project":project,"graph_version":"g",
        "path":"query.sql","content_sha256":hash,"start_line":start,"end_line":end,"analysis_run":"unverified-run"});
    let task = json!({"schema_version":1,"id":"t","project":project,"graph_version":"g","role":"reader",
        "account_lane":"local","scope":["query.sql"],"dependencies":[],"context_ref":"ctx",
        "expected_artifacts":["report"],"token_budget":100});
    let task_path = dir.path().join("task.json");
    let citation_path = dir.path().join("citation.json");
    fs::write(&task_path, serde_json::to_vec(&task).unwrap()).unwrap();
    fs::write(&citation_path, serde_json::to_vec(&citation).unwrap()).unwrap();
    let f = Fixture {
        db: dir.path().join("state.db"),
        _dir: dir,
        root,
        task: task_path,
        citation,
    };
    let recorded = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .arg("--database")
        .arg(&f.db)
        .arg("record-evidence")
        .arg(citation_path)
        .output()
        .unwrap();
    assert!(
        recorded.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded.stderr)
    );
    f
}

impl Fixture {
    fn run(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .arg("--database")
            .arg(&self.db)
            .arg("analyze-sql")
            .arg("sql-source")
            .arg(&self.task)
            .arg(&self.root)
            .args(extra)
            .output()
            .unwrap()
    }
}

fn success(out: &Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn failure(out: Output) {
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stderr).contains("private-token"));
}

#[test]
fn reports_full_citation_syntax_without_bodies_or_authority() {
    let f = fixture(SCHEMA, HASH, 1, 3);
    let out = f.run(&[]);
    let report = success(&out);
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["kind"], "sql_analysis");
    assert_eq!(report["dialect"], "sqlite");
    assert_eq!(report["evidence"], f.citation);
    for key in [
        "candidate_only",
        "source_is_untrusted",
        "content_hash_and_lines_verified",
    ] {
        assert_eq!(report[key], true);
    }
    for key in [
        "persisted",
        "analysis_run_verified",
        "semantic_verified",
        "relationship_verified",
    ] {
        assert_eq!(report[key], false);
    }
    assert_eq!(report["snapshot_binding"], "caller_supplied");
    assert_eq!(report["relation_semantics"], "syntactic_unresolved");
    assert_eq!(report["span_coverage"], "parser_hint_not_full_statement");
    let statements = report["statements"].as_array().unwrap();
    assert_eq!(statements.len(), 3);
    for (i, name) in ["orders", "outbox", "notifications"].iter().enumerate() {
        assert_eq!(statements[i]["ordinal"], i);
        assert_eq!(statements[i]["operation"], "create_table");
        assert_eq!(statements[i]["relations"], json!([name]));
    }
    assert!(!String::from_utf8_lossy(&out.stdout).contains("CREATE TABLE"));
    assert_eq!(f.run(&[]).stdout, out.stdout);
}

#[test]
fn output_and_source_limits_are_exact_and_fail_without_partial_json() {
    let f = fixture(SCHEMA, HASH, 1, 3);
    let baseline = f.run(&[]);
    success(&baseline);
    let exact = baseline.stdout.len().to_string();
    let small = (baseline.stdout.len() - 1).to_string();
    let exact_out = f.run(&["--max-output-bytes", &exact]);
    success(&exact_out);
    assert_eq!(exact_out.stdout, baseline.stdout);
    failure(f.run(&["--max-output-bytes", &small]));
    failure(f.run(&["--max-output-bytes", "0"]));
    success(&f.run(&["--max-source-bytes", &SCHEMA.len().to_string()]));
    failure(f.run(&["--max-source-bytes", &(SCHEMA.len() - 1).to_string()]));
    failure(f.run(&["--max-source-bytes", "131073"]));
}

#[test]
fn rejects_drift_missing_source_and_wrong_snapshot() {
    let f = fixture(SCHEMA, HASH, 1, 3);
    success(&f.run(&[]));
    fs::write(
        f.root.join("query.sql"),
        String::from_utf8_lossy(SCHEMA).replace("orders", "others"),
    )
    .unwrap();
    failure(f.run(&[]));
    fs::remove_file(f.root.join("query.sql")).unwrap();
    failure(f.run(&[]));
    fs::write(f.root.join("query.sql"), SCHEMA).unwrap();
    let mut task: Value = serde_json::from_slice(&fs::read(&f.task).unwrap()).unwrap();
    task["project"]["git_head"] = json!("different");
    fs::write(&f.task, serde_json::to_vec(&task).unwrap()).unwrap();
    failure(f.run(&[]));
}

#[test]
fn rejects_partial_citations_and_redacts_malformed_sql_and_task() {
    failure(fixture(SCHEMA, HASH, 1, 1).run(&[]));
    failure(fixture(SCHEMA, HASH, 2, 3).run(&[]));
    let f = fixture(
        b"SELECT 'private-token",
        "5f5aedafe830613234cc6adbf376c8b3018203ad683db75f36857ee5502cf801",
        1,
        1,
    );
    failure(f.run(&[]));
    fs::write(&f.task, b"{private-token").unwrap();
    failure(f.run(&[]));
}
