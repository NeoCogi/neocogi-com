//! A calculator provider with no dependency on the host implementation.

extern crate self as getting_started_plugin;

/// Provider-local bindings generated independently from the common source.
mod contracts {
    include!(env!("CALCULATOR_BINDINGS"));
}

/// Private application state implementing the public contract.
#[neocogi_com::com_object(contracts::Calculator)]
struct Calculator;

impl contracts::Calculator for Calculator {
    fn add(&self, left: i32, right: i32) -> Result<i32, neocogi_com::NcStatus> {
        left.checked_add(right).ok_or(neocogi_com::NC_E_OUT_OF_RANGE)
    }
}

/// Supplies one owned calculator through the fixed plugin bootstrap contract.
#[neocogi_com::plugin(name = "nc.tutorial.calculator")]
fn acquire(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<contracts::CalculatorAbi>, neocogi_com::NcStatus> {
    Calculator::from_value(Calculator).query()
}
