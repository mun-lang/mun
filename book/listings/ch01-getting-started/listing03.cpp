#include <iostream>

#include "mun/runtime.h"

int main() {
    if (argc < 2) {
        return 1;
    }

    auto lib_path = argv[1];
    if (auto runtime = mun::make_runtime(lib_path)) {
        while (true) {
            auto arg = mun::invoke_fn<int64_t>(*runtime, "arg").wait();
            auto result =
                mun::invoke_fn<int64_t>(*runtime, "fibonacci", arg).wait();
            std::cout << "fibonacci(" << std::to_string(arg) << ") = " << result
                      << std::endl;

            runtime->update();
        }
    }

    return 2;
}
