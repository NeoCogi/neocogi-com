//! Executable naming contracts across normal and printed production expansions.
//!
//! These consumers deliberately give application operations plausible wrong
//! values and incompatible types. Compilation alone cannot detect a same-typed
//! replacement; actual codec budgets, metadata and COM calls establish ownership.

use std::{fs, process::Command};

use proc_macro2::TokenStream;
use quote::quote;

/// Runs several complete source selections in one real SDK consumer.
/// The scratch target is removed with the consumer after every child has exited.
fn run_consumers(name: &str, cases: impl IntoIterator<Item = (String, String)>) {
    let consumer = crate::test_consumer::Consumer::new(name);
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

/// Prints production output, intentionally discarding proc-macro syntax contexts.
fn printed(tokens: TokenStream) -> String {
    prettyplease::unparse(&syn::parse2(tokens).unwrap())
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
    /// Emits the real normal attribute or calls its production expansion for printing.
    fn declaration(self, expand: bool) -> String {
        let is_enum = matches!(self, Self::CopiedEnum | Self::OwnedEnum);
        let declaration = if is_enum {
            quote! { pub enum Record { Ready { value: u32 } } }
        } else {
            quote! { pub struct Record { pub value: u32 } }
        };
        let args = quote! { assigned_signature = b"00000000000000000000000000000801" };
        match (self, expand) {
            (Self::CopiedRecord, false) => quote! { #[neocogi_com::com_struct(#args)] #declaration }.to_string(),
            (Self::CopiedRecord, true) => printed(crate::structs::expand_com_struct(syn::parse2(args).unwrap(), syn::parse2(declaration).unwrap()).unwrap()),
            (Self::CopiedEnum, false) => quote! { #[neocogi_com::com_enum(tag = u8, #args)] #declaration }.to_string(),
            (Self::CopiedEnum, true) => {
                printed(crate::enums::expand_com_enum(syn::parse2(quote! { tag = u8, #args }).unwrap(), syn::parse2(declaration).unwrap()).unwrap())
            }
            (_, false) => quote! { #[neocogi_com::com_value(#args)] #declaration }.to_string(),
            (_, true) => printed(crate::value::expand_owned(syn::parse2(args).unwrap(), syn::parse2(declaration).unwrap()).unwrap()),
        }
    }

    /// Constructs the selected native shape with an independently chosen sentinel.
    fn native(self, value: u32) -> String {
        match self {
            Self::CopiedEnum | Self::OwnedEnum => format!("Record::Ready {{ value: {value} }}"),
            _ => format!("Record {{ value: {value} }}"),
        }
    }

    /// Reads a converted native value without depending on generated helper names.
    fn read(self) -> &'static str {
        match self {
            Self::CopiedEnum | Self::OwnedEnum => "match decoded { Record::Ready { value } => value }",
            _ => "decoded.value",
        }
    }
}

/// A namespace trap shared by all value families and both expansion paths.
fn value_case(family: ValueFamily, incompatible: bool, expand: bool) -> String {
    let declaration = family.declaration(expand);
    let native = family.native(73);
    let wrong = family.native(99);
    let read = family.read();
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
        #[neocogi_com::com_interface(auto_guid = "test.NamingValue:v1",
            assigned_signature = b"00000000000000000000000000000802")]
        trait IValue {{
            fn observe(&self, value: neocogi_com::NcValueRef<'_, Record>) -> Result<u32, neocogi_com::NcStatus>;
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
            let (wire, storage) = <Record as neocogi_com::ComValue>::encode_value(&native, &mut neocogi_com::ValueBudget::new(0, 2, 2)).unwrap();
            let decoded = unsafe {{ <Record as neocogi_com::ComValue>::decode_value(wire, &mut neocogi_com::ValueBudget::new(0, 2, 2)) }}.unwrap();
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
                let body = value_case(family, incompatible, expand);
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
        let record: syn::ItemStruct = syn::parse_quote! { pub struct __NcWire { pub value: u32 } };
        let args = quote! { assigned_signature = b"00000000000000000000000000000803" };
        let first = if expand {
            printed(crate::structs::expand_com_struct(syn::parse2(args.clone()).unwrap(), record).unwrap())
        } else {
            quote! { #[neocogi_com::com_struct(#args)] #record }.to_string()
        };
        let enumeration: syn::ItemEnum = syn::parse_quote! { pub enum Selection { Wire(__NcWire), Storage(__NcWire) } };
        let second = if expand {
            printed(crate::enums::expand_com_enum(syn::parse2(quote! { tag = u8, #args }).unwrap(), enumeration).unwrap())
        } else {
            quote! { #[neocogi_com::com_enum(tag = u8, #args)] #enumeration }.to_string()
        };
        (
            format!("printed={expand}"),
            format!(
                r#"
            {first} {second}
            #[test] fn associated_wire_type_is_the_only_required_locator() {{
                for native in [Selection::Wire(__NcWire {{ value: 73 }}), Selection::Storage(__NcWire {{ value: 91 }})] {{
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

/// Exported declarative helpers and interface metadata obey the same support rule.
#[test]
fn core_support_cannot_be_replaced_by_application_traits_or_macros() {
    let interface: syn::ItemTrait = syn::parse_quote! {
        trait IValue { fn echo(&self, value: u32) -> Result<u32, neocogi_com::NcStatus>; }
    };
    let args = quote! { auto_guid = "test.NamingSupport:v1", assigned_signature = b"00000000000000000000000000000804" };
    let cases = [false, true].map(|expand| {
        let declaration = if expand {
            printed(crate::interface::expand_com_interface(syn::parse2(args.clone()).unwrap(), interface.clone()).unwrap())
        } else {
            quote! { #[neocogi_com::com_interface(#args)] #interface }.to_string()
        };
        (
            format!("printed={expand}"),
            format!(
                r#"
            #![allow(dead_code, unused_macros)]
            struct Default;
            macro_rules! matches {{ ($($input:tt)*) => {{ true }}; }}
            macro_rules! stringify {{ ($($input:tt)*) => {{ "captured" }}; }}
            macro_rules! concat {{ ($($input:tt)*) => {{ "captured" }}; }}
            macro_rules! assert {{ ($($input:tt)*) => {{ compile_error!("application assert selected") }}; }}
            macro_rules! panic {{ ($($input:tt)*) => {{ compile_error!("application panic selected") }}; }}
            neocogi_com::com_int_enum! {{ pub enum Tag: u8 {{ Ready = 1, Other = 3 }} }}
            neocogi_com::com_bit_enum! {{ pub bitenum Flags: u8 {{ KNOWN_BITS = 1, WRITABLE = 2 }} }}
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
            ),
        )
    });
    run_consumers("core-support-owner-proof", cases);
}
