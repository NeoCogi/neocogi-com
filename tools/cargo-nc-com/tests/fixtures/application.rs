//! Minimal executable proving that the runtime and resources survive relocation.

/// Calls the runtime C ABI and resolves assets relative to the copied executable.
fn main() {
    // SAFETY: The tool has built and linked a matching runtime shared library.
    let abi = unsafe { nc_com_runtime_sys::nc_com_runtime_abi_version() };
    let executable = std::env::current_exe().unwrap();
    let message = std::fs::read_to_string(executable.parent().unwrap().join("assets/message.txt")).unwrap();
    print!("runtime={abi}; {message}");
    if let Some(code) = std::env::args().nth(1) {
        std::process::exit(code.parse().unwrap());
    }
}
