#ifndef MUN_FUNCTION_H_
#define MUN_FUNCTION_H_

#include <string>
#include <vector>

#include "mun/runtime_capi.h"
#include "mun/static_type_info.h"
#include "mun/type_info.h"
#include "mun/util.h"

namespace mun {
/**
 * A wrapper around a C function with type information.
 */
struct RuntimeFunction {
    /**
     * Constructs a `RuntimeFunction` from a generic function pointer and a name.
     * \param name The name of the function used when added to the runtime
     * \param fn_ptr The function pointer to add
     */
    template <typename... TArgs>
    RuntimeFunction(std::string_view name, void(MUN_CALLTYPE* fn_ptr)(TArgs...))
        : name(name),
          arg_types({StaticTypeInfo<TArgs>::type_info().handle()...}),
          ret_type(StaticTypeInfo<std::tuple<>>::type_info().handle()),
          fn_ptr(reinterpret_cast<const void*>(fn_ptr)) {
        for (const auto& arg_type : arg_types) {
            mun_type_info_increment_strong_count(arg_type);
        }
        mun_type_info_increment_strong_count(ret_type);
    }

    /**
     * Constructs a `RuntimeFunction` from a generic function pointer and a name.
     * \param name The name of the function used when added to the runtime
     * \param fn_ptr The function pointer to add
     */
    template <typename TRet, typename... TArgs>
    RuntimeFunction(std::string_view name, TRet(MUN_CALLTYPE* fn_ptr)(TArgs...))
        : name(name),
          arg_types({StaticTypeInfo<TArgs>::type_info().handle()...}),
          ret_type(StaticTypeInfo<TRet>::type_info().handle()),
          fn_ptr(reinterpret_cast<const void*>(fn_ptr)) {
        for (const auto& arg_type : arg_types) {
            mun_type_info_increment_strong_count(arg_type);
        }
        mun_type_info_increment_strong_count(ret_type);
    }

    ~RuntimeFunction() {
        for (const auto& arg_type : arg_types) {
            mun_type_info_decrement_strong_count(arg_type);
        }
        mun_type_info_decrement_strong_count(ret_type);
    }

    RuntimeFunction(const RuntimeFunction&) = default;
    RuntimeFunction(RuntimeFunction&&) = default;
    RuntimeFunction& operator=(const RuntimeFunction&) = default;
    RuntimeFunction& operator=(RuntimeFunction&&) = default;

    std::string name;
    std::vector<MunTypeInfoHandle> arg_types;
    MunTypeInfoHandle ret_type;
    const void* fn_ptr;
};
}  // namespace mun

#endif
