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
    if (auto fn_info = runtime.find_function_info(fn_name, &error); error.is_error()) {
        std::cerr << "Failed to retrieve function info due to error: " << error.message().value()
                  << std::endl;
    } else if (!fn_info) {
        std::cerr << "Failed to obtain function '" << fn_name << "'" << std::endl;
    } else {
        const auto arg_types = fn_info->argument_types();
        if (arg_types.size() != NUM_ARGS) {
            std::cerr << "Invalid number of arguments. Expected: "
                      << std::to_string(arg_types.size()) << ". Found: " << std::to_string(NUM_ARGS)
                      << "." << std::endl;

            return make_error(runtime, fn_name, args...);
        }

        if constexpr (NUM_ARGS > 0) {
            auto arg_it = arg_types.begin();
            const std::optional<std::pair<std::string, std::string>> return_type_diffs[] = {
                reflection::equals_argument_type(*(arg_it++), args)...};

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

        if (auto diff = reflection::equals_return_type<Output>(fn_info->return_type())) {
            const auto& [expected, found] = *diff;
            std::cerr << "Invalid return type. Expected: " << expected << ". Found: " << found
                      << "." << std::endl;

            return make_error(runtime, fn_name, args...);
        }

        auto fn = reinterpret_cast<typename Marshal<Output>::type(MUN_CALLTYPE*)(
            typename Marshal<Args>::type...)>(const_cast<void*>(fn_info->function_pointer()));
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
