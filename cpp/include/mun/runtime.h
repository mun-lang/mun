#ifndef MUN_RUNTIME_CPP_BINDINGS_H_
#define MUN_RUNTIME_CPP_BINDINGS_H_

#include <cassert>
#include <optional>
#include <string_view>

#include "mun/error.h"
#include "mun/function_info.h"
#include "mun/runtime_capi.h"
#include "mun/runtime_function.h"
#include "mun/type_info.h"

namespace mun {

struct RuntimeOptions;

/** A wrapper around a `MunRuntimeHandle`.
 *
 * Frees the corresponding runtime object on destruction, if it exists.
 */
class Runtime {
    friend std::optional<Runtime> make_runtime(std::string_view library_path,
                                               const RuntimeOptions& options,
                                               Error* out_error) noexcept;

    /** Constructs a runtime from an instantiated `MunRuntimeHandle`.
     *
     * \param handle a runtime handle
     */
    Runtime(MunRuntimeHandle handle) noexcept : m_handle(handle) {}

public:
    /** Move constructs a runtime
     *
     * \param other an rvalue reference to a runtime
     */
    Runtime(Runtime&& other) noexcept : m_handle(other.m_handle) { other.m_handle._0 = nullptr; }

    /** Destructs a runtime */
    ~Runtime() noexcept { mun_runtime_destroy(m_handle); }

    /** Retrieves `MunFunctionDefinition` from the runtime for the corresponding
     * `fn_name`.
     *
     * \param fn_name the name of the desired function
     * \param out_error a pointer that will optionally return an error
     * \return possibly, the desired `MunFunctionDefinition` struct
     */
    std::optional<FunctionInfo> find_function_info(std::string_view fn_name,
                                                   Error* out_error = nullptr) noexcept {
        bool has_fn;
        MunFunctionInfoHandle fn_info;
        if (auto error =
                Error(mun_runtime_get_function_info(m_handle, fn_name.data(), &has_fn, &fn_info))) {
            if (out_error) {
                *out_error = std::move(error);
            }
            return std::nullopt;
        }

        return has_fn ? std::make_optional(std::move(fn_info)) : std::nullopt;
    }

    /**
     * Allocates an object in the runtime of the given `type_info`. If
     * successful, `obj` is returned, otherwise the nothing is returned and the
     * `out_error` is set - if it is not null.
     *
     * \param type_info the type to allocate
     * \param out_error a pointer to fill with a potential error
     * \return potentially, the handle of an allocated object
     */
    std::optional<MunGcPtr> gc_alloc(const TypeInfo& type_info,
                                     Error* out_error = nullptr) const noexcept {
        MunGcPtr obj;
        if (auto error = Error(mun_gc_alloc(m_handle, type_info.handle(), &obj))) {
            if (out_error) {
                *out_error = std::move(error);
            }
            return std::nullopt;
        }

        return std::make_optional(obj);
    }

    /** Collects all memory that is no longer referenced by rooted objects.
     *
     * Returns `true` if memory was reclaimed, `false` otherwise. This behavior
     * will likely change in the future.
     */
    bool gc_collect() const noexcept {
        bool reclaimed;
        auto error_handle = mun_gc_collect(m_handle, &reclaimed);
        assert(error_handle._0 == nullptr);

        return reclaimed;
    }

    /**
     * Roots the specified `obj`, which keeps it and objects it references
     * alive.
     *
     * Objects marked as root, must call `mun_gc_unroot` before they can
     * be collected. An object can be rooted multiple times, but you must make
     * sure to call `mun_gc_unroot` an equal number of times before the object
     * can be collected. If successful, `obj` has been rooted, otherwise a
     * non-zero error handle is returned.
     *
     * \param obj a garbage collection handle
     */
    void gc_root_ptr(MunGcPtr obj) const noexcept {
        const auto error_handle = mun_gc_root(m_handle, obj);
        assert(error_handle._0 == nullptr);
    }

    /**
     * Unroots the specified `obj`, potentially allowing it and objects it
     * references to be collected.
     *
     * An object can be rooted multiple times, so you must make sure to call
     * `gc_unroot_ptr` the same number of times as `gc_root_ptr` was called
     * before the object can be collected.
     *
     * \param obj a garbage collection handle
     */
    void gc_unroot_ptr(MunGcPtr obj) const noexcept {
        const auto error_handle = mun_gc_unroot(m_handle, obj);
        assert(error_handle._0 == nullptr);
    }

    /**
     * Retrieves the type information for the specified `obj`.
     *
     * \param obj a garbage collection handle
     * \return the handle's type information
     */
    TypeInfo ptr_type(MunGcPtr obj) const noexcept {
        MunTypeInfoHandle type_handle;
        const auto error_handle = mun_gc_ptr_type(m_handle, obj, &type_handle);
        assert(error_handle._0 == nullptr);
        return TypeInfo(type_handle);
    }

    /** Checks for updates to hot reloadable assemblies.
     *
     * \param out_error a pointer that will optionally return an error
     * \return whether the runtime was updated
     */
    bool update(Error* out_error = nullptr) {
        bool updated;
        if (auto error = Error(mun_runtime_update(m_handle, &updated))) {
            if (out_error) {
                *out_error = std::move(error);
            }
            return false;
        }
        return updated;
    }

private:
    MunRuntimeHandle m_handle;
};

struct RuntimeOptions {
    /**
     * The interval at which changes to the disk are detected. `0` will initialize this value to
     * default.
     */
    uint32_t delay_ms = 0;

    /**
     * A list of functions to add to the runtime, these functions can be called from Mun as *extern*
     * functions.
     */
    std::vector<RuntimeFunction> functions;
};

/** Construct a new runtime that loads the library at `library_path` and its dependencies.
 *
 * On failure, the error is returned through the `out_error` pointer, if set.
 *
 * \param library_path the path to a Mun library
 * \param options Additional options used by the construction of a runtime
 * \param out_error optionally, a pointer to an `Error` instance
 * \return potentially, a runtime
.*/
inline std::optional<Runtime> make_runtime(std::string_view library_path,
                                           const RuntimeOptions& options = {},
                                           Error* out_error = nullptr) noexcept {
    std::vector<MunExternalFunctionDefinition> function_definitions(options.functions.size());
    for (size_t i = 0; i < options.functions.size(); ++i) {
        auto& definition = function_definitions[i];
        const auto& func = options.functions[i];
        definition = MunExternalFunctionDefinition{
            func.name.c_str(), static_cast<uint32_t>(func.arg_types.size()), func.arg_types.data(),
            func.ret_type, func.fn_ptr};

        // The MunExternalFunctionDefinition has ownership over the stored types
        for (const auto& arg_type : func.arg_types) {
            mun_type_info_increment_strong_count(arg_type);
        }
        mun_type_info_increment_strong_count(func.ret_type);
    }

    MunRuntimeOptions runtime_options;
    runtime_options.functions =
        function_definitions.empty() ? nullptr : function_definitions.data();
    runtime_options.num_functions = static_cast<uint32_t>(function_definitions.size());

    MunRuntimeHandle handle;
    if (auto error = Error(mun_runtime_create(library_path.data(), runtime_options, &handle))) {
        if (out_error) {
            *out_error = std::move(error);
        }
        return std::nullopt;
    }

    return Runtime(handle);
}
}  // namespace mun

#endif /* MUN_RUNTIME_CPP_BINDINGS_H_ */
