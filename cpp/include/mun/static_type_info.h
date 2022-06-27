#ifndef MUN_STATIC_TYPE_INFO_H_
#define MUN_STATIC_TYPE_INFO_H_

#include "mun/runtime_capi.h"

namespace mun {
namespace details {
constexpr MunTypeId type_id(const char* type_name) noexcept {
    const auto hash = md5::compute(type_name);
    const MunGuid guid{
        hash[0], hash[1], hash[2],  hash[3],  hash[4],  hash[5],  hash[6],  hash[7],
        hash[8], hash[9], hash[10], hash[11], hash[12], hash[13], hash[14], hash[15],
    };
    MunTypeId id { Concrete };
    id.concrete = guid;
    return id;
}
}  // namespace details

template <typename T>
struct StaticTypeInfo;

#define IMPL_PRIMITIVE_TYPE_INFO(ty, name_literal)                                            \
    template <>                                                                               \
    struct StaticTypeInfo<ty> {                                                               \
        static constexpr MunTypeId id() noexcept {                                            \
            return details::type_id(StaticTypeInfo<ty>::name());                              \
        }                                                                                     \
        static constexpr const char* name() noexcept { return name_literal; }                 \
        static constexpr size_t size() noexcept { return sizeof(ty); }                        \
        static constexpr size_t alignment() noexcept { return std::alignment_of<ty>::value; } \
    };

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

template <>
struct StaticTypeInfo<void> {
    static constexpr MunTypeId id() noexcept {
        return details::type_id(StaticTypeInfo<void>::name());
    }
    static constexpr const char* name() noexcept { return "core::()"; }
    static constexpr size_t size() noexcept { return 0; }
    static constexpr size_t alignment() noexcept { return 1; }
};

}  // namespace mun

#endif  // MUN_STATIC_TYPE_INFO_H_
