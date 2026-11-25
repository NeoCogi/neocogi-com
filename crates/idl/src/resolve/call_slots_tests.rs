//! Bound method shape, source projection, and canonical native identity.
use std::path::Path;

use super::*;
use crate::{IdlType, parse_module, resolve_modules};

/// Resolves all variants under the same nominal module identity.
fn resolve(source: &str) -> Result<ResolvedModules, ModuleGraphError> {
    let module = parse_module(Path::new("proof/loans.ncidl"), source).unwrap();
    resolve_modules(vec![module])
}

/// Gets the actual methods of the named closed declaration.
fn methods<'a>(graph: &'a ResolvedModules, name: &str) -> &'a [Method] {
    let DeclarationKind::Interface(interface) = &graph.module("proof.loans").unwrap().source.declaration(name).unwrap().kind else {
        panic!("interface");
    };
    &interface.methods
}

/// A position closes to one passing rule. Storage continues to use ordinary enums.
#[test]
fn standard_optional_root_uses_directional_passing() {
    let graph = resolve(
        "interface ISource {
            fn receive(&self, read: &INcString, write: &mut ISource, optional: Option<&INcString>, text: &str);
            fn echo(&self, value: Option<u32>) -> Result<Option<u32>>;
            fn child(&self) -> Option<ComPtr<INcString>>;
            fn loan(&self) -> Result<neocogi::lang::Option<&INcString>>;
        }
        struct Stored { owner: Option<ComPtr<INcString>> }",
    )
    .unwrap();
    let [receive, echo, child, loan] = methods(&graph, "ISource") else {
        panic!("method count")
    };
    assert_eq!(
        receive.parameters.iter().map(|parameter| parameter.passing).collect::<Vec<_>>(),
        [
            ParameterPassing::SharedLoan { optional: false },
            ParameterPassing::MutableLoan,
            ParameterPassing::SharedLoan { optional: true },
            ParameterPassing::BorrowedText,
        ]
    );
    assert!(matches!(receive.parameters[2].payload(), IdlType::Named(_)));
    assert_eq!(
        (receive.return_passing, receive.return_projection),
        (ReturnPassing::Unit, ReturnProjection::Direct)
    );
    assert_eq!(echo.parameters[0].passing, ParameterPassing::Copied);
    assert_eq!(echo.return_passing, ReturnPassing::Copied);
    assert_eq!(
        (child.return_passing, child.return_projection),
        (ReturnPassing::SharedOwner { optional: true }, ReturnProjection::Direct)
    );
    assert_eq!(
        (loan.return_passing, loan.return_projection),
        (ReturnPassing::ScopedLoan, ReturnProjection::Fallible)
    );
    assert_eq!(
        graph
            .module("proof.loans")
            .unwrap()
            .source
            .declarations
            .iter()
            .filter(|declaration| declaration.specialization.is_some())
            .count(),
        2,
        "only the ordinary scalar and stored-owner Option values specialize"
    );
}

/// Following the standard spelling remains a lexical decision, including in templates.
#[test]
fn optional_slot_resolution_preserves_shadowing_and_closes_after_substitution() {
    let graph = resolve(
        "enum Option<T> { Empty, Item(T) }
        interface ILookup<T> { fn get(&self) -> Result<neocogi::lang::Option<T>>; }
        interface IGeneric<T> { fn get(&self) -> Result<T>; }
        interface IBuffer<T> { fn fill(&self, output: &mut T); }
        interface ILocal { fn get(&self) -> Result<Option<u32>>; }
        struct Paths {
            child: ComPtr<ILookup<ComPtr<INcString>>>,
            loan: ComPtr<ILookup<&INcString>>,
            scalar: ComPtr<ILookup<u32>>,
            whole: ComPtr<IGeneric<neocogi::lang::Option<ComPtr<INcString>>>>,
            buffers: ComPtr<IBuffer<Slice<u8>>>,
        }",
    )
    .unwrap();
    assert_eq!(methods(&graph, "ILocal")[0].return_passing, ReturnPassing::Copied);
    let actual = graph.module("proof.loans").unwrap().source.declarations.iter().filter_map(|declaration| {
        let DeclarationKind::Interface(interface) = &declaration.kind else {
            return None;
        };
        let specialization = declaration.specialization.as_ref()?;
        Some((specialization.template.segments[0].as_str(), &interface.methods[0]))
    });
    let mut lookup = actual
        .clone()
        .filter(|(template, _)| *template == "ILookup")
        .map(|(_, method)| method.return_passing)
        .collect::<Vec<_>>();
    assert_eq!(lookup.len(), 3);
    for expected in [ReturnPassing::SharedOwner { optional: true }, ReturnPassing::ScopedLoan, ReturnPassing::Copied] {
        let index = lookup.iter().position(|passing| *passing == expected).unwrap();
        lookup.swap_remove(index);
    }
    assert_eq!(
        actual.clone().find(|(template, _)| *template == "IGeneric").unwrap().1.return_passing,
        ReturnPassing::SharedOwner { optional: true }
    );
    assert_eq!(
        actual.clone().find(|(template, _)| *template == "IBuffer").unwrap().1.parameters[0].passing,
        ParameterPassing::OutputBuffer
    );
}

