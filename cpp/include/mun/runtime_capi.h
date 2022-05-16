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

typedef uintptr_t MunToken;

/**
 * A C-style handle to an error.
 */
typedef struct MunErrorHandle {
    MunToken _0;
} MunErrorHandle;

/**
 * Represents a globally unique identifier (GUID).
 */
typedef struct MunGuid {
    uint8_t _0[16];
} MunGuid;

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
    const struct MunTypeInfo *const *field_types;
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

/**
 * Contains data specific to a group of types that illicit the same characteristics.
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
        struct MunStructInfo struct_;
    };
} MunTypeInfoData;

/**
 * Represents the type declaration for a value type.
 *
 * TODO: add support for polymorphism, enumerations, type parameters, generic type definitions, and
 * constructed generic types.
 */
typedef struct MunTypeInfo {
    /**
     * Type GUID
     */
    struct MunGuid guid;
    /**
     * Type name
     */
    const char *name;
    /**
     * The exact size of the type in bits without any padding
     */
    uint32_t size_in_bits;
    /**
     * The alignment of the type
     */
    uint8_t alignment;
    /**
     * Type group
     */
    union MunTypeInfoData data;
} MunTypeInfo;

/**
 * Represents a function signature.
 */
typedef struct MunFunctionSignature {
    /**
     * Argument types
     */
    const struct MunTypeInfo *const *arg_types;
    /**
     * Optional return type
     */
    const struct MunTypeInfo *return_type;
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
 * A C-style handle to a runtime.
 */
typedef struct MunRuntimeHandle {
    void *_0;
} MunRuntimeHandle;

/**
 * `UnsafeTypeInfo` is a type that wraps a `NonNull<TypeInfo>` and indicates unsafe interior
 * operations on the wrapped `TypeInfo`. The unsafety originates from uncertainty about the
 * lifetime of the wrapped `TypeInfo`.
 *
 * Rust lifetime rules do not allow separate lifetimes for struct fields, but we can make `unsafe`
 * guarantees about their lifetimes. Thus the `UnsafeTypeInfo` type is the only legal way to obtain
 * shared references to the wrapped `TypeInfo`.
 */
typedef struct MunTypeInfo *MunUnsafeTypeInfo;

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

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

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
struct MunErrorHandle mun_runtime_get_function_definition(struct MunRuntimeHandle handle,
                                                          const char *fn_name,
                                                          bool *has_fn_info,
                                                          struct MunFunctionDefinition *fn_definition);

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
 * Deallocates a string that was allocated by the runtime.
 *
 * # Safety
 *
 * This function receives a raw pointer as parameter. Only when the argument is not a null pointer,
 * its content will be deallocated. Passing pointers to invalid data or memory allocated by other
 * processes, will lead to undefined behavior.
 */
void mun_destroy_string(const char *string);

/**
 * Destructs the error corresponding to `error_handle`.
 */
void mun_error_destroy(struct MunErrorHandle error_handle);

/**
 * Retrieves the error message corresponding to `error_handle`. If the `error_handle` exists, a
 * valid `char` pointer is returned, otherwise a null-pointer is returned.
 */
const char *mun_error_message(struct MunErrorHandle error_handle);

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
struct MunErrorHandle mun_gc_alloc(struct MunRuntimeHandle handle,
                                   MunUnsafeTypeInfo type_info,
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
                                      MunUnsafeTypeInfo *type_info);

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

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* MUN_RUNTIME_BINDINGS_H_ */
