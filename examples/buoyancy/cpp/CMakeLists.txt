add_executable(MunExampleBuoyancy
    main.cc
)

target_compile_features(MunExampleBuoyancy
    PRIVATE
        cxx_std_17
)

include(FindThreads)

target_link_libraries(MunExampleBuoyancy
    PRIVATE
        MunRuntime
        Threads::Threads
)

add_custom_command(TARGET MunExampleBuoyancy PRE_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:MunRuntime>
        $<TARGET_FILE_DIR:MunExampleBuoyancy>
)
