//! Generated host entries preserve ordinary Rust bodies and supplied access.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts body entry separately from idempotent host initialization.
static CALLS: AtomicUsize = AtomicUsize::new(0);

/// Ambient constants must not capture generated argument/local binding patterns.
#[allow(dead_code, non_upper_case_globals)]
mod entry {
    const __nc_host_api: usize = 17;
    const __nc_runtime: usize = 19;

    #[neocogi_com_sdk::com_main]
    /// Ordinary application body receives access and returns unit.
    pub fn main(runtime: neocogi_com_sdk::RuntimeHandle) {
        assert_eq!(runtime.abi_version(), neocogi_com_sdk::NC_COM_ABI_VERSION);
        super::CALLS.fetch_add(1, super::Ordering::SeqCst);
    }
}

/// Result/ExitCode and question-mark semantics stay with the authored function.
mod result_entry {
    #[neocogi_com_sdk::com_main]
    /// No conversion into a framework-specific application error is needed.
    pub fn main(runtime: neocogi_com_sdk::RuntimeHandle) -> Result<std::process::ExitCode, &'static str> {
        let admitted = Result::<_, &'static str>::Ok(runtime.abi_version())?;
        assert_eq!(admitted, neocogi_com_sdk::NC_COM_ABI_VERSION);
        Ok(std::process::ExitCode::SUCCESS)
    }
}

/// An early return still returns from the authored body before later statements.
mod exit_entry {
    #[neocogi_com_sdk::com_main]
    /// Returning an ordinary process code needs no executor or error wrapper.
    pub fn main(_runtime: neocogi_com_sdk::RuntimeHandle) -> std::process::ExitCode {
        std::process::ExitCode::SUCCESS
    }
}

/// Cfg removes the whole entry, including otherwise invalid body references.
mod excluded_entry {
    #[neocogi_com_sdk::com_main]
    #[cfg(any())]
    fn main(_runtime: neocogi_com_sdk::RuntimeHandle) {
        unavailable_application_body();
    }
}

#[test]
fn authored_body_runs_once_per_generated_entry_call() {
    entry::main();
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    // Repeated explicit entry calls still use the selected idempotent owner.
    entry::main();
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn ordinary_result_and_exit_code_returns_are_preserved() {
    assert_eq!(result_entry::main().unwrap(), std::process::ExitCode::SUCCESS);
    assert_eq!(exit_entry::main(), std::process::ExitCode::SUCCESS);
}

/// The outer C record may be call-lived; access retains its admitted static
/// children and supports concurrent copies without choosing another owner.
#[cfg(feature = "runtime")]
mod copied_access {
    #[neocogi_com_sdk::com_object(interfaces())]
    /// Minimal ordinary root keeps this check about host access, not product APIs.
    struct Root;

    #[neocogi_com_sdk::plugin(name = "nc.tests.host-access-copies")]
    /// Supplies an initialized local owner without discovering an implementation.
    fn acquire(_runtime: neocogi_com_sdk::RuntimeHandle) -> Result<neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown>, neocogi_com_sdk::NcStatus> {
        Ok(Root::from_value(Root))
    }

    #[test]
    fn copied_access_outlives_the_outer_record_and_shares_state_across_threads() {
        let runtime = {
            let record = *neocogi_com_host::initialize_process();
            // SAFETY: only this outer copy is temporary. Its reachable immutable
            // tables/code and synchronized host state all remain process-lived.
            unsafe { neocogi_com_sdk::RuntimeHandle::__from_host_api(&record) }.unwrap()
        };
        let copy = runtime;
        // Ordinary safe startup and explicit raw-record adoption must reach the
        // same host, including when the outer raw copy has already expired.
        let initialized = neocogi_com_host::initialize_runtime();
        std::thread::spawn(move || {
            // SAFETY: the descriptor and its complete provider graph are linked statics.
            unsafe { copy.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
        })
        .join()
        .unwrap();
        let id = plugin_descriptor().plugin_id;
        assert!(runtime.plugin_info(id).unwrap().is_some());
        assert!(initialized.plugin_info(id).unwrap().is_some());
        drop(runtime.acquire_plugin_root::<neocogi_com_sdk::IUnknown>(id).unwrap());
    }
}
