//! User-facing macro exports and the declarative integer-enum helper.
//!
//! With the `macros` feature enabled this module re-exports the procedural
//! macros that define interfaces, IDL descriptions, ABI-safe structs and enums,
//! and factory plugins. [`com_int_enum!`] remains declarative so it can generate
//! a transparent integer newtype that preserves unknown values while still
//! participating in the `AbiType` and `AbiPlainValue` contracts.

#[cfg(feature = "macros")]
pub use neocogi_com_macros::{
    ComDeserialize, com_class_description, com_enum, com_factory, com_factory_description, com_interface, com_object, com_struct, factory_plugin,
};

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

        impl From<$repr> for $name {
            /// Wraps a raw ABI value without rejecting unknown values.
            fn from(value: $repr) -> Self {
                Self::from_raw(value)
            }
        }

        impl From<$name> for $repr {
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

        impl $crate::ComDeserialize for $name
        where
            $repr: $crate::ComDeserialize,
        {
            /// Decodes the raw integer and preserves both known and unknown values.
            fn com_deserialize<'__nc_de, __NcDecoder>(
                decoder: __NcDecoder,
            ) -> core::result::Result<Self, __NcDecoder::Error>
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
        pub unsafe extern "C" fn nc_get_factory_plugin_description(out: *mut *const $crate::NcFactoryPluginDescription) -> $crate::NcStatus {
            // SAFETY: The accessor returns immutable static metadata and the
            // caller supplies the raw output slot validated by the helper.
            unsafe { $crate::return_factory_plugin_description(out, $description()) }
        }
        // Keep symbol shape synchronized with the core ABI declaration.
        const _: $crate::NcGetFactoryPluginDescription = nc_get_factory_plugin_description;
    };
}
