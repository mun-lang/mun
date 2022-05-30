#ifndef MUN_FIELD_INFO_H_
#define MUN_FIELD_INFO_H_

#include <cassert>

#include "mun/runtime_capi.h"
#include "mun/type_info.h"

namespace mun {
/**
 * @brief A wrapper around a Mun field information handle.
 */
class FieldInfo {
public:
    /**
     * @brief Constructs field information from an instantiated `MunFieldInfoHandle`.
     */
    FieldInfo(MunFieldInfoHandle handle) noexcept : m_handle(handle) {}

    /**
     * @brief Retrieves the field's name.
     */
    std::string_view name() const noexcept {
        const auto ptr = mun_field_info_name(m_handle);
        assert(ptr);
        return ptr;
    }

    /**
     * @brief Retrieves the field's type.
     */
    TypeInfo type() const noexcept {
        const auto type_handle = mun_field_info_type(m_handle);
        assert(type_handle._0);
        return TypeInfo(type_handle);
    }

    /**
     * @brief Retrieves the field's offset.
     */
    uint16_t offset() const noexcept {
        uint16_t offset;
        const auto error_handle = mun_field_info_offset(m_handle, &offset);
        assert(error_handle._0 == nullptr);
        return offset;
    }

private:
    MunFieldInfoHandle m_handle;
};

/**
 * @brief A wrapper around a span of Mun field informations, which are owned by the Mun runtime.
 */
class FieldInfoSpan {
public:
    /**
     * @brief Constructs a field information span from an instantiated `MunFieldInfoSpan`.
     */
    FieldInfoSpan(MunFieldInfoSpan span) noexcept : m_span(span) {}

    ~FieldInfoSpan() noexcept { mun_field_info_span_destroy(m_span); }

    /**
     * @brief Returns an iterator to the beginning.
     */
    const MunFieldInfoHandle* begin() const noexcept { return data(); }

    /**
     * @brief Returns an iterator to the end.
     */
    const MunFieldInfoHandle* end() const noexcept { return data() + size() + 1; }

    /**
     * @brief Returns a pointer to the beginning of the sequence of elements.
     */
    const MunFieldInfoHandle* data() const noexcept { return m_span.data; }

    /**
     * @brief Returns the number of elements in the sequence.
     */
    size_t size() const noexcept { return m_span.len; }

private:
    MunFieldInfoSpan m_span;
};
}  // namespace mun

#endif  // MUN_FIELD_INFO_H_
