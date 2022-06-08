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
    template <typename TRet, typename... TArgs>
    RuntimeFunction(std::string_view name, TRet(MUN_CALLTYPE* fn_ptr)(TArgs...))
        : name(name),
          arg_types({StaticTypeInfo<TArgs>::id()...}),
          ret_type(StaticTypeInfo<TRet>::id()),
          fn_ptr(reinterpret_cast<const void*>(fn_ptr)) {}

    RuntimeFunction(const RuntimeFunction&) = default;
    RuntimeFunction(RuntimeFunction&&) = default;
    RuntimeFunction& operator=(const RuntimeFunction&) = default;
    RuntimeFunction& operator=(RuntimeFunction&&) = default;

    std::string name;
    std::vector<MunTypeId> arg_types;
    MunTypeId ret_type;
    const void* fn_ptr;
};
}  // namespace mun

#endif
