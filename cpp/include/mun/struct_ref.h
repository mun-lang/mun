#ifndef MUN_STRUCT_REF_H_
#define MUN_STRUCT_REF_H_

#include <cassert>
#include <cstddef>
#include <cstring>
#include <iostream>
#include <optional>
#include <string>

#include "mun/gc.h"
#include "mun/marshal.h"
#include "mun/runtime.h"
#include "mun/static_type_info.h"
#include "mun/struct_type.h"
#include "mun/type.h"

namespace mun {
namespace details {
inline std::string format_struct_field(std::string_view struct_name,
                                       std::string_view field_name) noexcept {
    std::string formatted;
    formatted.reserve(struct_name.size() + 2 + field_name.size());

    return formatted.append(struct_name).append("::").append(field_name);
}
}  // namespace details

/** Type-agnostic wrapper for interoperability with a Mun struct.
 *
 * Roots and unroots the underlying object upon construction and destruction,
 * respectively.
 */
class StructRef {
public:
    /** Constructs a `StructRef` that wraps a raw Mun struct.
     *
     * \param runtime a reference to the runtime in which the object instance
     * was allocated
     * \param raw a raw garbage collection pointer to the object instance
     */
    StructRef(const Runtime& runtime, MunGcPtr raw) noexcept
        : m_runtime(&runtime), m_handle(GcRootPtr(runtime, raw)) {
        assert(runtime.ptr_type(raw).is_struct());
    }

    StructRef(const StructRef&) noexcept = default;
    StructRef(StructRef&&) noexcept = default;

    StructRef& operator=(StructRef&&) noexcept = default;

    /** Retrieves the raw garbage collection type_handle of the struct.
     *
     * \return a raw garbage collection type_handle
     */
    [[nodiscard]] constexpr MunGcPtr raw() const noexcept { return m_handle.handle(); }

    /** Retrieves the type information of the struct.
     *
     * Updating the runtime can invalidate the returned pointer, leading to
     * undefined behavior when it is accessed.
     *
     * \return a pointer to the struct's type information
     */
    [[nodiscard]] StructType type() const noexcept {
        // Safety: this is safe because a StructRef must always contain a struct type
        return StructType::try_cast(m_runtime->ptr_type(raw())).value();
    }

    /** Tries to retrieve the copied value of the field corresponding to
     * `field_name`.
     *
     * \param field_name the name of the desired field
     * \return possibly, the value of the desired field
     */
    template <typename T>
    std::optional<T> get(std::string_view field_name) const noexcept;

    /** Tries to replace the value of the field corresponding to
     * `field_name`, returning its original value.
     *
     * \param field_name the name of the desired field
     * \return possibly, the value of the replaced field
     */
    template <typename T>
    std::optional<T> replace(std::string_view field_name, T value) noexcept;

    /** Tries to set the value of the field corresponding to
     * `field_name` to the provided `value`.
     *
     * \param field_name the name of the desired field
     * \param value the new value of the field
     * \return whether the field was set successfully
     */
    template <typename T>
    bool set(std::string_view field_name, T value) noexcept;

private:
    const Runtime* m_runtime;
    GcRootPtr m_handle;
};

template <>
struct Marshal<StructRef> {
    using type = MunGcPtr;

    static StructRef from(type ptr, const Runtime& runtime) noexcept { return {runtime, ptr}; }

    static type to(StructRef value) noexcept { return value.raw(); }

    static StructRef copy_from(const type* ptr, const Runtime& runtime,
                               const Type& type_info) noexcept {
        MunTypeKind type_kind;
        MUN_ASSERT(mun_type_kind(type_info.type_handle(), &type_kind));

        // Safety: `mun_type_kind` is guaranteed to return a value for `StructRef`s.
        MunStructMemoryKind memory_kind;
        MUN_ASSERT(mun_struct_type_memory_kind(type_kind.struct_, &memory_kind));

        MunGcPtr gc_handle;
        if (memory_kind == MUN_STRUCT_MEMORY_KIND_VALUE) {
            // Create a new managed object
            gc_handle = *runtime.gc_alloc(type_info);

            // Copy the old object into the new object
            std::memcpy(*gc_handle, ptr, type_info.size());
        } else {
            // For a gc struct, `ptr` points to a `MunGcPtr`.
            gc_handle = *ptr;
        }

        return {runtime, gc_handle};
    }

    static void move_to(type value, type* ptr, const Type& type_info) noexcept {
        MunTypeKind type_kind;
        MUN_ASSERT(mun_type_kind(type_info.type_handle(), &type_kind));

        // Safety: `mun_type_kind` is guaranteed to return a value for `StructRef`s.
        MunStructMemoryKind memory_kind;
        MUN_ASSERT(mun_struct_type_memory_kind(type_kind.struct_, &memory_kind));

        if (memory_kind == MUN_STRUCT_MEMORY_KIND_VALUE) {
            // Copy the `struct(value)` into the old object
            std::memcpy(ptr, *value, type_info.size());
        } else {
            *ptr = std::move(value);
        }
    }

