use super::util::{EventAggregator, TypeInfo};
use mun_memory::gc::{Event, GcRootPtr, MarkSweep};
use std::sync::Arc;

#[test]
fn alloc() {
    let runtime = MarkSweep::<&'static TypeInfo, EventAggregator<Event>>::default();
    let handle = runtime.alloc(i64::type_info());

    assert!(std::ptr::eq(runtime.ptr_type(handle), i64::type_info()));

    let mut events = runtime.observer().take_all().into_iter();
    assert_eq!(events.next(), Some(Event::Allocation(handle)));
    assert_eq!(events.next(), None);
}

#[test]
fn collect_simple() {
    let runtime = MarkSweep::<&'static TypeInfo, EventAggregator<Event>>::default();
    let handle = runtime.alloc(i64::type_info());

    runtime.collect();

    let mut events = runtime.observer().take_all().into_iter();
    assert_eq!(events.next(), Some(Event::Allocation(handle)));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::Deallocation(handle)));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), None);
}

#[test]
fn collect_rooted() {
    let runtime = Arc::new(MarkSweep::<&'static TypeInfo, EventAggregator<Event>>::default());

    // Allocate simple object and rooted object
    let handle = runtime.alloc(i64::type_info());
    let rooted = GcRootPtr::new(&runtime, runtime.alloc(i64::type_info()));

    // Collect unreachable objects, should not collect the root handle
    runtime.collect();

    // Performing a collection cycle now should not do a thing
    runtime.collect();

    // Drop the rooted handle which should become collectable now
    let rooted_handle = rooted.unroot();

    // Collect unreachable objects, should now collect the rooted handle
    runtime.collect();

    // See if our version of events matched
    let mut events = runtime.observer().take_all().into_iter();
    assert_eq!(events.next(), Some(Event::Allocation(handle)));
    assert_eq!(events.next(), Some(Event::Allocation(rooted_handle)));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::Deallocation(handle)));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), Some(Event::Start));
    assert_eq!(events.next(), Some(Event::Deallocation(rooted_handle)));
    assert_eq!(events.next(), Some(Event::End));
    assert_eq!(events.next(), None);
}
