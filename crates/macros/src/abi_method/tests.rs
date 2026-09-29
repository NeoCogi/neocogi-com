//! Borrowed parameter validation is shared without erasing wrapper-specific rules.

use super::{AbiArgKind, lower_abi_arg};

/// Parse a declaration through the same lowering boundary used by `com_interface`.
fn lower(source: &str) -> syn::Result<AbiArgKind> {
    let ty = syn::parse_str(source)?;
    let (_, _, kind) = lower_abi_arg(ty, &syn::parse_quote!(::neocogi_com))?;
    Ok(kind)
}

/// Assert a policy diagnostic rather than accepting an unrelated Rust parse error.
fn rejects(source: &str, expected: &str) {
    match lower(source) {
        Err(error) => assert!(error.to_string().contains(expected), "{source}: {error}"),
        Ok(_) => panic!("accepted invalid borrowed parameter: {source}"),
    }
}

/// Every supported borrow accepts the anonymous lifetime, including qualified paths.
#[test]
fn anonymous_lifetimes_keep_each_wrappers_lowering() {
    for prefix in ["", "neocogi_com::", "::neocogi_com::"] {
        assert!(matches!(lower(&format!("{prefix}NcValueRef<'_, Value>")), Ok(AbiArgKind::BorrowedValue { .. })));
        assert!(matches!(
            lower(&format!("{prefix}NcCopyFromBufferOffset<'_, u32>")),
            Ok(AbiArgKind::CopyFromBufferOffset { .. })
        ));
        assert!(matches!(
            lower(&format!("{prefix}NcCopyToBufferOffset<'_, u32>")),
            Ok(AbiArgKind::CopyToBufferOffset { .. })
        ));
        assert!(matches!(
            lower(&format!("{prefix}ComRef<'_, IObjectAbi>")),
            Ok(AbiArgKind::ComRef { optional: false, .. })
        ));
    }
}

/// Named lifetimes cannot promise that the receiver's borrow outlives the ABI call.
#[test]
fn all_borrowed_wrappers_reject_static_and_named_lifetimes() {
    for wrapper in ["NcValueRef", "ComRef", "NcCopyFromBufferOffset", "NcCopyToBufferOffset"] {
        for lifetime in ["'static", "'borrowed"] {
            for prefix in ["", "neocogi_com::"] {
                rejects(&format!("{prefix}{wrapper}<{lifetime}, u32>"), "must use `'_`");
            }
        }
    }
}

/// Missing, reordered, or extra generic arguments must not disappear during parsing.
#[test]
fn value_and_buffer_borrows_require_exact_lifetime_and_type_shape() {
    for wrapper in ["NcValueRef", "NcCopyFromBufferOffset", "NcCopyToBufferOffset"] {
        for arguments in ["", "<u32>", "<'_>", "<'_, u32, u64>", "<'_, u32, 3>", "<'_, 'static, u32>"] {
            rejects(&format!("{wrapper}{arguments}"), "parameters must be");
        }
        for arguments in ["<u32, '_>", "<u32, u64>", "<3, u32>"] {
            rejects(&format!("{wrapper}{arguments}"), "must use `'_`");
        }
        for arguments in ["<'_, 3>", "<'_, 'static>", "<'_, Item = u32>"] {
            rejects(&format!("{wrapper}{arguments}"), "element parameter must be a type");
        }
    }
}

/// ComRef retains its optional threading argument and nullable form, with the same lifetime rule.
#[test]
fn interface_borrow_options_do_not_bypass_lifetime_validation() {
    for wrapper in ["ComRef<'_, IObjectAbi>", "ComRef<'_, IObjectAbi, ThreadLocal>"] {
        assert!(matches!(lower(wrapper), Ok(AbiArgKind::ComRef { optional: false, .. })));
        assert!(matches!(lower(&format!("Option<{wrapper}>")), Ok(AbiArgKind::ComRef { optional: true, .. })));
    }
    for lifetime in ["'static", "'borrowed"] {
        rejects(&format!("ComRef<{lifetime}, IObjectAbi, ThreadLocal>"), "must use `'_`");
        rejects(&format!("Option<ComRef<{lifetime}, IObjectAbi>>"), "must use `'_`");
    }
    rejects("ComRef<IObjectAbi>", "parameters must be");
    rejects("ComRef<IObjectAbi, '_>", "must use `'_`");
    rejects("ComRef<'_, IObjectAbi, ThreadLocal, Extra>", "parameters must be");
    rejects("ComRef<'_, 3>", "interface parameter must be a type");
    rejects("ComRef<'_, IObjectAbi, 3>", "threading mode parameter must be a type");
}
