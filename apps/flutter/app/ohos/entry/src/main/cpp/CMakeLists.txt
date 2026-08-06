cmake_minimum_required(VERSION 3.16)

project(operit_runtime_ohos)

set(OPERIT_OHOS_VROOT_HNP_DIRECTORY "${CMAKE_CURRENT_LIST_DIR}/../../../hnp/arm64-v8a")
set(OPERIT_OHOS_VROOT_ASSET_DIRECTORY "${CMAKE_CURRENT_LIST_DIR}/../resources/rawfile/ohos-vroot")
set(OPERIT_OHOS_VROOT_REQUIRED_ARTIFACTS
  "${OPERIT_OHOS_VROOT_HNP_DIRECTORY}/harmonix-public.hnp"
  "${OPERIT_OHOS_VROOT_HNP_DIRECTORY}/harmonix-private.hnp"
  "${OPERIT_OHOS_VROOT_ASSET_DIRECTORY}/alpine-minirootfs-3.22.1-aarch64.tar.gz"
  "${OPERIT_OHOS_VROOT_ASSET_DIRECTORY}/alpine-minirootfs-3.22.1-aarch64.tar.gz.sha256"
)

foreach(OPERIT_OHOS_VROOT_ARTIFACT IN LISTS OPERIT_OHOS_VROOT_REQUIRED_ARTIFACTS)
  if(NOT EXISTS "${OPERIT_OHOS_VROOT_ARTIFACT}")
    message(FATAL_ERROR "Missing required OpenHarmony QEMU-vroot runtime artifact: ${OPERIT_OHOS_VROOT_ARTIFACT}")
  endif()
endforeach()

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
