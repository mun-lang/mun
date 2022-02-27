#ifndef MUN_RESULT_H_
#define MUN_RESULT_H_

#include <chrono>
#include <functional>
#include <thread>
#include <variant>

namespace mun {
/** A variant that stores either the successful output of a function invocation
 * or the error state (i.e. callbacks and arguments) necessary to retry.
 */
template <typename Output, typename... Args>
class InvokeResult {
    using result_type = InvokeResult<Output, Args...>;
    using success_type = Output;
    using retry_fn_type = std::function<result_type(Args...)>;
    using update_fn_type = std::function<bool()>;
    using args_type = std::tuple<Args...>;
    using error_type = std::tuple<retry_fn_type, update_fn_type, args_type>;

   public:
    /** Constructs a result from the output of a successful function invocation.
     *
     * \param success the output of a successful function invocation
     */
    explicit InvokeResult(success_type &&success) : m_variant(std::move(success)) {}

    /** Constructs a result from a retry callback, its arguments, and an update
     * callback.
     *
     * \param retry_fn a retry callback
     * \param update_fn an update callback
     * \param args arguments for the retry_fn
     */
    explicit InvokeResult(retry_fn_type &&retry_fn, update_fn_type &&update_fn, Args &&... args)
        : m_variant(std::tuple(std::move(retry_fn), std::move(update_fn),
                               std::tuple(std::forward<Args>(args)...))) {}

    /** Retrieves whether the function invocation succeeded. */
    bool is_ok() noexcept { return std::holds_alternative<success_type>(m_variant); }

    /** Retrieves whether the function invocation failed. */
    bool is_err() noexcept { return std::holds_alternative<error_type>(m_variant); }

    /** Retrieves the output of a successful function invocation.
     *
     * BEWARE: Calling this on a failed invocation result will result in
     * undefined behavior.
     *
     * \return the output of a successful function invocation
     */
    success_type &&unwrap() noexcept { return std::move(std::get<0>(m_variant)); }

    /** Retrieves the error state of a failed function invocation.
     *
     * BEWARE: Calling this on a successful invocation result will result in
     * undefined behavior.
     *
     * \return the error state
     */
    error_type &&unwrap_err() noexcept { return std::move(std::get<1>(m_variant)); }

    /** Retries a failed function invocation and returns the result, or
     * immediately returns on prior success.
     *
     * This will wait on updates from the `update_fn` callback before retrying.
     *
     * \return the result of a retried function invocation
     */
    result_type &&retry() noexcept {
        if (is_err()) {
            return std::move(retry_impl(std::make_index_sequence<sizeof...(Args)>()));
        } else {
            return std::move(*this);
        }
    }

    /** Keeps retrying the function invocation until success, at which point it
     * returns the function's output.
     *
     * BEWARE: This function will not terminate until the function has been
     * successfully invoked.
     *
     * \return the output of a succesful function invocation
     */
    success_type &&wait() noexcept {
        while (is_err()) {
            *this = std::move(retry_impl(std::make_index_sequence<sizeof...(Args)>()));
        }
        return unwrap();
    }

   private:
    template <std::size_t... Is>
    result_type retry_impl(std::index_sequence<Is...>) {
        auto err = std::move(unwrap_err());
        auto &retry_fn = std::get<0>(err);
        auto &update_fn = std::get<1>(err);
        auto &args = std::get<2>(err);
        while (!update_fn()) {
            // TODO: Expose a Runtime API to wait for updates
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        return retry_fn(std::get<Is>(args)...);
    }

    std::variant<success_type, error_type> m_variant;
};

/** A variant that stores either the successful output of a function invocation
 * or the error state (i.e. callbacks and arguments) necessary to retry.
 */
template <typename... Args>
class InvokeResult<void, Args...> {
    using result_type = InvokeResult<void, Args...>;
    using retry_fn_type = std::function<result_type(Args...)>;
    using update_fn_type = std::function<bool()>;
    using args_type = std::tuple<Args...>;
    using error_type = std::tuple<retry_fn_type, update_fn_type, args_type>;

   public:
    /** Constructs a result from the output of a successful function invocation.
     *
     * \param success the output of a successful function invocation
     */
    explicit InvokeResult(std::monostate monostate) : m_variant(std::move(monostate)) {}

    /** Constructs a result from a retry callback, its arguments, and an update
     * callback.
     *
     * \param retry_fn a retry callback
     * \param update_fn an update callback
     * \param args arguments for the retry_fn
     */
    explicit InvokeResult(retry_fn_type &&retry_fn, update_fn_type &&update_fn, Args &&... args)
        : m_variant(std::tuple(std::move(retry_fn), std::move(update_fn),
                               std::tuple(std::forward<Args>(args)...))) {}

    /** Retrieves whether the function invocation succeeded. */
    bool is_ok() noexcept { return std::holds_alternative<std::monostate>(m_variant); }

    /** Retrieves whether the function invocation failed. */
    bool is_err() noexcept { return std::holds_alternative<error_type>(m_variant); }

    /** Retrieves the output of a successful function invocation.
     *
     * BEWARE: Calling this on a failed invocation result will result in
     * undefined behavior.
     *
     * \return the output of a successful function invocation
     */
    void unwrap() noexcept {}

    /** Retrieves the error state of a failed function invocation.
     *
     * BEWARE: Calling this on a successful invocation result will result in
     * undefined behavior.
     *
     * \return the error state
     */
    error_type &&unwrap_err() noexcept { return std::move(std::get<1>(m_variant)); }

    /** Retries a failed function invocation and returns the result, or
     * immediately returns on prior success.
     *
     * This will wait on updates from the `update_fn` callback before retrying.
     *
     * \return the result of a retried function invocation
     */
    result_type &&retry() noexcept {
        if (is_err()) {
            return retry_impl(std::make_index_sequence<sizeof...(Args)>());
        } else {
            return std::move(*this);
        }
    }

    /** Keeps retrying the function invocation until success, at which point it
     * returns the function's output.
     *
     * BEWARE: This function will not terminate until the function has been
     * successfully invoked.
     *
     * \return the output of a succesful function invocation
     */
    void wait() noexcept {
        while (is_err()) {
            *this = std::move(retry_impl(std::make_index_sequence<sizeof...(Args)>()));
        }
    }

   private:
    template <std::size_t... Is>
    result_type retry_impl(std::index_sequence<Is...>) {
        auto err = std::move(unwrap_err());
        auto &retry_fn = std::get<0>(err);
        auto &update_fn = std::get<1>(err);
        auto &args = std::get<2>(err);
        while (!update_fn()) {
            // TODO: Expose a Runtime API to wait for updates
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        return retry_fn(std::get<Is>(args)...);
    }

    std::variant<std::monostate, error_type> m_variant;
};
}  // namespace mun

#endif /* MUN_RESULT_H_ */
