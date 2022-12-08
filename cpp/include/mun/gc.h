#ifndef MUN_GC_H_
#define MUN_GC_H_

#include <optional>

#include "mun/error.h"
#include "mun/runtime.h"

namespace mun {
class Runtime;

class GcRootPtr {
public:
    /** Constructs a rooted garbage collection pointer from the provided raw
     * garbage collection type_handle.
     *
     * \param runtime a reference to a runtime
     * \param obj a garbage collected object type_handle
     * \return a rooted garbage collection pointer
    .*/
    GcRootPtr(const Runtime& runtime, MunGcPtr obj) noexcept : m_ptr(obj), m_runtime(&runtime) {
        runtime.gc_root_ptr(obj);
    }

    /** Copy constructs a `GcRootPtr`.
     *
     * Roots the specified `obj` in the process, which keeps it and objects it
     * references alive.
     *
     * \param other a reference to another `GcRootPtr`
     */
    GcRootPtr(const GcRootPtr& other) noexcept : m_ptr(other.m_ptr), m_runtime(other.m_runtime) {
        m_runtime->gc_root_ptr(m_ptr);
    }

    /** Move constructs a `GcRootPtr`
     *
     * \param other an rvalue reference to a `GcRootPtr`
     */
    GcRootPtr(GcRootPtr&& other) noexcept : m_ptr(other.m_ptr), m_runtime(other.m_runtime) {
        other.m_ptr = nullptr;
    }

    /** Move assignment operator for `GcRootPtr`
     *
     * \param other an rvalue reference to a `GcRootPtr`
     * \return a reference this instance
     */
    GcRootPtr& operator=(GcRootPtr&& other) noexcept {
        if (m_ptr == other.m_ptr) {
            // Prevent unrooting after the move
            other.m_ptr = nullptr;
        } else {
            std::swap(m_ptr, other.m_ptr);
        }
        m_runtime = other.m_runtime;
        return *this;
    }

    /** Destructs the `GcRootPtr`, unrooting the underlying `GcPtr`. */
    ~GcRootPtr() noexcept { unroot(); }

    /** Retrieves the raw garbage collection type_handle of this instance.
     *
     * \return a raw garbage collection type_handle
     */
    [[nodiscard]] constexpr MunGcPtr handle() const noexcept { return m_ptr; }

    /** Unroots the underlying `GcPtr`, returning the underlying garbage
     * collection type_handle.
     *
     * \return a raw garbage collection type_handle
     */
    MunGcPtr unroot() noexcept {
        const auto ptr = m_ptr;
        if (m_ptr) {
            m_runtime->gc_unroot_ptr(m_ptr);
            m_ptr = nullptr;
        }
        return ptr;
    }

private:
    MunGcPtr m_ptr;
    const Runtime* m_runtime;
};
}  // namespace mun

#endif
