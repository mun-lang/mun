include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[repr(u8)]
pub enum Privacy {
    Public = 0,
    Private = 1,
}
