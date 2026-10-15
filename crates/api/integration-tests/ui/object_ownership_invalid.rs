use neocogi_com::{com_object};

#[com_object(interfaces(), ownership = borrowed)]
struct Invalid;

#[com_object(interfaces(), ownership = shared, ownership = exclusive)]
struct Duplicate;

fn main() {}
