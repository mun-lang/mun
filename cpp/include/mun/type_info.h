#ifndef MUN_TYPE_INFO_H
#define MUN_TYPE_INFO_H

#include <optional>

#include "mun/md5.h"
#include "mun/runtime_capi.h"

namespace mun {
/**
 * @brief A wrapper around a Mun type information handle.
 */
class TypeInfo {
    friend class StructInfo;

public:
    /**
     * @brief Constructs type information from an instantiated `MunTypeInfoHandle`.
     */
    TypeInfo(MunTypeInfoHandle handle) noexcept : m_handle(handle) {}

    ~TypeInfo() noexcept {
        if (m_handle._0 != nullptr) {
            mun_type_info_decrement_strong_count(m_handle);
            m_handle._0 = nullptr;
        }
    }

    TypeInfo(const TypeInfo& other) noexcept : m_handle(other.m_handle) {
        mun_type_info_increment_strong_count(m_handle);
    }

    TypeInfo(TypeInfo&& other) noexcept : m_handle(other.m_handle) { other.m_handle._0 = nullptr; }

    TypeInfo& operator=(const TypeInfo& other) {
        m_handle = other.m_handle;
        mun_type_info_increment_strong_count(m_handle);
        return *this;
    }

    TypeInfo& operator=(TypeInfo&& other) noexcept {
        m_handle = other.m_handle;
        other.m_handle._0 = nullptr;
        return *this;
    }

    bool operator==(const TypeInfo& other) const {
        return mun_type_info_eq(m_handle, other.m_handle);
    }

    /**
     * Returns true if this TypeInfo represents a struct.
     */
    [[nodiscard]] bool is_struct() const noexcept {
        MunTypeInfoData data;
        const auto error_handle = mun_type_info_data(m_handle, &data);
        assert(error_handle._0 == nullptr);
        return data.tag == MunTypeInfoData_Struct;
    }

    /**
     * Returns true if this TypeInfo represents a pointer.
     */
    [[nodiscard]] bool is_pointer() const noexcept {
        MunTypeInfoData data;
        const auto error_handle = mun_type_info_data(m_handle, &data);
        assert(error_handle._0 == nullptr);
        return data.tag == MunTypeInfoData_Pointer;
    }

    /**
     * Returns true if this TypeInfo represents a primitive.
     */
    [[nodiscard]] bool is_primitive() const noexcept {
        MunTypeInfoData data;
        const auto error_handle = mun_type_info_data(m_handle, &data);
        assert(error_handle._0 == nullptr);
        return data.tag == MunTypeInfoData_Primitive;
    }

    /**
     * @brief Retrieves the type's name.
     */
    [[nodiscard]] std::string_view name() const noexcept {
        const auto ptr = mun_type_info_name(m_handle);
        assert(ptr);
        return ptr;
    }

    /**
     * @brief Retrieves the type's size in bytes.
     */
    [[nodiscard]] size_t size() const noexcept {
        size_t size;
        const auto error_handle = mun_type_info_size(m_handle, &size);
        assert(error_handle._0 == nullptr);
        return size;
    }

    /**
     * @brief Retrieves the type's alignment.
     */
    [[nodiscard]] size_t alignment() const noexcept {
        size_t align;
        const auto error_handle = mun_type_info_align(m_handle, &align);
        assert(error_handle._0 == nullptr);
        return align;
    }

    /**
     * @brief Returns the type's handle.
     */
    [[nodiscard]] MunTypeInfoHandle handle() const noexcept { return m_handle; }

private:
    MunTypeInfoHandle m_handle;
};

/**
 * @brief A wrapper around a span of Mun type informations, which are owned by the Mun runtime.
 */
class TypeInfoSpan {
public:
    /**
     * @brief Constructs a type information span from an instantiated `MunTypeInfoSpan`.
     */
    TypeInfoSpan(MunTypeInfoSpan span) noexcept : m_span(span) {}

    ~TypeInfoSpan() noexcept { mun_type_info_span_destroy(m_span); }

    TypeInfoSpan(const TypeInfoSpan&) = delete;
    TypeInfoSpan& operator=(const TypeInfoSpan&) = delete;

    TypeInfoSpan(TypeInfoSpan&&) = default;
    TypeInfoSpan& operator=(TypeInfoSpan&&) = default;

    /**
     * @brief Returns an iterator to the beginning.
     */
    const MunTypeInfoHandle* begin() const noexcept { return data(); }

    /**
     * @brief Returns an iterator to the end.
     */
    const MunTypeInfoHandle* end() const noexcept { return data() + size() + 1; }

    /**
     * @brief Returns a pointer to the beginning of the sequence of elements.
     */
    const MunTypeInfoHandle* data() const noexcept { return m_span.data; }

    /**
     * @brief Returns the number of elements in the sequence.
     */
    size_t size() const noexcept { return m_span.len; }

private:
    MunTypeInfoSpan m_span;
};
}  // namespace mun

#endif  // MUN_TYPE_INFO_H
