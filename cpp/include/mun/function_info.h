#ifndef MUN_FUNCTION_INFO_H_
#define MUN_FUNCTION_INFO_H_

#include <cassert>

#include "mun/runtime_capi.h"
#include "mun/type_info.h"

namespace mun {
/**
 * @brief A wrapper around a Mun function information handle.
 */
class FunctionInfo {
public:
    /**
     * @brief Constructs type information from an instantiated `MunFunctionInfoHandle`.
     */
    FunctionInfo(MunFunctionInfoHandle handle) noexcept : m_handle(handle) {}

    /**
     * @brief Retrieves the function's name.
     */
    std::string_view name() const noexcept {
        const auto ptr = mun_function_info_name(m_handle);
        assert(ptr);
        return ptr;
    }

    /**
     * @brief Retrieves the function's argument types.
     */
    TypeInfoSpan argument_types() const noexcept {
        MunTypeInfoSpan span;
        const auto error_handle = mun_function_info_argument_types(m_handle, &span);
        assert(error_handle._0 == nullptr);
        return TypeInfoSpan(span);
    }

    /**
     * @brief Retrieves the function's return type.
     */
    TypeInfo return_type() const noexcept {
        const auto handle = mun_function_info_return_type(m_handle);
        assert(handle._0);
        return TypeInfo(handle);
    }

    /**
     * @brief Retrieves the function's pointer.
     */
    const void* function_pointer() const noexcept {
        const void* fn_ptr = mun_function_info_fn_ptr(m_handle);
        assert(fn_ptr);
        return fn_ptr;
    }

private:
    MunFunctionInfoHandle m_handle;
};
}  // namespace mun

#endif  // MUN_FUNCTION_INFO_H_
