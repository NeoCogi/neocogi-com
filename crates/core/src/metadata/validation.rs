//! Structural and semantic validation for exported plugin descriptor graphs.
//!
//! Validation walks every reachable plugin, class, interface, method, passing,
//! type, and text record; checks pointer/count shapes and cross-record identity
//! rules; recomputes interface hashes; and reports multiple path-addressed
//! issues in one pass while preserving the first failing [`NcStatus`].
//!
//! Validation walks trusted readable metadata and reports every reachable
//! issue through a callback while returning the first failing status. It checks
//! pointer/count shape, naming policy, duplicate identities, typed method
//! descriptors, and recomputed interface hashes. It is not a memory probe or a
//! sandbox for hostile pointers.

#[cfg(feature = "alloc")]
use alloc::collections::BTreeSet;

use crate::hash::{NcAbiHasher, NcAbiSchemaHasher};
use crate::metadata::{validate_qualified_name, validate_version_text};
use crate::object::{IID_IUNKNOWN, IUNKNOWN_ABI_HASH, IUNKNOWN_QUALIFIED_NAME};
use crate::{
    NC_ABI_PASSING_COM_PTR_OUT, NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET, NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET, NC_ABI_PASSING_OUT,
    NC_ABI_PASSING_VALUE, NC_ABI_PASSING_VOID, NC_ABI_RECEIVER_MUTABLE, NC_ABI_RECEIVER_SHARED, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE,
    NC_ABI_TYPE_AGGREGATE, NC_ABI_TYPE_BOOL, NC_ABI_TYPE_F32, NC_ABI_TYPE_F64, NC_ABI_TYPE_I8, NC_ABI_TYPE_I16, NC_ABI_TYPE_I32, NC_ABI_TYPE_I64,
    NC_ABI_TYPE_INTERFACE, NC_ABI_TYPE_ISIZE, NC_ABI_TYPE_NC_ABI_HASH, NC_ABI_TYPE_NC_GUID, NC_ABI_TYPE_NC_STATUS, NC_ABI_TYPE_U8, NC_ABI_TYPE_U16,
    NC_ABI_TYPE_U32, NC_ABI_TYPE_U64, NC_ABI_TYPE_USIZE, NC_ABI_TYPE_VOID, NC_COM_CORE_ABI_VERSION, NC_E_ABI_VERSION, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK,
    NcAbiHash, NcAbiHashPart, NcAbiPassing, NcAbiPassingDescriptor, NcAbiReceiver, NcAbiThreadingMode, NcAbiType, NcAbiTypeDescriptor, NcClassDescriptor,
    NcClassInterfaceDescriptor, NcClassTable, NcInterfaceDescriptor, NcMethodDescriptor, NcMethodParamDescriptor, NcMethodReturnDescriptor,
    NcPluginDependencyDescriptor, NcPluginDescriptor, NcStaticUtf8, NcStatus, guid_from_qualified_name,
};

/// Location of a descriptor validation issue in a plugin descriptor graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcDescriptorPath {
    /// The plugin descriptor itself.
    Plugin,
    /// The plugin's class table.
    ClassTable,
    /// A hard plugin dependency descriptor.
    PluginDependency {
        /// Zero-based dependency-table position.
        dependency_index: u32,
    },
    /// A class descriptor in the plugin class table.
    Class {
        /// Zero-based class-table position.
        class_index: u32,
    },
    /// A class-interface mapping descriptor.
    ClassInterface {
        /// Zero-based class-table position.
        class_index: u32,
        /// Zero-based interface position within the class.
        interface_index: u32,
    },
    /// The interface descriptor referenced by a class-interface mapping.
    Interface {
        /// Zero-based class-table position.
        class_index: u32,
        /// Zero-based interface position within the class.
        interface_index: u32,
    },
    /// A method descriptor in an interface descriptor.
    Method {
        /// Zero-based class-table position.
        class_index: u32,
        /// Zero-based interface position within the class.
        interface_index: u32,
        /// Zero-based method position within the interface.
        method_index: u32,
    },
    /// A method parameter descriptor.
    Param {
        /// Zero-based class-table position.
        class_index: u32,
        /// Zero-based interface position within the class.
        interface_index: u32,
        /// Zero-based method position within the interface.
        method_index: u32,
        /// Zero-based parameter position within the method.
        param_index: u32,
    },
    /// A method return descriptor.
    Return {
        /// Zero-based class-table position.
        class_index: u32,
        /// Zero-based interface position within the class.
        interface_index: u32,
        /// Zero-based method position within the interface.
        method_index: u32,
    },
}

/// Machine-readable descriptor validation issue category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcDescriptorValidationKind {
    /// The top-level plugin descriptor pointer is null.
    NullPluginDescriptor,
    /// The descriptor declares a different core ABI generation.
    CoreAbiVersionMismatch,
    /// The plugin identity is the nil GUID.
    NilPluginId,
    /// The plugin qualified-name bytes are not valid UTF-8.
    InvalidPluginQualifiedNameUtf8,
    /// The plugin version bytes are not valid UTF-8.
    InvalidPluginVersionUtf8,
    /// The plugin qualified name violates the shared syntax or length policy.
    InvalidPluginQualifiedName,
    /// The plugin GUID is not derived from its qualified name.
    PluginIdQualifiedNameMismatch,
    /// The plugin version violates the shared text or length policy.
    InvalidPluginVersion,
    /// A nonzero plugin dependency count has a null table pointer.
    NullPluginDependencies,
    /// A hard dependency contains the nil plugin GUID.
    NilPluginDependencyId,
    /// The plugin declares its own identity as a hard dependency.
    SelfPluginDependency,
    /// The hard dependency table repeats a plugin GUID.
    DuplicatePluginDependencyId,
    /// The plugin descriptor has a null class-table pointer.
    NullClassTable,
    /// A nonzero class count has a null class-array pointer.
    NullClassTableClasses,
    /// A class identity is the nil GUID.
    NilClassId,
    /// Class qualified-name bytes are not valid UTF-8.
    InvalidClassQualifiedNameUtf8,
    /// Class version bytes are not valid UTF-8.
    InvalidClassVersionUtf8,
    /// A class qualified name violates the shared syntax or length policy.
    InvalidClassQualifiedName,
    /// A class GUID is not derived from its qualified name.
    ClassIdQualifiedNameMismatch,
    /// A class version violates the shared text or length policy.
    InvalidClassVersion,
    /// A class descriptor has no constructor function.
    MissingConstructor,
    /// A class threading field is not a defined `NC_ABI_THREADING_*` value.
    InvalidClassThreading,
    /// A nonzero class interface count has a null mapping-table pointer.
    NullClassInterfaces,
    /// A class does not expose exactly one canonical `IUnknown` mapping.
    MissingIUnknownInterface,
    /// A class's `IUnknown` mapping or descriptor is not canonical.
    InvalidIUnknownInterface,
    /// The plugin class table repeats a class GUID.
    DuplicateClassId,
    /// One class repeats a complete interface GUID/hash identity.
    DuplicateInterfaceId,
    /// A class-interface mapping has a null interface descriptor pointer.
    NullInterfaceDescriptor,
    /// An interface descriptor contains a nil IID.
    NilInterfaceDescriptorId,
    /// Interface qualified-name bytes are not valid UTF-8.
    InvalidInterfaceQualifiedNameUtf8,
    /// An interface qualified name violates the shared syntax or length policy.
    InvalidInterfaceQualifiedName,
    /// An interface IID is not derived from its qualified name.
    InterfaceIdQualifiedNameMismatch,
    /// An interface descriptor advertises the all-zero ABI hash.
    ZeroInterfaceAbiHash,
    /// Recomputed typed method metadata does not match the advertised ABI hash.
    InterfaceAbiHashMismatch,
    /// A base identity is malformed, absent from its class, incompatible, or cyclic.
    InvalidInterfaceBase,
    /// A nonzero method count has a null method-table pointer.
    NullInterfaceMethods,
    /// Method-name bytes are not valid UTF-8.
    InvalidMethodNameUtf8,
    /// A method descriptor contains an unknown receiver mode.
    InvalidMethodReceiver,
    /// A nonzero parameter count has a null parameter-table pointer.
    NullMethodParams,
    /// Parameter-name bytes are not valid UTF-8.
    InvalidParamNameUtf8,
    /// A passing descriptor contains an unknown tag or invalid mode payload.
    InvalidAbiPassing,
    /// A type descriptor contains an unknown tag or malformed payload.
    InvalidAbiType,
    /// A direct value/out contract names a type that is not valid in that role.
    InvalidValueType,
    /// A COM pointer passing contract does not name a valid interface type.
    InvalidInterfaceType,
}

