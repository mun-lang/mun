#ifndef MUN_RUNTIME_BINDINGS_H_
#define MUN_RUNTIME_BINDINGS_H_

/* Generated with cbindgen:0.23.0 */

#include <stdbool.h>
#include <stdint.h>

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
    Gc,
    /**
     * A value struct is allocated on the stack and uses value semantics when passed around.
     *
     * NOTE: When a value struct is used in an external API, a wrapper is created that _pins_ the
     * value on the heap. The heap-allocated value needs to be *manually deallocated*!
     */
    Value,
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
    const char *_0;
} MunErrorHandle;

/**
 * A C-style handle to a runtime.
 */
typedef struct MunRuntimeHandle {
    void *_0;
} MunRuntimeHandle;

/**
 * A C-style handle to a `TypeInfo`.
 */
typedef struct MunTypeInfoHandle {
    const void *_0;
} MunTypeInfoHandle;

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
 * Represents a globally unique identifier (GUID).
 */
typedef struct MunGuid {
    uint8_t _0[16];
} MunGuid;

/**
 * Represents a unique identifier for types. The runtime can use this to lookup the corresponding [`TypeInfo`].
 */
typedef struct MunTypeId {
    /**
     * The GUID of the type
     */
    struct MunGuid guid;
} MunTypeId;

/**
 * Represents a function signature.
 */
typedef struct MunFunctionSignature {
    /**
     * Argument types
     */
    const struct MunTypeId *arg_types;
    /**
     * Optional return type
     */
    struct MunTypeId return_type;
    /**
     * Number of argument types
     */
    uint16_t num_arg_types;
} MunFunctionSignature;

/**
 * Represents a function prototype. A function prototype contains the name, type signature, but
 * not an implementation.
 */
typedef struct MunFunctionPrototype {
    /**
     * Function name
     */
    const char *name;
    /**
     * The type signature of the function
     */
    struct MunFunctionSignature signature;
} MunFunctionPrototype;

/**
 * Represents a function definition. A function definition contains the name, type signature, and
 * a pointer to the implementation.
 *
 * `fn_ptr` can be used to call the declared function.
 */
typedef struct MunFunctionDefinition {
    /**
     * Function prototype
     */
    struct MunFunctionPrototype prototype;
    /**
     * Function pointer
     */
    const void *fn_ptr;
} MunFunctionDefinition;

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
    const struct MunFunctionDefinition *functions;
    /**
     * The number of functions in the [`functions`] array.
     */
    uint32_t num_functions;
} MunRuntimeOptions;

/**
 * A C-style handle to a `FunctionInfo`.
 */
typedef struct MunFunctionInfoHandle {
    const void *_0;
} MunFunctionInfoHandle;

/**
 * A C-style handle to a `FieldInfo`.
 */
typedef struct MunFieldInfoHandle {
    const void *_0;
} MunFieldInfoHandle;

/**
 * A C-style handle to an array of `FieldInfoHandle`s.
 */
typedef struct MunFieldInfoSpan {
    /**
     * Pointer to the start of the array buffer
     */
    const struct MunFieldInfoHandle *data;
    /**
     * Length of the array (and capacity)
     */
    uintptr_t len;
} MunFieldInfoSpan;

/**
 * A C-style handle to an array of `TypeInfoHandle`s.
 */
typedef struct MunTypeInfoSpan {
    /**
     * Pointer to the start of the array buffer
     */
    const struct MunTypeInfoHandle *data;
    /**
     * Length of the array (and capacity)
     */
    uintptr_t len;
} MunTypeInfoSpan;

/**
 * A C-style handle to a `StructInfo`.
 */
typedef struct MunStructInfoHandle {
    const void *_0;
} MunStructInfoHandle;

/**
 * An enum containing C-style handles a `TypeInfo`'s data.
 */
enum MunTypeInfoData_Tag
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
    /**
     * Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
     */
    Primitive,
    /**
     * Struct types (i.e. record, tuple, or unit structs)
     */
    Struct,
};
#ifndef __cplusplus
typedef uint8_t MunTypeInfoData_Tag;
#endif // __cplusplus

typedef union MunTypeInfoData {
    MunTypeInfoData_Tag tag;
    struct {
        MunTypeInfoData_Tag struct_tag;
        struct MunStructInfoHandle struct_;
    };
} MunTypeInfoData;

/**
 * Represents a struct declaration.
 */
typedef struct MunStructInfo {
    /**
     * Struct fields' names
     */
    const char *const *field_names;
    /**
     * Struct fields' information
     */
    const struct MunTypeId *field_types;
    /**
     * Struct fields' offsets
     */
    const uint16_t *field_offsets;
    /**
     * Number of fields
     */
    uint16_t num_fields;
    /**
     * Struct memory kind
     */
    MunStructMemoryKind memory_kind;
} MunStructInfo;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

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
 * Only call this function on an ErrorHandle once.
 */
