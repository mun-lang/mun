#ifndef MUN_RUNTIME_BINDINGS_H_
#define MUN_RUNTIME_BINDINGS_H_

#include <stdbool.h>
#include <stdint.h>

/**
 * Types of primitives supported by Mun.
 */
enum MunPrimitiveType
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
    MUN_PRIMITIVE_TYPE_BOOL,
    MUN_PRIMITIVE_TYPE_U8,
    MUN_PRIMITIVE_TYPE_U16,
    MUN_PRIMITIVE_TYPE_U32,
    MUN_PRIMITIVE_TYPE_U64,
    MUN_PRIMITIVE_TYPE_U128,
    MUN_PRIMITIVE_TYPE_I8,
    MUN_PRIMITIVE_TYPE_I16,
    MUN_PRIMITIVE_TYPE_I32,
    MUN_PRIMITIVE_TYPE_I64,
    MUN_PRIMITIVE_TYPE_I128,
    MUN_PRIMITIVE_TYPE_F32,
    MUN_PRIMITIVE_TYPE_F64,
    MUN_PRIMITIVE_TYPE_EMPTY,
    MUN_PRIMITIVE_TYPE_VOID,
};
#ifndef __cplusplus
typedef uint8_t MunPrimitiveType;
#endif // __cplusplus

/**
 * Represents the kind of memory management a struct uses.
 */
enum MunStructMemoryKind
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
    /**
     * A garbage collected struct is allocated on the heap and uses reference semantics when passed
     * around.
     */
    MUN_STRUCT_MEMORY_KIND_GC,
    /**
     * A value struct is allocated on the stack and uses value semantics when passed around.
     *
     * NOTE: When a value struct is used in an external API, a wrapper is created that _pins_ the
     * value on the heap. The heap-allocated value needs to be *manually deallocated*!
     */
    MUN_STRUCT_MEMORY_KIND_VALUE,
};
#ifndef __cplusplus
typedef uint8_t MunStructMemoryKind;
#endif // __cplusplus

/**
 * A C-style handle to an error message.
 *
 * If the handle contains a non-null pointer, an error occurred.
 */
typedef struct MunErrorHandle {
    const char *error_string;
} MunErrorHandle;

/**
 * A C-style handle to a runtime.
 */
typedef struct MunRuntime {
    void *_0;
} MunRuntime;

/**
 * A [`Type`] holds information about a mun type.
 */
typedef struct MunType {
    const void *_0;
    const void *_1;
} MunType;

/**
 * A `RawGcPtr` is an unsafe version of a `GcPtr`. It represents the raw internal pointer
 * semantics used by the runtime.
 */
typedef void *const *MunRawGcPtr;

/**
 * A `GcPtr` is what you interact with outside of the allocator. It is a pointer to a piece of
 * memory that points to the actual data stored in memory.
 *
 * This creates an indirection that must be followed to get to the actual data of the object. Note
 * that the `GcPtr` must therefore be pinned in memory whereas the contained memory pointer may
 * change.
 */
typedef MunRawGcPtr MunGcPtr;

/**
 * Definition of an external function that is callable from Mun.
 *
 * The ownership of the contained `TypeInfoHandles` is considered to lie with this struct.
 */
typedef struct MunExternalFunctionDefinition {
    /**
     * The name of the function
     */
    const char *name;
    /**
     * The number of arguments of the function
     */
    uint32_t num_args;
    /**
     * The types of the arguments
     */
    const struct MunType *arg_types;
    /**
     * The type of the return type
     */
    struct MunType return_type;
    /**
     * Pointer to the function
     */
    const void *fn_ptr;
} MunExternalFunctionDefinition;

/**
 * Options required to construct a [`RuntimeHandle`] through [`mun_runtime_create`]
 *
 * # Safety
 *
 * This struct contains raw pointers as parameters. Passing pointers to invalid data, will lead to
 * undefined behavior.
 */
