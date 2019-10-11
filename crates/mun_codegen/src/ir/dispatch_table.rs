use inkwell::module::Module;

pub(crate) struct DispatchTableBuilder<'a> {
    module: &'a Module,
}

impl<'a> DispatchTableBuilder<'a> {
    pub fn new(module: &'a Module) -> Self {
        DispatchTableBuilder {
            module
        }
    }
}
