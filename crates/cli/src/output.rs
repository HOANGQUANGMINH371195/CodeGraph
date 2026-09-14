pub use graph_protocol::output::json_line;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_budget_includes_unicode_escapes_and_newline() {
        for value in [
            serde_json::Value::Null,
            serde_json::json!({"source":"Tiếng Việt 🚀\n\t\"\\\u{0001}"}),
            serde_json::json!({"source":"x".repeat(100_000)}),
        ] {
            let mut expected = serde_json::to_vec(&value).unwrap();
            expected.push(b'\n');
            assert_eq!(json_line(&value, expected.len()).unwrap(), expected);
            assert!(json_line(&value, expected.len() - 1).is_err());
            assert!(json_line(&value, 0).is_err());
        }
    }
}
