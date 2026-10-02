//! Independent persistence writers exercise generated readers without sharing code.
pub mod contract;
#[cfg(test)]
mod tests {
    use crate::contract;
    use neocogi_com::{binary, ComSerialize};
    /// Copied active fields cover every enum payload convention.
    #[derive(ComSerialize)]
    enum Packet { Empty, Flag(bool), Pair(u8, u16), Values { flags: [bool; 3] } }
    /// Owned active fields use the same variant conventions and owned storage.
    #[derive(ComSerialize)]
    enum Owned { Empty, Text(String), Pair(String, String), Values { values: Vec<u32> } }
    /// Indirect recursion must remain finite for the decoder capability analysis.
    #[derive(ComSerialize)]
    enum Tree { Leaf(u32), Nodes(Vec<Tree>) }
    /// Generic library shapes use the ordinary enum protocol.
    #[derive(ComSerialize)]
    enum Maybe { None, Some(u32) }
    /// Avoids relying on compiler-generated specialization names in the writer.
    #[derive(ComSerialize)]
    struct Number { value: Maybe }

    /// Reconstructs variants, including tuple/named payloads and sparse wire tags.
    #[test]
    fn reconstructs_copied_owned_recursive_and_generic_enums() {
        for (input, expected) in [(3u8, contract::Mode::First), (9u8, contract::Mode::Second)] {
            let output: contract::Mode = binary::from_slice(&binary::to_vec(&input).unwrap()).unwrap();
            assert_eq!(output, expected);
        }
        assert_eq!(binary::from_slice::<contract::Mode>(&binary::to_vec(&250u8).unwrap()).unwrap().raw(), 250);
        let inputs = [Packet::Empty, Packet::Flag(true), Packet::Pair(2, 3), Packet::Values { flags: [true, false, true] }];
        for input in inputs {
            let output: contract::Packet = binary::from_slice(&binary::to_vec(&input).unwrap()).unwrap();
            assert_eq!(binary::to_vec(&input).unwrap(), binary::to_vec(&output).unwrap());
            match (input, output) {
                (Packet::Empty, contract::Packet::Empty) => {},
                (Packet::Flag(a), contract::Packet::Flag(b)) => assert_eq!(a, b),
                (Packet::Pair(a, b), contract::Packet::Pair(c, d)) => assert_eq!((a, b), (c, d)),
                (Packet::Values { flags: a }, contract::Packet::Values { flags: b }) => assert_eq!(a, b),
                _ => panic!("variant changed"),
            }
        }
        let inputs = [Owned::Empty, Owned::Text("hello".into()), Owned::Pair("left".into(), "right".into()), Owned::Values { values: vec![1, 2] }];
        for input in inputs {
            let output: contract::Owned = binary::from_slice(&binary::to_vec(&input).unwrap()).unwrap();
            assert_eq!(binary::to_vec(&input).unwrap(), binary::to_vec(&output).unwrap());
            match (input, output) {
                (Owned::Empty, contract::Owned::Empty) => {},
                (Owned::Text(a), contract::Owned::Text(b)) => assert_eq!(a, b),
                (Owned::Pair(a, b), contract::Owned::Pair(c, d)) => assert_eq!((a, b), (c, d)),
                (Owned::Values { values: a }, contract::Owned::Values { values: b }) => assert_eq!(a, b),
                _ => panic!("variant changed"),
            }
        }
        let tree = Tree::Nodes(vec![Tree::Leaf(42)]);
        let decoded: contract::Tree = binary::from_slice(&binary::to_vec(&tree).unwrap()).unwrap();
        assert_eq!(binary::to_vec(&tree).unwrap(), binary::to_vec(&decoded).unwrap());
        assert!(matches!(decoded, contract::Tree::Nodes(nodes) if matches!(nodes[0], contract::Tree::Leaf(42))));
        for value in [Maybe::None, Maybe::Some(7)] {
            let bytes = binary::to_vec(&Number { value }).unwrap();
            let decoded: contract::Number = binary::from_slice(&bytes).unwrap();
            assert_eq!(binary::to_vec(&decoded).unwrap(), bytes);
        }
    }
    /// Automatic leaf codecs preserve all representable raw values and exact UUID bytes.
    #[test]
    fn scalar_and_sequence_persistence_is_symmetric() {
        let value = contract::Scalars {
            id: neocogi_com::NcGuid::from_uuid_bytes([0xAB; 16]),
            status: neocogi_com::NcStatus::from_code(-12345),
            flags: contract::Flags::from_raw(0x8003),
            mode: contract::Mode::from_raw(250),
        };
        let bytes = binary::to_vec(&value).unwrap();
        let decoded: contract::Scalars = binary::from_slice(&bytes).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(binary::to_vec(&decoded).unwrap(), bytes);
        let position = contract::Position { x: -2, y: 9 };
        let bytes = binary::to_vec(&position).unwrap();
        assert_eq!(binary::from_slice::<contract::Position>(&bytes).unwrap(), position);
        assert_eq!(binary::from_slice::<[i32; 2]>(&bytes).unwrap(), [-2, 9]);
        assert!(binary::from_slice::<neocogi_com::NcGuid>(&binary::to_vec(&[0u8; 15]).unwrap()).is_err());
    }

}
