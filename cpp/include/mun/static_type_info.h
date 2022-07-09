#ifndef MUN_STATIC_TYPE_INFO_H_
#define MUN_STATIC_TYPE_INFO_H_

#include "mun/runtime_capi.h"
#include "mun/type_info.h"

namespace mun {

template <typename T>
struct StaticTypeInfo;

#define IMPL_PRIMITIVE_TYPE_INFO(ty, mun_primitive_type)                                       \
    template <>                                                                                \
    struct StaticTypeInfo<ty> {                                                                \
        static const TypeInfo& type_info() {                                                   \
            static TypeInfo TYPE_INFO = TypeInfo(mun_type_info_primitive(mun_primitive_type)); \
            return TYPE_INFO;                                                                  \
        }                                                                                      \
    };

IMPL_PRIMITIVE_TYPE_INFO(bool, MunPrimitiveType_Bool);
IMPL_PRIMITIVE_TYPE_INFO(float, MunPrimitiveType_F32);
IMPL_PRIMITIVE_TYPE_INFO(double, MunPrimitiveType_F64);
IMPL_PRIMITIVE_TYPE_INFO(int8_t, MunPrimitiveType_I8);
IMPL_PRIMITIVE_TYPE_INFO(int16_t, MunPrimitiveType_I16);
IMPL_PRIMITIVE_TYPE_INFO(int32_t, MunPrimitiveType_I32);
IMPL_PRIMITIVE_TYPE_INFO(int64_t, MunPrimitiveType_I64);
// IMPL_PRIMITIVE_TYPE_REFLECTION(int128_t, MunPrimitiveType_I128);
IMPL_PRIMITIVE_TYPE_INFO(uint8_t, MunPrimitiveType_U8);
IMPL_PRIMITIVE_TYPE_INFO(uint16_t, MunPrimitiveType_U16);
IMPL_PRIMITIVE_TYPE_INFO(uint32_t, MunPrimitiveType_U32);
IMPL_PRIMITIVE_TYPE_INFO(uint64_t, MunPrimitiveType_U64);
// IMPL_PRIMITIVE_TYPE_REFLECTION(uint128_t, MunPrimitiveType_U128);
IMPL_PRIMITIVE_TYPE_INFO(void, MunPrimitiveType_Void);
IMPL_PRIMITIVE_TYPE_INFO(std::tuple<>, MunPrimitiveType_Empty);

}  // namespace mun

#endif  // MUN_STATIC_TYPE_INFO_H_
