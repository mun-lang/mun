#ifndef MUN_REFLECTION_H_
#define MUN_REFLECTION_H_

#include <algorithm>
#include <cstdint>
#include <iterator>
#include <optional>

#include "mun/runtime_capi.h"
#include "mun/type_info.h"

namespace mun {
constexpr inline bool operator==(const MunGuid& lhs, const MunGuid& rhs) noexcept {
    for (auto idx = 0; idx < 16; ++idx) {
        if (lhs._0[idx] != rhs._0[idx]) {
            return false;
        }
    }
    return true;
}

constexpr inline bool operator!=(const MunGuid& lhs, const MunGuid& rhs) noexcept {
    return !(lhs == rhs);
}

template <typename T>
struct ArgumentReflection {
    static constexpr const char* type_name(const T&) noexcept {
        return StaticTypeInfo<T>::Type.name;
    }
    static constexpr MunGuid type_guid(const T&) noexcept { return StaticTypeInfo<T>::Type.guid; }
};

template <typename T>
struct ReturnTypeReflection {
    static constexpr const char* type_name() noexcept { return StaticTypeInfo<T>::Type.name; }
    static constexpr MunGuid type_guid() noexcept { return StaticTypeInfo<T>::Type.guid; }
};

template <>
struct ReturnTypeReflection<void> {
    static constexpr const char* type_name() noexcept { return "core::empty"; }
    static constexpr MunGuid type_guid() noexcept { return details::type_guid(type_name()); }
};

namespace reflection {
template <typename T, typename U>
constexpr bool equal_types() noexcept {
    return ReturnTypeReflection<T>::type_guid() == ReturnTypeReflection<U>::type_guid();
}

template <typename Arg>
inline std::optional<std::pair<const char*, const char*>> equals_argument_type(
    const MunTypeInfo& type_info, const Arg& arg) noexcept {
    if (type_info.guid == ArgumentReflection<Arg>::type_guid(arg)) {
        return std::nullopt;
    } else {
        const auto expected_name = ArgumentReflection<Arg>::type_name(arg);
        return std::make_pair(type_info.name, expected_name);
    }
}

template <typename T>
inline std::optional<std::pair<const char*, const char*>> equals_return_type(
    const MunTypeInfo& type_info) noexcept;

}  // namespace reflection

}  // namespace mun

#include "struct_ref.h"

namespace mun {
namespace reflection {
template <typename T>
std::optional<std::pair<const char*, const char*>> equals_return_type(
    const MunTypeInfo& type_info) noexcept {
    if (type_info.data.tag == MunTypeInfoData_Tag::Primitive) {
        if (type_info.guid != ReturnTypeReflection<T>::type_guid()) {
            return std::make_pair(type_info.name, ReturnTypeReflection<T>::type_name());
        }
    } else if (!reflection::equal_types<StructRef, T>()) {
        return std::make_pair(type_info.name, ReturnTypeReflection<T>::type_name());
    }

    return std::nullopt;
}
}  // namespace reflection
}  // namespace mun

#endif
