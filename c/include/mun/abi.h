#ifndef MUN_ABI_H_
#define MUN_ABI_H_

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
    const union MunTypeId *pointee;
    /**
     * Whether or not this pointer is mutable or not
     */
    bool mutable_;
} MunPointerTypeId;

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
    Concrete,
    /**
     * Represents a pointer to a type
     */
    Pointer,
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
} MunTypeId;

/**
 * Represents a function signature.
 */
typedef struct MunFunctionSignature {
    /**
     * Argument types
     */
    const union MunTypeId *arg_types;
    /**
     * Optional return type
     */
    union MunTypeId return_type;
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
typedef struct MunStructDefinition {
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
    const union MunTypeId *field_types;
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
} MunStructDefinition;

/**
 * Contains data specific to a group of types that illicit the same characteristics.
 */
enum MunTypeDefinitionData_Tag
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
    /**
     * Struct types (i.e. record, tuple, or unit structs)
     */
    Struct,
};
#ifndef __cplusplus
typedef uint8_t MunTypeDefinitionData_Tag;
#endif // __cplusplus

typedef union MunTypeDefinitionData {
    MunTypeDefinitionData_Tag tag;
    struct {
        MunTypeDefinitionData_Tag struct_tag;
        struct MunStructDefinition struct_;
    };
} MunTypeDefinitionData;

/**
 * Represents the type declaration for a type that is exported by an assembly.
 *
 * When multiple Mun modules reference the same type, only one module exports the type; the module
 * that contains the type definition. All the other Mun modules reference the type through a
 * [`TypeId`].
 *
 * The modules that defines the type exports the data to reduce the filesize of the assemblies and
 * to ensure only one definition exists. When linking all assemblies together the type definitions
 * from all assemblies are loaded and the information is shared to modules that reference the type.
 *
 * TODO: add support for polymorphism, enumerations, type parameters, generic type definitions, and
 *   constructed generic types.
 */
typedef struct MunTypeDefinition {
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
    union MunTypeDefinitionData data;
} MunTypeDefinition;

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
    const struct MunTypeDefinition *types;
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
    const union MunTypeId *type_ids;
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
