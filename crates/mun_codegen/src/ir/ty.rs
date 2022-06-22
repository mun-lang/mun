use std::sync::Arc;
use std::{cell::RefCell, collections::HashMap};

use inkwell::{
    context::Context,
    targets::TargetData,
    types::FunctionType,
    types::{AnyTypeEnum, BasicType, BasicTypeEnum, FloatType, IntType, StructType},
    AddressSpace,
};
use smallvec::SmallVec;

use abi::Guid;
use hir::{
    FloatBitness, HirDatabase, HirDisplay, IntBitness, ResolveBitness, Signedness, Ty, TyKind,
};

use crate::ir::IsIrType;
use crate::type_info::{HasStaticTypeId, TypeId, TypeIdData};

/// An object to cache and convert HIR types to Inkwell types.
pub struct HirTypeCache<'db, 'ink> {
    context: &'ink Context,
    db: &'db dyn HirDatabase,
    target_data: TargetData,
    types: RefCell<HashMap<hir::Ty, StructType<'ink>>>,
    struct_to_type_id: RefCell<HashMap<hir::Struct, Arc<TypeId>>>,
}

impl<'db, 'ink> HirTypeCache<'db, 'ink> {
    /// Constructs a new `HirTypeContext`
    pub fn new(context: &'ink Context, db: &'db dyn HirDatabase, target_data: TargetData) -> Self {
        Self {
            context,
            db,
            target_data,
            types: RefCell::new(HashMap::default()),
            struct_to_type_id: Default::default(),
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

        let ty = Ty::struct_ty(struct_ty);

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
            hir::StructMemoryKind::Gc => {
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
                    .into()
            })
            .collect();

        let return_type = ty.ret();
        match return_type.interned() {
            TyKind::Tuple(0, _) => self.context.void_type().fn_type(&param_tys, false),
            _ => self
                .get_basic_type(return_type)
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
                    .into()
            })
            .collect();

