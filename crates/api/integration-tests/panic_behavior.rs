#![cfg(unix)]

use neocogi_com::{ComPtr, IUnknown, NcStatus, com_object};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output};

const RUN_HELPER_ENV: &str = "NC_COM_RUN_ABI_PANIC_HELPER";
const RUN_DROP_HELPER_ENV: &str = "NC_COM_RUN_ABI_DROP_PANIC_HELPER";
const THUNK_RETURNED_STATUS_EXIT_CODE: i32 = 97;
const RELEASE_RETURNED_EXIT_CODE: i32 = 98;

extern crate self as fixture_panic_behavior;
neocogi_com_test_contracts::panic_behavior!();

#[com_object(interfaces(IPanics))]
pub struct PanickingObject;

impl IPanics for PanickingObject {
    fn panic_now(&self) -> Result<(), NcStatus> {
        panic!("intentional panic through generated ABI thunk")
    }
}

#[com_object(interfaces(IDropPanics))]
pub struct DropPanickingObject;

impl IDropPanics for DropPanickingObject {
    fn touch(&self) -> Result<(), NcStatus> {
        Ok(())
    }
}

impl Drop for DropPanickingObject {
    fn drop(&mut self) {
        panic!("intentional panic during generated COM Release")
    }
}

#[test]
fn panic_in_generated_abi_thunk_terminates_subprocess() {
    if std::env::var_os(RUN_HELPER_ENV).is_some() {
        return;
    }

    let output = run_helper("abi_panic_helper_exits_if_thunk_returns_status", RUN_HELPER_ENV);

    assert_helper_aborted_without_return(&output, THUNK_RETURNED_STATUS_EXIT_CODE, "ABI panic helper");
}

#[test]
fn panic_in_generated_release_thunk_terminates_subprocess() {
    if std::env::var_os(RUN_DROP_HELPER_ENV).is_some() {
        return;
    }

    let output = run_helper("abi_drop_panic_helper_exits_if_release_returns", RUN_DROP_HELPER_ENV);

    assert_helper_aborted_without_return(&output, RELEASE_RETURNED_EXIT_CODE, "ABI drop panic helper");
}

fn run_helper(test_name: &str, env_name: &str) -> Output {
    Command::new(std::env::current_exe().expect("test executable path"))
        .arg(test_name)
        .arg("--exact")
        .arg("--nocapture")
        .env(env_name, "1")
        .output()
        .expect("failed to run ABI panic helper subprocess")
}

fn assert_helper_aborted_without_return(output: &Output, returned_code: i32, label: &str) {
    assert!(
        !output.status.success(),
        "{label} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_ne!(
        output.status.code(),
        Some(returned_code),
        "{label} returned instead of terminating\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.signal().is_some(),
        "{label} exited normally instead of aborting\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn abi_panic_helper_exits_if_thunk_returns_status() {
    if std::env::var_os(RUN_HELPER_ENV).is_none() {
        return;
    }

    let unknown: ComPtr<IUnknown> = PanickingObject::from_value(PanickingObject);
    let panics = unknown.query::<IPanicsAbi>().unwrap();
    let raw = panics.as_raw();

    let status = unsafe { ((*raw.vtable).panic_now)(raw.object) };
    eprintln!("generated ABI thunk returned {status}");
    std::process::exit(THUNK_RETURNED_STATUS_EXIT_CODE);
}

#[test]
fn abi_drop_panic_helper_exits_if_release_returns() {
    if std::env::var_os(RUN_DROP_HELPER_ENV).is_none() {
        return;
    }

    let unknown: ComPtr<IUnknown> = DropPanickingObject::from_value(DropPanickingObject);
    let drop_panics = unknown.query::<IDropPanicsAbi>().unwrap();
    drop(drop_panics);
    drop(unknown);

    eprintln!("generated Release thunk returned after drop panic");
    std::process::exit(RELEASE_RETURNED_EXIT_CODE);
}
