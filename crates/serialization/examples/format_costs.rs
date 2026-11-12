//! Reproducible static-executable size target and end-to-end COM throughput probe.
use std::{collections::BTreeMap, hint::black_box, time::Instant};
use neocogi_com::{ComSerialize, ComDeserialize, NcCopyFromBufferOffset};
use neocogi_com::serialization::{Format, Limits};

/// Representative scalar record, acquired through named or positional fields.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct Item {
    id: u32,
    enabled: bool,
    weight: f32,
    label: String,
}
/// A bounded initialization document with records, a typed map and optional data.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct State {
    revision: u64,
    title: String,
    items: Vec<Item>,
    counts: BTreeMap<String, u32>,
    selected: Option<u32>,
}

/// Measures fresh session creation, serialization, output copying, parsing,
/// typed construction and destruction; allocations remain inside the timing.
fn main() {
    let iterations: usize = std::env::args().nth(1).map(|s| s.parse().expect("positive iteration count")).unwrap_or(2000);
    assert!(iterations > 0);
    let value = State {
        revision: 42,
        title: "Neocogi initialization state".into(),
        items: (0..32)
            .map(|id| Item {
                id,
                enabled: id % 2 == 0,
                weight: id as f32 * 0.125,
                label: format!("item-{id:02}-é"),
            })
            .collect(),
        counts: BTreeMap::from([("frames".into(), 120), ("objects".into(), 32)]),
        selected: Some(17),
    };
    let root = neocogi_com_serialization::factory();
    println!("format,encoded_bytes,iterations,roundtrips_per_second,ns_per_roundtrip");
    for name in ["ncb", "json"] {
        let Ok(factory) = root.format(NcCopyFromBufferOffset::new(0, name.as_bytes())) else {
            continue;
        };
        let format = Format::new(factory, Limits::default());
        let bytes = format.serialize(&value).unwrap();
        assert_eq!(format.deserialize::<State>(&bytes).unwrap(), value);
        for _ in 0..50 {
            let bytes = format.serialize(black_box(&value)).unwrap();
            black_box(format.deserialize::<State>(black_box(&bytes)).unwrap());
        }
        let start = Instant::now();
        for _ in 0..iterations {
            let bytes = format.serialize(black_box(&value)).unwrap();
            black_box(format.deserialize::<State>(black_box(&bytes)).unwrap());
        }
        let seconds = start.elapsed().as_secs_f64();
        println!(
            "{name},{},{iterations},{:.0},{:.0}",
            bytes.len(),
            iterations as f64 / seconds,
            seconds * 1e9 / iterations as f64
        );
    }
}
