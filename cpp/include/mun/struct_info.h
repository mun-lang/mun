#ifndef MUN_STRUCT_INFO_H
#define MUN_STRUCT_INFO_H

#include <cassert>

#include "mun/field_info.h"
#include "mun/runtime_capi.h"

namespace mun {
/**
 * @brief A wrapper around a Mun struct information handle.
 */
class StructInfo : public TypeInfo {
    /**
     * @brief Constructs struct information from an instantiated `MunStructInfoHandle`.
     */
    StructInfo(MunTypeInfoHandle type_handle, MunStructInfoHandle struct_handle) noexcept
        : TypeInfo(type_handle), m_struct_handle(struct_handle) {}

public:
    /**
     * @brief Tries to cast the specified `TypeInfo` into a `StructInfo`.
     * Returns `std::nullopt` if the `TypeInfo` does not represent a struct.
     * @param ty The `TypeInfo` to cast
     * @return An optional StructInfo.
     */
    static std::optional<StructInfo> try_cast(const TypeInfo& ty) {
        MunTypeInfoData data;
        auto type_info_handle = ty.handle();
        const auto error_handle = mun_type_info_data(type_info_handle, &data);
        assert(error_handle._0 == nullptr);
        if (data.tag != MunTypeInfoData_Struct) {
            return std::nullopt;
        }
        mun_type_info_increment_strong_count(type_info_handle);
        return StructInfo(type_info_handle, data.struct_);
    }

    /**
     * @brief Returns the struct's fields.
     */
    FieldInfoSpan fields() const noexcept {
        MunFieldInfoSpan span;
        const auto error_handle = mun_struct_info_fields(m_struct_handle, &span);
        assert(error_handle._0 == nullptr);

        return FieldInfoSpan(span);
    }

    /**
     * @brief Returns the struct's memory kind.
     */
    MunStructMemoryKind memory_kind() const noexcept {
        MunStructMemoryKind memory_kind;
        const auto error_handle = mun_struct_info_memory_kind(m_struct_handle, &memory_kind);
        assert(error_handle._0 == nullptr);

        return memory_kind;
    }

private:
    MunStructInfoHandle m_struct_handle;
};
}  // namespace mun

#endif  // MUN_STRUCT_INFO_H
