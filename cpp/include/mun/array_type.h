#ifndef MUN_ARRAY_TYPE_H
#define MUN_ARRAY_TYPE_H

#include <cassert>

#include "mun/error.h"
#include "mun/runtime_capi.h"
#include "mun/type.h"

namespace mun {
/**
 * @brief A wrapper around a Mun array type information handle.
 */
class ArrayType : public Type {
    /**
     * @brief Constructs struct information from a `MunStructInfo` and its associated Type.
     */
    constexpr ArrayType(MunType type_handle, MunArrayInfo array_info) noexcept
        : Type(type_handle), m_array_info(array_info) {}

public:
    /**
     * @brief Tries to cast the specified `Type` into a `StructType`.
     * Returns `std::nullopt` if the `Type` does not represent a struct.
     * \param ty The `Type` to cast
     * \return The StructType if the cast was successful.
     */
    static std::optional<ArrayType> try_cast(Type ty) {
        MunTypeKind kind;
        MUN_ASSERT(mun_type_kind(ty.type_handle(), &kind));
        if (kind.tag == MUN_TYPE_KIND_ARRAY) {
            return std::make_optional(ArrayType(std::move(ty).release_type_handle(), kind.array));
        } else {
            return std::nullopt;
        }
    }

    /**
     * @brief Returns the element type
     */
    [[nodiscard]] inline Type element_type() const noexcept {
        MunType ty;
        MUN_ASSERT(mun_array_type_element_type(m_array_info, &ty));
        return Type(ty);
    }

private:
    MunArrayInfo m_array_info;
};
}  // namespace mun

#endif  // MUN_ARRAY_TYPE_H
