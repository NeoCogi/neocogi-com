//! Matched static-size target and typed-array conversion throughput probe.
use std::{hint::black_box, time::Instant};
use neocogi_com::{ComDeserialize, ComSerialize, ComValue, NcValueRef};
use neocogi_com::serialization::{Format, Limits};

/// Times fresh COM decoding and destruction of one already encoded document.
fn format_case<T: ComSerialize + ComDeserialize>(name: &str, format: &Format, value: &T, iterations: usize) {
    let bytes = format.serialize(value).unwrap();
    for _ in 0..20 {
        black_box(format.deserialize::<T>(black_box(&bytes)).unwrap());
    }
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(format.deserialize::<T>(black_box(&bytes)).unwrap());
    }
    println!("{name},{iterations},{:.1}", start.elapsed().as_nanos() as f64 / iterations as f64);
}

/// Includes producer wire/backing creation, receiver acquisition and both drops.
fn value_case<T: ComValue>(name: &str, value: &T, iterations: usize) {
    for _ in 0..20 {
        black_box(NcValueRef::new(black_box(value)).unwrap().to_owned().unwrap());
    }
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(NcValueRef::new(black_box(value)).unwrap().to_owned().unwrap());
    }
    println!("{name},{iterations},{:.1}", start.elapsed().as_nanos() as f64 / iterations as f64);
}

/// Compares early shape rejection with failure at the final boolean element.
fn failure_cases(iterations: usize) {
    let short = *b"NCB\x01\0\0\0\0";
    let mut late = neocogi_com_serialization::ncb::to_vec(&[true; 512]).unwrap();
    *late.last_mut().unwrap() = 2;
    for (name, bytes) in [("ncb-wrong-length", short.as_slice()), ("ncb-late-bool", late.as_slice())] {
        let start = Instant::now();
        for _ in 0..iterations {
            assert!(black_box(neocogi_com_serialization::ncb::from_slice::<[bool; 512]>(black_box(bytes))).is_err());
        }
        println!("{name},{iterations},{:.1}", start.elapsed().as_nanos() as f64 / iterations as f64);
    }
}

/// Reports nanoseconds per conversion; run multiple samples on an idle machine.
fn main() {
    let iterations = std::env::args().nth(1).map(|s| s.parse::<usize>().unwrap()).unwrap_or(2000);
    assert!(iterations > 0);
    println!("case,iterations,ns_per_conversion");
    for (name, factory) in [
        ("ncb", neocogi_com_serialization::ncb::factory()),
        ("json", neocogi_com_serialization::json::factory()),
    ] {
        let format = Format::new(factory, Limits::default());
        format_case(&format!("{name}-scalar"), &format, &42u64, iterations);
        format_case(&format!("{name}-small"), &format, &[42u64; 8], iterations);
        format_case(&format!("{name}-large"), &format, &[42u64; 512], iterations);
        format_case(&format!("{name}-nested"), &format, &[[42u64; 32]; 32], iterations);
    }
    value_case("com-scalar", &42u64, iterations * 100);
    value_case("com-small", &[true; 8], iterations * 100);
    value_case("com-large", &[true; 512], iterations * 10);
    value_case("com-nested", &[[true; 32]; 32], iterations * 10);
    value_case("com-owned", &core::array::from_fn::<_, 8, _>(|_| String::from("backing")), iterations);
    failure_cases(iterations);
}
