#![allow(dead_code, unused_macros)]

use mun_memory::gc::{self, GcPtr};
use parking_lot::Mutex;
use std::alloc::Layout;

pub struct TypeInfo {
    pub size: usize,
    pub alignment: usize,
    pub tracer: Option<&'static fn(handle: GcPtr) -> Vec<GcPtr>>,
}

pub trait Trace {
    /// Called to collect all GC handles in the type
    fn trace(&self, handles: &mut Vec<GcPtr>);
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

#[macro_export]
macro_rules! impl_struct_ty {
    ($ty:ident) => {
        paste::item! {
            #[allow(non_upper_case_globals, non_snake_case)]
            fn [<trace_ $ty>](obj:GcPtr) -> Vec<GcPtr> {
                let mut result = Vec::new();
                let foo = unsafe { &(*obj.deref::<$ty>()) };
                foo.trace(&mut result);
                result
            }

            #[allow(non_upper_case_globals)]
            static [<TYPE_ $ty>]: TypeInfo = TypeInfo {
                size: std::mem::size_of::<$ty>(),
                alignment: std::mem::align_of::<$ty>(),
                tracer: Some(&([<trace_ $ty>] as fn(handle: GcPtr) -> Vec<GcPtr>))
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

impl mun_memory::TypeLayout for &'static TypeInfo {
    fn layout(&self) -> Layout {
        Layout::from_size_align(self.size as usize, self.alignment as usize)
            .expect("invalid layout specified by TypeInfo")
    }

    fn memory_kind(&self) -> Option<abi::StructMemoryKind> {
        // NOTE: This contrived test does not support structs
        None
    }
}

impl gc::TypeTrace for &'static TypeInfo {
    type Trace = <Vec<GcPtr> as IntoIterator>::IntoIter;

    fn trace(&self, obj: GcPtr) -> Self::Trace {
        let handles = if let Some(tracer) = self.tracer {
            tracer(obj)
        } else {
            Vec::new()
        };
        handles.into_iter()
    }
}

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
