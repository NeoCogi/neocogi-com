//! Independent persistence writers exercise generated readers without sharing code.
pub mod contract;
#[cfg(test)]
mod tests {
    use crate::contract;
    use neocogi_com::{binary, ComSerialize};
    /// Writes names/ordinals independently of sparse ABI discriminants.
    #[derive(ComSerialize)]
    enum Mode { First, Second, Invalid }
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
        for (input, expected) in [(Mode::First, contract::Mode::First), (Mode::Second, contract::Mode::Second)] {
            let output: contract::Mode = binary::from_slice(&binary::to_vec(&input).unwrap()).unwrap();
            assert_eq!(output, expected);
        }
        assert!(binary::from_slice::<contract::Mode>(&binary::to_vec(&Mode::Invalid).unwrap()).is_err());
        let inputs = [Packet::Empty, Packet::Flag(true), Packet::Pair(2, 3), Packet::Values { flags: [true, false, true] }];
        for input in inputs {
            let output: contract::Packet = binary::from_slice(&binary::to_vec(&input).unwrap()).unwrap();
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
        assert!(matches!(decoded, contract::Tree::Nodes(nodes) if matches!(nodes[0], contract::Tree::Leaf(42))));
        for value in [Maybe::None, Maybe::Some(7)] {
            let _: contract::Number = binary::from_slice(&binary::to_vec(&Number { value }).unwrap()).unwrap();
        }
    }
}
