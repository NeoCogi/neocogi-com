//! Public detached expressions and diagnostic selection keep distinct contracts.
use super::*;

/// Each constructor has an observable capability rule, independent of ownership.
#[test]
fn capability_leaves_and_wrapper_boundaries() {
    let mut graph = ValueGraph::default();
    for (name, copied, persistence) in [
        ("copied", true, Persistence::Supported),
        ("owned", false, Persistence::Supported),
        ("live", false, Persistence::Unsupported),
    ] {
        graph.definitions.insert(
            name.into(),
            ValueDefinition {
                name: name.into(),
                fields: Vec::new(),
                copied,
                persistence,
            },
        );
    }
    for (leaf, persisted, copied) in [
        (ValueType::Unit, true, true),
        (ValueType::Text, true, false),
        (ValueType::Scalar("bool".into()), true, true),
        (ValueType::Scalar("Guid".into()), true, true),
        (ValueType::Scalar("Status".into()), true, true),
        (ValueType::Scalar("unknown".into()), false, true),
        (ValueType::Named("copied".into()), true, true),
        (ValueType::Named("owned".into()), true, false),
        (ValueType::Named("live".into()), false, false),
        (ValueType::Named("missing".into()), false, false),
        (ValueType::Object("I".into()), false, false),
        (ValueType::WeakObject("I".into()), false, false),
        (ValueType::WeakBoxObject("I".into()), false, false),
    ] {
        assert_eq!((graph.can_persist(&leaf), graph.is_copied(&leaf)), (persisted, copied), "{leaf:?}");
        let array = ValueType::Array(Box::new(leaf), 0);
        assert_eq!((graph.can_persist(&array), graph.is_copied(&array)), (persisted, copied));
        let sequence = ValueType::Sequence(Box::new(array));
        assert_eq!((graph.can_persist(&sequence), graph.is_copied(&sequence)), (persisted, false));
    }
}

/// Array edges count for layout, while a sequence terminates inline reachability.
#[test]
fn inline_references_stop_at_indirection_and_keep_field_order() {
    let value = ValueDefinition {
        name: "Owner".into(),
        copied: false,
        persistence: Persistence::Supported,
        fields: vec![
            ValueType::Array(Box::new(ValueType::Named("Second".into())), 2),
            ValueType::Sequence(Box::new(ValueType::Named("Hidden".into()))),
            ValueType::Named("First".into()),
        ],
    };
    assert_eq!(value.inline_references(), ["Second", "First"]);
}

/// Private explanation still handles missing names and every unsupported leaf.
#[test]
fn unsupported_leaf_reasons_remain_distinct() {
    let graph = ValueGraph::default();
    for (ty, prefix) in [
        (ValueType::Object("I".into()), "ComPtr<I> transports an existing object"),
        (ValueType::WeakObject("I".into()), "WeakComPtr<I> transports lifetime bookkeeping"),
        (ValueType::WeakBoxObject("I".into()), "WeakComBox<I> transports exclusive lifetime bookkeeping"),
        (
            ValueType::Scalar("unknown".into()),
            "unknown has no format-neutral ComDeserialize implementation",
        ),
        (ValueType::Named("missing".into()), "missing has no resolved value definition"),
    ] {
        assert!(graph.persistence_failure(&ValueType::Array(Box::new(ty), 1)).unwrap().starts_with(prefix));
    }
    assert_eq!(graph.persistence_failure(&ValueType::Text), None);
}

/// Borrowed public queries cannot assume that a detached caller used the parser.
/// The fixture's own ordinary recursive Drop is outside this query assertion:
/// here a local dismantler releases one moved wrapper at a time after the query.
#[cfg(not(miri))]
#[test]
fn detached_capability_queries_use_constant_native_stack() {
    use std::process::Command;
    const SELECT: &str = "NEOCOGI_IDL_DETACHED_QUERY_CHILD";
    const TEST: &str = "values::tests::detached_capability_queries_use_constant_native_stack";
    if std::env::var_os(SELECT).is_none() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", TEST, "--nocapture", "--test-threads=1"]).env(SELECT, "1");
        let output = crate::bounded_child::run(command);
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    #[cfg(target_os = "linux")]
    crate::bounded_child::cap();
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let mut graph = ValueGraph::default();
            graph.definitions.insert(
                "cached".into(),
                ValueDefinition {
                    name: "cached".into(),
                    fields: Vec::new(),
                    copied: true,
                    persistence: Persistence::Supported,
                },
            );
            for (leaf, expected) in [
                (ValueType::Scalar("bool".into()), (true, true)),
                (ValueType::Named("cached".into()), (true, true)),
                (ValueType::Named("missing".into()), (false, false)),
                (ValueType::Object("I".into()), (false, false)),
            ] {
                let mut ty = leaf;
                for _ in 0..32768 {
                    ty = ValueType::Array(Box::new(ty), 1);
                }
                assert_eq!((graph.can_persist(&ty), graph.is_copied(&ty)), expected);
                ty = ValueType::Sequence(Box::new(ty));
                assert_eq!((graph.can_persist(&ty), graph.is_copied(&ty)), (expected.0, false));
                while let ValueType::Array(child, _) | ValueType::Sequence(child) = ty {
                    ty = *child;
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
