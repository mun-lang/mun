use mun_runtime::Runtime;

// Ensures the [`Runtime`] is Send
trait IsSend: Send {}

#[allow(unused)]
impl IsSend for Runtime {}
