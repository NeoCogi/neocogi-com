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

/// Constructs shallow field expressions whose named dependencies form a long chain.
fn chain(count: usize, tail: ValueType, sequence: bool) -> ValueGraph {
    let mut graph = ValueGraph::default();
    for index in 0..count {
        let name = format!("Item{index}");
        let field = if index + 1 == count {
            tail.clone()
        } else {
            let edge = ValueType::Named(format!("Item{}", index + 1));
            if sequence {
                ValueType::Sequence(Box::new(edge))
            } else {
                ValueType::Array(Box::new(edge), 1)
            }
        };
        graph.definitions.insert(
            name.clone(),
            ValueDefinition {
                name,
                fields: vec![field],
                copied: true,
                persistence: Persistence::Supported,
            },
        );
    }
    graph
}

/// The rejecting tail visits each reverse edge once and the scalar tail schedules nothing.
#[test]
fn capability_work_follows_affected_edges() {
    let count = 1_000;
    for (capability, tail, sequence, supported) in [
        (Capability::Persistent, ValueType::Object("I".into()), true, false),
        (Capability::Persistent, ValueType::Scalar("u32".into()), true, true),
        (Capability::Copied, ValueType::Text, false, false),
    ] {
        let mut graph = chain(count, tail, sequence);
        let mut work = PropagationWork::default();
        graph.resolve_capability(capability, Some(&mut work));
        assert!(graph.definitions.values().all(|definition| capability.supported(definition) == supported));
        assert_eq!(work.scheduled, if supported { 0 } else { count });
        assert_eq!(work.edges, if supported { 0 } else { count - 1 });
    }
}

/// A cycle stays persistent until one of its leaves is unsupported. A dependent
/// with two paths to the same failure still schedules only once.
#[test]
fn capability_cycles_follow_seeds_without_requeuing_dependents() {
    let mut graph = chain(3, ValueType::Sequence(Box::new(ValueType::Named("Item0".into()))), true);
    graph.definitions.get_mut("Item0").unwrap().fields.push(ValueType::Named("Item2".into()));
    graph.resolve_capabilities();
    assert!(
        graph
            .definitions
            .values()
            .all(|definition| definition.persistence == Persistence::Supported && !definition.copied)
    );
    graph.definitions.get_mut("Item2").unwrap().fields.push(ValueType::WeakObject("I".into()));
    let mut work = PropagationWork::default();
    graph.resolve_capability(Capability::Persistent, Some(&mut work));
    assert!(graph.definitions.values().all(|definition| definition.persistence == Persistence::Unsupported));
    assert_eq!(work.scheduled, 3);
    assert_eq!(work.edges, 4);
}
