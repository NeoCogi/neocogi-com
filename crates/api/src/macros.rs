//! User-facing macro exports and the declarative integer-enum helper.
//!
//! With the `macros` feature enabled this module re-exports the procedural
//! macros that define interfaces, classes, ABI-safe structs and enums, and
//! plugin descriptors. [`com_int_enum!`] remains declarative so it can generate
//! a transparent integer newtype that preserves unknown values while still
//! participating in the `AbiType` and `AbiPlainValue` contracts.

#[cfg(feature = "macros")]
pub use neocogi_com_macros::{ComDeserialize, com_class, com_enum, com_interface, com_object, com_struct, plugin};

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

/// Exports a plugin descriptor from the final dynamic-library assembly crate.
///
/// `plugin!` only defines metadata. Invoke this once per shared library; omit it
/// when statically linking several plugins into an executable or browser module.
/// Both forms reuse the same descriptor and validation. The optional path names
/// a descriptor function when the plugin implementation lives in another module
/// or a statically linked implementation crate.
///
/// ```ignore
/// neocogi_com::plugin! { /* plugin identity and classes */ }
/// #[cfg(feature = "dynamic-plugin")]
/// neocogi_com::export_plugin!();
/// ```
#[macro_export]
macro_rules! export_plugin {
    () => {
        $crate::export_plugin!(plugin_descriptor);
    };
    ($descriptor:path) => {
        /// Returns validated, immutable metadata through the dynamic plugin ABI.
        ///
        /// # Safety
        /// `out` must be null or writable for one descriptor pointer. The module
        /// containing the descriptor must stay loaded while its metadata is used.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn nc_get_plugin_descriptor(out: *mut *const $crate::NcPluginDescriptor) -> $crate::NcStatus {
            // SAFETY: The descriptor function returns static metadata, and the
            // caller supplies the output storage. The shared helper validates
            // the same descriptor used for static registration.
            unsafe { $crate::return_plugin_descriptor(out, $descriptor()) }
        }
        // Check the export signature without duplicating the ABI declaration.
        const _: $crate::NcGetPluginDescriptor = nc_get_plugin_descriptor;
    };
}
