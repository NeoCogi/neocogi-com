//! Parameter spelling is local syntax; its ordinal determines template meaning.
//!
//! Resolve before/after sources under identical logical module names so equality
//! proves signature stability, rather than accidentally comparing different types.
use std::path::Path;
use neocogi_idl::{ResolvedModules, parse_module, resolve_modules};

/// Resolves one template library and a separate consumer without sharing compiler state.
fn graph(templates: &str, consumer: &str) -> ResolvedModules {
    resolve_modules(vec![
        parse_module(Path::new("positions/library.ncidl"), templates).unwrap(),
        parse_module(
            Path::new("positions/consumer.ncidl"),
            &format!("extern positions.library; #[version(1)] interface IAnchor {{}} {consumer}"),
        )
        .unwrap(),
    ])
    .unwrap()
}

/// Compares module revisions and all concrete declaration identities/signatures.
/// Source spans and parameter names deliberately remain different in these graphs.
fn assert_same_contract(before: &ResolvedModules, after: &ResolvedModules) {
    for left in before.modules() {
        let right = after.module(&left.source.name.to_string()).unwrap();
        assert_eq!(left.abi_fingerprint, right.abi_fingerprint, "module {}", left.source.name);
        assert_eq!(left.declarations, right.declarations, "declarations in {}", left.source.name);
    }
    assert_eq!(before.modules().len(), after.modules().len());
    assert_eq!(before.values, after.values, "resolved value metadata must remain identical");
}

/// Renaming parameters is harmless through every supported aggregate/type position.
#[test]
fn parameter_renames_preserve_nested_records_and_enum_payloads() {
    let before = r#"
        #[version(1)] struct Node<T, I> {
            value: T, children: Vec<Node<T, I>>, slots: [Option<T>; 2], object: ComPtr<I>
        }
        #[version(1)] enum Change<T, I>: u8 {
            Empty, Tuple(T, Node<T, I>), Named { value: Option<T>, object: ComPtr<I> }
        }
    "#;
    let after = r#"
        #[version(1)] struct Node<Item, Capability> {
            value: Item, children: Vec<Node<Item, Capability>>, slots: [Option<Item>; 2], object: ComPtr<Capability>
        }
        #[version(1)] enum Change<Value, Object>: u8 {
            Empty, Tuple(Value, Node<Value, Object>), Named { value: Option<Value>, object: ComPtr<Object> }
        }
    "#;
    let consumer = "use positions.library::Change; #[version(1)] struct Payload { event: Change<bool, IUnknown> }";
    assert_same_contract(&graph(before, consumer), &graph(after, consumer));
}

/// Bases, borrowed/owned method positions, and both buffer directions share the rule.
#[test]
fn parameter_renames_preserve_interface_signatures() {
    let before = r#"
        #[version(1)] struct Payload { text: String }
        #[version(1)] interface IBase<T> { fn read(&self) -> Result<T>; }
        #[version(1)] interface IBuffer<T>: IBase<T> {
            fn accept(&self, values: &[T]) -> Result<()>;
            fn fill(&mut self, values: &mut [T]) -> Result<()>;
        }
        #[version(1)] interface IInspect<T, S> {
            fn inspect(&self, value: &T) -> Result<()>;
            fn items(&self, values: &S) -> Result<()>;
        }
    "#;
    let after = r#"
        #[version(1)] struct Payload { text: String }
        #[version(1)] interface IBase<ResultType> { fn read(&self) -> Result<ResultType>; }
        #[version(1)] interface IBuffer<Element>: IBase<Element> {
            fn accept(&self, values: &[Element]) -> Result<()>;
            fn fill(&mut self, values: &mut [Element]) -> Result<()>;
        }
        #[version(1)] interface IInspect<Value, Sequence> {
            fn inspect(&self, value: &Value) -> Result<()>;
            fn items(&self, values: &Sequence) -> Result<()>;
        }
    "#;
    let consumer = r#"
        use positions.library::{IBuffer, IInspect, Payload};
        #[version(1)] class Object: IBuffer<u32> + IInspect<Payload, Slice<u32>> {}
    "#;
    assert_same_contract(&graph(before, consumer), &graph(after, consumer));
}

/// A qualified declaration remains a free type even when its last segment names a parameter.
#[test]
fn parameter_renames_preserve_lexical_binding_and_source_names() {
    let before = r#"
        #[version(1)] struct Local { value: u32 }
        #[version(1)] struct Record<T> { value: T, other: positions::library::Local }
    "#;
    let after = r#"
        #[version(1)] struct Local { value: u32 }
        #[version(1)] struct Record<Local> { value: Local, other: positions::library::Local }
    "#;
    let consumer = "use positions.library::Record; #[version(1)] struct Payload { value: Record<bool> }";
    let first = graph(before, consumer);
    let second = graph(after, consumer);
    assert_same_contract(&first, &second);
    // Fingerprint normalization must not rewrite the source model used for diagnostics.
    assert_eq!(first.module("positions.library").unwrap().source.templates[0].parameters, ["T"]);
    assert_eq!(second.module("positions.library").unwrap().source.templates[0].parameters, ["Local"]);
}

