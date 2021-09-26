cmake_minimum_required(VERSION 3.10)
project(MunRuntime VERSION 0.1.0 LANGUAGES C CXX)

option(mun_build_examples "Build all of Mun's own examples." OFF)
option(mun_build_tests "Build all of Mun's own tests." OFF)
set(mun_library_url_or_path "" CACHE FILEPATH "Location of an archive on the web or a local directory containing containing the Mun libraries.")

set(cpp_source_dir ${CMAKE_CURRENT_SOURCE_DIR}/cpp)

# Determine platform (32/64)
if (${CMAKE_SIZEOF_VOID_P} EQUAL 8)
    set(X64 ON)
else ()
    set(X86 ON)
endif ()

add_library(MunRuntime SHARED IMPORTED GLOBAL)

set_target_properties(MunRuntime PROPERTIES
    INTERFACE_INCLUDE_DIRECTORIES
        "${cpp_source_dir}/include"
)

if (X64)
    if (WIN32)
        set(mun_os win64)
    elseif (UNIX AND NOT APPLE)
        set(mun_os linux64)
    elseif (UNIX AND APPLE)
        set(mun_os osx64)
    else ()
        message(ERROR "Unsupported operating system.")
    endif ()
else ()
    message(ERROR "Only 64-bit operating systems are supported at present.")
endif ()

include(ExternalProject)

if (mun_library_url_or_path)
    if (IS_DIRECTORY ${mun_library_url_or_path})
        set(mun_library_dir ${mun_library_url_or_path})
    else ()
        # Download the mun libraries
        ExternalProject_Add(
            mun_library_download
            PREFIX ${CMAKE_BINARY_DIR}/mun
            URL ${mun_library_url_or_path}
            CONFIGURE_COMMAND ""
            BUILD_COMMAND ""
            INSTALL_COMMAND ""
        )

        ExternalProject_Get_Property(mun_library_download source_dir)
        set(mun_library_dir ${source})
        add_dependencies(MunRuntime mun_library_download)
    endif ()
else ()
    message(FATAL_ERROR "You must specify the mun_library_url_or_path to be able to test the Mun Runtime")
endif ()

set_target_properties(MunRuntime PROPERTIES
    IMPORTED_LOCATION ${mun_library_dir}/${CMAKE_SHARED_LIBRARY_PREFIX}mun_runtime${CMAKE_SHARED_LIBRARY_SUFFIX}
)

if (WIN32)
    set_target_properties(MunRuntime PROPERTIES
        IMPORTED_IMPLIB ${mun_library_dir}/mun_runtime${CMAKE_SHARED_LIBRARY_SUFFIX}.lib
    )
endif ()

if (mun_build_examples)
    add_subdirectory(examples)
endif ()

include(CTest)
if (mun_build_tests)
    add_subdirectory(${cpp_source_dir}/tests)
endif ()
