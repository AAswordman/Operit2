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
