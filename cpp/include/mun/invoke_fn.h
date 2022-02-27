#ifndef MUN_INVOKE_FN_H_
#define MUN_INVOKE_FN_H_

#include <iostream>
#include <optional>
#include <string_view>

#include "mun/invoke_result.h"
#include "mun/marshal.h"
#include "mun/reflection.h"
#include "mun/runtime.h"
#include "mun/util.h"

namespace mun {
/** Invokes the runtime function corresponding to `fn_name` with arguments
 * `args`.
 *
 * \param runtime the runtime
 * \param fn_name the name of the desired function
 * \param args zero or more arguments to supply to the function invocation
 * \return an invocation result
 */
template <typename Output, typename... Args>
InvokeResult<Output, Args...> invoke_fn(Runtime& runtime, std::string_view fn_name,
                                        Args... args) noexcept {
    auto make_error = [](Runtime& runtime, std::string_view fn_name, Args... args) {
        return InvokeResult<Output, Args...>(
            [&runtime, fn_name](Args... fn_args) {
                return invoke_fn<Output, Args...>(runtime, fn_name, fn_args...);
            },
            [&runtime]() { return runtime.update(); }, std::move(args)...);
    };

    Error error;
    constexpr auto NUM_ARGS = sizeof...(Args);
    if (auto fn_info = runtime.find_function_definition(fn_name, &error); error) {
        std::cerr << "Failed to retrieve function info due to error: " << error.message()
                  << std::endl;
    } else if (!fn_info) {
        std::cerr << "Failed to obtain function '" << fn_name << "'" << std::endl;
    } else {
        const auto& prototype = fn_info->prototype;
        const auto& signature = prototype.signature;
        if (signature.num_arg_types != NUM_ARGS) {
            std::cerr << "Invalid number of arguments. Expected: "
                      << std::to_string(signature.num_arg_types)
                      << ". Found: " << std::to_string(NUM_ARGS) << "." << std::endl;

            return make_error(runtime, fn_name, args...);
        }

        if constexpr (NUM_ARGS > 0) {
            const MunTypeInfo* const* arg_ptr = signature.arg_types;
            const std::optional<std::pair<const char*, const char*>> return_type_diffs[] = {
                reflection::equals_argument_type(**(arg_ptr++), args)...};

            for (size_t idx = 0; idx < NUM_ARGS; ++idx) {
                if (auto diff = return_type_diffs[idx]) {
                    const auto& [expected, found] = *diff;
                    std::cerr << "Invalid argument type at index " << idx
                              << ". Expected: " << expected << ". Found: " << found << "."
                              << std::endl;

                    return make_error(runtime, fn_name, args...);
                }
            }
        }

        if (signature.return_type) {
            const auto& return_type = signature.return_type;
            if (auto diff = reflection::equals_return_type<Output>(*return_type)) {
                const auto& [expected, found] = *diff;
                std::cerr << "Invalid return type. Expected: " << expected << ". Found: " << found
                          << "." << std::endl;

                return make_error(runtime, fn_name, args...);
            }
        } else if (!reflection::equal_types<void, Output>()) {
            std::cerr << "Invalid return type. Expected: "
                      << ReturnTypeReflection<void>::type_name()
                      << ". Found: " << ReturnTypeReflection<Output>::type_name() << "."
                      << std::endl;

            return make_error(runtime, fn_name, args...);
        }

        auto fn = reinterpret_cast<typename Marshal<Output>::type(MUN_CALLTYPE*)(
            typename Marshal<Args>::type...)>(const_cast<void*>(fn_info->fn_ptr));
        if constexpr (std::is_same_v<Output, void>) {
            fn(Marshal<Args>::to(args)...);
            return InvokeResult<Output, Args...>(std::monostate{});
        } else {
            return InvokeResult<Output, Args...>(
                Marshal<Output>::from(fn(Marshal<Args>::to(args)...), runtime));
        }
    }

    return make_error(runtime, fn_name, args...);
}
}  // namespace mun

#endif
