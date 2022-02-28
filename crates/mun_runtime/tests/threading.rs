use mun_runtime::Runtime;

// Ensures the [`Runtime`] is Send
trait IsSend: Send {}
impl IsSend for Runtime {}
