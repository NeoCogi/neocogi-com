//! Loads the tutorial plugin and calls its independently generated interface.

extern crate self as getting_started_host;

use neocogi_com_sdk::ComStringExt;

/// Bindings generated from the same source used by the provider.
mod contracts {
    include!(env!("CALCULATOR_BINDINGS"));
}

/// The executable selects and owns the one host implementation.
#[neocogi_com_sdk::com_main]
fn main(runtime: neocogi_com_sdk::RuntimeHandle) {
    let path = std::env::args_os().nth(1).expect("pass the built calculator plugin path");
    // SAFETY: The caller supplies this tutorial's trusted build artifact. Native
    // initializers can execute on open; the host pins the library for its lifetime.
    let plugin = unsafe { runtime.load_plugin_library(std::path::PathBuf::from(path)) };
    let mut calculator = runtime
        .acquire_plugin_root::<contracts::CalculatorAbi>(plugin)
        .expect("compatible calculator factory");
    println!("2 + 3 = {}", calculator.add(2, 3).expect("checked addition"));
    assert_eq!(calculator.add(i32::MAX, 1), Err(neocogi_com_sdk::NC_E_OUT_OF_RANGE));

    let label = calculator.find_label(true).expect("available label");
    assert!(calculator.find_label(false).is_none());
    assert!(calculator.load_label(1).unwrap().is_none());
    assert_eq!(calculator.load_label(-1).unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
    let loaded = calculator.load_label(0).unwrap().expect("available label");
    calculator.invalidate();
    assert!(calculator.find_label(true).is_none());
    drop(calculator);
    // Both calls transferred a reference with a lifetime independent of the source.
    assert_eq!(label.to_string().unwrap(), "calculator");
    assert_eq!(loaded.to_string().unwrap(), "calculator");
}
