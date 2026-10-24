//! Direct Rust emission for resolved integer domains and combinable flag sets.
//! Application storage preserves raw integers; live COM conversion checks the
//! declared domain, while eligible persistence codecs retain unknown raw values.

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, LitStr, Type, Visibility};
use neocogi_com_core::NcComDenseEnumMember as DenseMember;
use crate::codegen::util::PrivateNames;

/// One bound scalar constant, including its original documentation/source annotation.
pub struct ScalarMember {
    /// Authored name retained as the public associated constant.
    pub name: Ident,
    /// Checked fixed-width integer value; all supported signed/unsigned 32-bit values fit.
    pub value: i64,
    /// Documentation and exact IDL origin attached to the final constant.
    pub docs: TokenStream,
}

/// Semantic validation and optional dense-table surface selected by resolved IDL.
pub enum ScalarDomain {
    /// Exactly the declared integer values are valid for live COM operations.
    Enumeration,
    /// The resolved domain is zero-based and contiguous in declaration order.
    DenseEnumeration {
        /// Optional canonical names in exactly the same order as the members.
        names: Option<Vec<LitStr>>,
    },
    /// Every combination containing only declared bits is valid, including zero.
    Flags,
}

/// Complete scalar emission input; no generated Rust declaration is parsed back into a schema.
pub struct ScalarPlan {
    /// Public application wrapper name with its authored source span.
    pub name: Ident,
    /// Visibility of the wrapper; its methods and constants form the public value API.
    pub visibility: Visibility,
    /// Type documentation and exact IDL origin emitted on the wrapper.
    pub docs: TokenStream,
    /// Resolved fixed-width primitive type, already range-checked by the compiler.
    pub repr: Type,
    /// Constants in normative declaration order, with checked raw values.
    pub members: Vec<ScalarMember>,
    /// Declared-value versus known-bit validation, plus dense reflection when selected.
    pub domain: ScalarDomain,
    /// Compiler-assigned nominal signature, independent of Rust formatting.
    pub signature: TokenStream,
    /// Whether the resolved value graph admits raw-integer NCB persistence.
    pub persistence: bool,
}

