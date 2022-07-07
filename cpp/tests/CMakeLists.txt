include(ExternalProject)

set(mun_executable_url "" CACHE FILEPATH "URL to an archive on the web containing the Mun executable. This is required for building the Mun tests.")
set(mun_examples_path "${CMAKE_SOURCE_DIR}/examples" CACHE FILEPATH "Location of Mun examples. This is required for building the Mun tests.")

add_executable(mun_executable IMPORTED)
if (mun_binaries_path)
    set_target_properties(mun_executable PROPERTIES IMPORTED_LOCATION ${mun_binaries_path}/mun${CMAKE_EXECUTABLE_SUFFIX})
elseif (mun_executable_url)
    # Download the mun executable
    ExternalProject_Add(
        mun_executable_download
        PREFIX ${CMAKE_CURRENT_BINARY_DIR}/mun
        URL ${mun_executable_url}
        CONFIGURE_COMMAND ""
        BUILD_COMMAND ""
        INSTALL_COMMAND ""
    )

    ExternalProject_Get_Property(mun_executable_download source_dir)
    set_target_properties(mun_executable PROPERTIES IMPORTED_LOCATION ${source_dir}/mun${CMAKE_EXECUTABLE_SUFFIX})
    add_dependencies(mun_executable mun_executable_download)
else ()
    message(FATAL_ERROR "You must specify the `mun_binaries_path` or `mun_executable_url` to be able to use the Mun Runtime")
endif ()

if (mun_examples_path)
    if (NOT IS_DIRECTORY ${mun_binaries_path})
        message(FATAL_ERROR "The provided location of Mun examples is invalid.")
    endif ()
else ()
    message(FATAL_ERROR "You must specify the mun_examples_path to be able to test the Mun Runtime")
endif ()

# Sanitize the path, since we're forwarding this to C++
file(TO_CMAKE_PATH ${mun_examples_path} mun_examples_path)

# Find all mun files
set(mun_folder ${CMAKE_CURRENT_SOURCE_DIR}/..)
file(GLOB_RECURSE mun_test_files RELATIVE ${mun_examples_path} ${mun_examples_path}/*.toml)
set(mun_tests)

foreach (mun_file ${mun_test_files})
    get_filename_component(mun_file_dir ${mun_file} DIRECTORY)
    set(abs_package_dir ${mun_examples_path}/${mun_file_dir})
    set(abs_toml_file ${abs_package_dir}/mun.toml)
    set(mun_executable_file mod.munlib)
    set(mun_executable_path ${abs_package_dir}/target/${mun_executable_file})
    add_custom_command(
            OUTPUT ${mun_executable_path}
            COMMAND mun_executable build --manifest-path "${abs_toml_file}"
            WORKING_DIRECTORY ${abs_package_dir}
            MAIN_DEPENDENCY ${abs_toml_file})
    list(APPEND mun_tests ${mun_executable_path})
endforeach ()

add_custom_target(mun_test_munlibs ALL
        DEPENDS ${mun_tests})

# Add the tests
add_executable(MunRuntimeTests
    catch_main.cc
    marshal.cc
    runtime.cc
    extern.cc
)

target_include_directories(MunRuntimeTests PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/catch2/single_include ${mun_folder}/include)
target_link_libraries(MunRuntimeTests MunRuntime)
add_dependencies(MunRuntimeTests mun_test_munlibs)
target_compile_definitions(MunRuntimeTests PRIVATE -DMUN_TEST_DIR="${mun_examples_path}/")
set_property(TARGET MunRuntimeTests PROPERTY CXX_STANDARD 17)

add_custom_command(TARGET MunRuntimeTests POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy
        $<TARGET_FILE:MunRuntime>
        $<TARGET_FILE_DIR:MunRuntimeTests>)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/catch2/contrib")
include(Catch)
catch_discover_tests(MunRuntimeTests)
