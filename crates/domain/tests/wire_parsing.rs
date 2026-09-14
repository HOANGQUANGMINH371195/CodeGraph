use graph_domain::{
    AssertionKind, Capability, ClaimState, ContextOperation, EventKind, EvidenceKind,
    UntrustedContentOrigin,
};

macro_rules! check_tokens {
    ($name:ident, $ty:ty, [$($token:literal),+]) => {
        #[test]
        fn $name() {
            for token in [$($token),+] {
                let parsed = token.parse::<$ty>().expect("canonical token");
                assert_eq!(parsed.as_str(), token);
                assert_eq!(parsed, <$ty>::from_str(token).expect("legacy parser"));
                for invalid in [format!(" {token}"), format!("{token} "), token.to_uppercase(), format!("{token}\0")] {
                    let error = invalid.parse::<$ty>().expect_err("noncanonical input");
                    assert_eq!(error.to_string(), <$ty>::from_str(&invalid).expect_err("legacy rejection").to_string());
                }
            }
            for invalid in ["", "unknown", "☃"] {
                assert!(invalid.parse::<$ty>().is_err());
            }
        }
    };
}

check_tokens!(
    operations,
    ContextOperation,
    [
        "orientation",
        "navigate",
        "impact",
        "test",
        "handoff",
        "trace",
        "change_review"
    ]
);
check_tokens!(
    capabilities,
    Capability,
    [
        "read_graph",
        "read_source",
        "read_evidence",
        "read_documents",
        "read_runtime",
        "run_analysis",
        "run_tests",
        "run_terminal",
        "submit_claims",
        "request_subagent",
        "write_worktree"
    ]
);
check_tokens!(
    origins,
    UntrustedContentOrigin,
    [
        "source",
        "document",
        "tool_output",
        "runtime_trace",
        "human_input",
        "model_output"
    ]
);
check_tokens!(
    assertions,
    AssertionKind,
    [
        "parser",
        "resolver",
        "cpg",
        "runtime_trace",
        "document",
        "human",
        "security_finding"
    ]
);
check_tokens!(
    evidence,
    EvidenceKind,
    [
        "source",
        "artifact",
        "analysis_run",
        "runtime_trace",
        "document",
        "human"
    ]
);
check_tokens!(
    claims,
    ClaimState,
    ["candidate", "accepted", "rejected", "superseded", "expired"]
);
check_tokens!(
    events,
    EventKind,
    [
        "rpc_launch_registered",
        "rpc_launch_claimed",
        "rpc_spawn_observed",
        "rpc_terminal_recorded",
        "check_policy_registered",
        "execution_launch_claimed",
        "queued",
        "leased",
        "submitted",
        "integrated",
        "rejected",
        "cancelled"
    ]
);
