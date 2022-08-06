#ifndef MUN_TYPE_INFO_H
#define MUN_TYPE_INFO_H

#include <optional>

#include "mun/md5.h"
#include "mun/runtime_capi.h"

namespace mun {
/**
 * @brief A wrapper around a Mun type information handle.
 *
 * Moving of `Type` leaves the class in an valid but undefined state. Calling any of the accessors
 * on a moved `Type` will result in an assertion.
 */
class Type {
    friend class StructType;

public:
    /**
     * @brief Constructs type information from an instantiated `MunType`.
     *
     * This function assumes it is granted ownership of `type_handle`.
     *
     * \param handle A C-style Mun type type_handle.
     */
    constexpr explicit Type(MunType handle) noexcept : m_handle(handle) {}

    ~Type() noexcept {
        if (m_handle._0 != nullptr) {
            MUN_ASSERT(mun_type_release(m_handle));
            m_handle._0 = nullptr;
        }
    }

    template <typename U, typename std::enable_if<std::is_base_of_v<Type, U>>::type>
    Type(const U& other) noexcept : m_handle(other.m_handle) {
        MUN_ASSERT(mun_type_add_reference(m_handle));
    }

    template <typename U, typename std::enable_if<std::is_base_of_v<Type, U>>::type>
    constexpr Type(U&& other) noexcept : m_handle(other.m_handle) {
        other.m_handle._0 = nullptr;
    }

    template <typename U, typename std::enable_if<std::is_base_of_v<Type, U>>::type>
    Type& operator=(const U& other) {
        if (other.m_handle._0 != nullptr) {
            MUN_ASSERT(mun_type_add_reference(other.m_handle));
        }
        if (m_handle._0 != nullptr) {
            MUN_ASSERT(mun_type_release(m_handle));
        }
        m_handle = other.m_handle;
    }

    template <typename U, typename std::enable_if<std::is_base_of_v<Type, U>>::type>
    constexpr Type& operator=(U&& other) noexcept {
        m_handle = other.m_handle;
        other.m_handle._0 = nullptr;
        return *this;
    }

    bool operator==(const Type& other) const { return mun_type_equal(m_handle, other.m_handle); }

    /**
     * Returns true if this TypeInfo represents a struct.
     */
    [[nodiscard]] bool is_struct() const noexcept {
        MunTypeKind type_kind;
        MUN_ASSERT(mun_type_kind(m_handle, &type_kind));
        return type_kind.tag == MUN_TYPE_KIND_STRUCT;
    }

    /**
     * Returns true if this TypeInfo represents a pointer.
     */
    [[nodiscard]] bool is_pointer() const noexcept {
        MunTypeKind type_kind;
        MUN_ASSERT(mun_type_kind(m_handle, &type_kind));
        return type_kind.tag == MUN_TYPE_KIND_POINTER;
    }

    /**
     * Returns true if this TypeInfo represents a primitive.
     */
    [[nodiscard]] bool is_primitive() const noexcept {
        MunTypeKind type_kind;
        MUN_ASSERT(mun_type_kind(m_handle, &type_kind));
        return type_kind.tag == MUN_TYPE_KIND_PRIMITIVE;
    }

    /**
     * @brief Retrieves the type's name.
     */
    [[nodiscard]] std::string name() const noexcept {
        const char* name;
        MUN_ASSERT(mun_type_name(m_handle, &name));
        std::string str(name);
        mun_string_destroy(name);
        return str;
    }

    /**
     * @brief Retrieves the type's size in bytes.
     */
    [[nodiscard]] size_t size() const noexcept {
        size_t size;
        MUN_ASSERT(mun_type_size(m_handle, &size));
        return size;
    }

    /**
     * @brief Retrieves the type's alignment in bytes.
     */
    [[nodiscard]] size_t alignment() const noexcept {
        size_t alignment;
        MUN_ASSERT(mun_type_alignment(m_handle, &alignment));
        return alignment;
    }

    /**
     * @brief Returns the wrapped C type handle.
     *
     * Ownership of the type_handle remains with this instance, it is not transferred. See
     * [`Type::release_type_handle`] to transfer ownership of the handle.
     */
    [[nodiscard]] inline constexpr const MunType& type_handle() const noexcept { return m_handle; }

    /**
     * @brief Returns the wrapped C type handle, transferring ownership.
     */
    [[nodiscard]] inline constexpr MunType release_type_handle() && noexcept {
        auto handle = m_handle;
        m_handle._0 = nullptr;
        return handle;
    }

private:
    MunType m_handle;
};

/**
 * @brief A wrapper around a span of Mun types.
 *
 * The array is owned by this instance.
 */
class TypeArray {
public:
    struct Iterator {
        using iterator_category = std::random_access_iterator_tag;
        using difference_type = std::ptrdiff_t;
        using value_type = Type;

        constexpr explicit Iterator(MunType const* ptr) : m_ptr(ptr){};

        value_type operator*() const {
            // MunTypes owns the Types inside it, mun::Type assumes it takes ownership of the data,
            // therefor we increase reference count here.
            MUN_ASSERT(mun_type_add_reference(*m_ptr));
            return Type(*m_ptr);
        }

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
        MunType const* m_ptr;
    };

    /**
     * @brief Constructs a type information span from an instantiated `MunTypeInfoSpan`.
     */
    constexpr explicit TypeArray(MunTypes types) noexcept : m_data(types) {}

    ~TypeArray() noexcept {
        if (m_data.types != nullptr) {
            mun_types_destroy(m_data);
            m_data.types = nullptr;
        }
    }

    TypeArray(const TypeArray&) = delete;
    TypeArray& operator=(const TypeArray&) = delete;

    constexpr TypeArray(TypeArray&& other) noexcept : m_data(other.m_data) {
        other.m_data.types = nullptr;
    };

    TypeArray& operator=(TypeArray&& other) noexcept {
        std::swap(m_data, other.m_data);
        return *this;
    };

    /**
     * @brief Returns an iterator to the beginning.
     */
    [[nodiscard]] inline constexpr Iterator begin() const noexcept {
        return Iterator(m_data.types);
    }

    /**
     * @brief Returns an iterator to the end.
     */
    [[nodiscard]] inline constexpr Iterator end() const noexcept {
        return Iterator(m_data.types + m_data.count);
    }

    /**
     * @brief Returns the number of elements in the array.
     */
    [[nodiscard]] inline constexpr size_t size() const noexcept { return m_data.count; }

private:
    MunTypes m_data;
};
}  // namespace mun

#endif  // MUN_TYPE_INFO_H
