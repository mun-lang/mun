#ifndef MUN_TYPE_INFO_H
#define MUN_TYPE_INFO_H

#include <optional>

#include "mun/md5.h"
#include "mun/runtime_capi.h"

namespace mun {
namespace details {
constexpr MunGuid type_guid(const char* type_name) noexcept {
    const auto hash = md5::compute(type_name);
    return MunGuid{
        hash[0], hash[1], hash[2],  hash[3],  hash[4],  hash[5],  hash[6],  hash[7],
        hash[8], hash[9], hash[10], hash[11], hash[12], hash[13], hash[14], hash[15],
    };
}
}  // namespace details

template <typename T>
struct TypeInfo;

#define IMPL_PRIMITIVE_TYPE_INFO(ty, name_literal) \
    template <>                                    \
    struct TypeInfo<ty> {                          \
        static constexpr MunTypeInfo Type{         \
            details::type_guid(name_literal),      \
            name_literal,                          \
            sizeof(ty),                            \
            std::alignment_of<ty>::value,          \
            MunTypeInfoData_Tag::Primitive,        \
        };                                         \
    }

IMPL_PRIMITIVE_TYPE_INFO(bool, "core::bool");
IMPL_PRIMITIVE_TYPE_INFO(float, "core::f32");
IMPL_PRIMITIVE_TYPE_INFO(double, "core::f64");
IMPL_PRIMITIVE_TYPE_INFO(int8_t, "core::i8");
IMPL_PRIMITIVE_TYPE_INFO(int16_t, "core::i16");
IMPL_PRIMITIVE_TYPE_INFO(int32_t, "core::i32");
IMPL_PRIMITIVE_TYPE_INFO(int64_t, "core::i64");
// IMPL_PRIMITIVE_TYPE_REFLECTION(int128_t, "core::i128");
IMPL_PRIMITIVE_TYPE_INFO(uint8_t, "core::u8");
IMPL_PRIMITIVE_TYPE_INFO(uint16_t, "core::u16");
IMPL_PRIMITIVE_TYPE_INFO(uint32_t, "core::u32");
IMPL_PRIMITIVE_TYPE_INFO(uint64_t, "core::u64");
// IMPL_PRIMITIVE_TYPE_REFLECTION(uint128_t, "core::u128");

/**
 * Returns the return type `MunTypeInfo` corresponding to type T, or none if the return type is
 * void.
 */
template <typename T>
std::optional<MunTypeInfo const*> return_type_info() {
    return &TypeInfo<T>::Type;
}

/**
 * Returns the return type `MunTypeInfo` corresponding to type T, or none if the return type is
 * void.
 */
template <>
inline std::optional<MunTypeInfo const*> return_type_info<void>() {
    return std::nullopt;
}

/**
 * Returns the argument type `MunTypeInfo` corresponding to type T.
 */
template <typename T>
MunTypeInfo const* arg_type_info() {
    return &TypeInfo<T>::Type;
}

}  // namespace mun

#endif  // MUN_TYPE_INFO_H
