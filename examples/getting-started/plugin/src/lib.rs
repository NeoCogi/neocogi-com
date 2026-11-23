//! A calculator provider with no dependency on the host implementation.

extern crate self as getting_started_plugin;

use neocogi_com_sdk::{ComPtr, INcStringAbi, NcStatus, NcString, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};

/// Provider-local bindings generated independently from the common source.
mod contracts {
    include!(env!("CALCULATOR_BINDINGS"));
}

/// Private application state implementing the public contract.
#[neocogi_com_sdk::com_object(contracts::Calculator)]
struct Calculator {
    /// Whether the label is still available.
    labeled: bool,
}

impl contracts::Calculator for Calculator {
    fn add(&self, left: i32, right: i32) -> Result<i32, NcStatus> {
        left.checked_add(right).ok_or(NC_E_OUT_OF_RANGE)
    }

    fn find_label(&self, found: bool) -> Option<ComPtr<INcStringAbi>> {
        (found && self.labeled).then(|| NcString::from_utf8_str("calculator"))
    }

    fn load_label(&self, key: i32) -> Result<Option<ComPtr<INcStringAbi>>, NcStatus> {
        if key < 0 {
            return Err(NC_E_INVALID_ARG);
        }
        Ok(self.find_label(key == 0))
    }

    fn invalidate(&mut self) {
        self.labeled = false;
    }
}

/// Supplies one owned calculator through the fixed plugin bootstrap contract.
#[neocogi_com_sdk::plugin(name = "nc.tutorial.calculator")]
fn acquire(_runtime: neocogi_com_sdk::RuntimeHandle) -> Result<neocogi_com_sdk::ComPtr<contracts::CalculatorAbi>, neocogi_com_sdk::NcStatus> {
    Calculator::from_value(Calculator { labeled: true }).query()
}
