use neocogi_com::{com_object, com_factory};

#[com_object(interfaces(), ownership = borrowed)]
struct Invalid;

#[com_object(interfaces(), ownership = shared, ownership = exclusive)]
struct Duplicate;

#[com_factory(UnknownFactory, ownership = exclusive)]
struct ExclusiveFactory;

fn main() {}
