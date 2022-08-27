#ifndef MUN_ARRAY_REF_H_
#define MUN_ARRAY_REF_H_

#include <cassert>
#include <cstddef>
#include <cstring>
#include <iostream>
#include <optional>
#include <string>

#include "mun/array_type.h"
#include "mun/gc.h"
#include "mun/marshal.h"
#include "mun/runtime.h"
#include "mun/static_type_info.h"
#include "mun/type.h"

namespace mun {

namespace details {

/// Computes the number of bytes to skip to get a next address that is also aligned.
inline size_t size_rounded_up(size_t size, size_t align) {
    return (size + align - 1) & ~(align - 1);
}
}  // namespace details

/** Type-agnostic wrapper for interoperability with a Mun array.
 *
 * Roots and unroots the underlying object upon construction and destruction, respectively.
 */
template <typename T>
class ArrayRef {
    struct Header {
        size_t length, capacity;
    };

public:
    struct Iterator {
        using iterator_category = std::input_iterator_tag;
        using difference_type = std::ptrdiff_t;
        using value_type = T;
        using reference = value_type&;
        using pointer = value_type*;

        Iterator(const std::byte* element_ptr, size_t element_stride, Type element_ty,
                 Runtime const* runtime)
            : m_element_ptr(element_ptr),
              m_element_stride(element_stride),
              m_element_type(std::move(element_ty)),
              m_runtime(runtime) {}

        value_type operator*() const {
            return Marshal<T>::copy_from(
                reinterpret_cast<const typename Marshal<T>::type*>(m_element_ptr), *m_runtime,
                m_element_type);
        }
        Iterator& operator++() {
            m_element_ptr += m_element_stride;
            return *this;
        }
        Iterator operator++(int) {
            Iterator tmp = *this;
            ++(*this);
            return tmp;
        }
        friend bool operator==(const Iterator& a, const Iterator& b) {
            return a.m_element_ptr == b.m_element_ptr;
        };
        friend bool operator!=(const Iterator& a, const Iterator& b) {
            return a.m_element_ptr != b.m_element_ptr;
        };

    private:
        std::byte const* m_element_ptr;
        size_t m_element_stride;
        Type m_element_type;
        Runtime const* m_runtime;
    };

public:
    /** Constructs a `ArrayRef` that wraps a raw Mun array.
     *
     * \param runtime a reference to the runtime in which the object instance
     * was allocated
     * \param raw a raw garbage collection pointer to the object instance
     */
    ArrayRef(const Runtime& runtime, MunGcPtr raw) noexcept
        : m_runtime(&runtime), m_handle(GcRootPtr(runtime, raw)) {
        assert(runtime.ptr_type(raw).is_array());
    }

    ArrayRef(const ArrayRef&) noexcept = default;
    ArrayRef(ArrayRef&&) noexcept = default;

    ArrayRef& operator=(ArrayRef&&) noexcept = default;

    /** Retrieves the raw garbage collection type_handle of the array.
     *
     * \return a raw garbage collection type_handle
     */
    [[nodiscard]] constexpr MunGcPtr raw() const noexcept { return m_handle.handle(); }

    /** Retrieves the type information of the array.
     *
     * Updating the runtime can invalidate the returned pointer, leading to
     * undefined behavior when it is accessed.
     *
     * \return a pointer to the array's type information
     */
    [[nodiscard]] ArrayType type() const noexcept {
        // Safety: this is safe because a ArrayRef must always contain an array type
        return ArrayType::try_cast(m_runtime->ptr_type(raw())).value();
    }

    /**
     * Returns the number of elements stored in the array.
     */
    [[nodiscard]] inline size_t size() const;

    /**
     * Returns the number of elements that can potentially be stored in the array without
     * reallocating.
     */
    [[nodiscard]] inline size_t capacity() const;

    /**
     * Returns true if this instance doesn't contain a single element.
     */
    [[nodiscard]] inline bool empty() const { return size() == 0; };

    /**
     * Returns the element at the given index, with bounds checking. If pos is not within the range
     * of the container, an exception of type `std::out_of_range` is thrown.
     */
    [[nodiscard]] T at(size_t idx) const;

