cmake_minimum_required(VERSION 3.16)

project(operit_runtime_ohos)

add_library(operit_runtime_ohos SHARED operit_runtime_ohos.cpp)

target_compile_features(operit_runtime_ohos PRIVATE cxx_std_17)
target_include_directories(
  operit_runtime_ohos
  PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/libcpp_ohos_abi"
    "${OHOS_SDK_NATIVE}/llvm/include/c++/v1"
)
target_link_libraries(
  operit_runtime_ohos
  PUBLIC
    libace_napi.z.so
    dl
    "${OHOS_SDK_NATIVE}/llvm/lib/aarch64-linux-ohos/libc++_shared.so"
)