/// One issue found while validating a plugin descriptor graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcDescriptorValidationIssue {
    /// ABI status corresponding to the validation failure.
    pub status: NcStatus,
    /// Descriptor graph location where the issue was found.
    pub path: NcDescriptorPath,
    /// Machine-readable issue category.
    pub kind: NcDescriptorValidationKind,
}

/// Accumulates all validation issues while retaining the first ABI status.
struct IssueReporter<R> {
    /// Caller-provided sink invoked once for every reachable issue.
    report: R,
    /// First negative status observed, or `NC_OK` before any issue.
    first_error: NcStatus,
    /// Number of issues emitted so subtrees can detect a clean validation pass.
    issue_count: usize,
}

impl<R> IssueReporter<R>
where
    R: FnMut(NcDescriptorValidationIssue),
{
    /// Creates an empty reporter around the caller's issue callback.
    fn new(report: R) -> Self {
        Self {
            report,
            first_error: NC_OK,
            issue_count: 0,
        }
    }

    /// Records an issue, preserves the first status, and notifies the callback.
    fn issue(&mut self, status: NcStatus, path: NcDescriptorPath, kind: NcDescriptorValidationKind) {
        if self.first_error.is_ok() {
            self.first_error = status;
        }
        self.issue_count += 1;
        (self.report)(NcDescriptorValidationIssue { status, path, kind });
    }

    /// Returns the number of issues reported so far.
    fn issue_count(&self) -> usize {
        self.issue_count
    }
}

/// Validates an unversioned plugin descriptor and reports every reachable issue.
///
/// The returned status is the first issue status, or [`NC_OK`] if the descriptor
/// graph is valid. Callers that only need pass/fail can pass `|_| {}` as the
/// reporter.
///
/// # Safety
///
/// `plugin` must be null or point to readable [`NcPluginDescriptor`] metadata.
/// If descriptor counts are nonzero, the corresponding table pointers must be
/// readable for those counts.
pub unsafe fn validate_plugin_descriptor_with_issues(plugin: *const NcPluginDescriptor, report: impl FnMut(NcDescriptorValidationIssue)) -> NcStatus {
    let mut reporter = IssueReporter::new(report);

    if plugin.is_null() {
        reporter.issue(NC_E_POINTER, NcDescriptorPath::Plugin, NcDescriptorValidationKind::NullPluginDescriptor);
        return reporter.first_error;
    }

    // SAFETY: The caller guarantees that non-null `plugin` points to readable
    // plugin descriptor metadata.
    let plugin = unsafe { &*plugin };
    validate_plugin_descriptor(plugin, &mut reporter);
    reporter.first_error
}

/// Validates the plugin root, identity text, dependencies, and class-table link.
fn validate_plugin_descriptor<R>(plugin: &NcPluginDescriptor, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    if plugin.core_abi_version != NC_COM_CORE_ABI_VERSION {
        reporter.issue(NC_E_ABI_VERSION, NcDescriptorPath::Plugin, NcDescriptorValidationKind::CoreAbiVersionMismatch);
        return;
    }
    if plugin.plugin_id.is_nil() {
        reporter.issue(NC_E_INVALID_ARG, NcDescriptorPath::Plugin, NcDescriptorValidationKind::NilPluginId);
    }

    if let Some(name) = unsafe {
        validate_static_text(
            plugin.qualified_name,
            NcDescriptorPath::Plugin,
            NcDescriptorValidationKind::InvalidPluginQualifiedNameUtf8,
            reporter,
        )
    } {
        let name_is_valid = validate_qualified_name(name).is_ok();
        if !name_is_valid {
            reporter.issue(
                NC_E_INVALID_ARG,
                NcDescriptorPath::Plugin,
                NcDescriptorValidationKind::InvalidPluginQualifiedName,
            );
        }
        if name_is_valid && plugin.plugin_id != guid_from_qualified_name(name) {
            reporter.issue(
                NC_E_INVALID_ARG,
                NcDescriptorPath::Plugin,
                NcDescriptorValidationKind::PluginIdQualifiedNameMismatch,
            );
        }
    }
    if let Some(version) = unsafe {
        validate_static_text(
            plugin.plugin_version,
            NcDescriptorPath::Plugin,
            NcDescriptorValidationKind::InvalidPluginVersionUtf8,
            reporter,
        )
    } {
        validate_version_text_with_issue(version, NcDescriptorPath::Plugin, NcDescriptorValidationKind::InvalidPluginVersion, reporter);
    }

    validate_plugin_dependencies(plugin, reporter);

    if plugin.class_table.is_null() {
        reporter.issue(NC_E_POINTER, NcDescriptorPath::ClassTable, NcDescriptorValidationKind::NullClassTable);
        return;
    }

    // SAFETY: Null was rejected above and the caller guarantees descriptor
    // graph pointers are readable for declared counts.
    let class_table = unsafe { &*plugin.class_table };
    validate_class_table(class_table, reporter);
}

/// Validates the dependency pointer/count pair and per-plugin uniqueness rules.
fn validate_plugin_dependencies<R>(plugin: &NcPluginDescriptor, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    let dependencies = if plugin.dependency_count == 0 {
        &[]
    } else if plugin.dependencies.is_null() {
        reporter.issue(NC_E_POINTER, NcDescriptorPath::Plugin, NcDescriptorValidationKind::NullPluginDependencies);
        return;
    } else {
        // SAFETY: Nonzero count has a non-null table pointer, and the caller's
        // descriptor graph contract makes the declared entries readable.
        unsafe { core::slice::from_raw_parts(plugin.dependencies, plugin.dependency_count as usize) }
    };

    #[cfg(feature = "alloc")]
    let mut seen = BTreeSet::new();
    for (dependency_index, dependency) in dependencies.iter().enumerate() {
        #[cfg(feature = "alloc")]
        let duplicate = !seen.insert(guid_key(dependency.plugin_id));
        #[cfg(not(feature = "alloc"))]
        let duplicate = dependencies[..dependency_index].iter().any(|other| other.plugin_id == dependency.plugin_id);
        validate_plugin_dependency(plugin, dependency_index, dependency, duplicate, reporter);
    }
}

/// Validates one dependency identity against nil, self, and duplicate edges.
fn validate_plugin_dependency<R>(
    plugin: &NcPluginDescriptor,
    dependency_index: usize,
    dependency: &NcPluginDependencyDescriptor,
    duplicate: bool,
    reporter: &mut IssueReporter<R>,
) where
    R: FnMut(NcDescriptorValidationIssue),
{
    let path = NcDescriptorPath::PluginDependency {
        dependency_index: dependency_index as u32,
    };
    if dependency.plugin_id.is_nil() {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::NilPluginDependencyId);
    } else if dependency.plugin_id == plugin.plugin_id {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::SelfPluginDependency);
    } else if duplicate {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::DuplicatePluginDependencyId);
    }
}