/// A retained generic argument cannot carry the bundled optional-loan wrapper.
#[test]
fn wrapped_generic_loans_report_the_argument_using_bound_option_identity() {
    for (import, argument) in [
        ("", "ComPtr<ILookup<Option<&INcString>>>"),
        ("use neocogi.lang::Option;", "Phantom<Option<&ILoan>>"),
        ("", "Phantom<Vec<neocogi::lang::Option<&ILoan>>>"),
    ] {
        let source = format!(
            "{import}\ninterface ILoan {{}}\ninterface ILookup<T> {{ fn get(&self) -> Result<T>; }}
             struct Phantom<T> {{ n: u32 }}\nstruct Use {{ value: {argument} }}"
        );
        let ModuleGraphError::Resolution(error) = resolve(&source).unwrap_err() else {
            panic!("expected a resolution diagnostic: {source}");
        };
        let [diagnostic] = error.diagnostics.as_slice() else {
            panic!("expected one argument diagnostic: {error}");
        };
        assert_eq!(diagnostic.code, "EIDL102", "{error}");
        assert_eq!(diagnostic.labels[0].span.path(), "proof/loans.ncidl");
        assert_eq!(diagnostic.labels[0].span.start().line, 5);
        assert!(error.to_string().contains("specialize T with &I"), "{error}");
    }
    for argument in ["Option<&str>", "Option<&mut ILoan>", "Option<ComBox<ILoan>>"] {
        let source = format!("interface ILoan {{}} struct Phantom<T> {{ n: u32 }} struct Use {{ field: Phantom<{argument}> }}");
        let error = resolve(&source).unwrap_err().to_string();
        assert!(!error.contains("EIDL102"), "{error}");
    }
    let shadowed = "interface ILoan {} enum Option<T> { Empty, Some(T) }
        struct Phantom<T> { n: u32 } struct Use { field: Phantom<Option<&ILoan>> }";
    assert!(!resolve(shadowed).unwrap_err().to_string().contains("EIDL102"));
}

/// Imported templates still resolve the bundled option after transitive substitution.
#[test]
fn transitive_specialization_keeps_optional_loan_slots() {
    let leaf = parse_module(
        Path::new("proof/leaf.ncidl"),
        "use neocogi.lang::Option;
        interface ILookup<T> {
            fn read(&self, value: Option<T>) -> Result<Option<T>>;
        }",
    )
    .unwrap();
    let bridge = parse_module(
        Path::new("proof/bridge.ncidl"),
        "extern proof.leaf; use proof.leaf::ILookup; interface IWrapped<T>: ILookup<T> {}",
    )
    .unwrap();
    let consumer = parse_module(
        Path::new("proof/loans.ncidl"),
        "extern proof.bridge; use proof.bridge::IWrapped; struct Root { lookup: ComPtr<IWrapped<&INcString>> }",
    )
    .unwrap();
    let graph = resolve_modules(vec![leaf, bridge, consumer]).unwrap();
    let instance = graph
        .module("proof.loans")
        .unwrap()
        .source
        .declarations()
        .iter()
        .find(|decl| decl.specialization.as_ref().is_some_and(|origin| origin.template.segments == ["ILookup"]))
        .unwrap();
    let method = &methods(&graph, &instance.name)[0];
    assert_eq!(method.parameters[0].passing, ParameterPassing::SharedLoan { optional: true });
    assert_eq!(method.return_passing, ReturnPassing::ScopedLoan);
}

