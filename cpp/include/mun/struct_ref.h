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
#include "mun/struct_info.h"
#include "mun/type_info.h"

namespace mun {
namespace details {
inline std::optional<FieldInfo> find_field(std::string_view type_name,
                                           MunStructInfoHandle struct_info,
                                           std::string_view field_name) noexcept {
    MunFieldInfoSpan fields;
    auto err = mun_struct_info_fields(struct_info, &fields);
    assert(err._0 == nullptr);

    const auto it = std::find_if(fields.data, fields.data + fields.len,
                                 [field_name](const MunFieldInfoHandle& field_handle) {
                                     return field_name == mun_field_info_name(field_handle);
                                 });

    if (it == fields.data + fields.len) {
        std::cerr << "StructRef `" << type_name << "` does not contain field `" << field_name
                  << "`." << std::endl;
        return std::nullopt;
    }

    return std::make_optional(FieldInfo(*it));
}

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

    /** Retrieves the raw garbage collection handle of the struct.
     *
     * \return a raw garbage collection handle
     */
    MunGcPtr raw() const noexcept { return m_handle.handle(); }

    /** Retrieves the type information of the struct.
     *
     * Updating the runtime can invalidate the returned pointer, leading to
     * undefined behavior when it is accessed.
     *
     * \return a pointer to the struct's type information
     */
    TypeInfo info() const noexcept { return m_runtime->ptr_type(raw()); }

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

    static StructRef from(type ptr, const Runtime& runtime) noexcept {
        return StructRef(runtime, ptr);
    }

    static type to(StructRef value) noexcept { return value.raw(); }

    static StructRef copy_from(const type* ptr, const Runtime& runtime,
                               const TypeInfo& type_info) noexcept {
        MunTypeInfoData type_data;
        auto err = mun_type_info_data(type_info.handle(), &type_data);
        assert(err._0 == nullptr);

        // Safety: `mun_type_info_data` is guaranteed to return a value for `StructRef`s.
        MunStructMemoryKind memory_kind;
        err = mun_struct_info_memory_kind(type_data.struct_, &memory_kind);
        assert(err._0 == nullptr);

        MunGcPtr gc_handle;
        if (memory_kind == MunStructMemoryKind::Value) {
            // Create a new managed object
            gc_handle = *runtime.gc_alloc(type_info);

            // Copy the old object into the new object
            std::memcpy(*gc_handle, ptr, type_info.size());
        } else {
            // For a gc struct, `ptr` points to a `MunGcPtr`.
            gc_handle = *ptr;
        }

        return StructRef(runtime, gc_handle);
    }

    static void move_to(type value, type* ptr, const TypeInfo& type_info) noexcept {
        MunTypeInfoData type_data;
        auto err = mun_type_info_data(type_info.handle(), &type_data);
        assert(err._0 == nullptr);

        // Safety: `mun_type_info_data` is guaranteed to return a value for `StructRef`s.
        MunStructMemoryKind memory_kind;
        err = mun_struct_info_memory_kind(type_data.struct_, &memory_kind);
        assert(err._0 == nullptr);

        if (memory_kind == MunStructMemoryKind::Value) {
            // Copy the `struct(value)` into the old object
            std::memcpy(ptr, *value, type_info.size());
        } else {
            *ptr = std::move(value);
        }
    }

    static StructRef swap_at(type value, type* ptr, const Runtime& runtime,
                             const TypeInfo& type_info) noexcept {
        MunTypeInfoData type_data;
        auto err = mun_type_info_data(type_info.handle(), &type_data);
        assert(err._0 == nullptr);

        // Safety: `mun_type_info_data` is guaranteed to return a value for `StructRef`s.
        MunStructMemoryKind memory_kind;
        err = mun_struct_info_memory_kind(type_data.struct_, &memory_kind);
        assert(err._0 == nullptr);

        MunGcPtr gc_handle;
        if (memory_kind == MunStructMemoryKind::Value) {
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

        return StructRef(runtime, gc_handle);
    }
};
}  // namespace mun

#include "mun/reflection.h"

namespace mun {

template <>
struct ArgumentReflection<StructRef> {
    static TypeInfo type_info(const StructRef& ref) noexcept { return ref.info(); }
};

template <>
struct ReturnTypeReflection<StructRef> {
    static bool accepts_type(const TypeInfo& ty) noexcept { return ty.is_struct(); }
    static std::string type_hint() {
        using namespace std::string_literals;
        return "struct"s;
    }
};

template <typename T>
std::optional<T> StructRef::get(std::string_view field_name) const noexcept {
    const auto type_info = info();

    MunTypeInfoData type_data;
    auto err = mun_type_info_data(type_info.handle(), &type_data);
    assert(err._0 == nullptr);

    // Safety: `mun_type_info_data` is guaranteed to return a value for `StructRef`s.
    if (const auto field_info =
            details::find_field(type_info.name(), type_data.struct_, field_name);
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
    const auto type_info = info();

    MunTypeInfoData type_data;
    auto err = mun_type_info_data(type_info.handle(), &type_data);
    assert(err._0 == nullptr);

    // Safety: `mun_type_info_data` is guaranteed to return a value for `StructRef`s.
    if (const auto field_info =
            details::find_field(type_info.name(), type_data.struct_, field_name);
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
    const auto type_info = info();

    MunTypeInfoData type_data;
    auto err = mun_type_info_data(type_info.handle(), &type_data);
    assert(err._0 == nullptr);

    // Safety: `mun_type_info_data` is guaranteed to return a value for `StructRef`s.
    if (const auto field_info =
            details::find_field(type_info.name(), type_data.struct_, field_name);
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
