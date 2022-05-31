#ifndef MUN_MARSHAL_H_
#define MUN_MARSHAL_H_

#include "mun/runtime.h"

namespace mun {
template <typename T>
struct Marshal;

#define IMPL_PRIMITIVE_TYPE_MARSHAL(ty)                                                            \
    template <>                                                                                    \
    struct Marshal<ty> {                                                                           \
        using type = ty;                                                                           \
                                                                                                   \
        static type from(type value, const Runtime&) noexcept { return value; }                    \
                                                                                                   \
        static type to(type value) noexcept { return value; }                                      \
                                                                                                   \
        static type copy_from(const type* value, const Runtime&, const TypeInfo&) noexcept {       \
            return *value;                                                                         \
        }                                                                                          \
                                                                                                   \
        static void move_to(type value, type* ptr, const TypeInfo&) noexcept {                     \
            *ptr = std::move(value);                                                               \
        }                                                                                          \
                                                                                                   \
        static type swap_at(type value, type* ptr, const Runtime&, const TypeInfo&) noexcept {     \
            std::swap(value, *ptr);                                                                \
            return std::move(value);                                                               \
        }                                                                                          \
    };

// TODO: Add support for 128-bit integers
IMPL_PRIMITIVE_TYPE_MARSHAL(bool);
IMPL_PRIMITIVE_TYPE_MARSHAL(float);
IMPL_PRIMITIVE_TYPE_MARSHAL(double);
IMPL_PRIMITIVE_TYPE_MARSHAL(int8_t);
IMPL_PRIMITIVE_TYPE_MARSHAL(int16_t);
IMPL_PRIMITIVE_TYPE_MARSHAL(int32_t);
IMPL_PRIMITIVE_TYPE_MARSHAL(int64_t);
// IMPL_PRIMITIVE_TYPE_MARSHAL(int128_t);
IMPL_PRIMITIVE_TYPE_MARSHAL(uint8_t);
IMPL_PRIMITIVE_TYPE_MARSHAL(uint16_t);
IMPL_PRIMITIVE_TYPE_MARSHAL(uint32_t);
IMPL_PRIMITIVE_TYPE_MARSHAL(uint64_t);
// IMPL_PRIMITIVE_TYPE_MARSHAL(uint128_t);

template <>
struct Marshal<void> {
    // The void type doesn't need to marshal anything. It merely needs to specify the type
    using type = void;
};

}  // namespace mun

#endif