typedef struct MunRuntimeOptions {
    /**
     * Function definitions that should be inserted in the runtime before a mun library is loaded.
     * This is useful to initialize `extern` functions used in a mun library.
     *
     * If the [`num_functions`] fields is non-zero this field must contain a pointer to an array
     * of [`abi::FunctionDefinition`]s.
     */
    const struct MunExternalFunctionDefinition *functions;
    /**
     * The number of functions in the [`functions`] array.
     */
    uint32_t num_functions;
} MunRuntimeOptions;

/**
 * Describes a `Function` accessible from a Mun [`super::runtime::Runtime`].
 *
 * An instance of `Function` shares ownership of the underlying data. To create a copy of the
 * `Function` object call [`mun_function_add_reference`] to make sure the number of references to
 * the data is properly tracked. Calling [`mun_function_release`] signals the runtime that the data
 * is no longer referenced through the specified object. When all references are released the
 * underlying data is deallocated.
 */
typedef struct MunFunction {
    const void *_0;
} MunFunction;

/**
 * Represents a globally unique identifier (GUID).
 */
typedef struct MunGuid {
    uint8_t _0[16];
} MunGuid;

/**
 * Represents a pointer to another type.
 */
typedef struct MunPointerTypeId {
    /**
     * The type to which this pointer points
     */
    const union MunTypeId *pointee;
    /**
     * Whether or not this pointer is mutable or not
     */
    bool mutable_;
} MunPointerTypeId;

/**
 * Represents an array of a specific type.
 */
typedef struct MunArrayTypeId {
    /**
     * The element type of the array
     */
    const union MunTypeId *element;
} MunArrayTypeId;

/**
 * Represents a unique identifier for types. The runtime can use this to lookup the corresponding
 * [`TypeInfo`]. A [`TypeId`] is a key for a [`TypeInfo`].
 *
 * A [`TypeId`] only contains enough information to query the runtime for a [`TypeInfo`].
 */
enum MunTypeId_Tag
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
    /**
     * Represents a concrete type with a specific Guid
     */
    MUN_TYPE_ID_CONCRETE,
    /**
     * Represents a pointer to a type
     */
    MUN_TYPE_ID_POINTER,
    /**
     * Represents an array of a specific type
     */
    MUN_TYPE_ID_ARRAY,
};
#ifndef __cplusplus
typedef uint8_t MunTypeId_Tag;
#endif // __cplusplus

typedef union MunTypeId {
    MunTypeId_Tag tag;
    struct {
        MunTypeId_Tag concrete_tag;
        struct MunGuid concrete;
    };
    struct {
        MunTypeId_Tag pointer_tag;
        struct MunPointerTypeId pointer;
    };
    struct {
        MunTypeId_Tag array_tag;
        struct MunArrayTypeId array;
    };
} MunTypeId;

/**
 * An array of [`Type`]s.
 *
 * The `Types` struct owns the `Type`s it references. Ownership of the `Type` can be shared by
 * calling [`mun_type_add_reference`].
 *
 * This is backed by a dynamically allocated array. Ownership is transferred via this struct
 * and its contents must be destroyed with [`mun_types_destroy`].
 */
typedef struct MunTypes {
    const struct MunType *types;
    uintptr_t count;
} MunTypes;

/**
 * Additional information of a pointer [`Type`].
 *
 * Ownership of this type lies with the [`Type`] that created this instance. As long as the
 * original type is not released through [`mun_type_release`] this type stays alive.
 */
typedef struct MunPointerInfo {
    const void *_0;
    const void *_1;
} MunPointerInfo;

/**
 * Additional information of a struct [`Type`].
 *
 * Ownership of this type lies with the [`Type`] that created this instance. As long as the
 * original type is not released through [`mun_type_release`] this type stays alive.
 */
typedef struct MunStructInfo {
    const void *_0;
    const void *_1;
} MunStructInfo;

/**
 * Additional information of an array [`Type`].
 *
 * Ownership of this type lies with the [`Type`] that created this instance. As long as the
 * original type is not released through [`mun_type_release`] this type stays alive.
 */
