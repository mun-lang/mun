#include <mun/mun.h>

#include <catch2/catch_test_macros.hpp>
#include <sstream>

/// Returns the absolute path to the munlib with the specified name
inline std::string get_munlib_path(std::string_view name) {
    std::stringstream ss;
    ss << MUN_TEST_DIR << name;
    return ss.str();
}

uint32_t internal_function(uint32_t a, uint32_t b) { return a + b; }
uint32_t some_function() { return 0; }

TEST_CASE("functions must be inserted into the runtime", "[extern]") {
    mun::RuntimeOptions options;

    mun::Error err;
    auto runtime =
        mun::make_runtime(get_munlib_path("mun-extern/target/mod.munlib"), options, &err);
    REQUIRE(!runtime);
    REQUIRE(err.is_error());
}

TEST_CASE("function must have correct signature", "[extern]") {
    mun::RuntimeOptions options;
    options.functions.emplace_back(mun::RuntimeFunction("extern_fn", some_function));

    mun::Error err;
    auto runtime =
        mun::make_runtime(get_munlib_path("mun-extern/target/mod.munlib"), options, &err);
    REQUIRE(!runtime);
    REQUIRE(err.is_error());
}

TEST_CASE("functions can be inserted into the runtime", "[extern]") {
    mun::RuntimeOptions options;
    options.functions.emplace_back(mun::RuntimeFunction("extern_fn", internal_function));

    mun::Error err;
    auto runtime =
        mun::make_runtime(get_munlib_path("mun-extern/target/mod.munlib"), options, &err);
    if (!runtime) {
        REQUIRE(err.is_error());
        FAIL(err.message().value());
    }

    REQUIRE(mun::invoke_fn<uint32_t, uint32_t, uint32_t>(*runtime, "main", 90, 2648).unwrap() ==
            90 + 2648);
}
