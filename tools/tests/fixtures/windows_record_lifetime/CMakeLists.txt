cmake_minimum_required(VERSION 3.23)
project(record_lifetime_test LANGUAGES CXX)

set(plugin "${OPERIT_REPO_ROOT}/apps/flutter/thirdparty/record_windows/windows")
set(flutter "${OPERIT_REPO_ROOT}/apps/flutter/app/windows/flutter/ephemeral")
set(wrapper "${flutter}/cpp_client_wrapper")
add_executable(record_lifetime_test
  record_lifetime_test.cpp
  "${plugin}/record_windows_plugin.cpp"
  "${plugin}/record_windows_plugin_c_api.cpp"
  "${plugin}/record.cpp"
  "${plugin}/record_readercallback.cpp"
  "${plugin}/record_iunknown.cpp"
  "${plugin}/record_mediatype.cpp"
  "${wrapper}/core_implementations.cc"
  "${wrapper}/standard_codec.cc"
  "${wrapper}/plugin_registrar.cc"
)
target_compile_features(record_lifetime_test PRIVATE cxx_std_17)
target_compile_definitions(record_lifetime_test PRIVATE FLUTTER_PLUGIN_IMPL UNICODE _UNICODE NOMINMAX)
target_include_directories(record_lifetime_test PRIVATE
  "${plugin}" "${flutter}" "${wrapper}/include")
target_link_libraries(record_lifetime_test PRIVATE
  "${flutter}/flutter_windows.dll.lib"
  mf mfplat mfreadwrite mfuuid Shlwapi)
