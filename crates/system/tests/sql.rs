use graph_system::{SqlError, SqlOperation as Op, analyze_sqlite};
use sha2::{Digest, Sha256};

#[test]
fn actual_orders_queries_have_ordered_syntax_observations() {
    let fixtures: &[(&str, Op, &[&str])] = &[
        (
            include_str!("../../../fixtures/orders/sql/ack.sql"),
            Op::Update,
            &["outbox"],
        ),
        (
            include_str!("../../../fixtures/orders/sql/begin.sql"),
            Op::Begin,
            &[],
        ),
        (
            include_str!("../../../fixtures/orders/sql/commit.sql"),
            Op::Commit,
            &[],
        ),
        (
            include_str!("../../../fixtures/orders/sql/rollback.sql"),
            Op::Rollback,
            &[],
        ),
        (
            include_str!("../../../fixtures/orders/sql/consume.sql"),
            Op::Insert,
            &["notifications"],
        ),
        (
            include_str!("../../../fixtures/orders/sql/enqueue.sql"),
            Op::Insert,
            &["outbox"],
        ),
        (
            include_str!("../../../fixtures/orders/sql/insert-order.sql"),
            Op::Insert,
            &["orders"],
        ),
        (
            include_str!("../../../fixtures/orders/sql/notifications.sql"),
            Op::Query,
            &["notifications"],
        ),
        (
            include_str!("../../../fixtures/orders/sql/order.sql"),
            Op::Query,
            &["orders"],
        ),
        (
            include_str!("../../../fixtures/orders/sql/pending.sql"),
            Op::Query,
            &["outbox"],
        ),
    ];
    for (source, operation, relations) in fixtures {
        let report = analyze_sqlite(source).unwrap();
        assert!(!report.semantic_verified);
        assert_eq!(
            report.source_sha256,
            format!("{:x}", Sha256::digest(source.as_bytes()))
        );
        assert_eq!(report.statements.len(), 1);
        let statement = &report.statements[0];
        assert_eq!(statement.ordinal, 0);
        assert_eq!(&statement.operation, operation);
        assert_eq!(&statement.relations, relations);
    }
    let schema = analyze_sqlite(include_str!("../../../fixtures/orders/sql/schema.sql")).unwrap();
    assert_eq!(schema.statements.len(), 3);
    for (ordinal, name) in ["orders", "outbox", "notifications"].iter().enumerate() {
        let statement = &schema.statements[ordinal];
        assert_eq!(statement.ordinal, ordinal);
        assert_eq!(statement.operation, Op::CreateTable);
        assert_eq!(statement.relations, [*name]);
    }
}

#[test]
fn comments_strings_quotes_and_semicolons_do_not_fabricate_relations() {
    let source = "-- FROM fake\nSELECT 'FROM secrets; DROP TABLE hidden' FROM \"order details\"; DELETE FROM [old orders]; PRAGMA cache_size = 10;";
    let report = analyze_sqlite(source).unwrap();
    assert_eq!(report.statements.len(), 3);
    assert_eq!(report.statements[0].relations, ["\"order details\""]);
    assert_eq!(report.statements[1].relations, ["[old orders]"]);
    assert_eq!(report.statements[1].operation, Op::Delete);
    assert_eq!(report.statements[2].operation, Op::Other);
    let output = format!("{report:?}");
    assert!(!output.contains("secrets"));
    assert!(!output.contains("hidden"));
    for statement in report.statements {
        if let Some(span) = statement.parser_span {
            assert!(span.start_line > 0 && span.start_column > 0);
            assert!(span.end_line >= span.start_line);
        }
    }
}

#[test]
fn cte_names_remain_unresolved_syntax_not_physical_tables() {
    let report =
        analyze_sqlite("WITH recent AS (SELECT id FROM orders) SELECT * FROM recent").unwrap();
    assert_eq!(report.statements[0].relations, ["orders", "recent"]);
    assert!(!report.semantic_verified);
    // The parser accepts syntax without consulting an actual SQLite schema.
    assert!(
        !analyze_sqlite("CREATE TABLE t (x INT, x INT)")
            .unwrap()
            .semantic_verified
    );
}

#[test]
fn malformed_source_returns_only_redacted_error_not_a_partial_report() {
    let error = analyze_sqlite("SELECT * FROM orders; SELECT 'private-token").unwrap_err();
    assert_eq!(error, SqlError::Syntax);
    assert!(!error.to_string().contains("private-token"));
    assert!(!format!("{error:?}").contains("orders"));
}

#[test]
fn independently_enforces_source_token_statement_and_recursion_limits() {
    let exact = format!("/*{}*/", "x".repeat(128 * 1024 - 4));
    assert!(analyze_sqlite(&exact).unwrap().statements.is_empty());
    assert_eq!(analyze_sqlite(&(exact + " ")), Err(SqlError::SourceLimit));
    assert!(
        analyze_sqlite(&" ".repeat(2048))
            .unwrap()
            .statements
            .is_empty()
    );
    assert_eq!(analyze_sqlite(&" ".repeat(2049)), Err(SqlError::TokenLimit));
    assert_eq!(
        analyze_sqlite(&"SELECT 1;".repeat(256))
            .unwrap()
            .statements
            .len(),
        256
    );
    assert_eq!(
        analyze_sqlite(&"SELECT 1;".repeat(257)),
        Err(SqlError::StatementLimit)
    );
    let deep = format!("SELECT {}1{}", "(".repeat(64), ")".repeat(64));
    assert_eq!(analyze_sqlite(&deep), Err(SqlError::Syntax));
}

#[test]
fn source_hash_preserves_comments_and_whitespace_even_when_syntax_matches() {
    let first = analyze_sqlite("SELECT 1;").unwrap();
    let second = analyze_sqlite("SELECT 1; -- comment\n").unwrap();
    assert_ne!(first.source_sha256, second.source_sha256);
    assert_eq!(
        first.statements[0].operation,
        second.statements[0].operation
    );
    assert!(
        analyze_sqlite("; ; -- empty\n")
            .unwrap()
            .statements
            .is_empty()
    );
}
