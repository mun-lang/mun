use crate::{
    ir::IsIrType,
    type_info::{TypeInfo, TypeSize},
};
use hir::{ty_app, FloatBitness, HirDatabase, IntBitness, ResolveBitness, Ty, TypeCtor};
use inkwell::{
    context::Context,
    targets::TargetData,
    types::FunctionType,
    types::{AnyTypeEnum, BasicType, BasicTypeEnum, FloatType, IntType, StructType},
    AddressSpace,
};
use std::{cell::RefCell, collections::HashMap};

/// An object to cache and convert HIR types to Inkwell types.
pub struct HirTypeCache<'db, 'ink> {
    context: &'ink Context,
    db: &'db dyn HirDatabase,
    target_data: TargetData,
    types: RefCell<HashMap<hir::Ty, StructType<'ink>>>,
}

impl<'db, 'ink> HirTypeCache<'db, 'ink> {
    /// Constructs a new `HirTypeContext`
    pub fn new(context: &'ink Context, db: &'db dyn HirDatabase, target_data: TargetData) -> Self {
        Self {
            context,
            db,
            target_data,
            types: RefCell::new(HashMap::default()),
        }
    }

    /// Returns the type of the specified floating-point type
    pub fn get_float_type(&self, ty: hir::FloatTy) -> FloatType<'ink> {
        match ty.bitness {
            FloatBitness::X64 => self.context.f64_type(),
            FloatBitness::X32 => self.context.f32_type(),
        }
    }

    /// Returns the type of the specified integer type
    pub fn get_int_type(&self, ty: hir::IntTy) -> IntType<'ink> {
        match ty.bitness {
            IntBitness::X128 => self.context.i128_type(),
            IntBitness::X64 => self.context.i64_type(),
            IntBitness::X32 => self.context.i32_type(),
            IntBitness::X16 => self.context.i16_type(),
            IntBitness::X8 => self.context.i8_type(),
            IntBitness::Xsize => usize::ir_type(self.context, &self.target_data),
        }
    }

    /// Returns the type for booleans
    pub fn get_bool_type(&self) -> IntType<'ink> {
        self.context.bool_type()
    }

    /// Returns the type of the specified integer type
    pub fn get_struct_type(&self, struct_ty: hir::Struct) -> StructType<'ink> {
        // TODO: This assumes the contents of the hir::Struct does not change. It definitely does
        //  between compilations. We have to have a way to uniquely identify the `hir::Struct` and
        //  its contents.

        let ty = Ty::simple(TypeCtor::Struct(struct_ty));

        // Get the type from the cache
        if let Some(ir_ty) = self.types.borrow().get(&ty) {
            return *ir_ty;
        };

        // Opaquely construct the struct type and store it in the cache
        let ir_ty = self
            .context
            .opaque_struct_type(&struct_ty.name(self.db).to_string());
        self.types.borrow_mut().insert(ty, ir_ty);

        // Fill the struct members
        let field_types: Vec<_> = struct_ty
            .fields(self.db)
            .into_iter()
            .map(|field| field.ty(self.db))
            .map(|ty| {
                self.get_basic_type(&ty)
                    .expect("could not convert struct field to basic type")
            })
            .collect();
        ir_ty.set_body(&field_types, false);

        ir_ty
    }

    /// Returns the type of the struct that should be used for variables.
    pub fn get_struct_reference_type(&self, struct_ty: hir::Struct) -> BasicTypeEnum<'ink> {
        let ir_ty = self.get_struct_type(struct_ty);
        match struct_ty.data(self.db.upcast()).memory_kind {
            hir::StructMemoryKind::GC => {
                // GC values are pointers to pointers
                // struct Foo {}
                // Foo**
                ir_ty
                    .ptr_type(AddressSpace::Generic)
                    .ptr_type(AddressSpace::Generic)
                    .into()
            }
            hir::StructMemoryKind::Value => {
                // Value structs are passed as values
                ir_ty.into()
            }
        }
    }

    /// Returns the type of the struct that should be used in the public API. In the public API we
    /// don't deal with value types, only with pointers.
    pub fn get_public_struct_reference_type(&self, struct_ty: hir::Struct) -> BasicTypeEnum<'ink> {
        let ir_ty = self.get_struct_type(struct_ty);

        // GC values are pointers to pointers
        // struct Foo {}
        // Foo**
        //
        // Value structs are converted to GC types in the public API.
        ir_ty
            .ptr_type(AddressSpace::Generic)
            .ptr_type(AddressSpace::Generic)
            .into()
    }

    /// Returns the type of the specified function definition
    pub fn get_function_type(&self, ty: hir::Function) -> FunctionType<'ink> {
        let ty = self.db.callable_sig(ty.into());
        let param_tys: Vec<_> = ty
            .params()
            .iter()
            .map(|p| {
                self.get_basic_type(p)
                    .expect("could not convert function argument to basic type")
            })
            .collect();

        match ty.ret() {
            Ty::Empty => self.context.void_type().fn_type(&param_tys, false),
            ty => self
                .get_basic_type(&ty)
                .expect("could not convert return value")
                .fn_type(&param_tys, false),
        }
    }

    /// Returns the type of a specified function definition that is callable from the outside of the
    /// Mun code. This function should be C ABI compatible.
    pub fn get_public_function_type(&self, ty: hir::Function) -> FunctionType<'ink> {
        let ty = self.db.callable_sig(ty.into());
        let param_tys: Vec<_> = ty
            .params()
            .iter()
            .map(|p| {
                self.get_public_basic_type(p)
                    .expect("could not convert function argument to public basic type")
            })
            .collect();

        match ty.ret() {
            Ty::Empty => self.context.void_type().fn_type(&param_tys, false),
            ty => self
                .get_public_basic_type(&ty)
                .expect("could not convert return value")
                .fn_type(&param_tys, false),
        }
    }

    /// Returns the inkwell type of the specified HIR type as a basic value. If the type cannot be
    /// represented as a basic type enum, `None` is returned.
    pub fn get_basic_type(&self, ty: &hir::Ty) -> Option<BasicTypeEnum<'ink>> {
        match ty {
            Ty::Empty => Some(self.get_empty_type().into()),
            ty_app!(hir::TypeCtor::Float(float_ty)) => Some(self.get_float_type(*float_ty).into()),
            ty_app!(hir::TypeCtor::Int(int_ty)) => Some(self.get_int_type(*int_ty).into()),
            ty_app!(hir::TypeCtor::Struct(struct_ty)) => {
                Some(self.get_struct_reference_type(*struct_ty))
            }
            ty_app!(hir::TypeCtor::Bool) => Some(self.get_bool_type().into()),
            _ => None,
        }
    }

    /// Returns the inkwell type of the specified HIR type as a basic value that is usable from the
    /// public API. Internally this means that struct types are always pointers. If the type cannot
    /// be represented as a basic type enum, `None` is returned.
    pub fn get_public_basic_type(&self, ty: &hir::Ty) -> Option<BasicTypeEnum<'ink>> {
        match ty {
            Ty::Empty => Some(self.get_empty_type().into()),
            ty_app!(hir::TypeCtor::Float(float_ty)) => Some(self.get_float_type(*float_ty).into()),
            ty_app!(hir::TypeCtor::Int(int_ty)) => Some(self.get_int_type(*int_ty).into()),
            ty_app!(hir::TypeCtor::Struct(struct_ty)) => {
                Some(self.get_public_struct_reference_type(*struct_ty))
            }
            ty_app!(hir::TypeCtor::Bool) => Some(self.get_bool_type().into()),
            _ => None,
        }
    }

    /// Returns the inkwell type of the specified HIR type. If the type cannot be represented as an
    /// inkwell type, `None` is returned.
    pub fn get_any_type(&self, ty: &hir::Ty) -> Option<AnyTypeEnum<'ink>> {
        match ty {
            Ty::Empty => Some(self.get_empty_type().into()),
            ty_app!(hir::TypeCtor::Float(float_ty)) => Some(self.get_float_type(*float_ty).into()),
            ty_app!(hir::TypeCtor::Int(int_ty)) => Some(self.get_int_type(*int_ty).into()),
            ty_app!(hir::TypeCtor::Struct(struct_ty)) => {
                Some(self.get_struct_type(*struct_ty).into())
            }
            ty_app!(hir::TypeCtor::Bool) => Some(self.context.bool_type().into()),
            ty_app!(hir::TypeCtor::FnDef(hir::CallableDef::Function(fn_ty))) => {
                Some(self.get_function_type(*fn_ty).into())
            }
            _ => None,
        }
    }

    /// Returns the empty type
    pub fn get_empty_type(&self) -> StructType<'ink> {
        self.context.struct_type(&[], false)
    }

    /// Returns a `TypeInfo` for the specified `ty`
    pub fn type_info(&self, ty: &Ty) -> TypeInfo {
        match ty {
            Ty::Apply(ctor) => match ctor.ctor {
                TypeCtor::Float(ty) => {
                    let ir_ty = self.get_float_type(ty);
                    let type_size = TypeSize::from_ir_type(&ir_ty, &self.target_data);
                    TypeInfo::new_primitive(
                        format!("core::{}", ty.resolve(&self.db.target_data_layout())),
                        type_size,
                    )
                }
                TypeCtor::Int(ty) => {
                    let ir_ty = self.get_int_type(ty);
                    let type_size = TypeSize::from_ir_type(&ir_ty, &self.target_data);
                    TypeInfo::new_primitive(
                        format!("core::{}", ty.resolve(&self.db.target_data_layout())),
                        type_size,
                    )
                }
                TypeCtor::Bool => {
                    let ir_ty = self.get_bool_type();
                    let type_size = TypeSize::from_ir_type(&ir_ty, &self.target_data);
                    TypeInfo::new_primitive("core::bool", type_size)
                }
                TypeCtor::Struct(s) => {
                    let ir_ty = self.get_struct_type(s);
                    let type_size = TypeSize::from_ir_type(&ir_ty, &self.target_data);
                    TypeInfo::new_struct(self.db, s, type_size)
                }
                _ => unreachable!("{:?} unhandled", ctor),
            },
            _ => unreachable!("{:?} unhandled", ty),
        }
    }
}
