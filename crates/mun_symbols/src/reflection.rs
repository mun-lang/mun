use crate::prelude::*;

// TODO: How to resolve generic fields and methods?
#[derive(Debug)]
pub struct TypeInfo {
    pub name: &'static str,
    pub uuid: Uuid,
    // module: &'static ModuleInfo,
    // fields: dyn Iterator<Item = &'static FieldInfo>,
    // methods: dyn Iterator<Item = &'static MethodInfo>,
}

pub trait Reflectable {
    fn type_info() -> &'static TypeInfo;
}

lazy_static! {
    static ref F32_TYPE_INFO: TypeInfo = TypeInfo {
        name: "f32",
        uuid: Uuid::parse_str("cd599d48-3a91-4e9f-b563-a96ee0e786d2").unwrap()
    };
}

impl Reflectable for f32 {
    fn type_info() -> &'static TypeInfo {
        &F32_TYPE_INFO
    }
}

// impl Reflectable for f32 {
//     fn type_info() -> TypeInfo {}

//     type FieldIter = iter::Empty<&'static FieldInfo>;
//     type MethodIter = iter::Empty<&'static MethodInfo>;

//     fn name(&self) -> &'static str {
//         "f32"
//     }

//     fn uuid(&self) -> &'static Uuid {
//         &F32_UUID
//     }

//     fn find_fields(&self, _: &FnMut(&FieldInfo)) -> Self::FieldIter {
//         iter::empty()
//     }

//     fn get_field(&self, _: &str) -> Option<&'static FieldInfo> {
//         None
//     }

//     fn get_fields(&self) -> Self::FieldIter {
//         iter::empty()
//     }

//     fn find_methods(&self, _: &FnMut(&MethodInfo)) -> Self::MethodIter {
//         iter::empty()
//     }

//     fn get_method(&self, _: &str) -> Option<&'static MethodInfo> {
//         None
//     }

//     fn get_methods(&self) -> Self::MethodIter {
//         iter::empty()
//     }
// }
