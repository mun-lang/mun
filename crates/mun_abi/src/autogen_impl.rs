use crate::prelude::*;

use std::convert::TryInto;
use std::ffi::{c_void, CStr};
use std::fmt::Formatter;
use std::marker::{Send, Sync};
use std::mem;
use std::str;
use std::{fmt, slice};

impl TypeInfo {
    /// Returns the type's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }

    /// Retrieves the type's struct information, if available.
    pub fn as_struct(&self) -> Option<&StructInfo> {
        if self.group.is_struct() {
            let ptr = (self as *const TypeInfo).cast::<u8>();
            let ptr = ptr.wrapping_add(mem::size_of::<TypeInfo>());
            let offset = ptr.align_offset(mem::align_of::<StructInfo>());
            let ptr = ptr.wrapping_add(offset);
            Some(unsafe { &*ptr.cast::<StructInfo>() })
        } else {
            None
        }
    }

    /// Returns the size of the type in bits
    pub fn size_in_bits(&self) -> usize {
        self.size_in_bits
            .try_into()
            .expect("cannot convert size in bits to platform size")
    }

    /// Returns the size of the type in bytes
    pub fn size_in_bytes(&self) -> usize {
        ((self.size_in_bits + 7) / 8)
            .try_into()
            .expect("cannot covert size in bytes to platform size")
    }

    /// Returns the alignment of the type in bytes
    pub fn alignment(&self) -> usize {
        self.alignment
            .try_into()
            .expect("cannot convert alignment to platform size")
    }
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

unsafe impl Send for TypeInfo {}
unsafe impl Sync for TypeInfo {}

impl FunctionSignature {
    /// Returns the function's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }

    /// Returns the function's privacy level.
    pub fn privacy(&self) -> Privacy {
        self.privacy
    }

    /// Returns the function's arguments' types.
    pub fn arg_types(&self) -> &[&TypeInfo] {
        if self.num_arg_types == 0 {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(
                    self.arg_types.cast::<&TypeInfo>(),
                    self.num_arg_types as usize,
                )
            }
        }
    }

    /// Returns the function's return type
    pub fn return_type(&self) -> Option<&TypeInfo> {
        unsafe { self.return_type.as_ref() }
    }
}

impl fmt::Display for FunctionSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name())?;
        for (i, arg) in self.arg_types().iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")?;
        if let Some(ret_type) = self.return_type() {
            write!(f, ":{}", ret_type)?
        }
        Ok(())
    }
}

unsafe impl Send for FunctionSignature {}
unsafe impl Sync for FunctionSignature {}

unsafe impl Send for FunctionInfo {}
unsafe impl Sync for FunctionInfo {}

impl StructInfo {
    /// Returns the struct's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }

    /// Returns the struct's field names.
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        let field_names = if self.num_fields == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.field_names, self.num_fields as usize) }
        };

        field_names
            .iter()
            .map(|n| unsafe { str::from_utf8_unchecked(CStr::from_ptr(*n).to_bytes()) })
    }

    /// Returns the struct's field types.
    pub fn field_types(&self) -> &[&TypeInfo] {
        if self.num_fields == 0 {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(
                    self.field_types.cast::<&TypeInfo>(),
                    self.num_fields as usize,
                )
            }
        }
    }

    /// Returns the struct's field offsets.
    pub fn field_offsets(&self) -> &[u16] {
        if self.num_fields == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.field_offsets, self.num_fields as usize) }
        }
    }

    /// Returns the index of the field matching the specified `field_name`.
    pub fn find_field_index(
        type_name: &str,
        struct_info: &StructInfo,
        field_name: &str,
    ) -> Result<usize, String> {
        struct_info
            .field_names()
            .enumerate()
            .find(|(_, name)| *name == field_name)
            .map(|(idx, _)| idx)
            .ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    type_name, field_name
                )
            })
    }
}

