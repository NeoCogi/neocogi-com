//! User-facing macro exports and the declarative integer-enum helper.
//!
//! With the `macros` feature enabled this module re-exports the procedural
//! macros that define interfaces, IDL descriptions, ABI-safe structs and enums,
//! and factory plugins. [`com_int_enum!`] remains declarative so it can generate
//! a transparent integer newtype that preserves unknown values while still
//! participating in the `AbiType` and `AbiPlainValue` contracts.

#[cfg(feature = "macros")]
pub use neocogi_com_macros::{ComDeserialize, ComSerialize, com_enum, com_class, com_factory, com_interface, com_object, com_struct, com_value, factory_plugin};

/// Defines an ABI-stable enum-like integer newtype.
///
/// This macro deliberately does **not** generate a Rust `enum`. Across COM,
/// plugin, compiler, and language boundaries, enum-like values should have an
/// explicit fixed-width integer representation and should preserve unknown
/// values for forward compatibility.
///
/// ```
/// use neocogi_com::com_int_enum;
///
/// com_int_enum! {
///     pub enum NodeType: u16 {
///         ELEMENT = 1,
///         TEXT = 3,
///         DOCUMENT = 9,
///     }
/// }
///
/// let kind = NodeType::ELEMENT;
/// assert_eq!(kind.raw(), 1);
/// assert!(kind.is_known());
/// assert!(NodeType::from_raw(99).is_unknown());
/// ```
///
/// Generated shape:
///
/// ```text
/// #[repr(transparent)]
/// pub struct NodeType(pub u16);
/// ```
#[macro_export]
// Support paths inside exported expansions must not resolve to application types.
macro_rules! com_int_enum {
    (
        $(#[$type_meta:meta])*
        $vis:vis enum $name:ident : $repr:ty {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident = $value:literal
            ),* $(,)?
        }
    ) => {
        $(#[$type_meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $vis struct $name(pub $repr);

        impl $name {
            $(
                $(#[$variant_meta])*
                // IDL enum cases follow Rust enum-variant casing even though
                // this forward-compatible representation uses constants.
                #[allow(non_upper_case_globals)]
                pub const $variant: Self = Self($value);
            )*

            /// Creates this value from its raw ABI representation.
            pub const fn from_raw(value: $repr) -> Self {
                Self(value)
            }

            /// Returns this value's raw ABI representation.
            pub const fn raw(self) -> $repr {
                self.0
            }

            /// Returns whether this value is one of the constants known at
            /// compile time.
            pub const fn is_known(self) -> bool {
                matches!(self.0, $($value)|*)
            }

            /// Returns whether this value is not one of the constants known at
            /// compile time.
            pub const fn is_unknown(self) -> bool {
                !self.is_known()
            }
        }

        impl ::core::convert::From<$repr> for $name {
            /// Wraps a raw ABI value without rejecting unknown values.
            fn from(value: $repr) -> Self {
                Self::from_raw(value)
            }
        }

        impl ::core::convert::From<$name> for $repr {
            /// Extracts the enum-like value's raw integer representation.
            fn from(value: $name) -> Self {
                value.raw()
            }
        }

        // SAFETY: The transparent wrapper has the exact ABI representation of
        // `$repr`, and `$repr: AbiType` supplies the plain ABI contract.
        unsafe impl $crate::AbiType for $name
        where
            $repr: $crate::AbiType
        {
            const ABI_TYPE: $crate::NcAbiType = <$repr as $crate::AbiType>::ABI_TYPE;
        }

        // SAFETY: The wrapper carries only the `$repr` value and therefore has
        // no hidden ownership or release protocol when `$repr` is plain.
        unsafe impl $crate::AbiPlainValue for $name
        where
            $repr: $crate::AbiPlainValue
        {}

        // SAFETY: the raw integer is validated before accepting a declared value.
        unsafe impl $crate::ComValue for $name {
            const FIXED_COST: ::core::option::Option<$crate::ValueCost> = <$repr as $crate::ComValue>::FIXED_COST;
            type Wire = $repr;
            type Storage = ();
            /// Accounts for the scalar before validating its authored domain.
            fn encode_value(&self, budget: &mut $crate::ValueBudget) -> ::core::result::Result<($repr, ()), $crate::NcStatus> {
                budget.value(|_| unsafe { Self::encode_prechecked(self) })
            }
            /// Accounts for the scalar before accepting any raw integer.
            unsafe fn decode_value(wire: $repr, budget: &mut $crate::ValueBudget) -> ::core::result::Result<Self, $crate::NcStatus> {
                budget.value(|_| unsafe { Self::decode_prechecked(wire) })
            }
            /// Preserves domain checks inside a prepaid fixed aggregate.
            unsafe fn encode_prechecked(&self) -> ::core::result::Result<($repr, ()), $crate::NcStatus> {
                match self.is_known() { true => ::core::result::Result::Ok((self.raw(), ())), false => ::core::result::Result::Err($crate::NC_E_INVALID_TAG) }
            }
            /// Rejects unknown tags/bits even when accounting was prepaid.
            unsafe fn decode_prechecked(wire: $repr) -> ::core::result::Result<Self, $crate::NcStatus> {
                let value = Self::from_raw(wire);
                match value.is_known() { true => ::core::result::Result::Ok(value), false => ::core::result::Result::Err($crate::NC_E_INVALID_TAG) }
            }
        }
        // SAFETY: a validated integer is self-contained and carries no pointers.
        unsafe impl $crate::ComInlineValue for $name {}

        // Format-neutral construction is emitted by IDL only when requested.
        // A unit enum uses the ordinary named enum/variant protocol; silently
        // decoding an arbitrary raw integer here would bypass that policy.
    };
}

/// Defines an ABI-stable set of named integer flags.
///
/// Unlike [`com_int_enum!`], every combination containing only declared bits
/// is valid. The raw storage remains private so callers use the generated typed
/// operations instead of depending on a synthetic struct field.
///
/// ```
/// use neocogi_com::com_bit_enum;
///
/// com_bit_enum! {
///     pub bitenum WindowOptions: u16 {
///         NONE = 0,
///         RESIZABLE = 1,
///         TITLED = 2,
///     }
/// }
///
/// let options = WindowOptions::RESIZABLE | WindowOptions::TITLED;
/// assert!(options.contains(WindowOptions::RESIZABLE));
/// assert!(options.is_valid());
/// assert!(!WindowOptions::from_raw(8).is_valid());
/// ```
#[macro_export]
macro_rules! com_bit_enum {
    (
        $(#[$type_meta:meta])*
        $vis:vis bitenum $name:ident : $repr:ty {
            $(
                $(#[$flag_meta:meta])*
                $flag:ident = $value:literal
            ),* $(,)?
        }
    ) => {
        $(#[$type_meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $vis struct $name($repr);

        impl $name {
            $(
                $(#[$flag_meta])*
                pub const $flag: Self = Self($value);
            )*

            /// Union of every bit present in a declared flag or alias.
            const KNOWN_BITS: $repr = 0 $(| $value)*;

            /// Creates a value from raw ABI storage without hiding unknown bits.
            pub const fn from_raw(value: $repr) -> Self {
                Self(value)
            }

            /// Returns the complete raw ABI bit pattern.
            pub const fn raw(self) -> $repr {
                self.0
            }

            /// Returns whether no bits are set.
            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }

            /// Returns whether every requested bit is present.
            pub const fn contains(self, requested: Self) -> bool {
                self.0 & requested.0 == requested.0
            }

            /// Returns whether this value and the requested value share a bit.
            pub const fn intersects(self, requested: Self) -> bool {
                self.0 & requested.0 != 0
            }

            /// Returns whether the raw representation contains only declared bits.
            pub const fn is_valid(self) -> bool {
                self.0 & !Self::KNOWN_BITS == 0
            }
        }

        impl Default for $name {
            /// Creates the empty bit set.
            fn default() -> Self {
                Self::from_raw(0)
            }
        }

        impl ::core::convert::From<$repr> for $name {
            /// Wraps raw ABI storage while preserving unknown bits for validation.
            fn from(value: $repr) -> Self {
                Self::from_raw(value)
            }
        }

        impl ::core::convert::From<$name> for $repr {
            /// Extracts the complete raw ABI bit pattern.
            fn from(value: $name) -> Self {
                value.raw()
            }
        }

        impl ::core::ops::BitOr for $name {
            type Output = Self;

            /// Produces the union of two typed bit sets.
            fn bitor(self, rhs: Self) -> Self {
                Self(self.0 | rhs.0)
            }
        }

        impl ::core::ops::BitOrAssign for $name {
            /// Adds every bit in the right-hand set.
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0;
            }
        }

        impl ::core::ops::BitAnd for $name {
            type Output = Self;

            /// Produces the intersection of two typed bit sets.
            fn bitand(self, rhs: Self) -> Self {
                Self(self.0 & rhs.0)
            }
        }

        impl ::core::ops::BitAndAssign for $name {
            /// Retains only bits present in the right-hand set.
            fn bitand_assign(&mut self, rhs: Self) {
                self.0 &= rhs.0;
            }
        }

        impl ::core::ops::BitXor for $name {
            type Output = Self;

            /// Produces the symmetric difference of two typed bit sets.
            fn bitxor(self, rhs: Self) -> Self {
                Self(self.0 ^ rhs.0)
            }
        }

        impl ::core::ops::BitXorAssign for $name {
            /// Toggles every bit present in the right-hand set.
            fn bitxor_assign(&mut self, rhs: Self) {
                self.0 ^= rhs.0;
            }
        }

        // SAFETY: The transparent wrapper has the exact ABI representation of
        // `$repr`, and `$repr: AbiType` supplies the plain ABI contract.
        unsafe impl $crate::AbiType for $name
        where
            $repr: $crate::AbiType
        {
            const ABI_TYPE: $crate::NcAbiType = <$repr as $crate::AbiType>::ABI_TYPE;
        }

        // SAFETY: The wrapper carries only the `$repr` value and therefore has
        // no hidden ownership or release protocol when `$repr` is plain.
        unsafe impl $crate::AbiPlainValue for $name
        where
            $repr: $crate::AbiPlainValue
        {}

        // SAFETY: the raw integer is validated before accepting a declared value.
        unsafe impl $crate::ComValue for $name {
            const FIXED_COST: ::core::option::Option<$crate::ValueCost> = <$repr as $crate::ComValue>::FIXED_COST;
            type Wire = $repr;
            type Storage = ();
            /// Accounts for the scalar before validating its authored domain.
            fn encode_value(&self, budget: &mut $crate::ValueBudget) -> ::core::result::Result<($repr, ()), $crate::NcStatus> {
                budget.value(|_| unsafe { Self::encode_prechecked(self) })
            }
            /// Accounts for the scalar before accepting any raw integer.
            unsafe fn decode_value(wire: $repr, budget: &mut $crate::ValueBudget) -> ::core::result::Result<Self, $crate::NcStatus> {
                budget.value(|_| unsafe { Self::decode_prechecked(wire) })
            }
            /// Preserves domain checks inside a prepaid fixed aggregate.
            unsafe fn encode_prechecked(&self) -> ::core::result::Result<($repr, ()), $crate::NcStatus> {
                match self.is_valid() { true => ::core::result::Result::Ok((self.raw(), ())), false => ::core::result::Result::Err($crate::NC_E_INVALID_TAG) }
            }
            /// Rejects unknown tags/bits even when accounting was prepaid.
            unsafe fn decode_prechecked(wire: $repr) -> ::core::result::Result<Self, $crate::NcStatus> {
                let value = Self::from_raw(wire);
                match value.is_valid() { true => ::core::result::Result::Ok(value), false => ::core::result::Result::Err($crate::NC_E_INVALID_TAG) }
            }
        }
        // SAFETY: a validated integer is self-contained and carries no pointers.
        unsafe impl $crate::ComInlineValue for $name {}

        impl $crate::ComSerialize for $name {
            /// Persists every raw bit pattern using the declared integer width.
            fn com_serialize(&self, serializer: &mut $crate::binary::Writer<'_>) -> ::core::result::Result<(), $crate::binary::Error> {
                $crate::ComSerialize::com_serialize(&self.raw(), serializer)
            }
        }

        impl $crate::ComDeserialize for $name
        where
            $repr: $crate::ComDeserialize,
        {
            /// Decodes raw storage while retaining unknown bits for boundary validation.
            fn com_deserialize<'__nc_de, __NcDecoder>(
                decoder: __NcDecoder,
            ) -> ::core::result::Result<Self, __NcDecoder::Error>
            where
                __NcDecoder: $crate::ComDeserializer<'__nc_de>,
            {
                <$repr as $crate::ComDeserialize>::com_deserialize(decoder).map(Self::from_raw)
            }
        }
    };
}

/// Exports a factory-only plugin description from a dynamic-library assembly.
///
/// `factory_plugin!` defines metadata and an ordinary Rust accessor. Dynamic
/// assemblies invoke this macro once; statically linked assemblies call the
/// accessor directly and omit the exported symbol.
#[macro_export]
macro_rules! export_factory_plugin {
    () => {
        $crate::export_factory_plugin!(factory_plugin_description);
    };
    ($description:path) => {
        /// Returns validated factory-only metadata through the dynamic ABI.
        ///
        /// # Safety
        ///
        /// `out` must be null or writable for one description pointer. The
        /// exporting module must remain loaded while that metadata is used.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn nc_get_factory_plugin_description_v1(out: *mut *const $crate::NcFactoryPluginDescription) -> $crate::NcStatus {
            // SAFETY: The accessor returns immutable static metadata and the
            // caller supplies the raw output slot validated by the helper.
            unsafe { $crate::return_factory_plugin_description(out, $description()) }
        }
        // Keep symbol shape synchronized with the core ABI declaration.
        const _: $crate::NcGetFactoryPluginDescription = nc_get_factory_plugin_description_v1;
    };
}
