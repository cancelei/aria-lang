cmake_minimum_required(VERSION 3.20)
project(bioflow-cpp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Options
option(BUILD_TESTS "Build tests" ON)
option(BUILD_BENCHMARKS "Build benchmarks" ON)

# Library
add_library(bioflow
    src/sequence.cpp
    src/kmer.cpp
    src/alignment.cpp
    src/quality.cpp
    src/stats.cpp
)

target_include_directories(bioflow PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)

target_compile_options(bioflow PRIVATE
    -Wall -Wextra -Wpedantic
    $<$<CONFIG:Release>:-O3 -march=native -DNDEBUG>
    $<$<CONFIG:Debug>:-g -O0>
)

# Main executable
add_executable(bioflow-exe src/main.cpp)
target_link_libraries(bioflow-exe PRIVATE bioflow)
set_target_properties(bioflow-exe PROPERTIES OUTPUT_NAME bioflow)

# Tests (using Google Test)
if(BUILD_TESTS)
    find_package(GTest QUIET)

    if(GTest_FOUND)
        enable_testing()

        add_executable(bioflow_tests
            tests/test_sequence.cpp
            tests/test_kmer.cpp
            tests/test_alignment.cpp
        )
        target_link_libraries(bioflow_tests PRIVATE bioflow GTest::gtest GTest::gtest_main)
        target_compile_options(bioflow_tests PRIVATE -Wall -Wextra)

        include(GoogleTest)
        gtest_discover_tests(bioflow_tests)

        add_test(NAME bioflow_tests COMMAND bioflow_tests)
    else()
        message(STATUS "Google Test not found - tests will not be built")
        message(STATUS "Install with: sudo apt-get install libgtest-dev")
    endif()
endif()

# Benchmarks (using Google Benchmark)
if(BUILD_BENCHMARKS)
    find_package(benchmark QUIET)

    if(benchmark_FOUND)
        add_executable(bioflow_bench benchmark/bench.cpp)
        target_link_libraries(bioflow_bench PRIVATE bioflow benchmark::benchmark benchmark::benchmark_main)
        target_compile_options(bioflow_bench PRIVATE -O3 -march=native)
    else()
        message(STATUS "Google Benchmark not found - benchmarks will not be built")
        message(STATUS "Install with: sudo apt-get install libbenchmark-dev")
    endif()
endif()

# Installation
install(TARGETS bioflow bioflow-exe
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
)

install(DIRECTORY include/bioflow DESTINATION include)
