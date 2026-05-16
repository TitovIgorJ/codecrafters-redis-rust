#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct BulkString {
    pub value: Vec<u8>,
}
