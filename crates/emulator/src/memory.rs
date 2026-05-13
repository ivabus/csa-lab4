use std::io::{Read, Write};

pub enum IO<'a> {
    I(Box<dyn Read + 'a>),
    O(Box<dyn Write + 'a>),
}
