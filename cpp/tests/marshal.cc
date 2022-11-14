#include <mun/mun.h>

#include <catch2/catch.hpp>
#include <sstream>
#include <vector>

/// Returns the absolute path to the munlib with the specified name
inline std::string get_munlib_path(std::string_view name) {
    std::stringstream ss;
    ss << MUN_TEST_DIR << name;
    return ss.str();
}

#define TEST_MARSHALLING(ty, lhs, rhs, expected)                                             \
    TEST_CASE("function can marshal " #ty, "[marshal]") {                                    \
        mun::Error err;                                                                      \
        if (auto runtime =                                                                   \
                mun::make_runtime(get_munlib_path("mun-marshal/target/mod.munlib"), {}, &err)) { \
            REQUIRE(err.is_ok());                                                            \
                                                                                             \
            const ty a = (lhs), b = (rhs);                                                   \
            auto res = mun::invoke_fn<ty>(*runtime, "marshal_" #ty, a, b);                   \
            REQUIRE(res.is_ok());                                                            \
            REQUIRE(res.wait() == (expected));                                               \
        } else {                                                                             \
            REQUIRE(err.is_error());                                                         \
            FAIL(err.message().value());                                                     \
        }                                                                                    \
    }                                                                                        \
    TEST_CASE("struct can get, set, and replace " #ty, "[marshal]") {                        \
        mun::Error err;                                                                      \
        if (auto runtime =                                                                   \
                mun::make_runtime(get_munlib_path("mun-marshal/target/mod.munlib"), {}, &err)) { \
            REQUIRE(err.is_ok());                                                            \
                                                                                             \
            const ty a = (lhs), b = (rhs);                                                   \
            auto res = mun::invoke_fn<mun::StructRef>(*runtime, "new_" #ty, a, b);           \
            REQUIRE(res.is_ok());                                                            \
                                                                                             \
            auto s = res.wait();                                                             \
            {                                                                                \
                const auto first = s.get<ty>("0");                                           \
                REQUIRE(first.has_value());                                                  \
                REQUIRE(*first == a);                                                        \
            }                                                                                \
            {                                                                                \
                const auto second = s.get<ty>("1");                                          \
                REQUIRE(second.has_value());                                                 \
                REQUIRE(*second == b);                                                       \
            }                                                                                \
            REQUIRE(s.set("0", b));                                                          \
            REQUIRE(s.set("1", a));                                                          \
            {                                                                                \
                const auto first = s.replace<ty>("0", a);                                    \
                REQUIRE(first.has_value());                                                  \
                REQUIRE(*first == b);                                                        \
            }                                                                                \
            {                                                                                \
                const auto second = s.replace<ty>("1", b);                                   \
                REQUIRE(second.has_value());                                                 \
                REQUIRE(*second == a);                                                       \
            }                                                                                \
            {                                                                                \
                const auto first = s.get<ty>("0");                                           \
                REQUIRE(first.has_value());                                                  \
                REQUIRE(*first == a);                                                        \
            }                                                                                \
            {                                                                                \
                const auto second = s.get<ty>("1");                                          \
                REQUIRE(second.has_value());                                                 \
                REQUIRE(*second == b);                                                       \
            }                                                                                \
        } else {                                                                             \
            REQUIRE(err.is_error());                                                         \
            FAIL(err.message().value());                                                     \
        }                                                                                    \
    }

// TODO: Add 128-bit integers
// TODO: Add error testing
TEST_MARSHALLING(bool, false, true, false || true);
TEST_MARSHALLING(float, -3.14f, 6.28f, -3.14f + 6.28f);
TEST_MARSHALLING(double, -3.14, 6.28, -3.14 + 6.28);
TEST_MARSHALLING(int8_t, 1, 64, 1 + 64);
TEST_MARSHALLING(int16_t, 1, 64, 1 + 64);
TEST_MARSHALLING(int32_t, 1, 64, 1 + 64);
TEST_MARSHALLING(int64_t, 1, 64, 1 + 64);
// TEST_MARSHALLING(int128_t, 1, 64, 1 + 64);
TEST_MARSHALLING(uint8_t, 1, 64, 1 + 64);
TEST_MARSHALLING(uint16_t, 1, 64, 1 + 64);
TEST_MARSHALLING(uint32_t, 1, 64, 1 + 64);
TEST_MARSHALLING(uint64_t, 1, 64, 1 + 64);
// TEST_MARSHALLING(uint128_t, 1, 64, 1 + 64);

TEST_CASE("struct can get, set, and replace struct", "[marshal]") {
    mun::Error err;
    if (auto runtime = mun::make_runtime(get_munlib_path("mun-marshal/target/mod.munlib"), {}, &err)) {
        REQUIRE(err.is_ok());

        float a = -3.14f, b = 6.28f;
        auto gc_struct_res = mun::invoke_fn<mun::StructRef>(*runtime, "new_gc_struct", a, b);
        REQUIRE(gc_struct_res.is_ok());
        auto value_struct_res = mun::invoke_fn<mun::StructRef>(*runtime, "new_value_struct", a, b);
        REQUIRE(value_struct_res.is_ok());

        // Test `InvokeResult::retry` and `InvokeResult::unwrap`
        auto gc_struct = gc_struct_res.retry().unwrap();
        auto value_struct = value_struct_res.retry().unwrap();

        auto gc_wrapper =
            mun::invoke_fn<mun::StructRef>(*runtime, "new_gc_wrapper", gc_struct, value_struct);
        REQUIRE(gc_wrapper.is_ok());

        auto value_wrapper =
            mun::invoke_fn<mun::StructRef>(*runtime, "new_value_wrapper", gc_struct, value_struct);
        REQUIRE(value_wrapper.is_ok());

        // Test `InvokeResult::wait`
        std::array<mun::StructRef, 2> structs = {gc_wrapper.wait(), value_wrapper.wait()};
        for (auto s : structs) {
            // `struct(gc)`
            auto gc = s.get<mun::StructRef>("0");
            REQUIRE(gc.has_value());

            REQUIRE(gc->set("0", b));
            REQUIRE(gc->set("1", a));

            // Replace the gc-struct's pointer.
            auto gc2 = s.replace("0", *gc);
            REQUIRE(gc2.has_value());

            // Verify that `replace` worked
            const auto gc2_0 = gc2->get<float>("0");
            REQUIRE(gc2_0.has_value());
            REQUIRE(*gc2_0 == b);

            const auto gc2_1 = gc2->get<float>("1");
            REQUIRE(gc2_1.has_value());
            REQUIRE(*gc2_1 == a);

            REQUIRE(gc2->set("0", a));
            REQUIRE(gc2->set("1", b));

            // Verify that a `struct(gc)` points to the same (modified)
            // object; for both instances: `gc` and `gc2`.
            const auto gc_0 = gc->get<float>("0");
            REQUIRE(gc_0.has_value());
            REQUIRE(*gc_0 == a);

            const auto gc_1 = gc->get<float>("1");
            REQUIRE(gc_1.has_value());
            REQUIRE(*gc_1 == b);

            // Set the gc-struct's pointer.
            REQUIRE(s.set<mun::StructRef>("0", *gc2));

            // Verify that `set` worked.
            auto gc3 = s.get<mun::StructRef>("0");
            REQUIRE(gc3.has_value());

            const auto gc3_0 = gc3->get<float>("0");
            REQUIRE(gc3_0.has_value());
            REQUIRE(*gc3_0 == a);

            const auto gc3_1 = gc3->get<float>("1");
            REQUIRE(gc3_1.has_value());
            REQUIRE(*gc3_1 == b);

            // `struct(value)`
            auto value = s.get<mun::StructRef>("1");
            REQUIRE(value.has_value());

            REQUIRE(value->set("0", b));
            REQUIRE(value->set("1", a));

            // Replace the value-struct's content.
            auto value2 = s.replace("1", *value);
            REQUIRE(value2.has_value());

            // Verify that `replace` worked
            auto value3 = s.get<mun::StructRef>("1");
            REQUIRE(value3.has_value());

            const auto value3_0 = value3->get<float>("0");
            REQUIRE(value3_0.has_value());
            REQUIRE(*value3_0 == b);

            const auto value3_1 = value3->get<float>("1");
            REQUIRE(value3_1.has_value());
            REQUIRE(*value3_1 == a);

            // Verify that a `struct(value)` does NOT point to the same
            // (modified) object; for both instances: `value` and `value2`.
            const auto value_0 = value->get<float>("0");
            REQUIRE(value_0.has_value());
            REQUIRE(*value_0 == b);

            const auto value_1 = value->get<float>("1");
            REQUIRE(value_1.has_value());
            REQUIRE(*value_1 == a);

            const auto value2_0 = value2->get<float>("0");
            REQUIRE(value2_0.has_value());
            REQUIRE(*value2_0 == a);

            const auto value2_1 = value2->get<float>("1");
            REQUIRE(value2_1.has_value());
            REQUIRE(*value2_1 == b);

            // Set the value-struct's content.
            REQUIRE(s.set<mun::StructRef>("1", *value2));

            // Verify that `set` worked.
            auto value4 = s.get<mun::StructRef>("1");
            REQUIRE(value4.has_value());

            const auto value4_0 = value4->get<float>("0");
            REQUIRE(value4_0.has_value());
            REQUIRE(*value4_0 == a);

            const auto value4_1 = value4->get<float>("1");
            REQUIRE(value4_1.has_value());
            REQUIRE(*value4_1 == b);
        }
    } else {
        REQUIRE(err.is_error());
        FAIL(err.message().value());
    }
}

TEST_CASE("can fetch array type", "[marshal]") {
    mun::Error err;
    if (auto runtime = mun::make_runtime(get_munlib_path("marshal/target/mod.munlib"), {}, &err)) {
        REQUIRE(err.is_ok());

        auto array_res = mun::invoke_fn<mun::ArrayRef<int32_t>>(*runtime, "new_array_i32", 1, 2, 3);
        REQUIRE(array_res.is_ok());
        auto array = array_res.unwrap();

        REQUIRE(array.size() == 3);
        REQUIRE(array.capacity() >= 3);

        REQUIRE(array.at(0) == 1);
        REQUIRE(array.at(1) == 2);
        REQUIRE(array.at(2) == 3);
        REQUIRE_THROWS(array.at(3));

        std::vector<int32_t> vec;
        vec.assign(array.begin(), array.end());
        REQUIRE(vec == std::vector<int32_t>({1,2,3}));
    }
}
