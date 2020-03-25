#[macro_use]
mod util;

use mun_gc::{Event, GcPtr, GcRootPtr, GcRuntime, HasIndirectionPtr, MarkSweep, Type};
use std::sync::Arc;
use util::{EventAggregator, HasTypeInfo, Trace, TypeInfo};

struct Foo {
    bar: GcPtr,
}

impl Trace for Foo {
    fn trace(&self, handles: &mut Vec<GcPtr>) {
        handles.push(self.bar)
    }
}

impl_struct_ty!(Foo);

#[test]
fn test_trace() {
    let runtime = MarkSweep::<&'static TypeInfo, EventAggregator<Event>>::default();
    let mut foo_handle = runtime.alloc(Foo::type_info());
    let bar_handle = runtime.alloc(i64::type_info());

    // Assign bar to foo.bar
    unsafe {
        (*foo_handle.deref_mut::<Foo>()).bar = bar_handle;
    }

    // Trace foo to see if we get bar back
    let mut trace = Foo::type_info().trace(foo_handle);

    assert_eq!(trace.next(), Some(bar_handle));
    assert_eq!(trace.next(), None)
}

#[test]
fn trace_collect() {
    let runtime = Arc::new(MarkSweep::<&'static TypeInfo, EventAggregator<Event>>::default());
    let mut foo = GcRootPtr::new(&runtime, runtime.alloc(Foo::type_info()));
    let bar = runtime.alloc(i64::type_info());

    // Assign bar to foo.bar
    unsafe {
        (*foo.deref_mut::<Foo>()).bar = bar;
    }

    // Collect garbage, bar should not be collected
    runtime.collect();

    // Drop foo
    let foo = foo.unroot();

    // Collect garbage, both foo and bar should be collected
    runtime.collect();

    let mut events = runtime.observer().take_all().into_iter();
    assert_eq!(events.next(), Some(Event::Allocation(foo)));
    assert_eq!(events.next(), Some(Event::Allocation(bar)));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), Some(Event::Start));
    assert_variant!(events.next(), Some(Event::Deallocation(..))); // Don't care about the order
    assert_variant!(events.next(), Some(Event::Deallocation(..)));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), None);
}

#[test]
fn trace_cycle() {
    let runtime = Arc::new(MarkSweep::<&'static TypeInfo, EventAggregator<Event>>::default());
    let mut foo = GcRootPtr::new(&runtime, runtime.alloc(Foo::type_info()));

    // Assign bar to foo.bar
    unsafe {
        (*foo.deref_mut::<Foo>()).bar = foo.handle();
    }

    // Collect garbage, nothing should be collected since foo is rooted
    runtime.collect();

    // Drop foo
    let foo = foo.unroot();

    // Collect garbage, foo should be collected
    runtime.collect();

    let mut events = runtime.observer().take_all().into_iter();
    assert_eq!(events.next(), Some(Event::Allocation(foo)));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::Deallocation(foo)));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), None);
}
