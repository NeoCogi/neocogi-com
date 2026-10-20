//! Public generated Rust names shared by admission and emission.
//!
//! These are intentional API members, unlike private helpers. IDL can diagnose
//! their actual conflicts before Rust generation; macros use the same spellings.
//! This build-time policy has no wire representation or runtime registry state.

/// Public companion types generated for each callable interface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcComInterfaceCompanion {
    /// Static interface marker used by typed handles.
    Abi,
    /// Physical C function table.
    VTable,
    /// Concrete provider's static projection and thunk owner.
    Provider,
    /// Borrowed Rust method view.
    Client,
}

impl NcComInterfaceCompanion {
    /// Complete public type inventory, in declaration order.
    pub const ALL: [Self; 4] = [Self::Abi, Self::VTable, Self::Provider, Self::Client];

    /// Appended to the authored interface name by every Rust emitter.
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Abi => "Abi",
            Self::VTable => "VTable",
            Self::Provider => "VtblFor",
            Self::Client => "Client",
        }
    }
}

/// Public associated members provided by the dense-enum convenience API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcComDenseEnumMember {
    /// Number of declared domain values.
    Count,
    /// Complete ordinal-ordered domain.
    All,
    /// Canonical strings, present only under the names policy.
    Names,
    /// Canonical-name lookup, present only under the names policy.
    CanonicalName,
}

impl NcComDenseEnumMember {
    /// Complete public associated-member inventory.
    pub const ALL: [Self; 4] = [Self::Count, Self::All, Self::Names, Self::CanonicalName];

    /// Exact Rust spelling used for emission and collision admission.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Count => "COUNT",
            Self::All => "ALL",
            Self::Names => "NAMES",
            Self::CanonicalName => "canonical_name",
        }
    }

    /// Whether the current dense declaration actually generates this member.
    pub const fn is_emitted(self, has_names: bool) -> bool {
        match self {
            Self::Count | Self::All => true,
            Self::Names | Self::CanonicalName => has_names,
        }
    }
}