/// Validates the class pointer/count pair and detects duplicate class IDs.
fn validate_class_table<R>(table: &NcClassTable, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    let classes = if table.class_count == 0 {
        &[]
    } else if table.classes.is_null() {
        reporter.issue(NC_E_POINTER, NcDescriptorPath::ClassTable, NcDescriptorValidationKind::NullClassTableClasses);
        return;
    } else {
        // SAFETY: Nonzero `class_count` has a non-null class table pointer, and
        // the caller guarantees the table is readable for that count.
        unsafe { core::slice::from_raw_parts(table.classes, table.class_count as usize) }
    };

    #[cfg(feature = "alloc")]
    let mut seen = BTreeSet::new();
    for (class_index, class) in classes.iter().enumerate() {
        #[cfg(feature = "alloc")]
        let duplicate = !seen.insert(guid_key(class.class_id));
        #[cfg(not(feature = "alloc"))]
        let duplicate = classes[..class_index].iter().any(|other| other.class_id == class.class_id);
        validate_class_descriptor(class_index, class, duplicate, reporter);
    }
}

/// Validates one class's identity, text, constructor, threading, and interfaces.
fn validate_class_descriptor<R>(class_index: usize, class: &NcClassDescriptor, duplicate: bool, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    let path = NcDescriptorPath::Class { class_index: class_index as u32 };

    if class.class_id.is_nil() {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::NilClassId);
    } else if duplicate {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::DuplicateClassId);
    }

    if let Some(name) = unsafe { validate_static_text(class.qualified_name, path, NcDescriptorValidationKind::InvalidClassQualifiedNameUtf8, reporter) } {
        let name_is_valid = validate_qualified_name(name).is_ok();
        if !name_is_valid {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidClassQualifiedName);
        }
        if name_is_valid && class.class_id != guid_from_qualified_name(name) {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::ClassIdQualifiedNameMismatch);
        }
    }
    if let Some(version) = unsafe { validate_static_text(class.class_version, path, NcDescriptorValidationKind::InvalidClassVersionUtf8, reporter) } {
        validate_version_text_with_issue(version, path, NcDescriptorValidationKind::InvalidClassVersion, reporter);
    }

    if class.construct.is_none() {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::MissingConstructor);
    }
    if NcAbiThreadingMode::from_raw(class.threading).is_none() {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidClassThreading);
    }

    let interfaces = if class.interface_count == 0 {
        &[]
    } else if class.interfaces.is_null() {
        reporter.issue(NC_E_POINTER, path, NcDescriptorValidationKind::NullClassInterfaces);
        return;
    } else {
        // SAFETY: Nonzero `interface_count` has a non-null table pointer, and
        // the caller guarantees the table is readable for that count.
        unsafe { core::slice::from_raw_parts(class.interfaces, class.interface_count as usize) }
    };

    #[cfg(feature = "alloc")]
    let mut seen = BTreeSet::new();
    for (interface_index, class_interface) in interfaces.iter().enumerate() {
        #[cfg(feature = "alloc")]
        let duplicate = if class_interface.interface_descriptor.is_null() {
            false
        } else {
            // SAFETY: The descriptor graph contract makes every non-null
            // interface descriptor readable during validation.
            let interface = unsafe { &*class_interface.interface_descriptor };
            !seen.insert(guid_key(interface.interface_id))
        };
        #[cfg(not(feature = "alloc"))]
        let duplicate = if class_interface.interface_descriptor.is_null() {
            false
        } else {
            // SAFETY: The current non-null descriptor is readable under the
            // caller's descriptor graph contract.
            let interface = unsafe { &*class_interface.interface_descriptor };
            interfaces[..interface_index].iter().any(|other| {
                if other.interface_descriptor.is_null() {
                    return false;
                }
                // SAFETY: Previous non-null descriptors are readable under the
                // same descriptor graph contract.
                unsafe { (*other.interface_descriptor).interface_id == interface.interface_id }
            })
        };
        validate_class_interface_descriptor(class_index as u32, interface_index, class_interface, duplicate, reporter);
    }

    let mut has_iunknown = false;
    for (interface_index, class_interface) in interfaces.iter().enumerate() {
        if class_interface.interface_descriptor.is_null() {
            continue;
        }
        // SAFETY: The caller guarantees every non-null descriptor-graph pointer
        // is readable for validation.
        let interface = unsafe { &*class_interface.interface_descriptor };
        // Base identities are resolved only inside this already-readable class
        // table. The hop bound prevents hostile ancestry cycles from recursing.
        if !unsafe { valid_interface_ancestry(interface, interfaces) } {
            reporter.issue(
                NC_E_ABI_VERSION,
                NcDescriptorPath::Interface {
                    class_index: class_index as u32,
                    interface_index: interface_index as u32,
                },
                NcDescriptorValidationKind::InvalidInterfaceBase,
            );
        }
        if interface.interface_id != IID_IUNKNOWN {
            continue;
        }
        has_iunknown = true;
        let name_is_canonical = unsafe { interface.qualified_name.as_str() } == Ok(IUNKNOWN_QUALIFIED_NAME);
        if interface.abi_hash != IUNKNOWN_ABI_HASH || !interface.base.is_none() || interface.method_count != 0 || !name_is_canonical {
            reporter.issue(
                NC_E_ABI_VERSION,
                NcDescriptorPath::Interface {
                    class_index: class_index as u32,
                    interface_index: interface_index as u32,
                },
                NcDescriptorValidationKind::InvalidIUnknownInterface,
            );
        }
    }
    if !has_iunknown {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::MissingIUnknownInterface);
    }
}

/// Checks the bounded single-base chain without following new foreign pointers.
///
/// # Safety
/// Every non-null descriptor pointer in `interfaces` must remain readable.
unsafe fn valid_interface_ancestry(mut current: &NcInterfaceDescriptor, interfaces: &[NcClassInterfaceDescriptor]) -> bool {
    for _ in 0..interfaces.len() {
        let base = current.base;
        if base.is_none() {
            return true;
        }
        if base.iid.is_nil() || base.abi_hash.is_zero() || base.iid == IID_IUNKNOWN {
            return false;
        }
        let mut found = None;
        for candidate in interfaces {
            if candidate.interface_descriptor.is_null() {
                continue;
            }
            // SAFETY: The caller guarantees this existing class-table pointer.
            let candidate = unsafe { &*candidate.interface_descriptor };
            if candidate.interface_id == base.iid {
                if candidate.abi_hash != base.abi_hash {
                    return false;
                }
                found = Some(candidate);
                break;
            }
        }
        let Some(parent) = found else {
            return false;
        };
        current = parent;
    }
    false
}

/// Validates one class/interface edge and its referenced interface descriptor.
fn validate_class_interface_descriptor<R>(
    class_index: u32,
    interface_index: usize,
    class_interface: &NcClassInterfaceDescriptor,
    duplicate: bool,
    reporter: &mut IssueReporter<R>,
) where
    R: FnMut(NcDescriptorValidationIssue),
{
    let path = NcDescriptorPath::ClassInterface {
        class_index,
        interface_index: interface_index as u32,
    };

    if class_interface.interface_descriptor.is_null() {
        reporter.issue(NC_E_POINTER, path, NcDescriptorValidationKind::NullInterfaceDescriptor);
        return;
    }

    // SAFETY: Null was rejected above and the caller guarantees descriptor
    // graph pointers are readable.
    let interface = unsafe { &*class_interface.interface_descriptor };
    validate_interface_descriptor(class_index, interface_index as u32, interface, reporter);
    if duplicate {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::DuplicateInterfaceId);
    }
}

