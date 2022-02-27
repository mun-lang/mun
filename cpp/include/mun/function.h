#ifndef MUN_FUNCTION_H_
#define MUN_FUNCTION_H_

#include <string>
#include <vector>

#include "mun/runtime_capi.h"
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
          arg_types({arg_type_info<TArgs>()...}),
          ret_type(return_type_info<TRet>()),
          fn_ptr(reinterpret_cast<const void*>(fn_ptr)) {}

    RuntimeFunction(const RuntimeFunction&) = default;
    RuntimeFunction(RuntimeFunction&&) = default;
    RuntimeFunction& operator=(const RuntimeFunction&) = default;
    RuntimeFunction& operator=(RuntimeFunction&&) = default;

    std::string name;
    std::vector<MunTypeInfo const*> arg_types;
    std::optional<MunTypeInfo const*> ret_type;
    const void* fn_ptr;
};
}  // namespace mun

#endif