    /**
     * Returns an iterator to the first element of the array. If the array is empty, the returned
     * iterator will be equal to end().
     */
    Iterator begin() const;

    /**
     * Returns an iterator to the element following the last element of the array. This element acts
     * as a placeholder; attempting to access it results in undefined behavior.
     */
    Iterator end() const;

private:
    const Runtime* m_runtime;
    GcRootPtr m_handle;
};

template <typename T>
struct Marshal<ArrayRef<T>> {
    using type = MunGcPtr;

    static ArrayRef<T> from(type ptr, const Runtime& runtime) noexcept { return {runtime, ptr}; }

    static type to(ArrayRef<T> value) noexcept { return value.raw(); }

    static ArrayRef<T> copy_from(const type* ptr, const Runtime& runtime,
                                 const Type& type_info) noexcept {
        return {runtime, *ptr};
    }

    static void move_to(type value, type* ptr, const Type& type_info) noexcept {
        *ptr = std::move(value);
    }

    static ArrayRef<T> swap_at(type value, type* ptr, const Runtime& runtime,
                             const Type& type_info) noexcept {
        return {runtime, *ptr};
    }
};
}  // namespace mun

#include "mun/reflection.h"

namespace mun {

template <typename T>
struct ArgumentReflection<ArrayRef<T>> {
    static Type type_info(const ArrayRef<T>& ref) noexcept { return ref.type(); }
};

template <typename T>
struct ReturnTypeReflection<ArrayRef<T>> {
    static bool accepts_type(const Type& ty) noexcept {
        auto array_ty = ArrayType::try_cast(ty);
        if (array_ty.has_value()) {
            return ReturnTypeReflection<T>::accepts_type(array_ty.value().element_type());
        }
        return false;
    }
    static std::string type_hint() {
        using namespace std::string_literals;
        return "["s + ReturnTypeReflection<T>::type_hint() + "]"s;
    }
};

template <typename T>
size_t ArrayRef<T>::size() const {
    return reinterpret_cast<const Header*>(*raw())->length;
}

template <typename T>
size_t ArrayRef<T>::capacity() const {
    return reinterpret_cast<const Header*>(*raw())->capacity;
}
template <typename T>
T ArrayRef<T>::at(size_t idx) const {
    if (idx >= size()) {
        throw std::out_of_range("array element out of range");
    }

    auto element_type = type().element_type();
    auto element_size = element_type.size();
    auto element_alignment = element_type.alignment();
    auto element_stride = details::size_rounded_up(element_size, element_alignment);
    auto header_offset = details::size_rounded_up(sizeof(Header), element_alignment);
    auto element_ptr =
        reinterpret_cast<const std::byte*>(*raw()) + header_offset + element_stride * idx;

    return Marshal<T>::copy_from(reinterpret_cast<const typename Marshal<T>::type*>(element_ptr),
                                 *m_runtime, element_type);
}

template <typename T>
typename ArrayRef<T>::Iterator ArrayRef<T>::begin() const {
    auto element_type = type().element_type();
    auto element_size = element_type.size();
    auto element_alignment = element_type.alignment();
    auto element_stride = details::size_rounded_up(element_size, element_alignment);
    auto header_offset = details::size_rounded_up(sizeof(Header), element_alignment);
    auto element_ptr = reinterpret_cast<const std::byte*>(*raw()) + header_offset;
    return Iterator(element_ptr, element_stride, std::move(element_type), m_runtime);
}

template <typename T>
typename ArrayRef<T>::Iterator ArrayRef<T>::end() const {
    auto element_type = type().element_type();
    auto element_size = element_type.size();
    auto element_alignment = element_type.alignment();
    auto element_stride = details::size_rounded_up(element_size, element_alignment);
    auto header_offset = details::size_rounded_up(sizeof(Header), element_alignment);
    auto element_ptr = reinterpret_cast<const std::byte*>(*raw()) + header_offset;
    return Iterator(element_ptr + element_stride * size(), element_stride, std::move(element_type),
                    m_runtime);
}

}  // namespace mun

#endif