/// Validates interface identity, text, methods, and the recomputed ABI hash.
fn validate_interface_descriptor<R>(class_index: u32, interface_index: u32, interface: &NcInterfaceDescriptor, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    let path = NcDescriptorPath::Interface { class_index, interface_index };
    let issue_count_before = reporter.issue_count();

    if interface.interface_id.is_nil() {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::NilInterfaceDescriptorId);
    }
    if let Some(name) = unsafe {
        validate_static_text(
            interface.qualified_name,
            path,
            NcDescriptorValidationKind::InvalidInterfaceQualifiedNameUtf8,
            reporter,
        )
    } {
        let name_is_valid = validate_qualified_name(name).is_ok();
        if !name_is_valid {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidInterfaceQualifiedName);
        }
        if name_is_valid
            && interface.interface_id != IID_IUNKNOWN
            && (name == IUNKNOWN_QUALIFIED_NAME || interface.interface_id != guid_from_qualified_name(name))
        {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InterfaceIdQualifiedNameMismatch);
        }
    }
    if interface.abi_hash == NcAbiHash::ZERO {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::ZeroInterfaceAbiHash);
    }

    let methods = if interface.method_count == 0 {
        &[]
    } else if interface.methods.is_null() {
        reporter.issue(NC_E_POINTER, path, NcDescriptorValidationKind::NullInterfaceMethods);
        return;
    } else {
        // SAFETY: Nonzero `method_count` has a non-null table pointer, and the
        // caller guarantees the table is readable for that count.
        unsafe { core::slice::from_raw_parts(interface.methods, interface.method_count as usize) }
    };

    for (method_index, method) in methods.iter().enumerate() {
        validate_method_descriptor(class_index, interface_index, method_index as u32, method, reporter);
    }

    if reporter.issue_count() == issue_count_before && interface.abi_hash != NcAbiHash::ZERO {
        // SAFETY: The descriptor graph has passed the structural checks in this
        // interface subtree, including UTF-8 method/parameter names and typed
        // ABI descriptor tags. The hash pass only rereads that validated static
        // metadata.
        match unsafe { hash_interface_descriptor(interface, methods) } {
            Ok(expected_hash) if expected_hash == interface.abi_hash => {}
            Ok(_) => reporter.issue(NC_E_ABI_VERSION, path, NcDescriptorValidationKind::InterfaceAbiHashMismatch),
            Err(_) => {
                // The earlier validation pass should have reported the precise
                // malformed descriptor. Avoid adding a second generic issue.
            }
        }
    }
}

/// Validates one method name, return contract, and parameter table.
fn validate_method_descriptor<R>(class_index: u32, interface_index: u32, method_index: u32, method: &NcMethodDescriptor, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    let path = NcDescriptorPath::Method {
        class_index,
        interface_index,
        method_index,
    };

    let _ = unsafe { validate_static_text(method.name, path, NcDescriptorValidationKind::InvalidMethodNameUtf8, reporter) };
    if !matches!(method.receiver, NC_ABI_RECEIVER_SHARED | NC_ABI_RECEIVER_MUTABLE) {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidMethodReceiver);
    }
    validate_method_return(class_index, interface_index, method_index, &method.returns, reporter);

    let params = if method.param_count == 0 {
        &[]
    } else if method.params.is_null() {
        reporter.issue(NC_E_POINTER, path, NcDescriptorValidationKind::NullMethodParams);
        return;
    } else {
        // SAFETY: Nonzero `param_count` has a non-null table pointer, and the
        // caller guarantees the table is readable for that count.
        unsafe { core::slice::from_raw_parts(method.params, method.param_count as usize) }
    };

    for (param_index, param) in params.iter().enumerate() {
        validate_method_param(class_index, interface_index, method_index, param_index as u32, param, reporter);
    }
}

/// Validates one parameter's name and legal passing/type combination.
fn validate_method_param<R>(
    class_index: u32,
    interface_index: u32,
    method_index: u32,
    param_index: u32,
    param: &NcMethodParamDescriptor,
    reporter: &mut IssueReporter<R>,
) where
    R: FnMut(NcDescriptorValidationIssue),
{
    let path = NcDescriptorPath::Param {
        class_index,
        interface_index,
        method_index,
        param_index,
    };

    let _ = unsafe { validate_static_text(param.name, path, NcDescriptorValidationKind::InvalidParamNameUtf8, reporter) };

    let passing_ok = validate_abi_passing_descriptor(param.passing, path, reporter);
    let ty_ok = validate_abi_type_descriptor(&param.ty, path, reporter);
    if !(passing_ok && ty_ok) {
        return;
    }

    match param.passing.tag {
        NC_ABI_PASSING_VALUE | NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET | NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET => {
            validate_non_interface_value_type(&param.ty, path, reporter);
        }
        NC_ABI_PASSING_COM_REF => {
            validate_interface_type(&param.ty, path, reporter);
        }
        _ => {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidAbiPassing);
        }
    }
}

/// Validates the supported void, value-out, or COM-pointer return shape.
fn validate_method_return<R>(class_index: u32, interface_index: u32, method_index: u32, returns: &NcMethodReturnDescriptor, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    let path = NcDescriptorPath::Return {
        class_index,
        interface_index,
        method_index,
    };

    let passing_ok = validate_abi_passing_descriptor(returns.passing, path, reporter);
    let ty_ok = validate_abi_type_descriptor(&returns.ty, path, reporter);
    if !(passing_ok && ty_ok) {
        return;
    }

    match returns.passing.tag {
        NC_ABI_PASSING_VOID => {
            if returns.ty.tag != NC_ABI_TYPE_VOID {
                reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidValueType);
            }
        }
        NC_ABI_PASSING_OUT => {
            validate_non_interface_value_type(&returns.ty, path, reporter);
        }
        NC_ABI_PASSING_COM_PTR_OUT => {
            validate_interface_type(&returns.ty, path, reporter);
        }
        _ => {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidAbiPassing);
        }
    }
}

/// Checks a raw passing tag and its associated threading field.
fn validate_abi_passing_descriptor<R>(passing: NcAbiPassingDescriptor, path: NcDescriptorPath, reporter: &mut IssueReporter<R>) -> bool
where
    R: FnMut(NcDescriptorValidationIssue),
{
    match passing.tag {
        NC_ABI_PASSING_VOID | NC_ABI_PASSING_VALUE | NC_ABI_PASSING_OUT | NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET | NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET
            if passing.threading == NC_ABI_THREADING_THREAD_LOCAL =>
        {
            true
        }
        NC_ABI_PASSING_VOID | NC_ABI_PASSING_VALUE | NC_ABI_PASSING_OUT | NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET | NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET => {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidAbiPassing);
            false
        }
        NC_ABI_PASSING_COM_REF | NC_ABI_PASSING_COM_PTR_OUT => match passing.threading {
            NC_ABI_THREADING_THREAD_LOCAL | NC_ABI_THREADING_THREAD_SAFE => true,
            _ => {
                reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidAbiPassing);
                false
            }
        },
        _ => {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidAbiPassing);
            false
        }
    }
}