void mun_error_destroy(struct MunErrorHandle error);

/**
 * Allocates an object in the runtime of the given `type_info`. If successful, `obj` is set,
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
struct MunErrorHandle mun_gc_alloc(struct MunRuntimeHandle runtime,
                                   struct MunTypeInfoHandle type_info,
                                   MunGcPtr *obj);

/**
 * Retrieves the `type_info` for the specified `obj` from the runtime. If successful, `type_info`
 * is set, otherwise a non-zero error handle is returned.
 *
 * If a non-zero error handle is returned, it must be manually destructed using
 * [`mun_error_destroy`].
 *
 * # Safety
 *
 * This function receives raw pointers as parameters. If any of the arguments is a null pointer,
 * an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
 */
struct MunErrorHandle mun_gc_ptr_type(struct MunRuntimeHandle handle,
                                      MunGcPtr obj,
                                      struct MunTypeInfoHandle *type_info);

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
struct MunErrorHandle mun_gc_root(struct MunRuntimeHandle handle, MunGcPtr obj);

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
struct MunErrorHandle mun_gc_unroot(struct MunRuntimeHandle handle, MunGcPtr obj);

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
struct MunErrorHandle mun_gc_collect(struct MunRuntimeHandle handle, bool *reclaimed);

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
                                         struct MunRuntimeHandle *handle);

/**
 * Destructs the runtime corresponding to `handle`.
 */
void mun_runtime_destroy(struct MunRuntimeHandle handle);

/**
 * Retrieves the [`FunctionDefinition`] for `fn_name` from the runtime corresponding to `handle`.
 * If successful, `has_fn_info` and `fn_info` are set, otherwise a non-zero error handle is
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
struct MunErrorHandle mun_runtime_get_function_info(struct MunRuntimeHandle handle,
                                                    const char *fn_name,
                                                    bool *has_fn_info,
                                                    struct MunFunctionInfoHandle *fn_info);

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
struct MunErrorHandle mun_runtime_get_type_info_by_name(struct MunRuntimeHandle runtime,
                                                        const char *type_name,
                                                        bool *has_type_info,
                                                        struct MunTypeInfoHandle *type_info);

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
struct MunErrorHandle mun_runtime_get_type_info_by_id(struct MunRuntimeHandle runtime,
                                                      const struct MunTypeId *type_id,
                                                      bool *has_type_info,
                                                      struct MunTypeInfoHandle *type_info);

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
struct MunErrorHandle mun_runtime_update(struct MunRuntimeHandle handle, bool *updated);

/**
 * Retrieves the field's name.
 *
 * # Safety
 *
 * The caller is responsible for calling `mun_string_destroy` on the return pointer - if it is not null.
 *
 * This function might result in undefined behavior if the [`crate::TypeInfoHandle`] associated
 * with this `FieldInfoHandle` has been deallocated.
 */
const char *mun_field_info_name(struct MunFieldInfoHandle field_info);

/**
 * Retrieves the field's type.
 *
 * # Safety
 *
 * This method is considered unsafe because the passed `field_info` might have been deallocated by
 * a call to [`mun_type_info_decrement_strong_count`] of the type that contains this field.
 */
struct MunTypeInfoHandle mun_field_info_type(struct MunFieldInfoHandle field_info);

/**
 * Retrieves the field's offset.
 *
 * # Safety
 *
 * This method is considered unsafe because the passed `field_info` might have been deallocated by
 * a call to [`mun_type_info_decrement_strong_count`] of the type that contains this field.
 */
struct MunErrorHandle mun_field_info_offset(struct MunFieldInfoHandle field_info,
                                            uint16_t *field_offset);

/**
 * Deallocates a span of `FieldInfo`s that was allocated by the runtime.
 *
 * Deallocating span only deallocates the data allocated for the span. Deallocating a span will not
 * deallocate the FieldInfo's it references. `FieldInfo`s are destroyed when the top-level
 * `TypeInfo` is destroyed.
 *
 * # Safety
 *
 * This function receives a span as parameter. Only when the spans data pointer is not null, is the
 * content deallocated. Passing pointers to invalid data of memory allocated by other processes,
 * will lead to undefined behavior.
 */
bool mun_field_info_span_destroy(struct MunFieldInfoSpan span);

/**
 * Decrements the strong count of the `Arc<FunctionDefinition>` associated with `handle`.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has already been deallocated by a previous
 * call to [`mun_function_info_decrement_strong_count`].
 */
bool mun_function_info_decrement_strong_count(struct MunFunctionInfoHandle fn_info);

/**
 * Increments the strong count of the `Arc<FunctionDefinition>` associated with `handle`.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_info_decrement_strong_count`].
 */
