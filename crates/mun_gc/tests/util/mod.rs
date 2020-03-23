#![allow(dead_code, unused_macros)]

use mun_gc::{Event, GCPtr};
use parking_lot::Mutex;

pub struct TypeInfo {
    pub size: usize,
    pub alignment: usize,
    pub tracer: Option<&'static fn(handle: GCPtr) -> Vec<GCPtr>>,
}

pub trait Trace {
    /// Called to collect all GC handles in the type
    fn trace(&self, handles: &mut Vec<GCPtr>);
}

pub trait HasTypeInfo {
    fn type_info() -> &'static TypeInfo;
}

macro_rules! impl_primitive_types {
    ($(
        $ty:ident
    ),+) => {
        $(
            paste::item! {
                #[allow(non_upper_case_globals)]
                static [<TYPE_ $ty>]: TypeInfo = TypeInfo {
                    size: std::mem::size_of::<$ty>(),
                    alignment: std::mem::align_of::<$ty>(),
                    tracer: None
                };

                impl HasTypeInfo for $ty {
                    fn type_info() -> &'static TypeInfo {
                        &[<TYPE_ $ty>]
                    }
                }
            }
        )+
    }
}

macro_rules! impl_struct_ty {
    ($ty:ident) => {
        paste::item! {
            #[allow(non_upper_case_globals, non_snake_case)]
            fn [<trace_ $ty>](obj:GCPtr) -> Vec<GCPtr> {
                let mut result = Vec::new();
                let foo = unsafe { &(*obj.deref::<$ty>()) };
                foo.trace(&mut result);
                result
            }

            #[allow(non_upper_case_globals)]
            static [<TYPE_ $ty>]: TypeInfo = TypeInfo {
                size: std::mem::size_of::<$ty>(),
                alignment: std::mem::align_of::<$ty>(),
                tracer: Some(&([<trace_ $ty>] as fn(handle: GCPtr) -> Vec<GCPtr>))
            };

            impl HasTypeInfo for $ty {
                fn type_info() -> &'static TypeInfo {
                    &[<TYPE_ $ty>]
                }
            }
        }
    };
}

impl_primitive_types!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool);

impl mun_gc::Type for &'static TypeInfo {
    type Trace = <Vec<GCPtr> as IntoIterator>::IntoIter;

    fn size(&self) -> usize {
        self.size
    }

    fn alignment(&self) -> usize {
        self.alignment
    }

    fn trace(&self, obj: GCPtr) -> Self::Trace {
        let handles = if let Some(tracer) = self.tracer {
            tracer(obj)
        } else {
            Vec::new()
        };
        handles.into_iter()
    }
}

#[derive(Default)]
pub struct EventAggregator {
    events: Mutex<Vec<Event>>,
}

impl EventAggregator {
    pub fn take_all(&self) -> Vec<Event> {
        self.events.lock().drain(..).collect()
    }
}

impl mun_gc::Observer for EventAggregator {
    fn event(&self, event: Event) {
        self.events.lock().push(event)
    }
}

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
