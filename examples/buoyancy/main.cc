#include <iostream>
#include <string>

#include "mun/mun.h"

extern "C" {
void log_f32(float value) { std::cout << std::to_string(value) << std::endl; }
}

// How to run?
// 1. On the CLI, navigate to the `example-cpp` directory.
// 2. Run the compiler daemon from the CLI:
//    `/path/to/mun build resources/buoyancy.mun --watch`
// 3. Run the application from the CLI:
//    `main /path/to/buoyancy.munlib`
int main(int argc, char* argv[]) {
    if (argc < 2) {
        return 1;
    }
    std::cout << "lib: " << argv[1] << std::endl;

    mun::RuntimeOptions options;
    options.functions.emplace_back(mun::RuntimeFunction("log_f32", log_f32));

    mun::Error error;
    if (auto runtime = mun::make_runtime(argv[1], options, &error)) {
        auto ctx = mun::invoke_fn<mun::StructRef>(*runtime, "new_sim").wait();

        using clock_t = std::chrono::high_resolution_clock;
        using fsec_t = std::chrono::duration<float>;

        auto previous = clock_t::now();
        constexpr auto FRAME_TIME = std::chrono::milliseconds(40);
        while (true) {
            std::this_thread::sleep_until(previous + FRAME_TIME);

            const auto now = clock_t::now();
            const auto elapsed =
                std::chrono::duration_cast<fsec_t>(now - previous);

            mun::invoke_fn<void>(*runtime, "sim_update", ctx, elapsed.count()).wait();
            previous = now;

            mun::Error update_error;
            if (!runtime->update(&update_error) && update_error) {
                std::cerr << "Failed to update runtime due to error: "
                          << update_error.message() << std::endl;
            }
        }
    }

    std::cerr << "Failed to construct Mun runtime due to error: "
              << error.message() << std::endl;
    return 2;
}
