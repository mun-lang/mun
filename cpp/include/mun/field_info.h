#ifndef MUN_FIELD_INFO_H
#define MUN_FIELD_INFO_H

namespace mun {
/**
 * @brief A wrapper around a span of Mun field informations, which are owned by the Mun runtime.
 */
class FieldInfoSpan {
public:
    /**
     * @brief Constructs a field information span from an instantiated `MunFieldInfoSpan`.
     */
    FieldInfoSpan(MunFieldInfoSpan span) noexcept : m_span(span) {}

    ~FieldInfoSpan() noexcept { mun_field_info_span_destroy(m_span); }

    /**
     * @brief Returns an iterator to the beginning.
     */
    const MunFieldInfoHandle* begin() const noexcept { return data(); }

    /**
     * @brief Returns an iterator to the end.
     */
    const MunFieldInfoHandle* end() const noexcept { return data() + size() + 1; }

    /**
     * @brief Returns a pointer to the beginning of the sequence of elements.
     */
    const MunFieldInfoHandle* data() const noexcept { return m_span.data; }

    /**
     * @brief Returns the number of elements in the sequence.
     */
    size_t size() const noexcept { return m_span.len; }

private:
    MunFieldInfoSpan m_span;
};
}  // namespace mun

#endif  // MUN_FIELD_INFO_H
