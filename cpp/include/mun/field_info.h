#ifndef MUN_FIELD_INFO_H_
#define MUN_FIELD_INFO_H_

#include <cassert>

#include "mun/runtime_capi.h"
#include "mun/type.h"

namespace mun {
/**
 * @brief A wrapper around a MunField.
 *
 * The Type from which this Field came must be kept alive during the lifetime of this instance.
 */
class FieldInfo {
public:
    /**
     * @brief Constructs field information from an instantiated `MunFieldInfoHandle`.
     */
    constexpr explicit FieldInfo(MunField handle) noexcept : m_handle(handle) {}

    /**
     * @brief Retrieves the field's name.
     */
    [[nodiscard]] std::string name() const noexcept {
        const char* name;
        MUN_ASSERT(mun_field_name(m_handle, &name));
        std::string str(name);
        mun_string_destroy(name);
        return str;
    }

    /**
     * @brief Retrieves the field's type.
     */
    [[nodiscard]] Type type() const noexcept {
        MunType ty;
        MUN_ASSERT(mun_field_type(m_handle, &ty));
        return Type(ty);
    }

    /**
     * @brief Retrieves the field's offset.
     */
    [[nodiscard]] uintptr_t offset() const noexcept {
        uintptr_t offset;
        MUN_ASSERT(mun_field_offset(m_handle, &offset));
        return offset;
    }

private:
    MunField m_handle;
};

/**
 * @brief A wrapper around MunTypes. Stores field information of a struct.
 *
 * Note that the StructType must not go out of scope or undefined behavior can occur.
 */
class StructFields {
public:
    struct Iterator {
        using iterator_category = std::random_access_iterator_tag;
        using difference_type = std::ptrdiff_t;
        using value_type = FieldInfo;

        constexpr explicit Iterator(MunField const* ptr) : m_ptr(ptr){};

        value_type operator*() const { return value_type(*m_ptr); }

        Iterator& operator++() {
            m_ptr++;
            return *this;
        }

        Iterator operator++(int) {
            Iterator tmp = *this;
            ++(*this);
            return tmp;
        }

        friend bool operator==(const Iterator& a, const Iterator& b) { return a.m_ptr == b.m_ptr; };
        friend bool operator!=(const Iterator& a, const Iterator& b) { return a.m_ptr != b.m_ptr; };

    private:
        MunField const* m_ptr;
    };

    /**
     * @brief Constructs a field information span from an instantiated `MunTypes`.
     *
     * This function assumes ownership is transferred.
     */
    constexpr explicit StructFields(MunFields fields) noexcept : m_data(fields) {}

    constexpr StructFields(StructFields&& other) noexcept : m_data(other.m_data) {
        other.m_data.fields = nullptr;
    }
    StructFields(const StructFields&) = delete;

    ~StructFields() noexcept {
        if (m_data.fields != nullptr) {
            mun_fields_destroy(m_data);
            m_data.fields = nullptr;
        }
    }

    StructFields& operator=(const StructFields&) = delete;
    StructFields& operator=(StructFields&& other) noexcept {
        std::swap(m_data, other.m_data);
        return *this;
    }

    /**
     * @brief Returns an iterator to the beginning.
     */
    [[nodiscard]] inline constexpr Iterator begin() const noexcept {
        return Iterator(m_data.fields);
    }

    /**
     * @brief Returns an iterator to the end.
     */
    [[nodiscard]] inline constexpr Iterator end() const noexcept {
        return Iterator(m_data.fields + m_data.count);
    }

    /**
     * Finds a certain field by its name.
     */
    [[nodiscard]] std::optional<FieldInfo> find_by_name(std::string_view name) const {
        MunField field;
        bool has_field;
        MUN_ASSERT(mun_fields_find_by_name(m_data, name.data(), name.size(), &has_field, &field));
        if (has_field) {
            return std::make_optional(FieldInfo(field));
        } else {
            return std::nullopt;
        }
    }

    /**
     * @brief Returns the number of fields.
     */
    [[nodiscard]] inline size_t size() const noexcept { return m_data.count; }

private:
    MunFields m_data;
};
}  // namespace mun

#endif  // MUN_FIELD_INFO_H_
