use std::sync::Arc;

use mun_memory::{
    gc::{Event, GcPtr, GcRootPtr, GcRuntime, HasIndirectionPtr, MarkSweep, TypeTrace},
    type_table::TypeTable,
};

use crate::{assert_variant, fake_struct};

use super::util::{EventAggregator, Trace};

struct FooObject {
    bar: GcPtr,
}

impl Trace for FooObject {
    fn trace(&self, handles: &mut Vec<GcPtr>) {
        handles.push(self.bar);
    }
}

#[test]
fn test_trace() {
    let mut type_table = TypeTable::default();

    let bar_type_info = fake_struct!(type_table, "core::Bar", "a" => i64);
    type_table.insert_type(bar_type_info.clone());

    let foo_type_info = fake_struct!(type_table, "core::Foo", "bar" => Bar);
    type_table.insert_type(foo_type_info.clone());

    let runtime = MarkSweep::<EventAggregator<Event>>::default();
    let mut foo_handle = runtime.alloc(&foo_type_info);
    let bar_handle = runtime.alloc(&bar_type_info);

    // Assign bar to foo.bar
    unsafe {
        (*foo_handle.deref_mut::<FooObject>()).bar = bar_handle;
    }

    // Trace foo to see if we get bar back
    let mut trace = foo_type_info.trace(foo_handle);

    assert_eq!(trace.next(), Some(bar_handle));
    assert_eq!(trace.next(), None);
}

#[test]
fn trace_collect() {
    let mut type_table = TypeTable::default();

    let bar_type_info = fake_struct!(type_table, "core::Bar", "a" => i64);
    type_table.insert_type(bar_type_info.clone());

    let foo_type_info = fake_struct!(type_table, "core::Foo", "bar" => Bar);
    type_table.insert_type(foo_type_info.clone());

    let runtime = Arc::new(MarkSweep::<EventAggregator<Event>>::default());
    let mut foo_ptr = GcRootPtr::new(&runtime, runtime.alloc(&foo_type_info));
    let bar = runtime.alloc(&bar_type_info);

    // Assign bar to foo.bar
    unsafe {
        (*foo_ptr.deref_mut::<FooObject>()).bar = bar;
    }

    // Collect garbage, bar should not be collected
    runtime.collect();

    // Drop foo
    let foo_instance = foo_ptr.unroot();

    // Collect garbage, both foo and bar should be collected
    runtime.collect();

    let mut events = runtime.observer().take_all().into_iter();
    assert_eq!(events.next(), Some(Event::Allocation(foo_instance)));
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
    let mut type_table = TypeTable::default();

    let bar_type_info = fake_struct!(type_table, "core::Bar", "a" => i64);
    type_table.insert_type(bar_type_info);

    let foo_type_info = fake_struct!(type_table, "core::Foo", "bar" => Bar);
    type_table.insert_type(foo_type_info.clone());

    let runtime = Arc::new(MarkSweep::<EventAggregator<Event>>::default());
    let mut foo_ptr = GcRootPtr::new(&runtime, runtime.alloc(&foo_type_info));

    // Assign foo to foo.bar
    unsafe {
        (*foo_ptr.deref_mut::<FooObject>()).bar = foo_ptr.handle();
    }

    // Collect garbage, nothing should be collected since foo is rooted
    runtime.collect();

    // Drop foo
    let unrooted_foo = foo_ptr.unroot();

    // Collect garbage, foo should be collected
    runtime.collect();

    let mut events = runtime.observer().take_all().into_iter();
    assert_eq!(events.next(), Some(Event::Allocation(unrooted_foo)));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::Deallocation(unrooted_foo)));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), None);
}
