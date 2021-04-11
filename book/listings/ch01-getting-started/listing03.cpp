#include <iostream>

#include "mun/mun.h"

int main(int argc, char *argv[]) {
    if (argc < 2) {
        return 1;
    }

    auto lib_path = argv[1];
    mun::RuntimeOptions options;
    mun::Error error;
    if (auto runtime = mun::make_runtime(lib_path, options, &error)) {
        while (true) {
            auto arg = mun::invoke_fn<int64_t>(*runtime, "arg").wait();
            auto result =
                mun::invoke_fn<int64_t>(*runtime, "fibonacci", arg).wait();
            std::cout << "fibonacci(" << std::to_string(arg) << ") = " << result
                      << std::endl;

            runtime->update();
        }

        return 0;
    }

    std::cerr << "Failed to construct Mun runtime due to error: "
              << error.message() << std::endl;

    return 2;
}
