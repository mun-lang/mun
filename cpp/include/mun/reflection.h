#ifndef MUN_REFLECTION_H_
#define MUN_REFLECTION_H_

#include <algorithm>
#include <cstdint>
#include <iterator>
#include <optional>

#include "mun/runtime_capi.h"
#include "mun/type.h"

namespace mun {

template <typename T>
struct ArgumentReflection {
    static Type type_info(const T&) noexcept { return StaticTypeInfo<T>::type_info(); }
};

template <typename T>
struct ReturnTypeReflection {
    static bool accepts_type(const Type& ty) noexcept {
        return StaticTypeInfo<T>::type_info() == ty;
    }
    static std::string type_hint() { return StaticTypeInfo<T>::type_info().name(); }
};

template <>
struct ReturnTypeReflection<void> {
    static bool accepts_type(const Type& ty) noexcept {
        return StaticTypeInfo<void>::type_info() == ty ||
               StaticTypeInfo<std::tuple<>>::type_info() == ty;
    }
    static std::string type_hint() { return StaticTypeInfo<void>::type_info().name(); }
};

namespace reflection {
template <typename Arg>
inline std::optional<std::pair<std::string, std::string>> equals_argument_type(
    const Type& type_info, const Arg& arg) noexcept {
    auto arg_type_info = ArgumentReflection<Arg>::type_info(arg);
    if (arg_type_info == type_info) {
        return std::nullopt;
    } else {
        const auto expected_name = arg_type_info.name();
        return std::make_pair(std::string(type_info.name()), std::string(expected_name));
    }
}
template <typename T>
inline std::optional<std::pair<std::string, std::string>> equals_return_type(
    const Type& type_info) noexcept {
    if (!ReturnTypeReflection<T>::accepts_type(type_info)) {
        return std::make_pair(std::string(type_info.name()),
                              std::string(ReturnTypeReflection<T>::type_hint()));
    }
    return std::nullopt;
}
}  // namespace reflection

}  // namespace mun

#endif