/// Checks a raw ABI type tag and any nested aggregate/interface identity.
fn validate_abi_type_descriptor<R>(ty: &NcAbiTypeDescriptor, path: NcDescriptorPath, reporter: &mut IssueReporter<R>) -> bool
where
    R: FnMut(NcDescriptorValidationIssue),
{
    match ty.tag {
        NC_ABI_TYPE_VOID
        | NC_ABI_TYPE_BOOL
        | NC_ABI_TYPE_U8
        | NC_ABI_TYPE_U16
        | NC_ABI_TYPE_U32
        | NC_ABI_TYPE_U64
        | NC_ABI_TYPE_USIZE
        | NC_ABI_TYPE_I8
        | NC_ABI_TYPE_I16
        | NC_ABI_TYPE_I32
        | NC_ABI_TYPE_I64
        | NC_ABI_TYPE_ISIZE
        | NC_ABI_TYPE_F32
        | NC_ABI_TYPE_F64
        | NC_ABI_TYPE_NC_GUID
        | NC_ABI_TYPE_NC_STATUS
        | NC_ABI_TYPE_NC_ABI_HASH => true,
        NC_ABI_TYPE_AGGREGATE => {
            if ty.aggregate_hash == NcAbiHash::ZERO {
                reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidAbiType);
                false
            } else {
                true
            }
        }
        NC_ABI_TYPE_INTERFACE => validate_interface_type(ty, path, reporter),
        _ => {
            reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidAbiType);
            false
        }
    }
}

/// Rejects void and interface descriptors from direct value positions.
fn validate_non_interface_value_type<R>(ty: &NcAbiTypeDescriptor, path: NcDescriptorPath, reporter: &mut IssueReporter<R>) -> bool
where
    R: FnMut(NcDescriptorValidationIssue),
{
    if ty.tag == NC_ABI_TYPE_VOID || ty.tag == NC_ABI_TYPE_INTERFACE {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidValueType);
        false
    } else {
        true
    }
}

/// Requires a complete, non-nil interface type descriptor.
fn validate_interface_type<R>(ty: &NcAbiTypeDescriptor, path: NcDescriptorPath, reporter: &mut IssueReporter<R>) -> bool
where
    R: FnMut(NcDescriptorValidationIssue),
{
    if ty.tag != NC_ABI_TYPE_INTERFACE || ty.interface.iid.is_nil() || ty.interface.abi_hash == NcAbiHash::ZERO {
        reporter.issue(NC_E_INVALID_ARG, path, NcDescriptorValidationKind::InvalidInterfaceType);
        false
    } else {
        true
    }
}

/// Reconstructs the canonical logical hash stream from a validated interface.
unsafe fn hash_interface_descriptor(interface: &NcInterfaceDescriptor, methods: &[NcMethodDescriptor]) -> Result<NcAbiHash, NcStatus> {
    let mut schema_hasher = NcAbiSchemaHasher::new();
    unsafe {
        // SAFETY: The caller validated the complete descriptor subtree. The
        // visitor consumes borrowed strings synchronously and retains nothing.
        visit_interface_hash_parts(interface, methods, |part| schema_hasher.update_abi_hash_part(part))?;
    }
    let schema = schema_hasher.finalize();

    let mut hasher = NcAbiHasher::new(schema);
    unsafe {
        // SAFETY: The same validated descriptor tree is traversed a second
        // time to append only compiler-resolved values to the schema digest.
        visit_interface_hash_parts(interface, methods, |part| hasher.update_abi_hash_part(part))?;
    }
    Ok(hasher.finalize())
}

/// Visits every logical hash part reconstructed from an interface descriptor.
///
/// The visitor is called synchronously, so method-name borrows never escape the
/// validated descriptor traversal.
unsafe fn visit_interface_hash_parts<F>(interface: &NcInterfaceDescriptor, methods: &[NcMethodDescriptor], mut visit: F) -> Result<(), NcStatus>
where
    F: for<'a> FnMut(NcAbiHashPart<'a>),
{
    visit(NcAbiHashPart::Interface { iid: interface.interface_id });

    if !interface.base.is_none() {
        visit(NcAbiHashPart::BaseInterface { interface: interface.base });
    }

    for (method_index, method) in methods.iter().enumerate() {
        let method_name = unsafe {
            // SAFETY: The caller validated method name UTF-8 metadata for this
            // descriptor subtree before computing the hash.
            method.name.as_str()?
        };
        visit(NcAbiHashPart::Method {
            index: method_index as u32,
            name: method_name,
        });
        let receiver = NcAbiReceiver::from_raw(method.receiver).ok_or(NC_E_INVALID_ARG)?;
        visit(NcAbiHashPart::Receiver { receiver });

        let params = unsafe {
            // SAFETY: The caller validated the method parameter pointer/count
            // pair before computing the hash.
            method.params()?
        };
        for (param_index, param) in params.iter().enumerate() {
            visit(NcAbiHashPart::Arg {
                index: param_index as u32,
                passing: descriptor_passing(param.passing)?,
                ty: descriptor_type(&param.ty)?,
            });
        }

        visit(NcAbiHashPart::Return {
            passing: descriptor_passing(method.returns.passing)?,
            ty: descriptor_type(&method.returns.ty)?,
        });
        visit(NcAbiHashPart::Status);
    }

    Ok(())
}

/// Converts a validated raw passing descriptor into its logical enum.
fn descriptor_passing(passing: NcAbiPassingDescriptor) -> Result<NcAbiPassing, NcStatus> {
    match passing.tag {
        NC_ABI_PASSING_VOID if passing.threading == NC_ABI_THREADING_THREAD_LOCAL => Ok(NcAbiPassing::Void),
        NC_ABI_PASSING_VALUE if passing.threading == NC_ABI_THREADING_THREAD_LOCAL => Ok(NcAbiPassing::Value),
        NC_ABI_PASSING_OUT if passing.threading == NC_ABI_THREADING_THREAD_LOCAL => Ok(NcAbiPassing::Out),
        NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET if passing.threading == NC_ABI_THREADING_THREAD_LOCAL => Ok(NcAbiPassing::CopyFromBufferOffset),
        NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET if passing.threading == NC_ABI_THREADING_THREAD_LOCAL => Ok(NcAbiPassing::CopyToBufferOffset),
        NC_ABI_PASSING_COM_REF => Ok(NcAbiPassing::ComRef {
            mode: descriptor_threading_mode(passing.threading)?,
        }),
        NC_ABI_PASSING_COM_PTR_OUT => Ok(NcAbiPassing::ComPtrOut {
            mode: descriptor_threading_mode(passing.threading)?,
        }),
        _ => Err(NC_E_INVALID_ARG),
    }
}

/// Converts a raw threading tag, rejecting values outside core ABI v5.
fn descriptor_threading_mode(threading: u32) -> Result<NcAbiThreadingMode, NcStatus> {
    NcAbiThreadingMode::from_raw(threading).ok_or(NC_E_INVALID_ARG)
}