typedef struct MunArrayInfo {
    const void *_0;
    const void *_1;
} MunArrayInfo;

/**
 * An enum that defines the kind of type.
 */
enum MunTypeKind_Tag
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
    MUN_TYPE_KIND_PRIMITIVE,
    MUN_TYPE_KIND_POINTER,
    MUN_TYPE_KIND_STRUCT,
    MUN_TYPE_KIND_ARRAY,
};
#ifndef __cplusplus
typedef uint8_t MunTypeKind_Tag;
#endif // __cplusplus

typedef union MunTypeKind {
    MunTypeKind_Tag tag;
    struct {
        MunTypeKind_Tag primitive_tag;
        struct MunGuid primitive;
    };
    struct {
        MunTypeKind_Tag pointer_tag;
        struct MunPointerInfo pointer;
    };
    struct {
        MunTypeKind_Tag struct_tag;
        struct MunStructInfo struct_;
    };
    struct {
        MunTypeKind_Tag array_tag;
        struct MunArrayInfo array;
    };
} MunTypeKind;

/**
 * Information of a field of a struct [`Type`].
 *
 * Ownership of this type lies with the [`Type`] that created this instance. As long as the
 * original type is not released through [`mun_type_release`] this type stays alive.
 */
typedef struct MunField {
    const void *_0;
    const void *_1;
} MunField;

/**
 * An array of [`Field`]s.
 *
 * This is backed by a dynamically allocated array. Ownership is transferred via this struct
 * and its contents must be destroyed with [`mun_fields_destroy`].
 */
typedef struct MunFields {
    const struct MunField *fields;
    uintptr_t count;
} MunFields;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Allocates an object in the runtime of the given `ty`. If successful, `obj` is set,
 * otherwise a non-zero error handle is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_gc_alloc(struct MunRuntime runtime, struct MunType ty, MunGcPtr *obj);

/**
 * Retrieves the `ty` for the specified `obj` from the runtime. If successful, `ty` is set,
 * otherwise a non-zero error handle is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_gc_ptr_type(struct MunRuntime runtime, MunGcPtr obj, struct MunType *ty);

/**
 * Roots the specified `obj`, which keeps it and objects it references alive. Objects marked as
 * root, must call `mun_gc_unroot` before they can be collected. An object can be rooted multiple
 * times, but you must make sure to call `mun_gc_unroot` an equal number of times before the
 * object can be collected. If successful, `obj` has been rooted, otherwise a non-zero error handle
 * is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_gc_root(struct MunRuntime runtime, MunGcPtr obj);

/**
 * Unroots the specified `obj`, potentially allowing it and objects it references to be
 * collected. An object can be rooted multiple times, so you must make sure to call `mun_gc_unroot`
 * the same number of times as `mun_gc_root` was called before the object can be collected. If
 * successful, `obj` has been unrooted, otherwise a non-zero error handle is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_gc_unroot(struct MunRuntime runtime, MunGcPtr obj);

/**
 * Collects all memory that is no longer referenced by rooted objects. If successful, `reclaimed`
 * is set, otherwise a non-zero error handle is returned. If `reclaimed` is `true`, memory was
 * reclaimed, otherwise nothing happend. This behavior will likely change in the future.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_gc_collect(struct MunRuntime runtime, bool *reclaimed);

/**
 * Constructs a new runtime that loads the library at `library_path` and its dependencies. If
 * successful, the runtime `handle` is set, otherwise a non-zero error handle is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * The runtime must be manually destructed using [`mun_runtime_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_runtime_create(const char *library_path,
                                         struct MunRuntimeOptions options,
                                         struct MunRuntime *handle);

/**
 * Destructs the runtime corresponding to `handle`.
 */
struct MunErrorHandle mun_runtime_destroy(struct MunRuntime runtime);

