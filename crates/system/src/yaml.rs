//! Bounded marked YAML syntax used only by system adapters.

use std::{collections::BTreeSet, str::Chars};
use yaml_rust2::{
    parser::{Event, Parser},
    scanner::{Marker, TScalarStyle},
};

pub(crate) struct Node {
    pub line: u32,
    pub value: Value,
}

pub(crate) enum Value {
    Null,
    Scalar(String),
    Sequence(Vec<Node>),
    Mapping(Vec<(String, u32, Node)>),
}

#[derive(Debug)]
pub(crate) struct YamlError {
    pub reason: &'static str,
    pub line: u32,
}

fn error(reason: &'static str, line: u32) -> YamlError {
    YamlError { reason, line }
}

struct Reader<'a> {
    parser: Parser<Chars<'a>>,
    remaining: usize,
    source_lines: Vec<u32>,
}

impl Reader<'_> {
    fn source_line(&self, marker: &Marker) -> u32 {
        self.source_lines
            .get(marker.line().saturating_sub(1))
            .copied()
            .unwrap_or(u32::MAX)
    }

    fn next(&mut self) -> Result<(Event, Marker), YamlError> {
        if self.remaining == 0 {
            return Err(error("YAML event budget exceeded", 1));
        }
        self.remaining -= 1;
        self.parser
            .next_token()
            .map_err(|err| error("malformed YAML", self.source_line(err.marker())))
    }

    fn node(&mut self, first: (Event, Marker), depth: usize) -> Result<Node, YamlError> {
        let (event, marker) = first;
        let line = self.source_line(&marker);
        if depth > 64 {
            return Err(error("YAML depth budget exceeded", line));
        }
        let value = match event {
            Event::Scalar(text, style, 0, None) => {
                if style == TScalarStyle::Plain
                    && matches!(text.as_str(), "" | "~" | "null" | "Null" | "NULL")
                {
                    Value::Null
                } else {
                    Value::Scalar(text)
                }
            }
            Event::SequenceStart(0, None) => {
                let mut values = Vec::new();
                loop {
                    let next = self.next()?;
                    if next.0 == Event::SequenceEnd {
                        break;
                    }
                    values.push(self.node(next, depth + 1)?);
                }
                Value::Sequence(values)
            }
            Event::MappingStart(0, None) => {
                let mut entries = Vec::new();
                let mut keys = BTreeSet::new();
                loop {
                    let next = self.next()?;
                    if next.0 == Event::MappingEnd {
                        break;
                    }
                    let key = self.node(next, depth + 1)?;
                    let Value::Scalar(name) = key.value else {
                        return Err(error("non-string YAML mapping key", key.line));
                    };
                    if name.is_empty() || name == "<<" {
                        return Err(error("unsupported YAML mapping key", key.line));
                    }
                    if !keys.insert(name.clone()) {
                        return Err(error("duplicate YAML mapping key", key.line));
                    }
                    let next = self.next()?;
                    let value = self.node(next, depth + 1)?;
                    entries.push((name, key.line, value));
                }
                Value::Mapping(entries)
            }
            Event::Alias(_)
            | Event::Scalar(..)
            | Event::SequenceStart(..)
            | Event::MappingStart(..) => {
                return Err(error(
                    "YAML anchors, aliases and tags are unsupported",
                    line,
                ));
            }
            _ => return Err(error("unexpected YAML event", line)),
        };
        Ok(Node { line, value })
    }
}

pub(crate) fn parse(text: &str) -> Result<Node, YamlError> {
    if text.len() > 1024 * 1024 {
        return Err(error("YAML source budget exceeded", 1));
    }
    // YAML recognizes CR, LF and CRLF; SourceEvidence uses LF-delimited lines.
    // Preserve original bytes and map parser marks instead of normalizing input.
    let mut source_lines = vec![1];
    let mut line = 1;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            line += 1;
            source_lines.push(line);
        } else if byte == b'\r' && text.as_bytes().get(index + 1) != Some(&b'\n') {
            source_lines.push(line);
        }
    }
    let mut reader = Reader {
        parser: Parser::new_from_str(text),
        remaining: 100_000,
        source_lines,
    };
    if reader.next()?.0 != Event::StreamStart || reader.next()?.0 != Event::DocumentStart {
        return Err(error("expected one YAML document", 1));
    }
    let first = reader.next()?;
    let root = reader.node(first, 0)?;
    if reader.next()?.0 != Event::DocumentEnd || reader.next()?.0 != Event::StreamEnd {
        return Err(error("multiple YAML documents are unsupported", 1));
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marked_flow_keys_and_quoted_null() {
        let root = parse("# heading\n{a: null, b: 'null', c: [value]}\n").unwrap();
        let Value::Mapping(entries) = root.value else {
            panic!("mapping")
        };
        assert!(entries.iter().all(|entry| entry.1 == 2));
        assert!(matches!(entries[0].2.value, Value::Null));
        assert!(matches!(&entries[1].2.value, Value::Scalar(text) if text == "null"));
    }

    #[test]
    fn parser_cr_marks_map_to_physical_source_lines() {
        for (text, expected) in [("a: 1\rb: 2\n", 1), ("a: 1\r\nb: 2\n", 2)] {
            let root = parse(text).unwrap();
            let Value::Mapping(entries) = root.value else {
                panic!("mapping")
            };
            assert_eq!(entries[1].1, expected);
            assert_eq!(entries[1].2.line, expected);
        }
    }

    #[test]
    fn syntax_and_budget_errors_never_echo_input() {
        for text in [
            "x: secret\nx: other",
            "x: &secret value",
            "x: !secret value",
            "---\nx: secret\n---\ny: other",
            "x: [secret",
            "? [secret]\n: value",
        ] {
            let err = parse(text).err().expect("reject");
            assert!(!format!("{err:?}").contains("secret"));
            assert!(err.line > 0);
        }
        for (text, reason) in [
            (
                format!("{}x{}", "[".repeat(66), "]".repeat(66)),
                "YAML depth budget exceeded",
            ),
            (" ".repeat(1024 * 1024 + 1), "YAML source budget exceeded"),
            (
                format!("[{}]", "x,".repeat(100_001)),
                "YAML event budget exceeded",
            ),
        ] {
            assert_eq!(parse(&text).err().expect("budget rejected").reason, reason);
        }
    }
}