/// Emits signed endpoints without narrowing the validated fixed-width value.
fn integer(value: i64) -> TokenStream {
    let magnitude = Literal::u64_unsuffixed(value.unsigned_abs());
    if value < 0 {
        quote! { -#magnitude }
    } else {
        quote! { #magnitude }
    }
}

/// Derives disjoint inclusive patterns from the declared live domain. Sorting
/// affects only membership testing, never authored constant or metadata order.
/// Contiguous runs use ranges so ordinary generated bindings pass strict Clippy.
fn enumeration_patterns(members: &[ScalarMember]) -> Vec<TokenStream> {
    let mut values = members.iter().map(|member| member.value).collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    let mut values = values.into_iter().peekable();
    let mut patterns = Vec::new();
    while let Some(first) = values.next() {
        let mut last = first;
        while values.peek().is_some_and(|next| last.checked_add(1) == Some(*next)) {
            last = values.next().expect("peeked adjacent value");
        }
        let start = integer(first);
        patterns.push(if first == last {
            start
        } else {
            let end = integer(last);
            quote! { #start..=#end }
        });
    }
    patterns
}

/// Emits the complete wrapper, public operations, live codecs and eligible persistence.
/// All implementation bindings have lexical owners so printed Rust retains hygiene.
pub fn emit_scalar(plan: &ScalarPlan, nc: &syn::Path) -> TokenStream {
    let ScalarPlan {
        name,
        visibility,
        docs,
        repr,
        members,
        domain,
        signature,
        persistence,
    } = plan;
    let member_names = members.iter().map(|member| &member.name).collect::<Vec<_>>();
    let mut names = PrivateNames::new(quote! { #name #nc #repr #(#member_names)* });
    let input = names.binding("__nc_input");
    let budget = names.binding("__nc_budget");
    let wire = names.binding("__nc_wire");
    let decoded = names.binding("__nc_decoded");
    let serializer = names.binding("__nc_serializer");
    let decoder = names.binding("__nc_decoder");
    let rhs = names.binding("__nc_rhs");
    let known_mask = names.binding("__nc_known_mask");
    let ordinal = names.binding("__nc_ordinal");
    let bindings = names.binding_imports();
    let values = members.iter().map(|member| integer(member.value)).collect::<Vec<_>>();
    let constants = members.iter().zip(&values).map(|(member, value)| {
        let ScalarMember { name, docs, .. } = member;
        quote! {
            #docs
            #[allow(non_upper_case_globals)]
            pub const #name: Self = Self(#value);
        }
    });
    let (field_visibility, predicate, validation, operations) = match domain {
        ScalarDomain::Enumeration | ScalarDomain::DenseEnumeration { .. } => {
            let patterns = enumeration_patterns(members);
            let validation = if patterns.is_empty() {
                quote! { false }
            } else {
                quote! { ::core::matches!(self.0, #(#patterns)|*) }
            };
            (
                quote! { pub },
                quote! { is_known },
                validation,
                quote! {
                    /// Returns whether this raw value lies outside the declared live domain.
                    pub const fn is_unknown(self) -> bool { !self.is_known() }
                },
            )
        }
        ScalarDomain::Flags => {
            // Fold the declared bits once at generation time. Zero and aliases
            // do not add runtime operations or change the public member order.
            let known_bits = members.iter().fold(0, |mask, member| mask | member.value);
            let mask = integer(known_bits);
            // A typed local states the mask in the actual storage width. The
            // same subset test covers empty, partial and full-width domains;
            // no public mask name or width-specific validation table is needed.
            let validation = quote! {
                let #known_mask: #repr = #mask;
                self.0 & !#known_mask == 0
            };
            (
                quote! {},
                quote! { is_valid },
                // The local mask leaves authored KNOWN_BITS an ordinary member.
                validation,
                quote! {
                /// Returns whether the bit set is empty.
                pub const fn is_empty(self) -> bool { self.0 == 0 }
                /// Returns whether every requested bit is present.
                pub const fn contains(self, #rhs: Self) -> bool { self.0 & #rhs.0 == #rhs.0 }
                /// Returns whether at least one requested bit is present.
                pub const fn intersects(self, #rhs: Self) -> bool { self.0 & #rhs.0 != 0 }
                },
            )
        }
    };
    let dense = match domain {
        ScalarDomain::DenseEnumeration { names } => {
            let count = Ident::new(DenseMember::Count.name(), name.span());
            let all = Ident::new(DenseMember::All.name(), name.span());
            let len = members.len();
            let canonical = names.as_ref().map(|names| {
                assert_eq!(names.len(), len, "resolved dense names must match their members");
                let names_member = Ident::new(DenseMember::Names.name(), name.span());
                let canonical = Ident::new(DenseMember::CanonicalName.name(), name.span());
                quote! {
                    /// Canonical text in the same order as the complete dense value table.
                    pub const #names_member: [&'static str; Self::#count] = [#(#names),*];
                    /// Returns the canonical name, or None for raw values outside the domain.
                    pub const fn #canonical(self) -> ::core::option::Option<&'static str> {
                        let #ordinal = self.raw() as usize;
                        if #ordinal < Self::#count {
                            ::core::option::Option::Some(Self::#names_member[#ordinal])
                        } else {
                            ::core::option::Option::None
                        }
                    }
                }
            });
            quote! {
                /// Number of values in the resolved zero-based contiguous domain.
                pub const #count: usize = #len;
                /// Every declared value in normative discriminant order.
                pub const #all: [Self; Self::#count] = [#(Self::#member_names),*];
                #canonical
            }
        }
        ScalarDomain::Enumeration | ScalarDomain::Flags => quote! {},
    };
    let flags = match domain {
        ScalarDomain::Flags => quote! {
            impl ::core::default::Default for #name {
                /// Creates the empty bit set.
                fn default() -> Self { Self::from_raw(0) }
            }
            impl ::core::ops::BitOr for #name {
                type Output = Self;
                /// Produces the union of both sets, retaining raw unknown bits.
                fn bitor(self, #rhs: Self) -> Self { Self(self.0 | #rhs.0) }
            }
            impl ::core::ops::BitOrAssign for #name {
                /// Adds the right-hand bits to this set.
                fn bitor_assign(&mut self, #rhs: Self) { self.0 |= #rhs.0; }
            }
            impl ::core::ops::BitAnd for #name {
                type Output = Self;
                /// Produces the intersection of both sets.
                fn bitand(self, #rhs: Self) -> Self { Self(self.0 & #rhs.0) }
            }
            impl ::core::ops::BitAndAssign for #name {
                /// Retains only bits also present in the right-hand set.
                fn bitand_assign(&mut self, #rhs: Self) { self.0 &= #rhs.0; }
            }
            impl ::core::ops::BitXor for #name {
                type Output = Self;
                /// Produces the symmetric difference of both sets.
                fn bitxor(self, #rhs: Self) -> Self { Self(self.0 ^ #rhs.0) }
            }
            impl ::core::ops::BitXorAssign for #name {
                /// Toggles every bit present in the right-hand set.
                fn bitxor_assign(&mut self, #rhs: Self) { self.0 ^= #rhs.0; }
            }
        },
        ScalarDomain::Enumeration | ScalarDomain::DenseEnumeration { .. } => quote! {},
    };
    let persistence = persistence.then(|| {
        quote! {
            impl #nc::ComSerialize for #name {
                /// Persists the declared integer width, including unknown raw values.
                fn com_serialize(&self, #serializer: &mut #nc::binary::Writer<'_>) -> ::core::result::Result<(), #nc::binary::Error> {
                    <#repr as #nc::ComSerialize>::com_serialize(&self.raw(), #serializer)
                }
            }
            impl #nc::ComDeserialize for #name {
                /// Restores raw integers; live COM domain validation is a separate operation.
                fn com_deserialize(#decoder: &mut #nc::binary::Reader<'_>) -> ::core::result::Result<Self, #nc::binary::Error> {
                    <#repr as #nc::ComDeserialize>::com_deserialize(#decoder).map(Self::from_raw)
                }
            }
        }
    });
    quote! {
        #docs
        #[repr(transparent)]
        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug, ::core::cmp::Eq,
            ::core::hash::Hash, ::core::cmp::Ord, ::core::cmp::PartialEq, ::core::cmp::PartialOrd)]
        #visibility struct #name(
            /// Complete raw integer; live COM conversion validates the declared domain.
            #field_visibility #repr
        );

        const _: () = {
        #bindings
        impl #name {
            #(#constants)*
            /// Wraps raw storage without rejecting unknown values.
            pub const fn from_raw(#input: #repr) -> Self { Self(#input) }
            /// Returns the complete underlying integer.
            pub const fn raw(self) -> #repr { self.0 }
            /// Checks the declared integer domain or union of known flag bits.
            pub const fn #predicate(self) -> bool { #validation }
            #operations
            #dense
        }
        impl ::core::convert::From<#repr> for #name {
            /// Preserves the raw integer without performing live COM validation.
            fn from(#input: #repr) -> Self { Self::from_raw(#input) }
        }
        impl ::core::convert::From<#name> for #repr {
            /// Extracts the complete raw integer.
            fn from(#input: #name) -> Self { #input.raw() }
        }
        #flags
        // SAFETY: transparent scalar storage has the primitive ABI and no ownership protocol.
        unsafe impl #nc::AbiType for #name {
            const ABI_TYPE: #nc::NcAbiType = <#repr as #nc::AbiType>::ABI_TYPE;
        }
        // SAFETY: the wrapper contains only its fixed-width primitive.
        unsafe impl #nc::AbiPlainValue for #name {}
        // SAFETY: raw integers are checked before accepting the declared live domain.
        // BORROW_WIRE remains false: identical layout cannot bypass domain validation.
        unsafe impl #nc::ComValue for #name {
            const FIXED_COST: ::core::option::Option<#nc::ValueCost> = <#repr as #nc::ComValue>::FIXED_COST;
            type Wire = #repr;
            type Storage = ();
            /// Accounts for one scalar, then validates the native domain.
            fn encode_value(&self, #budget: &mut #nc::ValueBudget) -> ::core::result::Result<(#repr, ()), #nc::NcStatus> {
                // SAFETY: value charges the scalar before the allocation-free checked converter.
                #budget.value(|_| unsafe { <Self as #nc::ComValue>::encode_prechecked(self) })
            }
            /// Accounts for one scalar, then validates the wire domain.
            unsafe fn decode_value(#wire: #repr, #budget: &mut #nc::ValueBudget) -> ::core::result::Result<Self, #nc::NcStatus> {
                // SAFETY: the raw primitive is initialized and value charges its scalar cost.
                #budget.value(|_| unsafe { <Self as #nc::ComValue>::decode_prechecked(#wire) })
            }
            /// Retains domain checks after allocation-free resource admission.
            unsafe fn encode_prechecked(&self) -> ::core::result::Result<(#repr, ()), #nc::NcStatus> {
                match self.#predicate() {
                    true => ::core::result::Result::Ok((self.raw(), ())),
                    false => ::core::result::Result::Err(#nc::NC_E_INVALID_TAG),
                }
            }
            /// Rejects unknown values/bits even when resource accounting is prepaid.
            unsafe fn decode_prechecked(#wire: #repr) -> ::core::result::Result<Self, #nc::NcStatus> {
                let #decoded = Self::from_raw(#wire);
                match #decoded.#predicate() {
                    true => ::core::result::Result::Ok(#decoded),
                    false => ::core::result::Result::Err(#nc::NC_E_INVALID_TAG),
                }
            }
        }
        // SAFETY: the checked scalar wire is self-contained and has no retained backing.
        unsafe impl #nc::ComInlineValue for #name {}
        impl #nc::ComValueType for #name {
            const VALUE_SIGNATURE: #nc::NcAbiSignature = #signature;
        }
        #persistence
        };
    }
}
