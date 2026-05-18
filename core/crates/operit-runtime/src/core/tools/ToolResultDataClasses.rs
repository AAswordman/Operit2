pub enum ToolResultData {
    BooleanResultData(BooleanResultData),
    StringResultData(StringResultData),
    IntResultData(IntResultData),
    BinaryResultData(BinaryResultData),
}

pub struct BooleanResultData {
    pub value: bool,
}

pub struct StringResultData {
    pub value: String,
}

pub struct IntResultData {
    pub value: i32,
}

pub struct BinaryResultData {
    pub value: Vec<u8>,
}
