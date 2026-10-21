//! Executable naming contracts across normal and printed production expansions.
//!
//! These consumers deliberately give application operations plausible wrong
//! values and incompatible types. Compilation alone cannot detect a same-typed
//! replacement; actual codec budgets, metadata and COM calls establish ownership.

use crate as neocogi_idl;
use std::{fs, process::Command};

use proc_macro2::TokenStream;
use quote::quote;

/// Builds ambient constants from the actual generated bindings, excluding every
/// authored identifier. This keeps the regression effective when private names
/// change or new bindings are added; consumer compilation checks lexical scope.
fn ambient_binding_constants(authored: &str, generated: &syn::File) -> String {
    use std::collections::BTreeSet;
    use syn::visit::Visit;

    /// Pattern bindings in the emitted Rust, independent of the name allocator.
    #[derive(Default)]
    struct Bindings(
        /// Deterministically ordered binding spellings, including raw identifiers.
        BTreeSet<String>,
    );
    impl<'ast> Visit<'ast> for Bindings {
        /// Includes bindings nested in destructuring and closure/function parameters.
        fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
            self.0.insert(pattern.ident.to_string());
            syn::visit::visit_pat_ident(self, pattern);
        }
    }
    /// Collects authored syntax before any implementation scope exists.
    fn identifiers(tokens: TokenStream, names: &mut BTreeSet<String>) {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Ident(ident) => {
                    names.insert(ident.to_string());
                }
                proc_macro2::TokenTree::Group(group) => identifiers(group.stream(), names),
                _ => {}
            }
        }
    }
    let mut authored_names = BTreeSet::new();
    identifiers(authored.parse().unwrap(), &mut authored_names);
    let mut bindings = Bindings::default();
    bindings.visit_file(generated);
    bindings
        .0
        .into_iter()
        .filter(|name| name != "self" && !authored_names.contains(name))
        .map(|name| format!("#[allow(dead_code, non_upper_case_globals)] const {name}: usize = 97;\n"))
        .collect()
}

/// Runs several complete source selections in one real SDK consumer.
/// The scratch target is removed with the consumer after every child has exited.
fn run_consumers(name: &str, cases: impl IntoIterator<Item = (String, String)>) {
    run_consumers_with_files(name, cases, &[]);
}

