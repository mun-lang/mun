use inkwell::values::BasicValueEnum;

pub(crate) trait OptName {
    fn get_name(&self) -> Option<&str>;
    fn set_name<T: AsRef<str>>(&self, name: T);
}

impl OptName for BasicValueEnum {
    fn get_name(&self) -> Option<&str> {
        match self {
            BasicValueEnum::ArrayValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::IntValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::FloatValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::PointerValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::StructValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::VectorValue(v) => v.get_name().to_str().ok(),
        }
    }

    fn set_name<T: AsRef<str>>(&self, name: T) {
        match self {
            BasicValueEnum::ArrayValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::IntValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::FloatValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::PointerValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::StructValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::VectorValue(v) => v.set_name(name.as_ref()),
        };
    }
}