/// Converts a validated physical type descriptor into its logical hash form.
fn descriptor_type(ty: &NcAbiTypeDescriptor) -> Result<NcAbiType, NcStatus> {
    match ty.tag {
        NC_ABI_TYPE_VOID => Ok(NcAbiType::Void),
        NC_ABI_TYPE_BOOL => Ok(NcAbiType::Bool),
        NC_ABI_TYPE_U8 => Ok(NcAbiType::U8),
        NC_ABI_TYPE_U16 => Ok(NcAbiType::U16),
        NC_ABI_TYPE_U32 => Ok(NcAbiType::U32),
        NC_ABI_TYPE_U64 => Ok(NcAbiType::U64),
        NC_ABI_TYPE_USIZE => Ok(NcAbiType::USize),
        NC_ABI_TYPE_I8 => Ok(NcAbiType::I8),
        NC_ABI_TYPE_I16 => Ok(NcAbiType::I16),
        NC_ABI_TYPE_I32 => Ok(NcAbiType::I32),
        NC_ABI_TYPE_I64 => Ok(NcAbiType::I64),
        NC_ABI_TYPE_ISIZE => Ok(NcAbiType::ISize),
        NC_ABI_TYPE_F32 => Ok(NcAbiType::F32),
        NC_ABI_TYPE_F64 => Ok(NcAbiType::F64),
        NC_ABI_TYPE_NC_GUID => Ok(NcAbiType::NcGuid),
        NC_ABI_TYPE_NC_STATUS => Ok(NcAbiType::NcStatus),
        NC_ABI_TYPE_NC_ABI_HASH => Ok(NcAbiType::NcAbiHash),
        NC_ABI_TYPE_AGGREGATE if ty.aggregate_hash != NcAbiHash::ZERO => Ok(NcAbiType::Aggregate(ty.aggregate_hash)),
        NC_ABI_TYPE_INTERFACE if !ty.interface.iid.is_nil() && ty.interface.abi_hash != NcAbiHash::ZERO => Ok(NcAbiType::Interface { iid: ty.interface.iid }),
        _ => Err(NC_E_INVALID_ARG),
    }
}

#[cfg(feature = "alloc")]
/// Produces an ordered key for duplicate GUID detection in allocation builds.
fn guid_key(guid: crate::NcGuid) -> (u32, u16, u16, [u8; 8]) {
    (guid.data1, guid.data2, guid.data3, guid.data4)
}

/// Reads static UTF-8 metadata and reports the supplied path-specific issue.
unsafe fn validate_static_text<'a, R>(
    value: NcStaticUtf8,
    path: NcDescriptorPath,
    kind: NcDescriptorValidationKind,
    reporter: &mut IssueReporter<R>,
) -> Option<&'a str>
where
    R: FnMut(NcDescriptorValidationIssue),
{
    // SAFETY: The caller guarantees the descriptor graph's static string
    // metadata remains valid for the returned lifetime.
    match unsafe { value.as_str() } {
        Ok(value) => Some(value),
        Err(status) => {
            reporter.issue(status, path, kind);
            None
        }
    }
}

