#include <mun/mun.h>

#include <catch2/catch.hpp>
#include <sstream>

/// Returns the absolute path to the munlib with the specified name
inline std::string get_munlib_path(std::string_view name) {
    std::stringstream ss;
    ss << MUN_TEST_DIR << name;
    return ss.str();
}

TEST_CASE("runtime can be constructed", "[runtime]") {
    mun::Error err;
    if (auto runtime = mun::make_runtime(get_munlib_path("fibonacci/target/mod.munlib"), {}, &err)) {
        REQUIRE(!err);
    } else {
        REQUIRE(err);
        FAIL(err.message());
    }
}

TEST_CASE("runtime can find `FunctionInfo`", "[runtime]") {
    mun::Error err;
    if (auto runtime = mun::make_runtime(get_munlib_path("fibonacci/target/mod.munlib"), {}, &err)) {
        REQUIRE(!err);
        REQUIRE(runtime.has_value());

        if (auto function_info = runtime->find_function_definition("fibonacci", &err)) {
            REQUIRE(!err);
        } else {
            REQUIRE(err);
            FAIL(err.message());
        }
    } else {
        REQUIRE(err);
        FAIL(err.message());
    }
}

// TODO: Test hot reloading
TEST_CASE("runtime can update", "[runtime]") {
    mun::Error err;
    if (auto runtime = mun::make_runtime(get_munlib_path("fibonacci/target/mod.munlib"), {}, &err)) {
        REQUIRE(!err);

        runtime->update(&err);
        if (err) {
            FAIL(err.message());
        }
        REQUIRE(!err);
    } else {
        REQUIRE(err);
        FAIL(err.message());
    }
}

TEST_CASE("runtime can garbage collect", "[runtime]") {
    mun::Error err;
    if (auto runtime = mun::make_runtime(get_munlib_path("marshal/target/mod.munlib"), {}, &err)) {
        REQUIRE(!err);
        {
            auto res = mun::invoke_fn<mun::StructRef>(*runtime, "new_bool", true, false);
            REQUIRE(res.is_ok());
            REQUIRE(!runtime->gc_collect());
        }
        REQUIRE(runtime->gc_collect());
    } else {
        REQUIRE(err);
        FAIL(err.message());
    }
}
