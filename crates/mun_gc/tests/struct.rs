#[macro_use]
mod util;

use mun_gc::{Event, GCHandle, GCRootHandle, GCRuntime, HasGCHandlePtr, MarkSweep, Type};
use std::sync::Arc;
use util::{EventAggregator, HasTypeInfo, Trace, TypeInfo};

struct Foo {
    bar: GCHandle,
}

impl Trace for Foo {
    fn trace(&self, handles: &mut Vec<GCHandle>) {
        handles.push(self.bar)
    }
}

impl_struct_ty!(Foo);

#[test]
fn test_trace() {
    let runtime = MarkSweep::<&'static TypeInfo, EventAggregator>::new();
    let foo_handle = runtime.alloc_object(Foo::type_info());
    let bar_handle = runtime.alloc_object(i64::type_info());

    // Assign bar to foo.bar
    unsafe {
        foo_handle.get_ptr::<Foo>().as_mut().bar = bar_handle;
    }

    // Trace foo to see if we get bar back
    let mut trace = Foo::type_info().trace(foo_handle);

    assert_eq!(trace.next(), Some(bar_handle));
    assert_eq!(trace.next(), None)
}

#[test]
fn trace_collect() {
    let runtime = Arc::new(MarkSweep::<&'static TypeInfo, EventAggregator>::new());
    let foo = unsafe { GCRootHandle::new(&runtime, runtime.alloc_object(Foo::type_info())) };
    let bar = runtime.alloc_object(i64::type_info());

    // Assign bar to foo.bar
    unsafe {
        foo.get_ptr::<Foo>().as_mut().bar = bar;
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
    let runtime = Arc::new(MarkSweep::<&'static TypeInfo, EventAggregator>::new());
    let foo = unsafe { GCRootHandle::new(&runtime, runtime.alloc_object(Foo::type_info())) };

    // Assign bar to foo.bar
    unsafe {
        foo.get_ptr::<Foo>().as_mut().bar = foo.handle();
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
