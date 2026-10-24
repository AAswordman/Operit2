cmake_minimum_required(VERSION 3.22)
project(operit_local_llm LANGUAGES C CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
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
set(MNN_BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
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
    PATCH_COMMAND ${CMAKE_COMMAND} -P ${CMAKE_CURRENT_LIST_DIR}/PatchMnnStrictAnsi.cmake)
FetchContent_MakeAvailable(mnn)
# Keep the platform C++ runtime outside the Rust cdylib on Windows and Android.
# The engine ships as a shared library that links the platform STL itself
# (ANDROID_STL=c++_static), so liboperit_flutter_bridge.so never references
# libc++ symbols and Android's dynamic linker can always resolve them.
if(WIN32 OR ANDROID)
    add_library(operit_local_llm SHARED inference.cpp)
    set_target_properties(operit_local_llm PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY "$<1:${OPERIT_LOCAL_LLM_RUNTIME_DIR}>"
        LIBRARY_OUTPUT_DIRECTORY "$<1:${OPERIT_LOCAL_LLM_RUNTIME_DIR}>")
    if(WIN32)
        # ggml's CPU detection owns the Windows registry dependency.
        target_link_libraries(operit_local_llm PRIVATE advapi32)
    else()
        # Export only the plain C boundary and keep llama/MNN/libc++ internal.
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