/// Additional integration targets compile as separate crates, so public wires
/// must remain usable without access to the native field-resolution scope.
fn run_consumers_with_files(name: &str, cases: impl IntoIterator<Item = (String, String)>, files: &[(&str, &str)]) {
    let consumer = super::test_consumer::Consumer::new(name);
    for (path, source) in files {
        let path = consumer.directory.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    for (label, source) in cases {
        fs::write(consumer.directory.path().join("src/lib.rs"), source).unwrap();
        let result = Command::new("cargo")
            .args(["test", "--offline", "--manifest-path"])
            .arg(consumer.directory.path().join("Cargo.toml"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{label}:\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

/// Expands top-level production attributes while leaving authored Rust intact.
/// Tests wrap these items in local blocks separately to cover non-module scope.
fn source_items(source: &str, expand: bool) -> String {
    let mut output = TokenStream::new();
    for item in syn::parse_file(source).unwrap().items {
        let tokens = match item {
            syn::Item::Struct(mut item) => {
                let mut serialize = false;
                let mut deserialize = false;
                let mut attributes = Vec::new();
                for attribute in item.attrs {
                    if attribute.path().is_ident("derive") {
                        let paths = attribute
                            .parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                            .unwrap();
                        let paths = paths
                            .into_iter()
                            .filter(|path| {
                                if path.segments.last().unwrap().ident == "ComSerialize" {
                                    serialize = true;
                                    false
                                } else if path.segments.last().unwrap().ident == "ComDeserialize" {
                                    deserialize = true;
                                    false
                                } else {
                                    true
                                }
                            })
                            .collect::<Vec<_>>();
                        if !paths.is_empty() {
                            attributes.push(syn::parse_quote!(#[derive(#(#paths),*)]));
                        }
                    } else {
                        attributes.push(attribute);
                    }
                }
                item.attrs = attributes;
                let serialization = if serialize {
                    neocogi_com_codegen::expand_com_serialize_derive(syn::parse2(quote! { #item }).unwrap()).unwrap()
                } else {
                    TokenStream::new()
                };
                let deserialization = if deserialize {
                    neocogi_com_codegen::expand_com_deserialize_derive(syn::parse2(quote! { #item }).unwrap()).unwrap()
                } else {
                    TokenStream::new()
                };
                let index = item
                    .attrs
                    .iter()
                    .position(|attribute| attribute.path().segments.last().unwrap().ident == "com_object");
                let definition = match index {
                    Some(index) => {
                        let attribute = item.attrs.remove(index);
                        neocogi_com_codegen::expand_com_object(attribute.parse_args().unwrap(), item)
                    }
                    None => quote! { #item },
                };
                quote! { #definition #serialization #deserialization }
            }
            syn::Item::Fn(mut item) => match item
                .attrs
                .iter()
                .position(|attribute| attribute.path().segments.last().unwrap().ident == "plugin")
            {
                Some(index) => {
                    let arguments = item.attrs.remove(index).parse_args().unwrap();
                    neocogi_com_codegen::expand_plugin(arguments, item).unwrap()
                }
                None => quote! { #item },
            },
            item => quote! { #item },
        };
        output.extend(tokens);
    }
    let generated: syn::File = syn::parse2(output).unwrap();
    let ambient = ambient_binding_constants(source, &generated);
    let definitions = if expand { prettyplease::unparse(&generated) } else { source.to_owned() };
    format!("{definitions}\n{ambient}")
}

/// Compiles a genuine declaration source, independent from implementation Rust.
pub(crate) fn contract(source: &str) -> String {
    let module = neocogi_idl::parse_module(std::path::Path::new("test/naming.ncidl"), source).unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    neocogi_idl::generate_rust(&graph, "test.naming").unwrap()
}

/// Concrete generic object binders remain private in both implementation modes.
#[test]
fn object_bindings_preserve_owned_generic_state_after_printing() {
    let bindings = contract("#[version(1)] interface IValue { fn value(&self) -> Result<u32>; fn replace(&mut self, value: u32) -> Result<()>; }");
    let implementations = r#"
        #[neocogi_com::com_object(interfaces(IValue))]
        struct Object<const value: usize, const contracts: usize, const __nc_initial_value: usize> { saved: u32 }
        #[neocogi_com::com_object(interfaces(IValue), ownership = exclusive)]
        struct Exclusive<const value: usize, const contracts: usize, const __nc_initial_value: usize> { saved: u32 }
        macro_rules! implement {
            ($object:ident) => {
                impl<const value: usize, const contracts: usize, const __nc_initial_value: usize> $object<value, contracts, __nc_initial_value> { const COM_VTABLES: u8 = 91; }
                impl<const value: usize, const contracts: usize, const __nc_initial_value: usize> IValue for $object<value, contracts, __nc_initial_value> {
                    fn value(&self) -> Result<u32, neocogi_com::NcStatus> { Ok(self.saved) }
                    fn replace(&mut self, replacement: u32) -> Result<(), neocogi_com::NcStatus> { self.saved=replacement; Ok(()) }
                }
            };
        }
        implement!(Object); implement!(Exclusive);
    "#;
    let cases = [false, true].map(|expand| {
        let definitions = source_items(implementations, expand);
        (
            format!("printed={expand}"),
            format!(
                r#"
            #![allow(dead_code, non_upper_case_globals)]
            {bindings}
            {definitions}
            #[test] fn real_shared_and_exclusive_calls() {{
                let mut shared = Object::<1,2,3>::from_value(Object {{saved:73}}).query::<IValueAbi>().unwrap();
                let mut exclusive = Exclusive::<1,2,3>::into_box(Exclusive {{saved:91}}).query::<IValueAbi>().unwrap();
                assert_eq!(shared.value().unwrap(), 73); assert_eq!(exclusive.value().unwrap(), 91);
                shared.replace(17).unwrap(); exclusive.replace(29).unwrap();
                assert_eq!(shared.value().unwrap(),17); assert_eq!(exclusive.value().unwrap(),29);
                assert_eq!(Object::<1,2,3>::COM_VTABLES,91); assert_eq!(Exclusive::<1,2,3>::COM_VTABLES,91);
            }}
        "#
            ),
        )
    });
    run_consumers("object-name-ownership-proof", cases);
}

/// Private descriptor items cannot capture an authored acquisition or dependency
/// path. Both success and failure execute the actual C acquisition adapter.
#[test]
fn plugin_implementation_items_have_private_lexical_ownership() {
    let bindings = contract("#[version(1)] interface IRoot { fn value(&self) -> Result<u32>; }");
    let cases = [false, true].map(|expand| {
        let mut source = String::from("#![allow(dead_code)]\n");
        for (index, identifier) in ["NC_COM_PLUGIN_DESCRIPTOR", "__NC_PROVIDED_GUID", "__NC_DERIVED_GUID", "PROVIDED"]
            .into_iter()
            .enumerate()
        {
            for rejected in [false, true] {
                let acquisition = if rejected {
                    "Err(neocogi_com::NC_E_CLOSED)"
                } else {
                    "Root::from_value(Root).query()"
                };
                let declarations = format!(
                    r#"
                    const {identifier}: neocogi_com::NcGuid = neocogi_com::guid_from_qualified_name("test.naming.Plugin");
                    const NC_COM_PLUGIN_DEPENDENCIES: neocogi_com::NcGuid = neocogi_com::guid_from_qualified_name("test.naming.Dependency");
                    {bindings}
                    #[neocogi_com::com_object(interfaces(IRoot))] struct Root;
                    impl IRoot for Root {{ fn value(&self) -> Result<u32, neocogi_com::NcStatus> {{ Ok(73) }} }}
                    #[neocogi_com::plugin(name = "test.naming.Plugin", plugin_id({identifier}), version = "1.2.3", dependencies = [NC_COM_PLUGIN_DEPENDENCIES])]
                    fn __nc_acquire_plugin_root() -> Result<neocogi_com::ComPtr<IRootAbi>, neocogi_com::NcStatus> {{ {acquisition} }}
                "#
                );
                let declarations = source_items(&declarations, expand);
                source += &format!(
                    r#"
                    mod case_{index}_{rejected} {{
                        {declarations}
                        const DESCRIPTOR: &neocogi_com::NcPluginDescriptor = plugin_descriptor();
                        #[test] fn typed_descriptor_and_acquisition_use_authored_references() {{
                            let descriptor = plugin_descriptor();
                            assert!(::core::ptr::eq(descriptor, DESCRIPTOR));
                            assert!(::core::ptr::eq(descriptor, plugin_descriptor()));
                            assert_eq!(descriptor.plugin_id, {identifier});
                            assert_eq!(descriptor.dependency_count, 1);
                            assert_eq!(unsafe {{ (*descriptor.dependencies).plugin_id }}, NC_COM_PLUGIN_DEPENDENCIES);
                            let root = Root::from_value(Root);
                            // A rejected adapter must overwrite even a non-null sentinel with null.
                            let mut raw = root.as_raw();
                            let status = unsafe {{ descriptor.acquire.unwrap()(&mut raw) }};
                            if {rejected} {{
                                assert_eq!(status, neocogi_com::NC_E_CLOSED);
                                assert!(raw.is_null());
                            }} else {{
                                assert!(status.is_ok());
                                let acquired = unsafe {{ neocogi_com::ComPtr::<neocogi_com::IUnknown>::from_raw_owned(raw) }}.unwrap();
                                assert_eq!(acquired.query::<IRootAbi>().unwrap().value().unwrap(), 73);
                            }}
                            assert_eq!(root.query::<IRootAbi>().unwrap().value().unwrap(), 73);
                        }}
                    }}
                "#
                );
            }
        }
        (format!("printed={expand}"), source)
    });
    run_consumers("plugin-item-ownership-proof", cases);
}

/// Authored const expressions stay in their own namespace even when an operation
/// introduces a local with the same historical or private-looking spelling.
#[test]
fn value_and_decoder_locals_do_not_capture_authored_const_expressions() {
    let declaration = r#"
        const __nc_budget: usize = 2;
        const __nc_budget_0: usize = 91;
        const __nc_reader_0: usize = 91;
        const __nc_serializer: usize = 91;
        const __nc_wire: usize = 91;
        const __nc_reader: usize = 2;
        const __nc_fields: usize = 2;
        const __NC_CASE_COST: usize = 2;
        const __nc_require_plain: usize = 2;
        const __nc_require_inline: usize = 2;
        #[derive(neocogi_com::ComSerialize, neocogi_com::ComDeserialize)]
        struct Data {
            first: [u32; __nc_budget],
            second: [u32; __nc_reader],
            third: [u32; __nc_fields],
            fourth: [u32; __NC_CASE_COST],
            fifth: [u32; __nc_require_plain],
            sixth: [u32; __nc_require_inline],
        }
    "#;
    let cases = [false, true].map(|expand| {
        let declaration = source_items(declaration, expand);
        (
            format!("printed={expand}"),
            format!(
                r#"
            #![allow(non_upper_case_globals)]
            {declaration}
            #[test] fn raw_and_persistent_values_preserve_every_array() {{
                let data = Data {{ first:[1,73], second:[2,91], third:[3,17], fourth:[4,29], fifth:[5,43], sixth:[6,57] }};
                let bytes = neocogi_com::binary::to_vec(&data).unwrap();
                let decoded = neocogi_com::binary::from_slice::<Data>(&bytes).unwrap();
                assert_eq!(decoded.second, [2,91]);
                assert_eq!(decoded.third, [3,17]);
                assert_eq!(decoded.fifth, [5,43]);
                assert_eq!(decoded.sixth, [6,57]);
            }}
        "#
            ),
        )
    });
    run_consumers("const-expression-ownership-proof", cases);
}

/// Field types resolve at the application owner before private codec/decoder
/// scopes exist. Opaque type macros and Self retain their original meanings.
#[test]
fn field_types_keep_application_self_and_opaque_macro_scope() {
    let declaration = r#"
        const __nc_reader: usize = 2;
        macro_rules! bytes_for { ($owner:ty) => { [u8; <$owner>::WIDTH] }; }
        macro_rules! ambient_bytes { () => { [u8; __nc_reader] }; }
        #[derive(neocogi_com::ComSerialize, neocogi_com::ComDeserialize)]
        struct Fixed { direct: [u8; Self::WIDTH], opaque: bytes_for!(Self) }
        impl Fixed { const WIDTH: usize = 2; }
        #[derive(neocogi_com::ComSerialize, neocogi_com::ComDeserialize)]
        struct Tree { bytes: bytes_for!(Self), children: Vec<Self> }
        impl Tree { const WIDTH: usize = 2; }
        // Persistence alone must not require any live COM value contract.
        #[derive(neocogi_com::ComSerialize, neocogi_com::ComDeserialize)]
        struct Persistent { bytes: ambient_bytes!() }
        #[derive(neocogi_com::ComSerialize, neocogi_com::ComDeserialize)]
        pub struct PublicPersistent { inner: Persistent }
        #[derive(neocogi_com::ComSerialize, neocogi_com::ComDeserialize)]
        pub struct Envelope { inner: Tree, tag: u8 }
        pub fn envelope() -> Envelope {
            Envelope { inner: Tree { bytes: [3, 17], children: vec![] }, tag: 17 }
        }
    "#;
    let cases = [false, true].map(|expand| {
        let declaration = source_items(declaration, expand);
        (
            format!("printed={expand}"),
            format!(
                r#"
                #![allow(non_upper_case_globals)]
                #![deny(private_interfaces)]
                {declaration}
                #[test] fn native_type_identity_survives_conversion_and_persistence() {{
                    let fixed = Fixed {{ direct: [1, 73], opaque: [2, 91] }};
                    let bytes = neocogi_com::binary::to_vec(&fixed).unwrap();
                    let decoded = neocogi_com::binary::from_slice::<Fixed>(&bytes).unwrap();
                    assert_eq!(decoded.opaque, [2, 91]);
                    let tree = Tree {{ bytes: [3, 17], children: vec![Tree {{ bytes: [4, 29], children: vec![] }}] }};
                    let bytes = neocogi_com::binary::to_vec(&tree).unwrap();
                    let decoded = neocogi_com::binary::from_slice::<Tree>(&bytes).unwrap();
                    assert_eq!(decoded.children[0].bytes, [4, 29]);
                    let bytes = neocogi_com::binary::to_vec(&Persistent {{ bytes: [5, 43] }}).unwrap();
                    let decoded = neocogi_com::binary::from_slice::<Persistent>(&bytes).unwrap();
                    assert_eq!(decoded.bytes, [5, 43]);
                    let bytes = neocogi_com::binary::to_vec(&PublicPersistent {{ inner: decoded }}).unwrap();
                    let decoded = neocogi_com::binary::from_slice::<PublicPersistent>(&bytes).unwrap();
                    assert_eq!(decoded.inner.bytes, [5, 43]);
                }}
            "#
            ),
        )
    });
    run_consumers_with_files(
        "field-type-ownership-proof",
        cases,
        &[(
            "tests/public_wire.rs",
            r#"
        #[test] fn public_wires_do_not_require_native_field_visibility() {
            use field_type_ownership_proof::{envelope, Envelope};
            let native = envelope();
            let bytes = neocogi_com::binary::to_vec(&native).unwrap();
            let decoded = neocogi_com::binary::from_slice::<Envelope>(&bytes).unwrap();
            assert_eq!(neocogi_com::binary::to_vec(&decoded).unwrap(), bytes);
        }
    "#,
        )],
    );
}

/// Each supported value family delegates to the same private, checked converter.
#[derive(Clone, Copy, Debug)]
enum ValueFamily {
    /// Copied records can prove their wire layout identical to application layout.
    CopiedRecord,
    /// Owning records retain concrete storage even when their current field is fixed.
    OwnedRecord,
    /// Copied enums validate a tag before examining the active wire alternative.
    CopiedEnum,
    /// Owning enums share that tag/field converter with typed backing storage.
    OwnedEnum,
}

impl ValueFamily {
    /// Compiles the actual owning or copied IDL shape through the production backend.
    fn declaration(self) -> String {
        let retained = if matches!(self, Self::OwnedRecord | Self::OwnedEnum) {
            ", retained: Vec<u8>"
        } else {
            ""
        };
        let declaration = if matches!(self, Self::CopiedEnum | Self::OwnedEnum) {
            format!("enum Record: u8 {{ Ready {{ value: u32{retained} }} }}")
        } else {
            format!("struct Record {{ value: u32{retained} }}")
        };
        contract(&format!(
            "#[version(1)] {declaration} #[version(1)] interface IValue {{ fn observe(&self, value: &Record) -> Result<u32>; }}"
        ))
    }

    /// Constructs the selected native shape with an independently chosen sentinel.
    fn native(self, value: u32) -> String {
        let retained = if matches!(self, Self::OwnedRecord | Self::OwnedEnum) {
            ", retained: vec![]"
        } else {
            ""
        };
        if matches!(self, Self::CopiedEnum | Self::OwnedEnum) {
            format!("Record::Ready {{ value: {value}{retained} }}")
        } else {
            format!("Record {{ value: {value}{retained} }}")
        }
    }

    /// Reads only semantic application data, never a codec's private wire names.
    fn read(self) -> &'static str {
        match self {
            Self::CopiedEnum | Self::OwnedEnum => "match decoded { Record::Ready { value, .. } => value }",
            _ => "decoded.value",
        }
    }
}

/// A namespace trap shared by all value families and both expansion paths.
fn value_case(family: ValueFamily, incompatible: bool) -> String {
    let declaration = family.declaration();
    let native = family.native(73);
    let wrong = family.native(99);
    let read = family.read();
    let cost = if matches!(family, ValueFamily::OwnedRecord | ValueFamily::OwnedEnum) {
        3
    } else {
        2
    };
    let encode_return = if incompatible {
        "u32"
    } else {
        "Result<(<Self as neocogi_com::ComValue>::Wire, <Self as neocogi_com::ComValue>::Storage), neocogi_com::NcStatus>"
    };
    let decode = if incompatible {
        "fn decode_prechecked(_: <Self as neocogi_com::ComValue>::Wire) -> u32 { 99 }".to_owned()
    } else {
        format!("fn decode_prechecked(_: <Self as neocogi_com::ComValue>::Wire) -> Result<Self, neocogi_com::NcStatus> {{ Ok({wrong}) }}")
    };
    format!(
        r#"
        const __nc_budget: usize = 91;
        const __nc_field_0: usize = 91;
        // Old helper spellings must be free application items, including public ones.
        pub struct __RecordValueWire;
        pub struct __RecordValuePayload;
        pub struct __RecordValueStorage;
        pub struct __RecordValueCase0;
        const __nc_Record_abi_signature_bytes: u32 = 91;
        {declaration}
        impl Record {{
            const FIXED_COST: Option<neocogi_com::ValueCost> = Some(neocogi_com::ValueCost::SCALAR);
            const BORROW_WIRE: u32 = 7;
            fn encode_prechecked(&self) -> {encode_return} {{ unreachable!("application encode must not run") }}
            {decode}
        }}
        #[neocogi_com::com_object(interfaces(IValue))]
        struct Object;
        impl IValue for Object {{
            fn observe(&self, value: neocogi_com::NcValueRef<'_, Record>) -> Result<u32, neocogi_com::NcStatus> {{
                let decoded = value.to_owned()?;
                Ok({read})
            }}
        }}
        #[test]
        fn trait_codec_owns_conversion_and_accounting() {{
            let native = {native};
            // A record/alternative plus its u32 requires exactly two values/depth two.
            assert!(<Record as neocogi_com::ComValue>::encode_value(&native, &mut neocogi_com::ValueBudget::new(0, 1, 1)).is_err());
            let (wire, storage) = <Record as neocogi_com::ComValue>::encode_value(&native, &mut neocogi_com::ValueBudget::new(0, {cost}, 2)).unwrap();
            let decoded = unsafe {{ <Record as neocogi_com::ComValue>::decode_value(wire, &mut neocogi_com::ValueBudget::new(0, {cost}, 2)) }}.unwrap();
            assert_eq!({read}, 73);
            drop(storage);
            let owner = Object::from_value(Object).query::<IValueAbi>().unwrap();
            assert_eq!(owner.observe(neocogi_com::NcValueRef::new(&native).unwrap()).unwrap(), 73);
        }}
    "#
    )
}

/// Same-typed wrong results and wrong-return operations must both be irrelevant.
#[test]
fn value_support_has_trait_and_lexical_owners_in_normal_and_printed_code() {
    let cases = [false, true].map(|expand| {
        let mut source = String::from("#![allow(dead_code, non_upper_case_globals, dropping_copy_types)]\n");
        for (index, family) in [
            ValueFamily::CopiedRecord,
            ValueFamily::OwnedRecord,
            ValueFamily::CopiedEnum,
            ValueFamily::OwnedEnum,
        ]
        .into_iter()
        .enumerate()
        {
            for incompatible in [false, true] {
                let body = value_case(family, incompatible);
                let body = source_items(&body, expand);
                source += &format!("mod case_{index}_{incompatible} {{ {body} }}\n");
            }
        }
        (format!("printed={expand}"), source)
    });
    run_consumers("value-name-ownership-proof", cases);
}

/// Associated type spellings and implementation helper stems remain legal values.
#[test]
fn authored_wire_names_are_distinct_from_private_wire_definitions() {
    let cases = [false, true].map(|expand| {
        let first = contract("#[version(1)] struct Wire { value: u32 } #[version(1)] enum Selection: u8 { Wire(Wire), Storage(Wire) }");
        let second = "";
        (
            format!("printed={expand}"),
            format!(
                r#"
            {first} {second}
            #[test] fn associated_wire_type_is_the_only_required_locator() {{
                for native in [Selection::Wire(Wire {{ value: 73 }}), Selection::Storage(Wire {{ value: 91 }})] {{
                    let wire = neocogi_com::encode_inline(&native).unwrap();
                    let decoded = unsafe {{ neocogi_com::decode_inline::<Selection>(wire) }}.unwrap();
                    match (native, decoded) {{
                        (Selection::Wire(a), Selection::Wire(b)) | (Selection::Storage(a), Selection::Storage(b)) => assert_eq!(a.value, b.value),
                        _ => panic!("variant changed"),
                    }}
                }}
            }}
        "#
            ),
        )
    });
    run_consumers("wire-name-ownership-proof", cases);
}

/// Direct scalar emission and interface metadata obey the same support rule.
#[test]
fn core_support_cannot_be_replaced_by_application_traits_or_macros() {
    let cases = [false, true].map(|expand| {
        let declaration = contract(
            "#[version(1)] enum Tag: u8 { Ready = 1, Other = 3 } \
             #[version(1)] bitenum Flags: u8 { KNOWN_BITS = 1, WRITABLE = 2 } \
             #[version(1)] interface IValue { fn echo(&self, value: u32) -> Result<u32>; }",
        );
        let source = format!(
            r#"
            #![allow(dead_code, unused_macros)]
            struct Default;
            macro_rules! matches {{ ($($input:tt)*) => {{ true }}; }}
            macro_rules! stringify {{ ($($input:tt)*) => {{ "captured" }}; }}
            macro_rules! concat {{ ($($input:tt)*) => {{ "captured" }}; }}
            macro_rules! assert {{ ($($input:tt)*) => {{ compile_error!("application assert selected") }}; }}
            macro_rules! panic {{ ($($input:tt)*) => {{ compile_error!("application panic selected") }}; }}
            impl Tag {{
                fn encode_prechecked(&self) -> u32 {{ 99 }}
                fn decode_prechecked(_: u8) -> Result<Self, neocogi_com::NcStatus> {{ Ok(Self::Other) }}
            }}
            impl Flags {{
                fn encode_prechecked(&self) -> u32 {{ 99 }}
                fn decode_prechecked(_: u8) -> u32 {{ 99 }}
            }}
            {declaration}
            #[neocogi_com::com_object(interfaces(IValue))] struct Object;
            impl IValue for Object {{ fn echo(&self, value: u32) -> Result<u32, neocogi_com::NcStatus> {{ Ok(value) }} }}
            #[test] fn values_masks_and_metadata_keep_their_meaning() {{
                ::core::assert!(!Tag::from_raw(99).is_known());
                ::core::assert_eq!(neocogi_com::encode_inline(&Tag::from_raw(99)).unwrap_err(), neocogi_com::NC_E_INVALID_TAG);
                ::core::assert_eq!(neocogi_com::encode_inline(&Tag::Ready).unwrap(), 1);
                let decoded = unsafe {{ <Tag as neocogi_com::ComValue>::decode_value(1, &mut neocogi_com::ValueBudget::default()) }}.unwrap();
                ::core::assert_eq!(decoded.raw(), 1);
                let flags = Flags::KNOWN_BITS | Flags::WRITABLE;
                ::core::assert!(flags.is_valid());
                ::core::assert!(!Flags::from_raw(4).is_valid());
                ::core::assert_eq!(<Flags as ::core::default::Default>::default().raw(), 0);
                ::core::assert_eq!(neocogi_com::encode_inline(&flags).unwrap(), 3);
                ::core::assert_eq!(neocogi_com::encode_inline(&Flags::from_raw(4)).unwrap_err(), neocogi_com::NC_E_INVALID_TAG);
                let decoded = unsafe {{ <Flags as neocogi_com::ComValue>::decode_value(3, &mut neocogi_com::ValueBudget::default()) }}.unwrap();
                ::core::assert_eq!(decoded.raw(), 3);
                let method = IValueAbi::METHOD_DESCRIPTORS[0];
                ::core::assert_eq!(unsafe {{ method.name.as_str() }}.unwrap(), "echo");
                ::core::assert_eq!(unsafe {{ (*method.params).name.as_str() }}.unwrap(), "value");
                let owner = Object::from_value(Object).query::<IValueAbi>().unwrap();
                ::core::assert_eq!(owner.echo(73).unwrap(), 73);
            }}
        "#
        );
        (format!("printed={expand}"), source_items(&source, expand))
    });
    run_consumers("core-support-owner-proof", cases);
}
