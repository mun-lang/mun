use memory::gc;

/// Defines the garbage collector used by the `Runtime`.
pub type GarbageCollector = gc::MarkSweep<gc::NoopObserver<gc::Event>>;

pub type GcRootPtr = gc::GcRootPtr<GarbageCollector>;
