#ifndef MUN_FUNCTION_INFO_H_
#define MUN_FUNCTION_INFO_H_

#include <cassert>

#include "mun/runtime_capi.h"
#include "mun/type.h"

namespace mun {
/**
 * @brief A wrapper around a Mun function handle.
 */
class Function {
public:
    /**
     * @brief Constructs type information from a MunFunction handle.
     *
     * This function assumes that ownership is transferred.
     */
    constexpr explicit Function(MunFunction handle) noexcept : m_handle(handle) {}

    Function(const Function& other) : m_handle(other.m_handle) {
        MUN_ASSERT(mun_function_add_reference(m_handle));
    }
    constexpr Function(Function&& other) : m_handle(other.m_handle) { other.m_handle._0 = nullptr; }

    ~Function() {
        if (m_handle._0 != nullptr) {
            MUN_ASSERT(mun_function_release(m_handle));
            m_handle._0 = nullptr;
        }
    }

    Function& operator=(const Function& other) {
        if (other.m_handle._0 != nullptr) {
            MUN_ASSERT(mun_function_add_reference(other.m_handle));
        }
        if (m_handle._0 != nullptr) {
            MUN_ASSERT(mun_function_release(m_handle));
        }
        m_handle = other.m_handle;
    }

    /**
     * @brief Retrieves the function's name.
     */
    [[nodiscard]] std::string name() const noexcept {
        const char* name;
        MUN_ASSERT(mun_function_name(m_handle, &name));
        std::string name_str(name);
        mun_string_destroy(name);
        return name_str;
    }

    /**
     * @brief Retrieves the function's argument types.
     */
    [[nodiscard]] TypeArray argument_types() const noexcept {
        MunTypes types;
        MUN_ASSERT(mun_function_argument_types(m_handle, &types));
        return TypeArray(types);
    }

    /**
     * @brief Retrieves the function's return type.
     */
    [[nodiscard]] Type return_type() const noexcept {
        MunType handle;
        MUN_ASSERT(mun_function_return_type(m_handle, &handle));
        return Type(handle);
    }

    /**
     * @brief Retrieves the function's pointer.
     */
    [[nodiscard]] const void* function_pointer() const noexcept {
        const void* ptr;
        MUN_ASSERT(mun_function_fn_ptr(m_handle, &ptr));
        return ptr;
    }

private:
    MunFunction m_handle;
};
}  // namespace mun

#endif  // MUN_FUNCTION_INFO_H_