/**
 * Retrieves the [`FunctionDefinition`] for `fn_name` from the `runtime`. If successful,
 * `has_fn_info` and `fn_info` are set, otherwise a non-zero error handle is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_runtime_find_function_definition(struct MunRuntime runtime,
                                                           const char *fn_name,
                                                           uintptr_t fn_name_len,
                                                           bool *has_fn_info,
                                                           struct MunFunction *fn_info);

/**
 * Retrieves the type information corresponding to the specified `type_name` from the runtime.
 * If successful, `has_type_info` and `type_info` are set, otherwise a non-zero error handle is
 * returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_runtime_get_type_info_by_name(struct MunRuntime runtime,
                                                        const char *type_name,
                                                        bool *has_type_info,
                                                        struct MunType *type_info);

/**
 * Retrieves the type information corresponding to the specified `type_id` from the runtime. If
 * successful, `has_type_info` and `type_info` are set, otherwise a non-zero error handle is
 * returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_runtime_get_type_info_by_id(struct MunRuntime runtime,
                                                      const union MunTypeId *type_id,
                                                      bool *has_type_info,
                                                      struct MunType *type_info);

/**
 * Updates the runtime corresponding to `handle`. If successful, `updated` is set, otherwise a
 * non-zero error handle is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_runtime_update(struct MunRuntime runtime, bool *updated);

/**
 * Notifies the runtime an additional references exists to the function. This ensures that the data
 * is kept alive even if [`mun_function_release`] is called for the existing references. Only
 * after all references have been released can the underlying data be deallocated.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has already been deallocated by a previous
 * call to [`mun_function_release`].
 */
struct MunErrorHandle mun_function_add_reference(struct MunFunction function);

/**
 * Notifies the runtime that one of the references to the function is no longer in use. The data
 * may not immediately be destroyed. Only after all references have been released can the
 * underlying data be deallocated.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_release`].
 */
struct MunErrorHandle mun_function_release(struct MunFunction function);

/**
 * Retrieves the function's function pointer.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_release`].
 */
struct MunErrorHandle mun_function_fn_ptr(struct MunFunction function, const void **ptr);

/**
 * Retrieves the function's name.
 *
 * If the function is successful, the caller is responsible for calling [`mun_string_destroy`] on
 * the return pointer.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_release`].
 */
struct MunErrorHandle mun_function_name(struct MunFunction function, const char **name);

/**
 * Retrieves the function's argument types.
 *
 * If successful, ownership of the [`Types`] is transferred to the caller. It must be deallocated
 * with a call to [`mun_types_destroy`].
 *
 * # Safety
 *
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_release`].
 */
struct MunErrorHandle mun_function_argument_types(struct MunFunction function,
                                                  struct MunTypes *arg_types);

/**
 * Retrieves the function's return type.
 *
 * Ownership of the [`Type`] is transferred to the called. It must be released with a call to
 * [`mun_type_release`].
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_release`].
 */
struct MunErrorHandle mun_function_return_type(struct MunFunction function, struct MunType *ty);

/**
 * Deallocates a string that was allocated by the runtime.
 *
 * # Safety
 *
 * This function receives a raw pointer as parameter. Only when the argument is not a null pointer,
 * its content will be deallocated. Passing pointers to invalid data or memory allocated by other
 * processes, will lead to undefined behavior.
 */
void mun_string_destroy(const char *string);

/**
 * Destructs the error message corresponding to the specified handle.
 *
 * # Safety
 *
 * Only call this function on an [`ErrorHandle`] once.
 */
void mun_error_destroy(struct MunErrorHandle error);

/**
 * Notifies the runtime that the specified type is no longer used. Any use of the type after
 * calling this function results in undefined behavior.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type` has been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_release(struct MunType ty);

/**
 * Increments the usage count of the specified type.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type` has been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_add_reference(struct MunType ty);

/**
 * Retrieves the type's name.
 *
 * # Safety
 *
 * The caller is responsible for calling `mun_string_destroy` on the return pointer - if it is not
 * null.
 *
 * This function results in undefined behavior if the passed in `Type` has been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_name(struct MunType ty, const char **name);

/**
 * Compares two different Types. Returns `true` if the two types are equal. If either of the two
 * types is invalid because for instance it contains null pointers this function returns `false`.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type`s have been deallocated in a
 * previous call to [`mun_type_release`].
 */
