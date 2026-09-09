use neocogi_com::{ComPtr, IUnknown, NcStatus, com_object, com_interface};

#[com_interface(auto_guid = "nc.examples.calculator.Calculator:v1")]
pub trait ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
    fn subtract(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.examples.calculator.Formatter:v1")]
pub trait IFormatter {
    fn describe(&self, value: i32) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.examples.calculator.NotImplemented:v1")]
pub trait INotImplemented {
    fn noop(&self, value: i32) -> Result<i32, NcStatus>;
}

#[com_object(interfaces(ICalculator, IFormatter))]
pub struct Calculator;

impl ICalculator for Calculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(a + b)
    }

    fn subtract(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(a - b)
    }
}

impl IFormatter for Calculator {
    fn describe(&self, value: i32) -> Result<i32, NcStatus> {
        // This first version only supports C-shaped scalar values.
        // A real string ABI would use a separate buffer/string interface.
        Ok(value)
    }
}

fn main() -> Result<(), NcStatus> {
    let unknown: ComPtr<IUnknown> = Calculator::create();
    println!("created Calculator as IUnknown: {unknown:?}");

    let calculator = unknown.query::<ICalculatorAbi>()?;
    println!("queried IUnknown -> ICalculatorAbi");
    println!("2 + 3 = {}", calculator.add(2, 3)?);
    println!("10 - 4 = {}", calculator.subtract(10, 4)?);

    let formatter = calculator.query::<IFormatterAbi>()?;
    println!("queried ICalculatorAbi -> IFormatterAbi");
    println!("formatter echoed {}", formatter.describe(42)?);

    let cloned = formatter.clone();
    println!("cloned IFormatterAbi pointer: {cloned:?}");
    drop(cloned);
    println!("dropped clone");

    let unsupported = unknown.query::<INotImplementedAbi>();
    println!(
        "query IUnknown -> INotImplementedAbi supported? {}",
        unsupported.is_ok()
    );

    Ok(())
}
