cmake_minimum_required(VERSION 3.22)
project(operit_local_llm LANGUAGES C CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
if(OHOS)
    # Expose musl's CPU affinity declarations used by the native inference engines.
    add_compile_definitions(_GNU_SOURCE)
    # Match the __n1 namespace exported by the OHOS SDK's libc++ archives.
    set(operit_ohos_cxx_config_dir "${CMAKE_CURRENT_BINARY_DIR}/ohos-libcpp")
    file(READ "${OHOS_SDK_NATIVE}/llvm/include/c++/v1/__config_site" operit_ohos_cxx_config)
    string(REGEX REPLACE "#define _LIBCPP_ABI_NAMESPACE [^\r\n]+"
        "#define _LIBCPP_ABI_NAMESPACE __n1" operit_ohos_cxx_config "${operit_ohos_cxx_config}")
    file(MAKE_DIRECTORY "${operit_ohos_cxx_config_dir}")
    file(CONFIGURE OUTPUT "${operit_ohos_cxx_config_dir}/__config_site"
        CONTENT "${operit_ohos_cxx_config}" @ONLY)
    list(PREPEND CMAKE_CXX_STANDARD_INCLUDE_DIRECTORIES "${operit_ohos_cxx_config_dir}")
endif()
# Export the C++ runtime selected by the toolchain for Rust's final link.
set(operit_cxx_runtime "")
foreach(library IN LISTS CMAKE_CXX_IMPLICIT_LINK_LIBRARIES)
    if(library STREQUAL "stdc++" OR library STREQUAL "c++")
        string(APPEND operit_cxx_runtime "${library}\n")
    endif()
endforeach()
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/operit_cxx_runtime.txt" "${operit_cxx_runtime}")
include(FetchContent)
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_TOOLS OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_SERVER OFF CACHE BOOL "" FORCE)
set(LLAMA_CURL OFF CACHE BOOL "" FORCE)
set(GGML_NATIVE OFF CACHE BOOL "" FORCE)
set(GGML_OPENMP OFF CACHE BOOL "" FORCE)
set(GGML_METAL OFF CACHE BOOL "" FORCE)
set(GGML_BLAS OFF CACHE BOOL "" FORCE)
set(GGML_AVX512 OFF CACHE BOOL "" FORCE)
set(GGML_AVX2 OFF CACHE BOOL "" FORCE)
set(GGML_FMA OFF CACHE BOOL "" FORCE)
set(GGML_F16C OFF CACHE BOOL "" FORCE)
FetchContent_Declare(llama URL https://codeload.github.com/ggml-org/llama.cpp/zip/refs/tags/b11829
    URL_HASH SHA256=70281a1798762c91045a304cddb64016f605db3290de3c4b28c385b2da051961)
FetchContent_MakeAvailable(llama)
if(OHOS)
    # OHOS exposes current-thread affinity through sched_setaffinity.
    set(operit_llama_cpu_source "${llama_SOURCE_DIR}/ggml/src/ggml-cpu/ggml-cpu.c")
    file(READ "${operit_llama_cpu_source}" operit_llama_cpu)
    string(REPLACE "#ifdef __ANDROID__\n    err = sched_setaffinity("
        "#if defined(__ANDROID__) || defined(__OHOS__)\n    err = sched_setaffinity("
        operit_llama_cpu "${operit_llama_cpu}")
    file(CONFIGURE OUTPUT "${operit_llama_cpu_source}" CONTENT "${operit_llama_cpu}" @ONLY)
endif()
set(MNN_BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
# Use the standalone CMake build without Android Studio header-copy hooks.
set(MNN_BUILD_FOR_ANDROID_COMMAND ON CACHE BOOL "" FORCE)
set(MNN_SEP_BUILD OFF CACHE BOOL "" FORCE)
set(MNN_BUILD_LLM ON CACHE BOOL "" FORCE)
set(MNN_BUILD_TOOLS OFF CACHE BOOL "" FORCE)
set(MNN_BUILD_CONVERTER OFF CACHE BOOL "" FORCE)
set(MNN_BUILD_QUANTOOLS OFF CACHE BOOL "" FORCE)
set(MNN_BUILD_TEST OFF CACHE BOOL "" FORCE)
set(MNN_LLM_BUILD_DEMO OFF CACHE BOOL "" FORCE)
set(MNN_LOW_MEMORY ON CACHE BOOL "" FORCE)
set(MNN_CPU_WEIGHT_DEQUANT_GEMM ON CACHE BOOL "" FORCE)
set(MNN_SUPPORT_TRANSFORMER_FUSE ON CACHE BOOL "" FORCE)
set(MNN_BUILD_OPENCV OFF CACHE BOOL "" FORCE)
set(MNN_BUILD_AUDIO OFF CACHE BOOL "" FORCE)
set(MNN_OPENMP OFF CACHE BOOL "" FORCE)
set(MNN_OPENCL OFF CACHE BOOL "" FORCE)
set(MNN_VULKAN OFF CACHE BOOL "" FORCE)
set(MNN_METAL OFF CACHE BOOL "" FORCE)
set(MNN_WIN_RUNTIME_MT OFF CACHE BOOL "" FORCE)
set(LLM_SUPPORT_HTTP_RESOURCE OFF CACHE BOOL "" FORCE)
FetchContent_Declare(mnn URL https://codeload.github.com/alibaba/MNN/zip/refs/tags/3.6.1
    URL_HASH SHA256=93d60848879e68be554c247afe376657630a30652a7d8ec713e9e7d1bbf69260
    PATCH_COMMAND ${CMAKE_COMMAND} -P ${CMAKE_CURRENT_LIST_DIR}/PatchMnnBuild.cmake)
FetchContent_MakeAvailable(mnn)
# Isolate the platform C++ runtime from V8 behind the exported C interface.
if(WIN32 OR ANDROID OR OHOS)
    add_library(operit_local_llm SHARED inference.cpp)
    set_target_properties(operit_local_llm PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY "$<1:${OPERIT_LOCAL_LLM_RUNTIME_DIR}>"
        LIBRARY_OUTPUT_DIRECTORY "$<1:${OPERIT_LOCAL_LLM_RUNTIME_DIR}>")
    if(WIN32)
        # ggml's CPU detection owns the Windows registry dependency.
        target_link_libraries(operit_local_llm PRIVATE advapi32)
    else()
        set(operit_local_llm_exports "${CMAKE_CURRENT_SOURCE_DIR}/operit_local_llm.map")
        set_target_properties(operit_local_llm PROPERTIES
            CXX_VISIBILITY_PRESET hidden
            VISIBILITY_INLINES_HIDDEN ON
            LINK_DEPENDS "${operit_local_llm_exports}")
        target_link_options(operit_local_llm PRIVATE
            "LINKER:--exclude-libs,ALL"
            "LINKER:--version-script,${operit_local_llm_exports}")
        if(ANDROID)
            target_link_libraries(operit_local_llm PRIVATE log)
        endif()
    endif()
else()
    add_library(operit_local_llm STATIC inference.cpp)
endif()
include(GenerateExportHeader)
generate_export_header(operit_local_llm)
target_include_directories(operit_local_llm PRIVATE ${CMAKE_CURRENT_BINARY_DIR})
target_include_directories(operit_local_llm PRIVATE
    ${llama_SOURCE_DIR}/include
    ${mnn_SOURCE_DIR}/include
    ${mnn_SOURCE_DIR}/transformers/llm/engine/include
    ${mnn_SOURCE_DIR}/apps/frameworks/3rd_party/include)
target_link_libraries(operit_local_llm PRIVATE llama MNN)
