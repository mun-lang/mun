#ifndef MUN_STRUCT_INFO_H
#define MUN_STRUCT_INFO_H

#include <cassert>

#include "mun/field_info.h"
#include "mun/runtime_capi.h"

namespace mun {
/**
 * @brief A wrapper around a Mun struct information handle.
 */
class StructType : public Type {
    /**
     * @brief Constructs struct information from a `MunStructInfo` and its associated Type.
     */
    constexpr StructType(MunType type_handle, MunStructInfo struct_info) noexcept
        : Type(type_handle), m_struct_info(struct_info) {}

public:
    /**
     * @brief Tries to cast the specified `Type` into a `StructType`.
     * Returns `std::nullopt` if the `Type` does not represent a struct.
     * \param ty The `Type` to cast
     * \return The StructType if the cast was successful.
     */
    static std::optional<StructType> try_cast(Type ty) {
        MunTypeKind kind;
        MUN_ASSERT(mun_type_kind(ty.type_handle(), &kind));
        if (kind.tag == MUN_TYPE_KIND_STRUCT) {
            return std::make_optional(
                StructType(std::move(ty).release_type_handle(), kind.struct_));
        } else {
            return std::nullopt;
        }
    }

    /**
     * @brief Returns the struct's fields.
     */
    StructFields fields() const noexcept {
        MunFields fields;
        MUN_ASSERT(mun_struct_type_fields(m_struct_info, &fields));
        return StructFields(fields);
    }

    /**
     * @brief Returns the struct's memory kind.
     */
    [[nodiscard]] MunStructMemoryKind memory_kind() const noexcept {
        MunStructMemoryKind memory_kind;
        MUN_ASSERT(mun_struct_type_memory_kind(m_struct_info, &memory_kind));
        return memory_kind;
    }

private:
    MunStructInfo m_struct_info;
};
}  // namespace mun

#endif  // MUN_STRUCT_INFO_H
