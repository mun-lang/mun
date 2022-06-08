#ifndef MUN_STRUCT_INFO_H
#define MUN_STRUCT_INFO_H

#include <cassert>

#include "mun/field_info.h"
#include "mun/runtime_capi.h"

namespace mun {
/**
 * @brief A wrapper around a Mun struct information handle.
 */
class StructInfo {
public:
    /**
     * @brief Constructs struct information from an instantiated `MunStructInfoHandle`.
     */
    StructInfo(MunStructInfoHandle handle) noexcept : m_handle(handle) {}

    /**
     * @brief Returns the struct's fields.
     */
    FieldInfoSpan fields() const noexcept {
        MunFieldInfoSpan span;
        const auto error_handle = mun_struct_info_fields(m_handle, &span);
        assert(error_handle._0 == nullptr);

        return FieldInfoSpan(span);
    }

    /**
     * @brief Returns the struct's memory kind.
     */
    MunStructMemoryKind memory_kind() const noexcept {
        MunStructMemoryKind memory_kind;
        const auto error_handle = mun_struct_info_memory_kind(m_handle, &memory_kind);
        assert(error_handle._0 == nullptr);

        return memory_kind;
    }

private:
    MunStructInfoHandle m_handle;
};
}  // namespace mun

#endif  // MUN_STRUCT_INFO_H
