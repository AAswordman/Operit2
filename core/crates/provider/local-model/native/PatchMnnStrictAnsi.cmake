# Removes MNN's unconditional Linux -D__STRICT_ANSI__ flag from its fetched
# CMakeLists.txt. The flag is applied on top of a gnu C++ dialect, where
# __GLIBCXX_TYPE_INT_N_0 is already defined; GCC 16's libstdc++ additionally
# provides the __int128 std specializations in strict mode, so both branches
# fire and every MNN translation unit fails with redefinition errors.
# Runs inside the fetched MNN source directory (FetchContent PATCH_COMMAND).
if(NOT EXISTS "CMakeLists.txt")
    message(FATAL_ERROR "PatchMnnStrictAnsi must run inside the fetched MNN source directory")
endif()
file(READ "CMakeLists.txt" mnnListsContent)
set(strictAnsiLine [==[set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -D__STRICT_ANSI__")]==])
string(FIND "${mnnListsContent}" "${strictAnsiLine}" strictAnsiOffset)
if(strictAnsiOffset EQUAL -1)
    message(STATUS "MNN CMakeLists has no -D__STRICT_ANSI__ flag; nothing to patch")
else()
    string(REPLACE "${strictAnsiLine}" "" mnnListsContent "${mnnListsContent}")
    file(WRITE "CMakeLists.txt" "${mnnListsContent}")
    message(STATUS "Removed MNN Linux -D__STRICT_ANSI__ flag")
endif()

# Neutralizes the Android-only POST_BUILD custom command on the `llm` OBJECT
# library in transformers/llm/engine/CMakeLists.txt. When MNN_SEP_BUILD=OFF
# (this project forces it), `llm` is an OBJECT library, and CMake rejects
# PRE_BUILD/PRE_LINK/POST_BUILD commands on OBJECT targets, aborting the
# Android configure step. The command only copies engine headers into
# NATIVE_INCLUDE_OUTPUT for MNN's own Android gradle builds; this project
# includes ${mnn_SOURCE_DIR}/transformers/llm/engine/include directly, so the
# copy is unnecessary here. Windows/macOS/Linux builds take the INSTALL branch
# and are unaffected.
set(llmEngineCMake "transformers/llm/engine/CMakeLists.txt")
if(EXISTS "${llmEngineCMake}")
    file(READ "${llmEngineCMake}" llmEngineContent)
    set(llmPostBuildBlock [==[add_custom_command(
  TARGET llm
  POST_BUILD
  COMMAND ${CMAKE_COMMAND}
  ARGS -E copy_directory ${CMAKE_CURRENT_LIST_DIR}/include ${NATIVE_INCLUDE_OUTPUT}
)]==])
    string(FIND "${llmEngineContent}" "${llmPostBuildBlock}" llmPostBuildOffset)
    if(llmPostBuildOffset EQUAL -1)
        message(STATUS "MNN llm engine CMakeLists has no llm POST_BUILD block; nothing to patch")
    else()
        string(REPLACE "${llmPostBuildBlock}" "# operit: llm POST_BUILD header copy removed (OBJECT library)" llmEngineContent "${llmEngineContent}")
        file(WRITE "${llmEngineCMake}" "${llmEngineContent}")
        message(STATUS "Removed MNN llm POST_BUILD command on Android OBJECT library")
    endif()
else()
    message(STATUS "MNN llm engine CMakeLists not found; nothing to patch")
endif()