impl fmt::Display for StructInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ModuleInfo {
    /// Returns the module's full path.
    pub fn path(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.path).to_bytes()) }
    }

    // /// Finds the type's fields that match `filter`.
    // pub fn find_fields(&self, filter: fn(&&FieldInfo) -> bool) -> impl Iterator<Item = &FieldInfo> {
    //     self.fields.iter().map(|f| *f).filter(filter)
    // }

    // /// Retrieves the type's field with the specified `name`, if it exists.
    // pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
    //     self.fields.iter().find(|f| f.name == name).map(|f| *f)
    // }

    // /// Retrieves the type's fields.
    // pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
    //     self.fields.iter().map(|f| *f)
    // }

    /// Returns the module's functions.
    pub fn functions(&self) -> &[FunctionInfo] {
        if self.num_functions == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.functions, self.num_functions as usize) }
        }
    }

    /// Returns the module's types.
    pub fn types(&self) -> &[&TypeInfo] {
        if self.num_types == 0 {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(self.types.cast::<&TypeInfo>(), self.num_types as usize)
            }
        }
    }
}

unsafe impl Send for ModuleInfo {}
unsafe impl Sync for ModuleInfo {}

impl DispatchTable {
    /// Returns an iterator over pairs of mutable function pointers and signatures.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut *const c_void, &FunctionSignature)> {
        if self.num_entries == 0 {
            (&mut []).iter_mut().zip((&[]).iter())
        } else {
            let ptrs =
                unsafe { slice::from_raw_parts_mut(self.fn_ptrs, self.num_entries as usize) };
            let signatures =
                unsafe { slice::from_raw_parts(self.signatures, self.num_entries as usize) };

            ptrs.iter_mut().zip(signatures.iter())
        }
    }

    /// Returns mutable functions pointers.
    pub fn ptrs_mut(&mut self) -> &mut [*const c_void] {
        if self.num_entries == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(self.fn_ptrs, self.num_entries as usize) }
        }
    }

    /// Returns function signatures.
    pub fn signatures(&self) -> &[FunctionSignature] {
        if self.num_entries == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.signatures, self.num_entries as usize) }
        }
    }

    /// Returns a function pointer, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution! Calling this method with an
    /// out-of-bounds index is _undefined behavior_ even if the resulting reference is not used.
    /// For a safe alternative see [get_ptr](#method.get_ptr).
    ///
    /// # Safety
    ///
    /// The `idx` is not bounds checked and should therefor be used with care.
    pub unsafe fn get_ptr_unchecked(&self, idx: u32) -> *const c_void {
        *self.fn_ptrs.offset(idx as isize)
    }

    /// Returns a function pointer at the given index, or `None` if out of bounds.
    pub fn get_ptr(&self, idx: u32) -> Option<*const c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_ptr_unchecked(idx) })
        } else {
            None
        }
    }

    /// Returns a mutable reference to a function pointer, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution! Calling this method with an
    /// out-of-bounds index is _undefined behavior_ even if the resulting reference is not used.
    /// For a safe alternative see [get_ptr_mut](#method.get_ptr_mut).
    ///
    /// # Safety
    ///
    /// The `idx` is not bounds checked and should therefor be used with care.
    pub unsafe fn get_ptr_unchecked_mut(&mut self, idx: u32) -> &mut *const c_void {
        &mut *self.fn_ptrs.offset(idx as isize)
    }

    /// Returns a mutable reference to a function pointer at the given index, or `None` if out of
    /// bounds.
    pub fn get_ptr_mut(&mut self, idx: u32) -> Option<&mut *const c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_ptr_unchecked_mut(idx) })
        } else {
            None
        }
    }
}

impl AssemblyInfo {
    /// Returns an iterator over the assembly's dependencies.
    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        let dependencies = if self.num_dependencies == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.dependencies, self.num_dependencies as usize) }
        };

        dependencies
            .iter()
            .map(|d| unsafe { str::from_utf8_unchecked(CStr::from_ptr(*d).to_bytes()) })
    }
}

unsafe impl Send for AssemblyInfo {}
unsafe impl Sync for AssemblyInfo {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::os::raw::c_char;
    use std::ptr;

    /// A dummy struct for initializing a struct's `TypeInfo`
    #[allow(dead_code)]
    struct StructTypeInfo {
        type_info: TypeInfo,
        struct_info: StructInfo,
    }

    fn fake_type_info(name: &CStr, group: TypeGroup, size: u32, alignment: u8) -> TypeInfo {
        TypeInfo {
            guid: FAKE_TYPE_GUID,
            name: name.as_ptr(),
            size_in_bits: size,
            alignment,
            group,
        }
    }

