use mun_gc::Event;
use parking_lot::Mutex;

pub struct TypeInfo {
    size: usize,
    alignment: usize,
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

impl_primitive_types!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool);

impl mun_gc::Type for &'static TypeInfo {
    fn size(&self) -> usize {
        self.size
    }

    fn alignment(&self) -> usize {
        self.alignment
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

impl mun_gc::GCObserver for EventAggregator {
    fn event(&self, event: Event) {
        self.events.lock().push(event)
    }
}
