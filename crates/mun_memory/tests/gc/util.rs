#![allow(dead_code, unused_macros)]

use mun_memory::gc::{self, GcPtr};
use parking_lot::Mutex;

// pub struct TypeInfo {
//     pub size: usize,
//     pub alignment: usize,
//     pub tracer: Option<&'static fn(handle: GcPtr) -> Vec<GcPtr>>,
// }

pub trait Trace {
    /// Called to collect all GC handles in the type
    fn trace(&self, handles: &mut Vec<GcPtr>);
}

// impl gc::TypeTrace for &'static TypeInfo {
//     type Trace = <Vec<GcPtr> as IntoIterator>::IntoIter;

//     fn trace(&self, obj: GcPtr) -> Self::Trace {
//         let handles = if let Some(tracer) = self.tracer {
//             tracer(obj)
//         } else {
//             Vec::new()
//         };
//         handles.into_iter()
//     }
// }

pub struct EventAggregator<T: Sync + Send + Sized> {
    events: Mutex<Vec<T>>,
}

impl<T: Sync + Send + Sized> Default for EventAggregator<T> {
    fn default() -> Self {
        EventAggregator {
            events: Mutex::new(Vec::new()),
        }
    }
}

impl<T: Sync + Send + Sized> EventAggregator<T> {
    pub fn take_all(&self) -> Vec<T> {
        self.events.lock().drain(..).collect()
    }
}

impl<T: Sync + Send + Sized> gc::Observer for EventAggregator<T> {
    type Event = T;

    fn event(&self, event: T) {
        self.events.lock().push(event)
    }
}

#[macro_export]
macro_rules! assert_variant {
    ($value:expr, $pattern:pat) => {{
        let value = &$value;

        if let $pattern = value {
        } else {
            panic!(
                r#"assertion failed (value doesn't match pattern):
   value: `{:?}`,
 pattern: `{}`"#,
                value,
                stringify!($pattern)
            )
        }
    }}; // TODO: Additional patterns for trailing args, like assert and assert_eq
}