/// The admitted subset leaves no escaping borrow or lost consuming owner.
#[test]
fn invalid_direct_and_scoped_forms_reject_before_generation() {
    for source in [
        "interface IService { fn get(&self) -> u32; }",
        "interface IService { fn get(&self) -> ComPtr<INcString>; }",
        "interface IService { fn get(&self) -> Option<u32>; }",
        "interface IService { fn get(&self) -> Result<Option<ComBox<INcString>>>; }",
        "interface IService { fn get(&self) -> Result<Option<WeakComPtr<INcString>>>; }",
        "interface IService { fn get(&self) -> Result<&INcString>; }",
        "interface IService { fn get(&mut self) -> Option<&INcString>; }",
        "interface IService { fn get(&self, bytes: &mut [u8]) -> Result<Option<&INcString>>; }",
        "interface IService { fn get(&self, child: ComBox<INcString>) -> Result<Option<&INcString>>; }",
        "interface IService { fn accept(&self, child: Option<&mut INcString>); }",
        "interface IService { fn accept(&self, child: &mut u32); }",
        "interface IService { fn accept(&self, child: &mut str); }",
        "interface IService { fn get(&self) -> Option<&str>; }",
        "struct Data { n: u32 } interface IService { fn accept(&self, child: Option<&Data>); }",
        "interface IService { fn accept(&self, child: ComPtr<INcString>); }",
        "interface IService { fn accept(&self, child: ComBox<INcString>); }",
        "interface IService<T> { fn get(&self, child: T) -> Option<ComPtr<INcString>>; }
         struct Use { value: ComPtr<IService<ComBox<INcString>>> }",
        "enum Option<T> { Empty, Some(T) } interface IService { fn get(&self) -> Result<Option<&INcString>>; }",
        "struct Store { value: Option<&INcString> }",
    ] {
        let error = resolve(source).expect_err(source).to_string();
        assert!(!error.contains("EIDL099") && !error.contains("EIDL102"), "{source}: {error}");
    }
}

/// The marker helper and an ordinary inherited method occupy distinct Rust namespaces.
#[test]
fn scoped_getter_coexists_with_inherited_with_prefix_method() {
    let graph = resolve(
        "interface IBase { fn with_caption(&self) -> Result<u32>; }
         interface IDerived: IBase { fn caption(&self) -> Option<&INcString>; }",
    )
    .unwrap();
    assert_eq!(methods(&graph, "IDerived")[0].return_passing, ReturnPassing::ScopedLoan);
}

/// Only native slot differences enter the interface and specialization identities.
#[test]
fn native_identity_distinguishes_slots_but_ignores_rust_status_projection() {
    let identities = |method: &str| {
        let graph = resolve(&format!("interface IService {{ {method} }}")).unwrap();
        let declaration = graph.module("proof.loans").unwrap().declaration("IService").unwrap();
        (declaration.id, declaration.abi_signature, declaration.rust_binding)
    };
    let omitted = identities("fn get(&self);");
    let explicit_unit = identities("fn get(&self) -> ();");
    assert_eq!(omitted, explicit_unit);
    for (plain, checked) in [
        ("fn get(&self);", "fn get(&self) -> Result<()>;"),
        (
            "fn get(&self) -> Option<ComPtr<INcString>>;",
            "fn get(&self) -> Result<neocogi::lang::Option<ComPtr<INcString>>>;",
        ),
        ("fn get(&self) -> Option<&INcString>;", "fn get(&self) -> Result<Option<&INcString>>;"),
    ] {
        let plain = identities(plain);
        let checked = identities(checked);
        assert_eq!((plain.0, plain.1), (checked.0, checked.1));
        assert_ne!(plain.2, checked.2);
    }
    let differing = [
        identities("fn get(&self, value: &INcString) -> Result<()>;"),
        identities("fn get(&self, value: &mut INcString) -> Result<()>;"),
        identities("fn get(&self, value: Option<&INcString>) -> Result<()>;"),
        identities("fn get(&self) -> Result<ComPtr<INcString>>;"),
        identities("fn get(&self) -> Result<Option<ComPtr<INcString>>>;"),
        identities("fn get(&self) -> Result<Option<&INcString>>;"),
    ];
    for first in 0..differing.len() {
        for second in first + 1..differing.len() {
            assert_ne!(differing[first].1, differing[second].1);
        }
    }
    let direct = resolve("interface ILookup<T> { fn get(&self) -> Option<T>; } struct Use { item: ComPtr<ILookup<ComPtr<INcString>>> }").unwrap();
    let checked = resolve("interface ILookup<T> { fn get(&self) -> Result<Option<T>>; } struct Use { item: ComPtr<ILookup<ComPtr<INcString>>> }").unwrap();
    let closed = |graph: &ResolvedModules| {
        let module = graph.module("proof.loans").unwrap();
        let instance = module
            .source
            .declarations
            .iter()
            .find(|declaration| matches!(declaration.kind, DeclarationKind::Interface(_)))
            .unwrap();
        let declaration = module.declaration(&instance.name).unwrap();
        (declaration.id, declaration.abi_signature, declaration.rust_binding)
    };
    let direct = closed(&direct);
    let checked = closed(&checked);
    assert_eq!((direct.0, direct.1), (checked.0, checked.1));
    assert_ne!(direct.2, checked.2);
}