bool mun_type_equal(struct MunType a, struct MunType b);

/**
 * Returns the storage size required for a type. The storage size does not include any padding to
 * align the size. Call [`mun_type_alignment`] to request the alignment of the type.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type`s have been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_size(struct MunType ty, uintptr_t *size);

/**
 * Returns the alignment requirements of the type.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type`s have been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_alignment(struct MunType ty, uintptr_t *align);

/**
 * Returns a new [`Type`] that is a pointer to the specified type.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type`s have been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_pointer_type(struct MunType ty,
                                            bool mutable_,
                                            struct MunType *pointer_ty);

/**
 * Returns a new [`Type`] that is an array of the specified type.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type`s have been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_array_type(struct MunType ty, struct MunType *array_ty);

/**
 * Returns information about what kind of type this is.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Type`s have been deallocated in a
 * previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_type_kind(struct MunType ty, union MunTypeKind *kind);

/**
 * Destroys the contents of a [`Types`] struct.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Types` has been deallocated
 * by a previous call to [`mun_types_destroy`].
 */
struct MunErrorHandle mun_types_destroy(struct MunTypes types);

/**
 * Returns the type of the elements stored in this type. Ownership is transferred if this function
 * returns successfully.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `ArrayInfo` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_array_type_element_type(struct MunArrayInfo ty,
                                                  struct MunType *element_ty);

/**
 * Returns the type that this instance points to. Ownership is transferred if this function returns
 * successfully.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `PointerType` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_pointer_type_pointee(struct MunPointerInfo ty, struct MunType *pointee);

/**
 * Returns true if this is a mutable pointer.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `PointerType` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_pointer_is_mutable(struct MunPointerInfo ty, bool *mutable_);

/**
 * Returns a [`Type`] that represents the specified primitive type.
 */
struct MunType mun_type_primitive(MunPrimitiveType primitive_type);

/**
 * Returns the globally unique identifier (GUID) of the struct.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `StructType` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_struct_type_guid(struct MunStructInfo ty, struct MunGuid *guid);

/**
 * Returns the type of memory management to apply for the struct.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `StructType` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_struct_type_memory_kind(struct MunStructInfo ty,
                                                  MunStructMemoryKind *memory_kind);

/**
 * Retrieves the field with the given name.
 *
 * The name can be passed as a non nul-terminated string it must be UTF-8 encoded.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Fields` has been deallocated
 * by a previous call to [`mun_fields_destroy`].
 */
struct MunErrorHandle mun_fields_find_by_name(struct MunFields fields,
                                              const char *name,
                                              uintptr_t len,
                                              bool *has_field,
                                              struct MunField *field);

/**
 * Destroys the contents of a [`Fields`] struct.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Fields` has been deallocated
 * by a previous call to [`mun_fields_destroy`].
 */
struct MunErrorHandle mun_fields_destroy(struct MunFields fields);

/**
 * Retrieves all the fields of the specified struct type.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `StructType` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_struct_type_fields(struct MunStructInfo ty, struct MunFields *fields);

/**
 * Returns the name of the field in the parent struct. Ownership of the name is transferred and
 * must be destroyed with [`mun_string_destroy`]. If this function fails a nullptr is returned.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Field` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_field_name(struct MunField field, const char **name);

/**
 * Returns the type of the field. Ownership of the returned [`Type`] is transferred and must be
 * released with a call to [`mun_type_release`].
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Field` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_field_type(struct MunField field, struct MunType *ty);

/**
 * Returns the offset of the field in bytes from the start of the parent struct.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `Field` has been deallocated
 * by a previous call to [`mun_type_release`].
 */
struct MunErrorHandle mun_field_offset(struct MunField field, uintptr_t *offset);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* MUN_RUNTIME_BINDINGS_H_ */
