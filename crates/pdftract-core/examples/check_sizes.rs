use std::sync::Arc;
use indexmap::IndexMap;

fn main() {
    println!("IndexMap<Arc<str>, ()>: {}", std::mem::size_of::<IndexMap<Arc<str>, ()>>());
    println!("Vec<u8>: {}", std::mem::size_of::<Vec<u8>>());
    println!("Vec<()>: {}", std::mem::size_of::<Vec<()>>());
    println!("Arc<str>: {}", std::mem::size_of::<Arc<str>>());
}