/// Applies version-text policy and translates failure into a descriptor issue.
fn validate_version_text_with_issue<R>(value: &str, path: NcDescriptorPath, kind: NcDescriptorValidationKind, reporter: &mut IssueReporter<R>)
where
    R: FnMut(NcDescriptorValidationIssue),
{
    if validate_version_text(value).is_err() {
        reporter.issue(NC_E_INVALID_ARG, path, kind);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::{IUnknown, NcGuid};
    use std::vec::Vec;

    const PLUGIN_ID: NcGuid = guid_from_qualified_name("nc.tests.Plugin");
    const CLASS_ID: NcGuid = guid_from_qualified_name("nc.tests.Class");
    const INTERFACE_ID: NcGuid = guid_from_qualified_name("nc.tests.Interface");
    const DESCRIBED_INTERFACE_HASH: NcAbiHash = NcAbiHash::from_abi_parts(&[
        NcAbiHashPart::Interface { iid: INTERFACE_ID },
        NcAbiHashPart::Method { index: 0, name: "add" },
        NcAbiHashPart::Receiver { receiver: NcAbiReceiver::Shared },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::Value,
            ty: NcAbiType::I32,
        },
        NcAbiHashPart::Arg {
            index: 1,
            passing: NcAbiPassing::Value,
            ty: NcAbiType::I32,
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Out,
            ty: NcAbiType::I32,
        },
        NcAbiHashPart::Status,
    ]);
    static DESCRIBED_INTERFACE_PARAMS: [NcMethodParamDescriptor; 2] = [
        NcMethodParamDescriptor {
            name: NcStaticUtf8::from_static("a"),
            passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Value),
            ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::I32),
        },
        NcMethodParamDescriptor {
            name: NcStaticUtf8::from_static("b"),
            passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Value),
            ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::I32),
        },
    ];
    static DESCRIBED_INTERFACE_METHODS: [NcMethodDescriptor; 1] = [NcMethodDescriptor {
        name: NcStaticUtf8::from_static("add"),
        receiver: NC_ABI_RECEIVER_SHARED,
        param_count: DESCRIBED_INTERFACE_PARAMS.len() as u32,
        params: DESCRIBED_INTERFACE_PARAMS.as_ptr(),
        returns: NcMethodReturnDescriptor {
            passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Out),
            ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::I32),
        },
    }];

    unsafe extern "C" fn dummy_constructor(_class: *const NcClassDescriptor, _arguments: *const core::ffi::c_void, _out: *mut *mut IUnknown) -> NcStatus {
        NC_E_INVALID_ARG
    }

    const IUNKNOWN_CLASS_INTERFACE: NcClassInterfaceDescriptor = NcClassInterfaceDescriptor {
        interface_descriptor: &IUnknown::INTERFACE_DESCRIPTOR,
    };

    fn collect_issues(plugin: *const NcPluginDescriptor) -> (NcStatus, Vec<NcDescriptorValidationIssue>) {
        let mut issues = Vec::new();
        let status = unsafe { validate_plugin_descriptor_with_issues(plugin, |issue| issues.push(issue)) };
        (status, issues)
    }

    #[test]
    fn null_plugin_reports_pointer_issue_and_returns_first_status() {
        let (status, issues) = collect_issues(core::ptr::null());

        assert_eq!(status, NC_E_POINTER);
        assert_eq!(
            issues,
            std::vec![NcDescriptorValidationIssue {
                status: NC_E_POINTER,
                path: NcDescriptorPath::Plugin,
                kind: NcDescriptorValidationKind::NullPluginDescriptor,
            }]
        );
    }

    #[test]
    fn plugin_validation_reports_multiple_reachable_issues() {
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: NcGuid::nil(),
            qualified_name: NcStaticUtf8::from_static(""),
            plugin_version: NcStaticUtf8::from_static("1 0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: core::ptr::null(),
        };

        let (status, issues) = collect_issues(&PLUGIN);

        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(
            issues,
            std::vec![
                NcDescriptorValidationIssue {
                    status: NC_E_INVALID_ARG,
                    path: NcDescriptorPath::Plugin,
                    kind: NcDescriptorValidationKind::NilPluginId,
                },
                NcDescriptorValidationIssue {
                    status: NC_E_INVALID_ARG,
                    path: NcDescriptorPath::Plugin,
                    kind: NcDescriptorValidationKind::InvalidPluginQualifiedName,
                },
                NcDescriptorValidationIssue {
                    status: NC_E_INVALID_ARG,
                    path: NcDescriptorPath::Plugin,
                    kind: NcDescriptorValidationKind::InvalidPluginVersion,
                },
                NcDescriptorValidationIssue {
                    status: NC_E_POINTER,
                    path: NcDescriptorPath::ClassTable,
                    kind: NcDescriptorValidationKind::NullClassTable,
                },
            ]
        );
    }

    #[test]
    fn plugin_validation_rejects_invalid_dependency_entries() {
        const PLUGIN_ID: NcGuid = guid_from_qualified_name("nc.tests.Dependencies");
        const OTHER_ID: NcGuid = NcGuid::from_fields(5, 6, 7, [8; 8]);
        static DEPENDENCIES: [NcPluginDependencyDescriptor; 4] = [
            NcPluginDependencyDescriptor { plugin_id: NcGuid::nil() },
            NcPluginDependencyDescriptor { plugin_id: PLUGIN_ID },
            NcPluginDependencyDescriptor { plugin_id: OTHER_ID },
            NcPluginDependencyDescriptor { plugin_id: OTHER_ID },
        ];
        static CLASS_TABLE: NcClassTable = NcClassTable {
            class_count: 0,
            classes: core::ptr::null(),
        };
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: PLUGIN_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.Dependencies"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: DEPENDENCIES.len() as u32,
            dependencies: DEPENDENCIES.as_ptr(),
            class_table: &CLASS_TABLE,
        };

        let (status, issues) = collect_issues(&PLUGIN);
        assert_eq!(status, NC_E_INVALID_ARG);
        assert!(issues.iter().any(|issue| { issue.kind == NcDescriptorValidationKind::NilPluginDependencyId }));
        assert!(issues.iter().any(|issue| { issue.kind == NcDescriptorValidationKind::SelfPluginDependency }));
        assert!(
            issues
                .iter()
                .any(|issue| { issue.kind == NcDescriptorValidationKind::DuplicatePluginDependencyId })
        );
    }

    #[test]
    fn nested_issue_reports_precise_path_and_kind() {
        static INTERFACE: NcInterfaceDescriptor = NcInterfaceDescriptor {
            interface_id: INTERFACE_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.Interface"),
            abi_hash: NcAbiHash::ZERO,
            base: crate::NcInterfaceId::NONE,
            method_count: 0,
            methods: core::ptr::null(),
        };
        static CLASS_INTERFACES: [NcClassInterfaceDescriptor; 2] = [IUNKNOWN_CLASS_INTERFACE, NcClassInterfaceDescriptor { interface_descriptor: &INTERFACE }];
        static CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: CLASS_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.Class"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(dummy_constructor),
            interface_count: CLASS_INTERFACES.len() as u32,
            interfaces: CLASS_INTERFACES.as_ptr(),
        }];
        static CLASS_TABLE: NcClassTable = NcClassTable {
            class_count: CLASSES.len() as u32,
            classes: CLASSES.as_ptr(),
        };
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: PLUGIN_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.Plugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &CLASS_TABLE,
        };

        let (status, issues) = collect_issues(&PLUGIN);

        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(
            issues,
            std::vec![NcDescriptorValidationIssue {
                status: NC_E_INVALID_ARG,
                path: NcDescriptorPath::Interface { class_index: 0, interface_index: 1 },
                kind: NcDescriptorValidationKind::ZeroInterfaceAbiHash,
            }]
        );
    }

    #[test]
    fn interface_descriptor_hash_must_match_typed_metadata() {
        static VALID_INTERFACE: NcInterfaceDescriptor = NcInterfaceDescriptor {
            interface_id: INTERFACE_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.Interface"),
            abi_hash: DESCRIBED_INTERFACE_HASH,
            base: crate::NcInterfaceId::NONE,
            method_count: DESCRIBED_INTERFACE_METHODS.len() as u32,
            methods: DESCRIBED_INTERFACE_METHODS.as_ptr(),
        };
        static VALID_CLASS_INTERFACES: [NcClassInterfaceDescriptor; 2] =
            [IUNKNOWN_CLASS_INTERFACE, NcClassInterfaceDescriptor { interface_descriptor: &VALID_INTERFACE }];
        static VALID_CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: guid_from_qualified_name("nc.tests.ValidClass"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.ValidClass"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(dummy_constructor),
            interface_count: VALID_CLASS_INTERFACES.len() as u32,
            interfaces: VALID_CLASS_INTERFACES.as_ptr(),
        }];
        static VALID_CLASS_TABLE: NcClassTable = NcClassTable {
            class_count: VALID_CLASSES.len() as u32,
            classes: VALID_CLASSES.as_ptr(),
        };
        static VALID_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: guid_from_qualified_name("nc.tests.ValidPlugin"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.ValidPlugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &VALID_CLASS_TABLE,
        };

        let (status, issues) = collect_issues(&VALID_PLUGIN);

        assert_eq!(status, NC_OK);
        assert!(issues.is_empty());

        static MISMATCHED_INTERFACE: NcInterfaceDescriptor = NcInterfaceDescriptor {
            interface_id: INTERFACE_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.Interface"),
            abi_hash: NcAbiHash::from_bytes([0x7b; 32]),
            base: crate::NcInterfaceId::NONE,
            method_count: DESCRIBED_INTERFACE_METHODS.len() as u32,
            methods: DESCRIBED_INTERFACE_METHODS.as_ptr(),
        };
        static MISMATCHED_CLASS_INTERFACES: [NcClassInterfaceDescriptor; 2] = [
            IUNKNOWN_CLASS_INTERFACE,
            NcClassInterfaceDescriptor {
                interface_descriptor: &MISMATCHED_INTERFACE,
            },
        ];
        static MISMATCHED_CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: guid_from_qualified_name("nc.tests.MismatchedClass"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.MismatchedClass"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(dummy_constructor),
            interface_count: MISMATCHED_CLASS_INTERFACES.len() as u32,
            interfaces: MISMATCHED_CLASS_INTERFACES.as_ptr(),
        }];
        static MISMATCHED_CLASS_TABLE: NcClassTable = NcClassTable {
            class_count: MISMATCHED_CLASSES.len() as u32,
            classes: MISMATCHED_CLASSES.as_ptr(),
        };
        static MISMATCHED_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: guid_from_qualified_name("nc.tests.MismatchedPlugin"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.MismatchedPlugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &MISMATCHED_CLASS_TABLE,
        };

        let (status, issues) = collect_issues(&MISMATCHED_PLUGIN);

        assert_eq!(status, NC_E_ABI_VERSION);
        assert_eq!(
            issues,
            std::vec![NcDescriptorValidationIssue {
                status: NC_E_ABI_VERSION,
                path: NcDescriptorPath::Interface { class_index: 0, interface_index: 1 },
                kind: NcDescriptorValidationKind::InterfaceAbiHashMismatch,
            }]
        );
    }

    #[test]
    fn class_descriptor_rejects_unknown_threading_mode() {
        static CLASS_INTERFACES: [NcClassInterfaceDescriptor; 1] = [IUNKNOWN_CLASS_INTERFACE];
        static CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: guid_from_qualified_name("nc.tests.InvalidThreading"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.InvalidThreading"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: 0xffff_ffff,
            construct: Some(dummy_constructor),
            interface_count: CLASS_INTERFACES.len() as u32,
            interfaces: CLASS_INTERFACES.as_ptr(),
        }];
        static CLASS_TABLE: NcClassTable = NcClassTable {
            class_count: CLASSES.len() as u32,
            classes: CLASSES.as_ptr(),
        };
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: guid_from_qualified_name("nc.tests.InvalidThreadingPlugin"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.InvalidThreadingPlugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &CLASS_TABLE,
        };

        let (status, issues) = collect_issues(&PLUGIN);
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(
            issues,
            std::vec![NcDescriptorValidationIssue {
                status: NC_E_INVALID_ARG,
                path: NcDescriptorPath::Class { class_index: 0 },
                kind: NcDescriptorValidationKind::InvalidClassThreading,
            }]
        );
    }

    #[test]
    fn descriptor_ids_must_match_their_qualified_names() {
        const WRONG_INTERFACE_ID: NcGuid = guid_from_qualified_name("nc.tests.Identity.WrongInterface");
        const WRONG_INTERFACE_HASH: NcAbiHash = NcAbiHash::from_abi_parts(&[NcAbiHashPart::Interface { iid: WRONG_INTERFACE_ID }]);
        static INTERFACE: NcInterfaceDescriptor = NcInterfaceDescriptor {
            interface_id: WRONG_INTERFACE_ID,
            qualified_name: NcStaticUtf8::from_static("nc.tests.Identity.Interface"),
            abi_hash: WRONG_INTERFACE_HASH,
            base: crate::NcInterfaceId::NONE,
            method_count: 0,
            methods: core::ptr::null(),
        };
        static INTERFACES: [NcClassInterfaceDescriptor; 2] = [IUNKNOWN_CLASS_INTERFACE, NcClassInterfaceDescriptor { interface_descriptor: &INTERFACE }];
        static CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: guid_from_qualified_name("nc.tests.Identity.WrongClass"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.Identity.Class"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(dummy_constructor),
            interface_count: INTERFACES.len() as u32,
            interfaces: INTERFACES.as_ptr(),
        }];
        static CLASS_TABLE: NcClassTable = NcClassTable {
            class_count: CLASSES.len() as u32,
            classes: CLASSES.as_ptr(),
        };
        static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: guid_from_qualified_name("nc.tests.Identity.WrongPlugin"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.Identity.Plugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &CLASS_TABLE,
        };

        let (status, issues) = collect_issues(&PLUGIN);
        assert_eq!(status, NC_E_INVALID_ARG);
        assert!(
            issues
                .iter()
                .any(|issue| { issue.kind == NcDescriptorValidationKind::PluginIdQualifiedNameMismatch })
        );
        assert!(
            issues
                .iter()
                .any(|issue| { issue.kind == NcDescriptorValidationKind::ClassIdQualifiedNameMismatch })
        );
        assert!(
            issues
                .iter()
                .any(|issue| { issue.kind == NcDescriptorValidationKind::InterfaceIdQualifiedNameMismatch })
        );
    }

    #[test]
    fn class_descriptor_requires_one_canonical_iunknown_interface() {
        static NO_INTERFACES_CLASS: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: guid_from_qualified_name("nc.tests.MissingIUnknown"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.MissingIUnknown"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(dummy_constructor),
            interface_count: 0,
            interfaces: core::ptr::null(),
        }];
        static NO_INTERFACES_TABLE: NcClassTable = NcClassTable {
            class_count: NO_INTERFACES_CLASS.len() as u32,
            classes: NO_INTERFACES_CLASS.as_ptr(),
        };
        static NO_INTERFACES_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: guid_from_qualified_name("nc.tests.MissingIUnknownPlugin"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.MissingIUnknownPlugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &NO_INTERFACES_TABLE,
        };

        let (status, issues) = collect_issues(&NO_INTERFACES_PLUGIN);
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(
            issues,
            std::vec![NcDescriptorValidationIssue {
                status: NC_E_INVALID_ARG,
                path: NcDescriptorPath::Class { class_index: 0 },
                kind: NcDescriptorValidationKind::MissingIUnknownInterface,
            }]
        );

        const INVALID_IUNKNOWN_HASH: NcAbiHash = NcAbiHash::from_abi_parts(&[
            NcAbiHashPart::Interface { iid: IID_IUNKNOWN },
            NcAbiHashPart::Method { index: 0, name: "not_root" },
            NcAbiHashPart::Receiver { receiver: NcAbiReceiver::Shared },
            NcAbiHashPart::Return {
                passing: NcAbiPassing::Void,
                ty: NcAbiType::Void,
            },
            NcAbiHashPart::Status,
        ]);
        static INVALID_IUNKNOWN_METHOD: [NcMethodDescriptor; 1] = [NcMethodDescriptor {
            name: NcStaticUtf8::from_static("not_root"),
            receiver: NC_ABI_RECEIVER_SHARED,
            param_count: 0,
            params: core::ptr::null(),
            returns: NcMethodReturnDescriptor {
                passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Void),
                ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::Void),
            },
        }];
        static INVALID_IUNKNOWN: NcInterfaceDescriptor = NcInterfaceDescriptor {
            interface_id: IID_IUNKNOWN,
            qualified_name: NcStaticUtf8::from_static("IUnknown"),
            abi_hash: INVALID_IUNKNOWN_HASH,
            base: crate::NcInterfaceId::NONE,
            method_count: INVALID_IUNKNOWN_METHOD.len() as u32,
            methods: INVALID_IUNKNOWN_METHOD.as_ptr(),
        };
        static INVALID_IUNKNOWN_INTERFACES: [NcClassInterfaceDescriptor; 1] = [NcClassInterfaceDescriptor { interface_descriptor: &INVALID_IUNKNOWN }];
        static INVALID_IUNKNOWN_CLASS: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: guid_from_qualified_name("nc.tests.InvalidIUnknown"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.InvalidIUnknown"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(dummy_constructor),
            interface_count: INVALID_IUNKNOWN_INTERFACES.len() as u32,
            interfaces: INVALID_IUNKNOWN_INTERFACES.as_ptr(),
        }];
        static INVALID_IUNKNOWN_TABLE: NcClassTable = NcClassTable {
            class_count: INVALID_IUNKNOWN_CLASS.len() as u32,
            classes: INVALID_IUNKNOWN_CLASS.as_ptr(),
        };
        static INVALID_IUNKNOWN_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: guid_from_qualified_name("nc.tests.InvalidIUnknownPlugin"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.InvalidIUnknownPlugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &INVALID_IUNKNOWN_TABLE,
        };

        let (status, issues) = collect_issues(&INVALID_IUNKNOWN_PLUGIN);
        assert_eq!(status, NC_E_ABI_VERSION);
        assert_eq!(
            issues,
            std::vec![NcDescriptorValidationIssue {
                status: NC_E_ABI_VERSION,
                path: NcDescriptorPath::Interface { class_index: 0, interface_index: 0 },
                kind: NcDescriptorValidationKind::InvalidIUnknownInterface,
            }]
        );

        static DUPLICATE_IUNKNOWN_INTERFACES: [NcClassInterfaceDescriptor; 2] = [IUNKNOWN_CLASS_INTERFACE, IUNKNOWN_CLASS_INTERFACE];
        static DUPLICATE_IUNKNOWN_CLASS: [NcClassDescriptor; 1] = [NcClassDescriptor {
            class_id: guid_from_qualified_name("nc.tests.DuplicateIUnknown"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.DuplicateIUnknown"),
            class_version: NcStaticUtf8::from_static("1.0.0"),
            constructor_abi_hash: crate::NcAbiHash::ZERO,
            threading: NC_ABI_THREADING_THREAD_LOCAL,
            construct: Some(dummy_constructor),
            interface_count: DUPLICATE_IUNKNOWN_INTERFACES.len() as u32,
            interfaces: DUPLICATE_IUNKNOWN_INTERFACES.as_ptr(),
        }];
        static DUPLICATE_IUNKNOWN_TABLE: NcClassTable = NcClassTable {
            class_count: DUPLICATE_IUNKNOWN_CLASS.len() as u32,
            classes: DUPLICATE_IUNKNOWN_CLASS.as_ptr(),
        };
        static DUPLICATE_IUNKNOWN_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
            core_abi_version: NC_COM_CORE_ABI_VERSION,
            plugin_id: guid_from_qualified_name("nc.tests.DuplicateIUnknownPlugin"),
            qualified_name: NcStaticUtf8::from_static("nc.tests.DuplicateIUnknownPlugin"),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependency_count: 0,
            dependencies: core::ptr::null(),
            class_table: &DUPLICATE_IUNKNOWN_TABLE,
        };

        let (status, issues) = collect_issues(&DUPLICATE_IUNKNOWN_PLUGIN);
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(
            issues,
            std::vec![NcDescriptorValidationIssue {
                status: NC_E_INVALID_ARG,
                path: NcDescriptorPath::ClassInterface { class_index: 0, interface_index: 1 },
                kind: NcDescriptorValidationKind::DuplicateInterfaceId,
            }]
        );
    }
}
