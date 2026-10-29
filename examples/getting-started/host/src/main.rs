//! Loads the tutorial plugin and calls its independently generated interface.

extern crate self as getting_started_host;

/// Bindings generated from the same source used by the provider.
mod contracts {
    include!(env!("CALCULATOR_BINDINGS"));
}

/// The executable selects and owns the one host implementation.
#[neocogi_com::com_main]
fn main(runtime: neocogi_com::Runtime) {
    let path = std::env::args_os().nth(1).expect("pass the built calculator plugin path");
    // SAFETY: The caller supplies this tutorial's trusted build artifact. Native
    // initializers can execute on open; the host pins the library for its lifetime.
    let plugin = unsafe { runtime.load_plugin_library(std::path::PathBuf::from(path)) };
    let calculator = runtime
        .acquire_plugin_root::<contracts::CalculatorAbi>(plugin)
        .expect("compatible calculator factory");
    println!("2 + 3 = {}", calculator.add(2, 3).expect("checked addition"));
    assert_eq!(calculator.add(i32::MAX, 1), Err(neocogi_com::NC_E_OUT_OF_RANGE));
}
