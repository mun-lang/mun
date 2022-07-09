/// An array in Mun is represented in memory by a header followed by the rest of the bytes.
#[repr(C)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
}
