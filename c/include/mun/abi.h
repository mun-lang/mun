#ifndef MUN_ABI_H_
#define MUN_ABI_H_

/* Generated with cbindgen:0.23.0 */

#include <stdint.h>

/**
 * Defines the current ABI version
 */
#define MUN_ABI_VERSION 300

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
    const struct MunTypeId *pointee;
    /**
     * Whether or not this pointer is mutable or not
     */
    bool mutable_;
} MunPointerTypeId;

/**
 * Represents a unique identifier for types. The runtime can use this to lookup the corresponding
 * [`TypeInfo`]. A [`TypeId`] is a key for a [`TypeInfo`].
 *
 * A [`TypeId`] only contains enough information to query the runtime for a concrete type.
 */
typedef enum MunTypeId_Tag {
    /**
     * Represents a concrete type with a specific Guid
     */
    Concrete,
    /**
     * Represents a pointer to a type
     */
    Pointer,
} MunTypeId_Tag;

typedef struct MunTypeId {
    MunTypeId_Tag tag;
    union {
        struct {
            struct MunGuid concrete;
        };
        struct {
            struct MunPointerTypeId pointer;
        };
    };
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
 * Represents a struct declaration.
 */
typedef struct MunStructInfo {
    /**
     * The unique identifier of this struct
     */
    struct MunGuid guid;
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

/**
 * Pointer type information
 */
typedef struct MunPointerInfo {
    /**
     * The type to which this pointer points.
     */
    struct MunTypeId pointee;
    /**
     * Whether or not the pointed to value is mutable or not
     */
    bool mutable_;
} MunPointerInfo;

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
    /**
     * Pointer to another type
     */
    Pointer,
};
#ifndef __cplusplus
typedef uint8_t MunTypeInfoData_Tag;
#endif // __cplusplus

typedef union MunTypeInfoData {
    MunTypeInfoData_Tag tag;
    struct {
        MunTypeInfoData_Tag primitive_tag;
        struct MunGuid primitive;
    };
    struct {
        MunTypeInfoData_Tag struct_tag;
        struct MunStructInfo struct_;
    };
    struct {
        MunTypeInfoData_Tag pointer_tag;
        struct MunPointerInfo pointer;
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
 * Represents a module declaration.
 */
typedef struct MunModuleInfo {
    /**
     * Module path
     */
    const char *path;
    /**
     * Module functions
     */
    const struct MunFunctionDefinition *functions;
    /**
     * Module types
     */
    const struct MunTypeInfo *types;
    /**
     * Number of module functions
     */
    uint32_t num_functions;
    /**
     * Number of module types
     */
    uint32_t num_types;
} MunModuleInfo;

/**
 * Represents a function dispatch table. This is used for runtime linking.
 *
 * Function signatures and pointers are stored separately for cache efficiency.
 */
typedef struct MunDispatchTable {
    /**
     * Function signatures
     */
    const struct MunFunctionPrototype *prototypes;
    /**
     * Function pointers
     */
    const void **fn_ptrs;
    /**
     * Number of functions
     */
    uint32_t num_entries;
} MunDispatchTable;

/**
 * Represents a lookup table for type information. This is used for runtime linking.
 *
 * Type IDs and handles are stored separately for cache efficiency.
 */
typedef struct MunTypeLut {
    /**
     * Type IDs
     */
    const struct MunTypeId *type_ids;
    /**
     * Type information handles
     */
    const void **type_handles;
    /**
     * Debug names
     */
    const char *const *type_names;
    /**
     * Number of types
     */
    uint32_t num_entries;
} MunTypeLut;

/**
 * Represents an assembly declaration.
 */
typedef struct MunAssemblyInfo {
    /**
     * Symbols of the top-level module
     */
    struct MunModuleInfo symbols;
    /**
     * Function dispatch table
     */
    struct MunDispatchTable dispatch_table;
    /**
     * Type lookup table
     */
    struct MunTypeLut type_lut;
    /**
     * Paths to assembly dependencies
     */
    const char *const *dependencies;
    /**
     * Number of dependencies
     */
    uint32_t num_dependencies;
} MunAssemblyInfo;

#endif /* MUN_ABI_H_ */