bool mun_function_info_increment_strong_count(struct MunFunctionInfoHandle fn_info);

/**
 * Retrieves the function's function pointer.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_info_decrement_strong_count`].
 */
const void *mun_function_info_fn_ptr(struct MunFunctionInfoHandle fn_info);

/**
 * Retrieves the function's name.
 *
 * # Safety
 *
 * The caller is responsible for calling `mun_string_destroy` on the return pointer - if it is not null.
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_info_decrement_strong_count`].
 */
const char *mun_function_info_name(struct MunFunctionInfoHandle fn_info);

/**
 * Retrieves the function's argument types.
 *
 * # Safety
 *
 * If a non-null handle is returned, the caller is responsible for calling
 * `mun_type_info_span_destroy` on the returned handle.
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_info_decrement_strong_count`].
 */
struct MunErrorHandle mun_function_info_argument_types(struct MunFunctionInfoHandle fn_info,
                                                       struct MunTypeInfoSpan *arg_types);

/**
 * Retrieves the function's return type.
 *
 * # Safety
 *
 * This function might be unsafe if the underlying data has been deallocated by a previous call
 * to [`mun_function_info_decrement_strong_count`].
 */
struct MunTypeInfoHandle mun_function_info_return_type(struct MunFunctionInfoHandle fn_info);

/**
 * Retrieves information about the struct's fields.
 *
 * # Safety
 *
 * The caller is responsible for calling `mun_field_info_span_destroy` on the returned span.
 *
 * This function might result in undefined behavior if the [`crate::TypeInfoHandle`] associated
 * with this `StructInfoHandle` has been deallocated.
 */
struct MunErrorHandle mun_struct_info_fields(struct MunStructInfoHandle struct_info,
                                             struct MunFieldInfoSpan *field_info_span);

/**
 * Retrieves the struct's memory kind.
 *
 * # Safety
 *
 * This function might result in undefined behavior if the [`crate::TypeInfoHandle`] associated
 * with this `StructInfoHandle` has been deallocated.
 */
struct MunErrorHandle mun_struct_info_memory_kind(struct MunStructInfoHandle struct_info,
                                                  MunStructMemoryKind *memory_kind);

/**
 * Decrements the strong count of the `Arc<TypeInfo>` associated with `handle`.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `TypeInfoHandle` has already been
 * deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
 */
bool mun_type_info_decrement_strong_count(struct MunTypeInfoHandle handle);

/**
 * Increments the strong count of the `Arc<TypeInfo>` associated with `handle`.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `TypeInfoHandle` has already been
 * deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
 */
bool mun_type_info_increment_strong_count(struct MunTypeInfoHandle handle);

/**
 * Retrieves the type's ID.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `TypeInfoHandle` has been
 * deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
 */
struct MunErrorHandle mun_type_info_id(struct MunTypeInfoHandle type_info,
                                       struct MunTypeId *type_id);

/**
 * Retrieves the type's name.
 *
 * # Safety
 *
 * The caller is responsible for calling `mun_string_destroy` on the return pointer - if it is not null.
 *
 * This function results in undefined behavior if the passed in `TypeInfoHandle` has been
 * deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
 */
const char *mun_type_info_name(struct MunTypeInfoHandle type_info);

/**
 * Retrieves the type's size.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `TypeInfoHandle` has been
 * deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
 */
struct MunErrorHandle mun_type_info_size(struct MunTypeInfoHandle type_info, uintptr_t *size);

/**
 * Retrieves the type's alignment.
 *
 * # Safety
 *
 * This function results in undefined behavior if the passed in `TypeInfoHandle` has been
 * deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
 */
struct MunErrorHandle mun_type_info_align(struct MunTypeInfoHandle type_info, uintptr_t *align);

/**
 * Retrieves the type's data.
 *
 * # Safety
 *
 * The original `TypeInfoHandle` needs to stay alive as long as the `TypeInfoData` lives. The
 * `TypeInfoData` is destroyed at the same time as the `TypeInfo`. A `TypeInfo` might be destroyed
 * through a call to [`mun_type_info_decrement_strong_count`].
 */
struct MunErrorHandle mun_type_info_data(struct MunTypeInfoHandle type_info,
                                         union MunTypeInfoData *type_info_data);

/**
 * Deallocates an span of `TypeInfo` that was allocated by the runtime.
 *
 * # Safety
 *
 * This function receives a span as parameter. Only when the spans data pointer is not null, its
 * content will be deallocated. Passing pointers to invalid data or memory allocated by other
 * processes, will lead to undefined behavior.
 */
bool mun_type_info_span_destroy(struct MunTypeInfoSpan array_handle);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* MUN_RUNTIME_BINDINGS_H_ */
