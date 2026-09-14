use graph_execution::{SandboxError, SandboxRuntime};

pub fn runtime_or_skip() -> Option<SandboxRuntime> {
    require_or_skip(
        SandboxRuntime::discover(),
        std::env::var_os("GRAPH_REQUIRE_SANDBOX").is_some(),
    )
}

fn require_or_skip<T>(result: Result<T, SandboxError>, required: bool) -> Option<T> {
    match result {
        Ok(runtime) => Some(runtime),
        Err(error) => {
            assert!(
                !required,
                "required sandbox capability unavailable: {error}"
            );
            eprintln!("sandbox integration skipped: {error}");
            None
        }
    }
}

#[test]
fn unavailable_optional_backend_is_explicitly_skipped() {
    assert!(require_or_skip::<()>(Err(SandboxError::Unsupported), false).is_none());
    assert_eq!(require_or_skip(Ok(7), true), Some(7));
}

#[test]
#[should_panic(expected = "required sandbox capability unavailable")]
fn unavailable_required_backend_fails() {
    require_or_skip::<()>(Err(SandboxError::Unsupported), true);
}
