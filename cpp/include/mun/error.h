#ifndef MUN_ERROR_H_
#define MUN_ERROR_H_

#include <optional>
#include <string_view>

#include "mun/runtime_capi.h"

namespace mun {
/** A wrapper around a `MunErrorHandle`.
 *
 * Frees the corresponding error object on destruction, if it exists.
 */
class Error {
public:
    /** Default constructs an error. */
    constexpr Error() noexcept : m_handle{nullptr} {}

    /** Constructs an error from a `MunErrorHandle`.
     *
     * \param handle an error type_handle
     */
    constexpr Error(MunErrorHandle handle) noexcept : m_handle(handle) {}

    /** Move constructs an error.
     *
     * \param other an rvalue reference to an error
     */
    constexpr Error(Error&& other) noexcept : m_handle(other.m_handle) {
        other.m_handle.error_string = nullptr;
    }

    /** Move assigns an error.
     *
     * \param other an rvalue reference to an error
     */
    constexpr Error& operator=(Error&& other) noexcept {
        m_handle = other.m_handle;
        other.m_handle.error_string = nullptr;
        return *this;
    }

    /** Destructs the error. */
    ~Error() noexcept { mun_error_destroy(m_handle); }

    /**
     * Returns the error message, if it exists, otherwise returns `std::nullopt`. The message is
     * UTF-8 encoded.
     */
    [[nodiscard]] constexpr std::optional<std::string_view> message() const noexcept {
        if (m_handle.error_string != nullptr) return std::make_optional(m_handle.error_string);
        return std::nullopt;
    }

    /**
     * Returns true if this instance contains an error message. False if the result is Ok.
     */
    [[nodiscard]] constexpr inline bool is_error() const {
        return m_handle.error_string != nullptr;
    }

    /**
     * Returns true if this instance represents an OK result.
     */
    [[nodiscard]] constexpr inline bool is_ok() const { return m_handle.error_string == nullptr; }

private:
    MunErrorHandle m_handle;
};
}  // namespace mun

#ifdef NDEBUG
#define MUN_ASSERT(expr) ((void)expr)
#else
#define MUN_ASSERT(expr)                                                           \
    do {                                                                           \
        ::mun::Error err(expr);                                                    \
        if (auto message = err.message(); message) {                               \
            std::cerr << "Assertion Error: " #expr " failed: " << message.value(); \
            std::abort();                                                          \
        }                                                                          \
    } while (0)
#endif

#endif