    static StructRef swap_at(type value, type* ptr, const Runtime& runtime,
                             const Type& type_info) noexcept {
        MunTypeKind type_kind;
        MUN_ASSERT(mun_type_kind(type_info.type_handle(), &type_kind));

        // Safety: `mun_type_kind` is guaranteed to return a value for `StructRef`s.
        MunStructMemoryKind memory_kind;
        MUN_ASSERT(mun_struct_type_memory_kind(type_kind.struct_, &memory_kind));

        MunGcPtr gc_handle;
        if (memory_kind == MUN_STRUCT_MEMORY_KIND_VALUE) {
            // Create a new managed object
            gc_handle = *runtime.gc_alloc(type_info);

            const auto size = type_info.size();

            // Copy the old object into the new object
            std::memcpy(*gc_handle, ptr, size);
            // Copy the `struct(value)` into the old object
            std::memcpy(ptr, *value, size);
        } else {
            // For a gc struct, `ptr` points to a `MunGcPtr`.
            gc_handle = *ptr;
        }

        return {runtime, gc_handle};
    }
};
}  // namespace mun

#include "mun/reflection.h"

namespace mun {

template <>
struct ArgumentReflection<StructRef> {
    static Type type_info(const StructRef& ref) noexcept { return ref.type(); }
};

template <>
struct ReturnTypeReflection<StructRef> {
    static bool accepts_type(const Type& ty) noexcept { return ty.is_struct(); }
    static std::string type_hint() {
        using namespace std::string_literals;
        return "struct"s;
    }
};

template <typename T>
std::optional<T> StructRef::get(std::string_view field_name) const noexcept {
    auto type_info = type();
    if (const auto field_info = type_info.fields().find_by_name(field_name);
        field_info.has_value()) {
        if (!ReturnTypeReflection<T>::accepts_type(field_info->type())) {
            std::cerr << "Mismatched types for `"
                      << details::format_struct_field(type_info.name(), field_name)
                      << "`. Expected: `" << ReturnTypeReflection<T>::type_hint << "`. Found: `"
                      << type_info.name() << "`." << std::endl;

            return std::nullopt;
        }

        const auto offset = static_cast<size_t>(field_info->offset());
        const auto byte_ptr = reinterpret_cast<const std::byte*>(*raw());
        return std::make_optional(Marshal<T>::copy_from(
            reinterpret_cast<const typename Marshal<T>::type*>(byte_ptr + offset), *m_runtime,
            field_info->type()));
    } else {
        return std::nullopt;
    }
}
template <typename T>
std::optional<T> StructRef::replace(std::string_view field_name, T value) noexcept {
    auto type_info = type();
    if (const auto field_info = type_info.fields().find_by_name(field_name);
        field_info.has_value()) {
        if (!ReturnTypeReflection<T>::accepts_type(field_info->type())) {
            std::cerr << "Mismatched types for `"
                      << details::format_struct_field(type_info.name(), field_name)
                      << "`. Expected: `" << ReturnTypeReflection<T>::type_hint << "`. Found: `"
                      << type_info.name() << "`." << std::endl;

            return std::nullopt;
        }

        auto byte_ptr = reinterpret_cast<std::byte*>(*raw());
        return std::make_optional(Marshal<T>::swap_at(
            Marshal<T>::to(std::move(value)),
            reinterpret_cast<typename Marshal<T>::type*>(byte_ptr + field_info->offset()),
            *m_runtime, field_info->type()));
    } else {
        return std::nullopt;
    }
}
template <typename T>
bool StructRef::set(std::string_view field_name, T value) noexcept {
    auto type_info = type();
    if (const auto field_info = type_info.fields().find_by_name(field_name);
        field_info.has_value()) {
        if (!ReturnTypeReflection<T>::accepts_type(field_info->type())) {
            std::cerr << "Mismatched types for `"
                      << details::format_struct_field(type_info.name(), field_name)
                      << "`. Expected: `" << ReturnTypeReflection<T>::type_hint << "`. Found: `"
                      << type_info.name() << "`." << std::endl;

            return false;
        }

        auto byte_ptr = reinterpret_cast<std::byte*>(*raw());

        Marshal<T>::move_to(
            Marshal<T>::to(std::move(value)),
            reinterpret_cast<typename Marshal<T>::type*>(byte_ptr + field_info->offset()),
            field_info->type());
        return true;
    } else {
        return false;
    }
}
}  // namespace mun

#endif
