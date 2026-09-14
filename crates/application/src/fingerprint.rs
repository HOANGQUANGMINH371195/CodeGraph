use graph_domain::{CheckCommand, DomainError};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

/// SHA-256 over a versioned, length-delimited encoding, independent of JSON.
/// Declared executable/environment digests are not independently verified here.
pub fn command_fingerprint(command: &CheckCommand) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"project-graph/check-command/v1\0");
    text(&mut hasher, command.program());
    hasher.update((command.args().len() as u64).to_be_bytes());
    for arg in command.args() {
        text(&mut hasher, arg);
    }
    for value in [
        command.cwd(),
        command.executable_sha256(),
        command.environment_sha256(),
    ] {
        text(&mut hasher, value);
    }
    for value in [
        command.timeout_ms(),
        command.cleanup_timeout_ms(),
        command.stdout_max_bytes(),
        command.stderr_max_bytes(),
    ] {
        hasher.update(value.to_be_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Hashes a host-supplied explicit UTF-8 environment; never reads ambient env.
/// Names are exact/case-sensitive. Host must reject target-OS aliases/duplicates
/// before constructing this map. A digest does not protect low-entropy secrets.
pub fn environment_fingerprint(
    environment: &BTreeMap<String, String>,
) -> Result<String, DomainError> {
    let mut hasher = Sha256::new();
    hasher.update(b"project-graph/environment/v1\0");
    hasher.update((environment.len() as u64).to_be_bytes());
    for (key, value) in environment {
        if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
            return Err(DomainError::Invalid("invalid explicit environment entry"));
        }
        text(&mut hasher, key);
        text(&mut hasher, value);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(args: Vec<String>, changed: u8) -> CheckCommand {
        CheckCommand::new(
            if changed == 0 { "other" } else { "fixture" }.into(),
            if changed == 1 {
                vec!["changed".into()]
            } else {
                args
            },
            if changed == 2 { "src" } else { "." }.into(),
            if changed == 3 { "c" } else { "a" }.repeat(64),
            if changed == 4 { "d" } else { "b" }.repeat(64),
            if changed == 5 { 1001 } else { 1000 },
            if changed == 6 { 501 } else { 500 },
            if changed == 7 { 1025 } else { 1024 },
            if changed == 8 { 2049 } else { 2048 },
        )
        .unwrap()
    }

    #[test]
    fn command_matches_independent_golden_and_binds_every_field() {
        // Independently computed with Node crypto SHA-256 + Buffer big-endian lengths.
        let args = vec!["".into(), "a b".into()];
        let golden = "0e57be1f303c6d2dc11d6e046d043841cf56fb975328f56c2e004623876f0577";
        assert_eq!(command_fingerprint(&command(args.clone(), 255)), golden);
        for field in 0..9 {
            assert_ne!(
                command_fingerprint(&command(args.clone(), field)),
                golden,
                "field {field}"
            );
        }
        for (left, right) in [
            (vec!["ab", "c"], vec!["a", "bc"]),
            (vec!["a b"], vec!["a", "b"]),
            (vec![""], vec![]),
            (vec!["é"], vec!["e\u{301}"]),
        ] {
            let convert = |values: Vec<&str>| values.into_iter().map(str::to_owned).collect();
            assert_ne!(
                command_fingerprint(&command(convert(left), 255)),
                command_fingerprint(&command(convert(right), 255))
            );
        }
    }

    #[test]
    fn environment_is_order_independent_unambiguous_and_validated() {
        let values = [
            ("LANG".to_owned(), "C.UTF-8".to_owned()),
            ("MODE".to_owned(), "test".to_owned()),
        ];
        let environment = BTreeMap::from(values.clone());
        let golden = "4f6f1744f1db357d0d7952cd076058e6c28d5213d0a69b387c129a99142e6bd5";
        assert_eq!(environment_fingerprint(&environment).unwrap(), golden);
        assert_eq!(
            environment_fingerprint(&values.into_iter().rev().collect()).unwrap(),
            golden
        );
        for (key, value) in [("", "v"), ("A=B", "v"), ("A\0B", "v"), ("A", "v\0")] {
            assert!(
                environment_fingerprint(&BTreeMap::from([(key.into(), value.into())])).is_err()
            );
        }
        let a = BTreeMap::from([("A".into(), "BC".into())]);
        let b = BTreeMap::from([("AB".into(), "C".into())]);
        assert_ne!(
            environment_fingerprint(&a).unwrap(),
            environment_fingerprint(&b).unwrap()
        );
        let mut changed = environment.clone();
        changed.insert("MODE".into(), "other".into());
        assert_ne!(environment_fingerprint(&changed).unwrap(), golden);
        let empty = environment_fingerprint(&BTreeMap::new()).unwrap();
        let one_empty =
            environment_fingerprint(&BTreeMap::from([("A".into(), "".into())])).unwrap();
        assert_ne!(empty, one_empty);
    }
}
