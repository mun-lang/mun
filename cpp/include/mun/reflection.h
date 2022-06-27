#ifndef MUN_REFLECTION_H_
#define MUN_REFLECTION_H_

#include <algorithm>
#include <cstdint>
#include <iterator>
#include <optional>

#include "mun/runtime_capi.h"
#include "mun/type_info.h"

namespace mun {
constexpr inline bool operator==(const MunTypeId& lhs, const MunTypeId& rhs) noexcept {
    if (lhs.tag != rhs.tag) {
        return false;
    }

    if (lhs.tag == MunTypeId_Tag::Concrete) {
        for (auto idx = 0; idx < 16; ++idx) {
            if (lhs.concrete._0[idx] != rhs.concrete._0[idx]) {
                return false;
            }
        }
    }
    return true;
}

constexpr inline bool operator!=(const MunTypeId& lhs, const MunTypeId& rhs) noexcept {
    return !(lhs == rhs);
}

template <typename T>
struct ArgumentReflection {
    static constexpr const char* type_name(const T&) noexcept { return StaticTypeInfo<T>::name(); }
    static constexpr MunTypeId type_id(const T&) noexcept { return StaticTypeInfo<T>::id(); }
};

template <typename T>
struct ReturnTypeReflection {
    static constexpr const char* type_name() noexcept { return StaticTypeInfo<T>::name(); }
    static constexpr MunTypeId type_id() noexcept { return StaticTypeInfo<T>::id(); }
};

namespace reflection {
template <typename T, typename U>
constexpr bool equal_types() noexcept {
    return ReturnTypeReflection<T>::type_id() == ReturnTypeReflection<U>::type_id();
}

template <typename Arg>
inline std::optional<std::pair<const char*, const char*>> equals_argument_type(
    const TypeInfo& type_info, const Arg& arg) noexcept {
    if (type_info.id() == ArgumentReflection<Arg>::type_id(arg)) {
        return std::nullopt;
    } else {
        const auto expected_name = ArgumentReflection<Arg>::type_name(arg);
        return std::make_pair(type_info.name().data(), expected_name);
    }
}

template <typename T>
inline std::optional<std::pair<const char*, const char*>> equals_return_type(
    const TypeInfo& type_info) noexcept;

}  // namespace reflection

}  // namespace mun

#include "struct_ref.h"

namespace mun {
namespace reflection {
template <typename T>
std::optional<std::pair<const char*, const char*>> equals_return_type(
    const TypeInfo& type_info) noexcept {
    if (type_info.data().tag == MunTypeInfoData_Tag::MunTypeInfoData_Primitive) {
        if (type_info.id() != ReturnTypeReflection<T>::type_id()) {
            return std::make_pair(type_info.name().data(), ReturnTypeReflection<T>::type_name());
        }
    } else if (!reflection::equal_types<StructRef, T>()) {
        return std::make_pair(type_info.name().data(), ReturnTypeReflection<T>::type_name());
    }

    return std::nullopt;
}
}  // namespace reflection
}  // namespace mun

#endif