    fn fake_struct_type_info(
        name: &CStr,
        struct_info: StructInfo,
        size: u32,
        alignment: u8,
    ) -> StructTypeInfo {
        StructTypeInfo {
            type_info: fake_type_info(name, TypeGroup::StructTypes, size, alignment),
            struct_info,
        }
    }

    const FAKE_TYPE_GUID: Guid = Guid {
        b: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    };

    const FAKE_TYPE_NAME: &str = "type-name";
    const FAKE_FIELD_NAME: &str = "field-name";

    #[test]
    fn test_type_info_name() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        assert_eq!(type_info.name(), FAKE_TYPE_NAME);
    }

    #[test]
    fn test_type_info_size_alignment() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 24, 8);

        assert_eq!(type_info.size_in_bits(), 24);
        assert_eq!(type_info.size_in_bytes(), 3);
        assert_eq!(type_info.alignment(), 8);
    }

    #[test]
    fn test_type_info_group_fundamental() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_group = TypeGroup::FundamentalTypes;
        let type_info = fake_type_info(&type_name, type_group, 1, 1);

        assert_eq!(type_info.group, type_group);
        assert!(type_info.group.is_fundamental());
        assert!(!type_info.group.is_struct());
    }

    #[test]
    fn test_type_info_group_struct() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_group = TypeGroup::StructTypes;
        let type_info = fake_type_info(&type_name, type_group, 1, 1);

        assert_eq!(type_info.group, type_group);
        assert!(type_info.group.is_struct());
        assert!(!type_info.group.is_fundamental());
    }

    #[test]
    fn test_type_info_eq() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        assert_eq!(type_info, type_info);
    }

    fn fake_fn_signature(
        name: &CStr,
        arg_types: &[&TypeInfo],
        return_type: Option<&TypeInfo>,
        privacy: Privacy,
    ) -> FunctionSignature {
        FunctionSignature {
            name: name.as_ptr(),
            arg_types: arg_types.as_ptr().cast::<*const TypeInfo>(),
            return_type: return_type.map_or(ptr::null(), |t| t as *const TypeInfo),
            num_arg_types: arg_types.len() as u16,
            privacy,
        }
    }

    const FAKE_FN_NAME: &str = "fn-name";

    #[test]
    fn test_fn_signature_name() {
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], None, Privacy::Public);

        assert_eq!(fn_signature.name(), FAKE_FN_NAME);
    }

    #[test]
    fn test_fn_signature_privacy() {
        let privacy = Privacy::Private;
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], None, privacy);

        assert_eq!(fn_signature.privacy(), privacy);
    }

    #[test]
    fn test_fn_signature_arg_types_none() {
        let arg_types = &[];
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, arg_types, None, Privacy::Public);

        assert_eq!(fn_signature.arg_types(), arg_types);
    }

    #[test]
    fn test_fn_signature_arg_types_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let arg_types = &[&type_info];
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, arg_types, None, Privacy::Public);

        assert_eq!(fn_signature.arg_types(), arg_types);
    }

    #[test]
    fn test_fn_signature_return_type_none() {
        let return_type = None;
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        assert_eq!(fn_signature.return_type(), return_type);
    }

    #[test]
    fn test_fn_signature_return_type_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        assert_eq!(fn_signature.return_type(), return_type);
    }

    fn fake_struct_info(
        name: &CStr,
        field_names: &[*const c_char],
        field_types: &[&TypeInfo],
        field_offsets: &[u16],
        memory_kind: StructMemoryKind,
    ) -> StructInfo {
        assert!(field_names.len() == field_types.len());
        assert!(field_types.len() == field_offsets.len());

        StructInfo {
            name: name.as_ptr(),
            field_names: field_names.as_ptr(),
            field_types: field_types.as_ptr().cast::<*const TypeInfo>(),
            field_offsets: field_offsets.as_ptr(),
            num_fields: field_names.len() as u16,
            memory_kind,
        }
    }

    const FAKE_STRUCT_NAME: &str = "struct-name";

    #[test]
    fn test_struct_info_name() {
        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name.");
        let struct_info = fake_struct_info(&struct_name, &[], &[], &[], Default::default());

        assert_eq!(struct_info.name(), FAKE_STRUCT_NAME);
    }

    #[test]
    fn test_struct_info_fields_none() {
        let field_names = &[];
        let field_types = &[];
        let field_offsets = &[];
        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name.");
        let struct_info = fake_struct_info(
            &struct_name,
            field_names,
            field_types,
            field_offsets,
            Default::default(),
        );

        assert_eq!(struct_info.field_names().count(), 0);
        assert_eq!(struct_info.field_types(), field_types);
        assert_eq!(struct_info.field_offsets(), field_offsets);
    }

    #[test]
    fn test_struct_info_fields_some() {
        let field_name = CString::new(FAKE_FIELD_NAME).expect("Invalid fake field name.");
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let field_names = &[field_name.as_ptr()];
        let field_types = &[&type_info];
        let field_offsets = &[1];
        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name.");
        let struct_info = fake_struct_info(
            &struct_name,
            field_names,
            field_types,
            field_offsets,
            Default::default(),
        );

        for (lhs, rhs) in struct_info.field_names().zip([FAKE_FIELD_NAME].iter()) {
            assert_eq!(lhs, *rhs)
        }
        assert_eq!(struct_info.field_types(), field_types);
        assert_eq!(struct_info.field_offsets(), field_offsets);
    }

    #[test]
    fn test_struct_info_memory_kind_gc() {
        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name.");
        let struct_memory_kind = StructMemoryKind::GC;
        let struct_info = fake_struct_info(&struct_name, &[], &[], &[], struct_memory_kind.clone());

        assert_eq!(struct_info.memory_kind, struct_memory_kind);
    }

    #[test]
    fn test_struct_info_memory_kind_value() {
        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name.");
        let struct_memory_kind = StructMemoryKind::Value;
        let struct_info = fake_struct_info(&struct_name, &[], &[], &[], struct_memory_kind.clone());

        assert_eq!(struct_info.memory_kind, struct_memory_kind);
    }

    fn fake_module_info(
        path: &CStr,
        functions: &[FunctionInfo],
        types: &[&TypeInfo],
    ) -> ModuleInfo {
        ModuleInfo {
            path: path.as_ptr(),
            functions: functions.as_ptr(),
            num_functions: functions.len() as u32,
            types: types.as_ptr().cast::<*const TypeInfo>(),
            num_types: types.len() as u32,
        }
    }

    const FAKE_MODULE_PATH: &str = "path::to::module";

    #[test]
    fn test_module_info_path() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[], &[]);

        assert_eq!(module.path(), FAKE_MODULE_PATH);
    }

    #[test]
    fn test_module_info_types_none() {
        let functions = &[];
        let types = &[];
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, functions, types);

        assert_eq!(module.functions().len(), functions.len());
        assert_eq!(module.types().len(), types.len());
    }

    #[test]
    fn test_module_info_types_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let fn_info = FunctionInfo {
            signature: fn_signature,
            fn_ptr: ptr::null(),
        };
        let functions = &[fn_info];

        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name");
        let struct_info = fake_struct_info(&struct_name, &[], &[], &[], Default::default());
        let struct_type_info = fake_struct_type_info(&struct_name, struct_info, 1, 1);
        let types = &[unsafe { mem::transmute(&struct_type_info) }];

        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, functions, types);

        let result_functions = module.functions();
        assert_eq!(result_functions.len(), functions.len());
        for (lhs, rhs) in result_functions.iter().zip(functions.iter()) {
            assert_eq!(lhs.fn_ptr, rhs.fn_ptr);
            assert_eq!(lhs.signature.name(), rhs.signature.name());
            assert_eq!(lhs.signature.arg_types(), rhs.signature.arg_types());
            assert_eq!(lhs.signature.return_type(), rhs.signature.return_type());
            assert_eq!(lhs.signature.privacy(), rhs.signature.privacy());
        }

        let result_types: &[&TypeInfo] = module.types();
        assert_eq!(result_types.len(), types.len());
        for (lhs, rhs) in result_types.iter().zip(types.iter()) {
            assert_eq!(lhs, rhs);
            assert_eq!(lhs.name(), rhs.name());
            assert_eq!(lhs.group, rhs.group);
            if lhs.group == TypeGroup::StructTypes {
                let lhs_struct = lhs.as_struct().unwrap();
                let rhs_struct = rhs.as_struct().unwrap();
                assert_eq!(lhs_struct.name(), rhs_struct.name());
                assert_eq!(lhs_struct.field_types(), rhs_struct.field_types());
            }
        }
    }

    fn fake_dispatch_table(
        fn_signatures: &[FunctionSignature],
        fn_ptrs: &mut [*const c_void],
    ) -> DispatchTable {
        assert!(fn_signatures.len() == fn_ptrs.len());

        DispatchTable {
            signatures: fn_signatures.as_ptr(),
            fn_ptrs: fn_ptrs.as_mut_ptr(),
            num_entries: fn_signatures.len() as u32,
        }
    }

    #[test]
    fn test_dispatch_table_iter_mut_none() {
        let signatures = &[];
        let fn_ptrs = &mut [];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let iter = fn_ptrs.iter_mut().zip(signatures.iter());
        assert_eq!(dispatch_table.iter_mut().count(), iter.count());
    }

    #[test]
    fn test_dispatch_table_iter_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let iter = fn_ptrs.iter_mut().zip(signatures.iter());
        assert_eq!(dispatch_table.iter_mut().count(), iter.len());

        for (lhs, rhs) in dispatch_table.iter_mut().zip(iter) {
            assert_eq!(lhs.0, rhs.0);
            assert_eq!(lhs.1.name(), rhs.1.name());
            assert_eq!(lhs.1.arg_types(), rhs.1.arg_types());
            assert_eq!(lhs.1.return_type(), rhs.1.return_type());
            assert_eq!(lhs.1.privacy(), rhs.1.privacy());
        }
    }

    #[test]
    fn test_dispatch_table_ptrs_mut_none() {
        let signatures = &[];
        let fn_ptrs = &mut [];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        assert_eq!(dispatch_table.ptrs_mut().len(), 0);
    }

    #[test]
    fn test_dispatch_table_ptrs_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let result = dispatch_table.ptrs_mut();
        assert_eq!(result.len(), fn_ptrs.len());
        for (lhs, rhs) in result.iter().zip(fn_ptrs.iter()) {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn test_dispatch_table_signatures_none() {
        let signatures = &[];
        let fn_ptrs = &mut [];
        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        assert_eq!(dispatch_table.signatures().len(), 0);
    }

    #[test]
    fn test_dispatch_table_signatures_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];
        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let result = dispatch_table.signatures();
        assert_eq!(result.len(), signatures.len());
        for (lhs, rhs) in result.iter().zip(signatures.iter()) {
            assert_eq!(lhs.name(), rhs.name());
            assert_eq!(lhs.arg_types(), rhs.arg_types());
            assert_eq!(lhs.return_type(), rhs.return_type());
            assert_eq!(lhs.privacy(), rhs.privacy());
        }
    }

    #[test]
    fn test_dispatch_table_get_ptr_unchecked() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(unsafe { dispatch_table.get_ptr_unchecked(0) }, fn_ptrs[0]);
    }

    #[test]
    fn test_dispatch_table_get_ptr_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr(1), None);
    }

    #[test]
    fn test_dispatch_table_get_ptr_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr(0), Some(fn_ptrs[0]));
    }

    #[test]
    fn test_dispatch_table_get_ptr_unchecked_mut() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(
            unsafe { dispatch_table.get_ptr_unchecked_mut(0) },
            &mut fn_ptrs[0]
        );
    }

    #[test]
    fn test_dispatch_table_get_ptr_mut_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr_mut(1), None);
    }

    #[test]
    fn test_dispatch_table_get_ptr_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr_mut(0), Some(&mut fn_ptrs[0]));
    }

    fn fake_assembly_info(
        symbols: ModuleInfo,
        dispatch_table: DispatchTable,
        dependencies: &[*const c_char],
    ) -> AssemblyInfo {
        AssemblyInfo {
            symbols,
            dispatch_table,
            dependencies: dependencies.as_ptr(),
            num_dependencies: dependencies.len() as u32,
        }
    }

    const FAKE_DEPENDENCY: &str = "path/to/dependency.dylib";

    #[test]
    fn test_assembly_info_dependencies() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[], &[]);

        let dispatch_table = fake_dispatch_table(&[], &mut []);

        let dependency = CString::new(FAKE_DEPENDENCY).expect("Invalid fake dependency.");
        let dependencies = &[dependency.as_ptr()];
        let assembly = fake_assembly_info(module, dispatch_table, dependencies);

        assert_eq!(assembly.dependencies().count(), dependencies.len());
        for (lhs, rhs) in assembly.dependencies().zip([FAKE_DEPENDENCY].iter()) {
            assert_eq!(lhs, *rhs)
        }
    }
}