        let return_type = ty.ret();
        match return_type.interned() {
            TyKind::Tuple(0, _) => self.context.void_type().fn_type(&param_tys, false),
            _ => self
                .get_public_basic_type(return_type)
                .expect("could not convert return value")
                .fn_type(&param_tys, false),
        }
    }

    /// Returns the inkwell type of the specified HIR type as a basic value. If the type cannot be
    /// represented as a basic type enum, `None` is returned.
    pub fn get_basic_type(&self, ty: &hir::Ty) -> Option<BasicTypeEnum<'ink>> {
        match ty.interned() {
            TyKind::Tuple(_, substs) => Some(self.get_tuple_type(substs).into()),
            TyKind::Float(float_ty) => Some(self.get_float_type(*float_ty).into()),
            TyKind::Int(int_ty) => Some(self.get_int_type(*int_ty).into()),
            TyKind::Struct(struct_ty) => Some(self.get_struct_reference_type(*struct_ty)),
            TyKind::Bool => Some(self.get_bool_type().into()),
            _ => None,
        }
    }

    /// Returns the inkwell type of the specified HIR type as a basic value that is usable from the
    /// public API. Internally this means that struct types are always pointers. If the type cannot
    /// be represented as a basic type enum, `None` is returned.
    pub fn get_public_basic_type(&self, ty: &hir::Ty) -> Option<BasicTypeEnum<'ink>> {
        match ty.interned() {
            TyKind::Tuple(_, substs) => Some(self.get_tuple_type(substs).into()),
            TyKind::Float(float_ty) => Some(self.get_float_type(*float_ty).into()),
            TyKind::Int(int_ty) => Some(self.get_int_type(*int_ty).into()),
            TyKind::Struct(struct_ty) => Some(self.get_public_struct_reference_type(*struct_ty)),
            TyKind::Bool => Some(self.get_bool_type().into()),
            _ => None,
        }
    }

    /// Returns the inkwell type of the specified HIR type. If the type cannot be represented as an
    /// inkwell type, `None` is returned.
    pub fn get_any_type(&self, ty: &hir::Ty) -> Option<AnyTypeEnum<'ink>> {
        match ty.interned() {
            TyKind::Tuple(_, substs) => Some(self.get_tuple_type(substs).into()),
            TyKind::Float(float_ty) => Some(self.get_float_type(*float_ty).into()),
            TyKind::Int(int_ty) => Some(self.get_int_type(*int_ty).into()),
            TyKind::Struct(struct_ty) => Some(self.get_struct_type(*struct_ty).into()),
            TyKind::FnDef(hir::CallableDef::Function(fn_ty), type_params) => {
                if !type_params.is_empty() {
                    unimplemented!("cannot yet deal with type parameters in functions");
                }
                Some(self.get_function_type(*fn_ty).into())
            }
            TyKind::Bool => Some(self.get_bool_type().into()),
            _ => None,
        }
    }

    /// Returns the empty type
    pub fn get_empty_type(&self) -> StructType<'ink> {
        self.context.struct_type(&[], false)
    }

    /// Returns the type for a tuple
    pub fn get_tuple_type(&self, type_params: &[Ty]) -> StructType<'ink> {
        let mut tuple_ir_types: SmallVec<[BasicTypeEnum<'ink>; 2]> =
            SmallVec::with_capacity(type_params.len());
        for ty in type_params {
            tuple_ir_types.push(
                self.get_basic_type(ty)
                    .expect("tuple type should be a basic type"),
            )
        }
        self.context.struct_type(&tuple_ir_types, false)
    }

    /// Returns a `TypeInfo` for the specified `ty`
    pub fn type_id(&self, ty: &Ty) -> Arc<TypeId> {
        match ty.interned() {
            &TyKind::Float(ty) => match ty.bitness {
                FloatBitness::X32 => f32::type_id().clone(),
                FloatBitness::X64 => f64::type_id().clone(),
            },
            &TyKind::Int(ty) => {
                match (
                    ty.signedness,
                    ty.bitness.resolve(&self.db.target_data_layout()),
                ) {
                    (Signedness::Signed, IntBitness::X8) => i8::type_id().clone(),
                    (Signedness::Signed, IntBitness::X16) => i16::type_id().clone(),
                    (Signedness::Signed, IntBitness::X32) => i32::type_id().clone(),
                    (Signedness::Signed, IntBitness::X64) => i64::type_id().clone(),
                    (Signedness::Signed, IntBitness::X128) => i128::type_id().clone(),
                    (Signedness::Unsigned, IntBitness::X8) => u8::type_id().clone(),
                    (Signedness::Unsigned, IntBitness::X16) => u16::type_id().clone(),
                    (Signedness::Unsigned, IntBitness::X32) => u32::type_id().clone(),
                    (Signedness::Unsigned, IntBitness::X64) => u64::type_id().clone(),
                    (Signedness::Unsigned, IntBitness::X128) => u128::type_id().clone(),
                    (_, IntBitness::Xsize) => unreachable!(
                        "after resolve there should no longer be an undefined size type"
                    ),
                }
            }
            TyKind::Bool => bool::type_id().clone(),
            &TyKind::Struct(s) => {
                self.struct_to_type_id
                    .borrow_mut()
                    .entry(s)
                    .or_insert_with(|| {
                        Arc::new(TypeId {
                            name: s.full_name(self.db),
                            data: TypeIdData::Concrete(guid_from_struct(self.db, s)),
                        })
                    }).clone()
            }
            _ => unimplemented!("{} unhandled", ty.display(self.db)),
        }
    }
}

fn guid_from_struct(db: &dyn HirDatabase, s: hir::Struct) -> Guid {
    let name = s.full_name(db);
    let fields: Vec<String> = s
        .fields(db)
        .into_iter()
        .map(|f| {
            let ty_string = f
                .ty(db)
                .guid_string(db)
                .expect("type should be convertible to a string");
            format!("{}: {}", f.name(db), ty_string)
        })
        .collect();

    Guid::from_str(&format!(
        "struct {name}{{{fields}}}",
        name = &name,
        fields = fields.join(",")
    ))
}
