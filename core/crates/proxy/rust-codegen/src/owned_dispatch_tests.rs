use super::*;
use std::process::Command;

/// Builds a non-cloneable locked service model with both capture and synchronous calls.
fn fixture_object() -> SourceObject {
    let methods = [
        ("run", true, CallProtocol::Unit),
        (
            "fail",
            true,
            CallProtocol::ResultUnit {
                error_type: "String".into(),
            },
        ),
        ("touch", false, CallProtocol::Unit),
    ]
    .into_iter()
    .map(|(name, owned, protocol)| SourceMethod {
        name: name.into(),
        sdk_exposed: false,
        args: if name == "run" {
            vec![SourceArg {
                name: "label".into(),
                ty: "&str".into(),
            }]
        } else {
            vec![]
        },
        rust_return_type: "()".into(),
        is_async: owned,
        returns_owned_future: owned,
        cfg_attrs: vec![],
        doc_lines: vec![],
        protocol: MethodProtocol::Call(protocol),
    })
    .collect();
    SourceObject {
        object_id: "core/test".into(),
        schema_key: "test".into(),
        dispatch_name: "test".into(),
        full_type: "Subject".into(),
        access: ObjectAccess::ContextGetInstanceArcMutexConstruct,
        path_match: ObjectPathMatch::Exact,
        methods,
    }
}

/// Refuses implicit object snapshots before producing unsafe or unexpectedly expensive dispatch.
#[test]
#[should_panic(expected = "must return impl Future<Output = T> + 'static")]
fn borrowed_async_method_on_locked_object_is_rejected() {
    let mut object = fixture_object();
    object.methods[0].returns_owned_future = false;
    render_constructed_dispatch(&object, DispatchMode::Call);
}

/// Compiles and executes the actual generated dispatcher against a non-Clone, reentrant fixture.
#[test]
fn generated_capture_releases_lock_and_owns_operation_lifetime() {
    let object = fixture_object();
    let errors = HashMap::new();
    let source = format!("{}\n{}\n{}\nasync fn invoke(object: std::sync::Arc<std::sync::Mutex<Subject>>, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n{}\n}}\n",
        include_str!("owned_dispatch_fixture.rs.txt"),
        render_owned_call_dispatch(&object, &errors),
        render_object_sync_call_dispatch(&object, &errors),
        render_constructed_dispatch(&object, DispatchMode::Call));
    let temp = std::env::temp_dir().canonicalize().unwrap();
    let directory = temp.join(format!("operit-owned-dispatch-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let fixture = directory.join("fixture.rs");
    let executable = directory.join(format!("fixture{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&fixture, source).unwrap();
    let compiled = Command::new("rustc")
        .args(["--edition=2021", "--test", "-Awarnings"])
        .arg(&fixture)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let tested = Command::new(&executable).output().unwrap();
    assert!(
        tested.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&tested.stdout),
        String::from_utf8_lossy(&tested.stderr)
    );
    let resolved = directory.canonicalize().unwrap();
    assert_eq!(resolved.parent(), Some(temp.as_path()));
    std::fs::remove_dir_all(resolved).unwrap();
}