/// Unused/open templates still contribute parameter arity, but never parameter spelling.
#[test]
fn unused_parameter_names_are_ignored_but_arity_is_preserved() {
    let first = graph("#[version(1)] struct Marker<T> { value: u32 }", "");
    let renamed = graph("#[version(1)] struct Marker<Unused> { value: u32 }", "");
    assert_same_contract(&first, &renamed);
    let extra_parameter = graph("#[version(1)] struct Marker<T, U> { value: u32 }", "");
    assert_ne!(
        first.module("positions.library").unwrap().abi_fingerprint,
        extra_parameter.module("positions.library").unwrap().abi_fingerprint
    );
}

/// Using another parameter changes the template even when both supplied types are equal.
#[test]
fn parameter_positions_remain_significant_with_identical_concrete_arguments() {
    for (before, after, consumer) in [
        (
            "#[version(1)] struct Pair<A, B> { first: A, second: B }",
            "#[version(1)] struct Pair<A, B> { first: B, second: A }",
            "use positions.library::Pair; #[version(1)] struct Payload { value: Pair<u32, u32> }",
        ),
        (
            "#[version(1)] struct Pair<A, B> { first: A, second: B }",
            "#[version(1)] struct Pair<B, A> { first: A, second: B }",
            "use positions.library::Pair; #[version(1)] struct Payload { value: Pair<u32, u32> }",
        ),
        (
            "#[version(1)] struct Leaf<A, B> { first: A, second: B } #[version(1)] struct Pair<A, B> { value: Vec<Option<Leaf<A, B>>> }",
            "#[version(1)] struct Leaf<A, B> { first: A, second: B } #[version(1)] struct Pair<A, B> { value: Vec<Option<Leaf<B, A>>> }",
            "use positions.library::Pair; #[version(1)] struct Payload { value: Pair<u32, u32> }",
        ),
        (
            "#[version(1)] enum Choice<A, B> { First(A), Second { value: B } }",
            "#[version(1)] enum Choice<A, B> { First(B), Second { value: A } }",
            "use positions.library::Choice; #[version(1)] struct Payload { value: Choice<u32, u32> }",
        ),
        (
            "#[version(1)] interface IValue<A, B> { fn value(&self) -> Result<A>; }",
            "#[version(1)] interface IValue<A, B> { fn value(&self) -> Result<B>; }",
            "use positions.library::IValue; #[version(1)] class Object: IValue<u32, u32> {}",
        ),
    ] {
        let first = graph(before, consumer);
        let changed = graph(after, consumer);
        let first = first.module("positions.consumer").unwrap();
        let changed = changed.module("positions.consumer").unwrap();
        let index = first
            .source
            .declarations
            .iter()
            .position(|value| {
                value
                    .specialization
                    .as_ref()
                    .is_some_and(|origin| matches!(origin.template.segments.last().map(String::as_str), Some("Pair" | "Choice" | "IValue")))
            })
            .unwrap();
        assert_eq!(first.declarations[index].id, changed.declarations[index].id, "nominal type is unchanged");
        assert_ne!(first.declarations[index].abi_signature, changed.declarations[index].abi_signature, "{before}");
    }
}

/// Field/variant names and member order are contract data, not generic binding names.
#[test]
fn parameter_normalization_preserves_member_contract_changes() {
    for (before, after) in [
        ("struct Record<T> { first: T, second: u32 }", "struct Record<T> { second: u32, first: T }"),
        ("struct Record<T> { value: T }", "struct Record<T> { renamed: T }"),
        ("struct Record<T> { value: [T; 2] }", "struct Record<T> { value: [T; 3] }"),
        ("enum Record<T>: u8 { Zero = 0 }", "enum Record<T>: u8 { Zero = 7 }"),
        ("enum Record<T>: u8 { Value(T) }", "enum Record<T>: u8 { Renamed(T) }"),
    ] {
        let first = graph(&format!("#[version(1)] {before}"), "");
        let changed = graph(&format!("#[version(1)] {after}"), "");
        assert_ne!(
            first.module("positions.library").unwrap().abi_fingerprint,
            changed.module("positions.library").unwrap().abi_fingerprint,
            "{before} versus {after}"
        );
    }
}
