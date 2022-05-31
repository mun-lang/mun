#ifndef MUN_ERROR_H_
#define MUN_ERROR_H_

#include "mun/runtime_capi.h"

namespace mun {
/** A wrapper around a `MunErrorHandle`.
 *
 * Frees the corresponding error object on destruction, if it exists.
 */
class Error {
public:
    /** Default constructs an error. */
    Error() noexcept : m_handle{nullptr} {}

    /** Constructs an error from a `MunErrorHandle`.
     *
     * \param handle an error handle
     */
    Error(MunErrorHandle handle) noexcept : m_handle(handle) {}

    /** Move constructs an error.
     *
     * \param other an rvalue reference to an error
     */
    Error(Error&& other) noexcept : m_handle(other.m_handle) { other.m_handle._0 = nullptr; }

    /** Move assigns an error.
     *
     * \param other an rvalue reference to an error
     */
    Error& operator=(Error&& other) noexcept {
        m_handle = other.m_handle;
        other.m_handle._0 = nullptr;
        return *this;
    }

    /** Destructs the error. */
    ~Error() noexcept { mun_error_destroy(m_handle); }

    /** Returns the error message, if it exists, otherwise returns a nullptr.
     *
     * The message is UTF-8 encoded.
     */
    const char* message() noexcept { return m_handle._0; }

    /** Retrieves whether an error occurred */
    operator bool() const noexcept { return m_handle._0 != nullptr; }

private:
    MunErrorHandle m_handle;
};
}  // namespace mun

#endif
