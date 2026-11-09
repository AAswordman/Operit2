cmake_minimum_required(VERSION 3.20)
project(OperitArm64Runtime VERSION 0.1.0 LANGUAGES C CXX ASM)

option(OPERIT_ARM64_GLOBAL_VALUES "Enable native SSA value numbering and global CSE" OFF)
option(OPERIT_ARM64_MEMORY_SCHEDULING "Schedule independent private-page native loads" OFF)
option(OPERIT_ARM64_DIRECT_CHAINS "Directly enter known block guards on static edges" OFF)
option(OPERIT_ARM64_DEFER_CALL_TARGETS "Discover direct call targets on first execution" OFF)
option(OPERIT_ARM64_RETAIN_LOOP_CALLS "Keep direct calls within discovered backward edges in the same graph" OFF)
option(OPERIT_ARM64_ADAPTIVE_CALLS "Promote hot cross-graph calls into their caller graph" OFF)
if(OPERIT_ARM64_RETAIN_LOOP_CALLS AND NOT OPERIT_ARM64_DEFER_CALL_TARGETS)
    message(FATAL_ERROR "OPERIT_ARM64_RETAIN_LOOP_CALLS requires OPERIT_ARM64_DEFER_CALL_TARGETS")
endif()
if(OPERIT_ARM64_ADAPTIVE_CALLS AND (NOT OPERIT_ARM64_DEFER_CALL_TARGETS OR OPERIT_ARM64_RETAIN_LOOP_CALLS OR OPERIT_ARM64_GLOBAL_VALUES))
    message(FATAL_ERROR "OPERIT_ARM64_ADAPTIVE_CALLS requires deferred calls and excludes loop retention and global values")
endif()
set(OPERIT_ARM64_SUPERBLOCK_PROFILE "" CACHE FILEPATH "Diagnostic profile for experimental build-time superblocks")
option(OPERIT_ARM64_SUPERBLOCK_SPECIALIZE "Specialize profiled operation fields and promote shadow values" OFF)
set(OPERIT_ARM64_GUEST_AOT_PROFILE "" CACHE FILEPATH "Profile for experimental build-time guest AOT")
set(OPERIT_ARM64_GUEST_AOT_EXTRA_PROFILES "" CACHE STRING "Additional independent profiles for build-time guest AOT")
set(OPERIT_ARM64_GUEST_AOT_ELFS "" CACHE STRING "Semicolon-separated ELF images for experimental build-time guest AOT")
set(OPERIT_ARM64_GUEST_AOT_LIMIT 1024 CACHE STRING "Maximum guest AOT sequences")
set(OPERIT_ARM64_GUEST_AOT_MAX_LENGTH 8 CACHE STRING "Maximum guest AOT sequence length")
option(OPERIT_ARM64_GUEST_AOT_SCORE_LENGTH "Rank guest AOT sequences by estimated eliminated dispatches" OFF)
option(OPERIT_ARM64_GUEST_AOT_DIVERSE "Defer guest AOT patterns contained in selected patterns" OFF)
option(OPERIT_ARM64_GUEST_AOT_COVERAGE "Rank guest AOT patterns by weighted profile edge coverage" OFF)
option(OPERIT_ARM64_GUEST_AOT_BRANCHES "Compile conditional branch tails in experimental guest AOT" OFF)
option(OPERIT_ARM64_GUEST_AOT_INDIRECT "Compile indirect branch tails in experimental guest AOT" OFF)
option(OPERIT_ARM64_GUEST_AOT_FOLDED "Allow folded guest operations and PC-relative addresses in guest AOT" OFF)
option(OPERIT_ARM64_GUEST_AOT_INLINE_MEMORY "Inline serial page-cache checks in guest AOT" OFF)
option(OPERIT_ARM64_GUEST_AOT_REGISTER_MEMORY "Compile register-offset integer memory operations in guest AOT" OFF)
option(OPERIT_ARM64_GUEST_AOT_TRACES "Compile profile-guided local fallthrough traces" OFF)
option(OPERIT_ARM64_GUEST_AOT_TRACE_REGISTER_MEMORY "Allow register-offset memory in guest AOT traces" OFF)
if(OPERIT_ARM64_GUEST_AOT_TRACE_REGISTER_MEMORY AND
   (NOT OPERIT_ARM64_GUEST_AOT_TRACES OR NOT OPERIT_ARM64_GUEST_AOT_REGISTER_MEMORY))
    message(FATAL_ERROR "Register memory traces require guest AOT traces and register memory")
endif()
set(OPERIT_ARM64_GUEST_AOT_TRACE_LIMIT 512 CACHE STRING "Maximum profile-guided guest AOT traces")
set(OPERIT_ARM64_GUEST_AOT_TRACE_MAX_BLOCKS 2 CACHE STRING "Maximum blocks in a profile-guided guest AOT trace")

option(OPERIT_ARM64_WASM "Enable the Apple WebKit Wasm execution backend" OFF)

option(OPERIT_EXECUTION_DIAGNOSTICS "Collect static executor handler, path and sampled cursor diagnostics" OFF)

option(OPERIT_ARM64_PGO_GENERATE "Instrument host C++ code for LLVM profile collection" OFF)
set(OPERIT_ARM64_PGO_PROFILE "" CACHE FILEPATH "Merged LLVM host-code profile for build-time optimization")
if(OPERIT_ARM64_PGO_GENERATE AND OPERIT_ARM64_PGO_PROFILE)
    message(FATAL_ERROR "Host PGO generation and profile use are mutually exclusive")
endif()
if((OPERIT_ARM64_PGO_GENERATE OR OPERIT_ARM64_PGO_PROFILE) AND NOT CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    message(FATAL_ERROR "Host PGO requires Clang's instrumentation profile format")
endif()
if(OPERIT_ARM64_PGO_PROFILE)
    if(NOT EXISTS "${OPERIT_ARM64_PGO_PROFILE}" OR IS_DIRECTORY "${OPERIT_ARM64_PGO_PROFILE}")
        message(FATAL_ERROR "Host PGO profile does not exist: ${OPERIT_ARM64_PGO_PROFILE}")
    endif()
    file(REAL_PATH "${OPERIT_ARM64_PGO_PROFILE}" OPERIT_ARM64_PGO_PROFILE_PATH)
    file(SHA256 "${OPERIT_ARM64_PGO_PROFILE_PATH}" OPERIT_ARM64_PGO_PROFILE_SHA)
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${OPERIT_ARM64_PGO_PROFILE_PATH}")
endif()
function(operit_host_pgo target)
    if(OPERIT_ARM64_PGO_GENERATE)
        target_compile_options(${target} PRIVATE $<$<COMPILE_LANGUAGE:CXX>:-fprofile-instr-generate>)
    elseif(OPERIT_ARM64_PGO_PROFILE)
        target_compile_options(${target} PRIVATE
            "$<$<COMPILE_LANGUAGE:CXX>:-fprofile-instr-use=${OPERIT_ARM64_PGO_PROFILE_PATH}>"
            $<$<COMPILE_LANGUAGE:CXX>:-Wno-profile-instr-unprofiled>)
        # The compiler does not list its profile in depfiles. Including its
        # digest in the command forces recompilation when the profile changes.
        target_compile_definitions(${target} PRIVATE "OPERIT_ARM64_PGO_PROFILE_SHA=\"${OPERIT_ARM64_PGO_PROFILE_SHA}\"")
    endif()
endfunction()

option(OPERIT_ARM64_LTO "Optimize Release builds across execution compiler module boundaries" ON)
# XCFramework packaging requires native Mach-O objects, not LLVM bitcode archives.
# The embedding app can choose its own final-link optimization policy.
if(OPERIT_ARM64_LTO AND NOT OPERIT_ARM64_SANITIZERS AND NOT CMAKE_SYSTEM_NAME STREQUAL "iOS")
    include(CheckIPOSupported)
    check_ipo_supported(RESULT OPERIT_ARM64_IPO_AVAILABLE OUTPUT OPERIT_ARM64_IPO_ERROR LANGUAGES CXX)
    if(OPERIT_ARM64_IPO_AVAILABLE)
        set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE TRUE)
    else()
        message(STATUS "Release LTO unavailable: ${OPERIT_ARM64_IPO_ERROR}")
    endif()
endif()

find_package(Python3 COMPONENTS Interpreter REQUIRED)
set(OPERIT_EXECUTOR_GENERATED "${CMAKE_CURRENT_BINARY_DIR}/static-executor")
add_custom_command(
    OUTPUT "${OPERIT_EXECUTOR_GENERATED}/static_entries.cpp"
           "${OPERIT_EXECUTOR_GENERATED}/static_handlers.inc"
           "${OPERIT_EXECUTOR_GENERATED}/static_tables.inc"
    COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tools/generate_static_executor.py"
        --definitions "${CMAKE_CURRENT_SOURCE_DIR}/src/cpu/aarch64/superoperators.def"
        --output "${OPERIT_EXECUTOR_GENERATED}"
    DEPENDS tools/generate_static_executor.py src/cpu/aarch64/superoperators.def
    VERBATIM)
set_source_files_properties(src/cpu/aarch64/static_executor.S PROPERTIES
    OBJECT_DEPENDS "${OPERIT_EXECUTOR_GENERATED}/static_handlers.inc;${OPERIT_EXECUTOR_GENERATED}/static_tables.inc")
add_library(operit_arm64 STATIC
    src/acceleration/fixed_components.cpp
    src/api/c_api.cpp
    src/base/host_io.cpp
    src/base/random.cpp
    src/base/sha256.cpp
    src/cpu/aarch64/decoder.cpp
    src/cpu/aarch64/exclusive.cpp
    src/cpu/aarch64/semantics.cpp
    src/cpu/aarch64/fixed_conversion.cpp
    src/cpu/aarch64/floating_point.cpp
    src/cpu/aarch64/block_graph.cpp
    src/cpu/aarch64/memory_effects.cpp
    src/cpu/aarch64/optimizer.cpp
    src/cpu/aarch64/global_values.cpp
    src/cpu/aarch64/static_lowering.cpp
    src/cpu/aarch64/wasm_region.cpp
    src/cpu/aarch64/wasm_executor.cpp
    src/cpu/aarch64/execution_engine.cpp
    src/cpu/aarch64/execution_diagnostics.cpp
    src/cpu/aarch64/static_executor.S
    "${OPERIT_EXECUTOR_GENERATED}/static_entries.cpp"
    src/cpu/aarch64/simd.cpp
    src/cpu/aarch64/vector_arithmetic.cpp
    src/cpu/aarch64/vector_floating.cpp
    src/cpu/aarch64/vector_fp_fixed.cpp
    src/cpu/aarch64/vector_fp_indexed.cpp
    src/cpu/aarch64/vector_integer.cpp
    src/cpu/aarch64/vector_indexed.cpp
    src/cpu/aarch64/vector_memory.cpp
    src/cpu/aarch64/vector_narrow.cpp
    src/cpu/aarch64/vector_shift.cpp
    src/cpu/aarch64/vector_unary.cpp
    src/cpu/aarch64/vector_widen.cpp
    src/elf/initial_stack.cpp
    src/elf/loader.cpp
    src/fs/descriptor_table.cpp
    src/fs/filesystem.cpp
    src/fs/mount/operations.cpp
    src/fs/mount/source.cpp
    src/fs/mount/storage.cpp
    src/fs/virtual/devfs.cpp
    src/fs/virtual/node.cpp
    src/fs/virtual/procfs.cpp
    src/linux/debug/ptrace.cpp
    src/linux/error_mapping.cpp
    src/linux/events/epoll.cpp
    src/linux/events/inotify.cpp
    src/linux/files/metadata.cpp
    src/linux/files/close_range.cpp
    src/linux/files/allocation.cpp
    src/linux/files/advice.cpp
    src/linux/files/open_how.cpp
    src/linux/files/memory_file.cpp
    src/linux/files/pipe_buffer.cpp
    src/linux/files/record_lock.cpp
    src/linux/files/vector_io.cpp
    src/linux/io_uring/io_uring.cpp
    src/linux/ipc/posix_mq.cpp
    src/linux/ipc/sysv_msg.cpp
    src/linux/ipc/sysv_sem.cpp
    src/linux/ipc/sysv_shared_memory.cpp
    src/linux/network/resolver.cpp
    src/linux/network/socket.cpp
    src/linux/network/message_batch.cpp
    src/linux/network/unix_namespace.cpp
    src/linux/namespaces/namespaces.cpp
    src/linux/process/console.cpp
    src/linux/process/control.cpp
    src/linux/process/pidfd.cpp
    src/linux/process/resource_limits.cpp
    src/linux/process/limit_enforcement.cpp
    src/linux/process/state.cpp
    src/linux/security/seccomp.cpp
    src/linux/signals/delivery.cpp
    src/linux/signals/frame.cpp
    src/linux/signals/state.cpp
    src/linux/signals/signal_file.cpp
    src/linux/syscall_descriptor.cpp
    src/linux/syscall_events.cpp
    src/linux/syscall_file.cpp
    src/linux/syscall_file_control.cpp
    src/linux/syscall_file_transfer.cpp
    src/linux/syscall_io.cpp
    src/linux/syscall_io_uring.cpp
    src/linux/syscall_ipc.cpp
    src/linux/syscall_identity.cpp
    src/linux/syscall_inotify.cpp
    src/linux/syscall_limits.cpp
    src/linux/syscall_memory.cpp
    src/linux/syscall_mount.cpp
    src/linux/syscall_namespace.cpp
    src/linux/syscall_path.cpp
    src/linux/syscall_open.cpp
    src/linux/syscall_poll.cpp
    src/linux/syscall_proc.cpp
    src/linux/syscall_process.cpp
    src/linux/syscall_process_vm.cpp
    src/linux/syscall_signal.cpp
    src/linux/syscall_socket.cpp
    src/linux/syscall_socket_message.cpp
    src/linux/syscall_socket_waitall.cpp
    src/linux/syscall_system.cpp
    src/linux/system/clocks.cpp
    src/linux/system/information.cpp
    src/linux/syscall_terminal.cpp
    src/linux/syscall_threads.cpp
    src/linux/syscall_timer.cpp
    src/linux/syscall_virtual.cpp
    src/linux/syscalls.cpp
    src/linux/terminal/manager.cpp
    src/linux/terminal/software_pty.cpp
    src/linux/terminal/termios.cpp
    src/linux/timers/host_events.cpp
    src/linux/threads/futex_pi.cpp
    src/linux/threads/rseq.cpp
    src/linux/timers/timer_file.cpp
    src/linux/timers/process_timers.cpp
    src/linux/timers/timer_service.cpp
    src/runtime/console/console.cpp
    src/runtime/console/session.cpp
    src/runtime/machine.cpp
    src/runtime/parallel.cpp
    src/runtime/process/image_loader.cpp
    src/runtime/process/accounting.cpp
    src/runtime/process/lifecycle.cpp
    src/runtime/process/observability.cpp
    src/runtime/process/ptrace.cpp
    src/runtime/process/signals.cpp
    src/runtime/process/timers.cpp
    src/runtime/process/wait.cpp
    src/runtime/tasks/scheduler.cpp
    src/runtime/tasks/executor.cpp
)
target_compile_definitions(operit_arm64 PUBLIC OPERIT_EXECUTION_DIAGNOSTICS=$<BOOL:${OPERIT_EXECUTION_DIAGNOSTICS}>)
target_compile_definitions(operit_arm64 PRIVATE OPERIT_ARM64_WASM=$<BOOL:${OPERIT_ARM64_WASM}>)
if(OPERIT_ARM64_GUEST_AOT_PROFILE OR OPERIT_ARM64_GUEST_AOT_EXTRA_PROFILES OR OPERIT_ARM64_GUEST_AOT_ELFS)
    set(OPERIT_GUEST_AOT_RANKING_ARGS)
    set(OPERIT_GUEST_AOT_INPUT_ARGS)
    set(OPERIT_GUEST_AOT_INPUT_FILES)
    if(OPERIT_ARM64_GUEST_AOT_SCORE_LENGTH)
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --score-length)
    endif()
    if(OPERIT_ARM64_GUEST_AOT_DIVERSE)
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --diverse)
    endif()
    if(OPERIT_ARM64_GUEST_AOT_COVERAGE)
        if(OPERIT_ARM64_GUEST_AOT_DIVERSE OR OPERIT_ARM64_GUEST_AOT_ELFS)
            message(FATAL_ERROR "Guest AOT coverage ranking requires profile-only input and cannot be combined with diverse ranking")
        endif()
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --coverage)
    endif()
    if(OPERIT_ARM64_GUEST_AOT_BRANCHES)
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --branches)
    endif()
    if(OPERIT_ARM64_GUEST_AOT_INDIRECT)
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --indirect)
    endif()
    if(OPERIT_ARM64_GUEST_AOT_FOLDED)
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --folded)
    endif()
    if(OPERIT_ARM64_GUEST_AOT_INLINE_MEMORY)
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --inline-memory)
    endif()
    if(OPERIT_ARM64_GUEST_AOT_REGISTER_MEMORY)
        list(APPEND OPERIT_GUEST_AOT_RANKING_ARGS --register-memory)
    endif()
    if(OPERIT_ARM64_GLOBAL_VALUES OR OPERIT_ARM64_MEMORY_SCHEDULING)
        message(FATAL_ERROR "Guest AOT currently requires global values and memory scheduling to be disabled")
    endif()
    set(OPERIT_GUEST_AOT_PROFILE_ARGS)
    set(OPERIT_GUEST_AOT_PROFILES ${OPERIT_ARM64_GUEST_AOT_PROFILE} ${OPERIT_ARM64_GUEST_AOT_EXTRA_PROFILES})
    list(REMOVE_DUPLICATES OPERIT_GUEST_AOT_PROFILES)
    foreach(profile IN LISTS OPERIT_GUEST_AOT_PROFILES)
        if(NOT EXISTS "${profile}")
            message(FATAL_ERROR "Guest AOT profile does not exist: ${profile}")
        endif()
        list(APPEND OPERIT_GUEST_AOT_PROFILE_ARGS --profile "${profile}")
        list(APPEND OPERIT_GUEST_AOT_INPUT_FILES "${profile}")
    endforeach()
    list(APPEND OPERIT_GUEST_AOT_INPUT_ARGS ${OPERIT_GUEST_AOT_PROFILE_ARGS})
    foreach(elf IN LISTS OPERIT_ARM64_GUEST_AOT_ELFS)
        if(NOT EXISTS "${elf}")
            message(FATAL_ERROR "Guest AOT ELF does not exist: ${elf}")
        endif()
        list(APPEND OPERIT_GUEST_AOT_INPUT_ARGS --elf "${elf}")
        list(APPEND OPERIT_GUEST_AOT_INPUT_FILES "${elf}")
    endforeach()
    add_custom_command(
        OUTPUT "${OPERIT_EXECUTOR_GENERATED}/guest_aot.inc" "${OPERIT_EXECUTOR_GENERATED}/guest_aot.cpp"
        COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tools/generate_guest_aot.py"
            ${OPERIT_GUEST_AOT_INPUT_ARGS} --output "${OPERIT_EXECUTOR_GENERATED}"
            --limit "${OPERIT_ARM64_GUEST_AOT_LIMIT}" --max-length "${OPERIT_ARM64_GUEST_AOT_MAX_LENGTH}"
            ${OPERIT_GUEST_AOT_RANKING_ARGS}
        DEPENDS tools/generate_guest_aot.py ${OPERIT_GUEST_AOT_INPUT_FILES} VERBATIM)
    set_property(SOURCE src/cpu/aarch64/static_executor.S APPEND PROPERTY OBJECT_DEPENDS
        "${OPERIT_EXECUTOR_GENERATED}/guest_aot.inc")
    target_sources(operit_arm64 PRIVATE "${OPERIT_EXECUTOR_GENERATED}/guest_aot.cpp")
    target_compile_definitions(operit_arm64 PRIVATE OPERIT_ARM64_GUEST_AOT=1)
    if(OPERIT_ARM64_GUEST_AOT_TRACES)
        if(NOT OPERIT_GUEST_AOT_PROFILES)
            message(FATAL_ERROR "Guest AOT traces require a diagnostic profile")
        endif()
        set(OPERIT_GUEST_TRACE_ARGS)
        if(OPERIT_ARM64_GUEST_AOT_FOLDED)
            list(APPEND OPERIT_GUEST_TRACE_ARGS --folded)
        endif()
        if(OPERIT_ARM64_GUEST_AOT_INLINE_MEMORY)
            list(APPEND OPERIT_GUEST_TRACE_ARGS --inline-memory)
        endif()
        if(OPERIT_ARM64_GUEST_AOT_TRACE_REGISTER_MEMORY)
            list(APPEND OPERIT_GUEST_TRACE_ARGS --register-memory)
        endif()
        add_custom_command(
            OUTPUT "${OPERIT_EXECUTOR_GENERATED}/guest_traces.inc" "${OPERIT_EXECUTOR_GENERATED}/guest_traces.cpp"
            COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tools/generate_guest_traces.py"
                ${OPERIT_GUEST_AOT_PROFILE_ARGS} --output "${OPERIT_EXECUTOR_GENERATED}"
                --limit "${OPERIT_ARM64_GUEST_AOT_TRACE_LIMIT}"
                --max-length "${OPERIT_ARM64_GUEST_AOT_MAX_LENGTH}"
                --max-blocks "${OPERIT_ARM64_GUEST_AOT_TRACE_MAX_BLOCKS}" ${OPERIT_GUEST_TRACE_ARGS}
            DEPENDS tools/generate_guest_traces.py tools/generate_guest_aot.py ${OPERIT_GUEST_AOT_INPUT_FILES} VERBATIM)
        set_property(SOURCE src/cpu/aarch64/static_executor.S APPEND PROPERTY OBJECT_DEPENDS
            "${OPERIT_EXECUTOR_GENERATED}/guest_traces.inc")
        target_sources(operit_arm64 PRIVATE "${OPERIT_EXECUTOR_GENERATED}/guest_traces.cpp")
        target_compile_definitions(operit_arm64 PRIVATE OPERIT_ARM64_GUEST_AOT_TRACES=1)
    endif()
endif()
if(OPERIT_ARM64_SUPERBLOCK_PROFILE)
    if(NOT EXISTS "${OPERIT_ARM64_SUPERBLOCK_PROFILE}")
        message(FATAL_ERROR "Superblock profile does not exist: ${OPERIT_ARM64_SUPERBLOCK_PROFILE}")
    endif()
    set(OPERIT_PROFILE_SPECIALIZE_ARGS)
    if(OPERIT_ARM64_SUPERBLOCK_SPECIALIZE)
        list(APPEND OPERIT_PROFILE_SPECIALIZE_ARGS --specialize)
    endif()
    add_custom_command(
        OUTPUT "${OPERIT_EXECUTOR_GENERATED}/profile_superblocks.inc"
               "${OPERIT_EXECUTOR_GENERATED}/profile_superblocks.cpp"
        COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tools/generate_profile_superblocks.py"
            --profile "${OPERIT_ARM64_SUPERBLOCK_PROFILE}"
            --handlers "${OPERIT_EXECUTOR_GENERATED}/static_handlers.inc"
            --output "${OPERIT_EXECUTOR_GENERATED}"
            ${OPERIT_PROFILE_SPECIALIZE_ARGS}
        DEPENDS tools/generate_profile_superblocks.py "${OPERIT_ARM64_SUPERBLOCK_PROFILE}"
                "${OPERIT_EXECUTOR_GENERATED}/static_handlers.inc"
        VERBATIM)
    set_property(SOURCE src/cpu/aarch64/static_executor.S APPEND PROPERTY OBJECT_DEPENDS
        "${OPERIT_EXECUTOR_GENERATED}/profile_superblocks.inc")
    target_sources(operit_arm64 PRIVATE "${OPERIT_EXECUTOR_GENERATED}/profile_superblocks.cpp")
    target_compile_definitions(operit_arm64 PRIVATE OPERIT_ARM64_PROFILE_SUPERBLOCKS=1)
elseif(OPERIT_ARM64_SUPERBLOCK_SPECIALIZE)
    message(FATAL_ERROR "OPERIT_ARM64_SUPERBLOCK_SPECIALIZE requires OPERIT_ARM64_SUPERBLOCK_PROFILE")
endif()
target_compile_features(operit_arm64 PRIVATE cxx_std_17)
target_include_directories(operit_arm64 PUBLIC include PRIVATE src "${OPERIT_EXECUTOR_GENERATED}")
if(APPLE AND OPERIT_ARM64_WASM)
    enable_language(OBJCXX)
    target_sources(operit_arm64 PRIVATE src/cpu/aarch64/wasm_webkit.mm)
    set_source_files_properties(src/cpu/aarch64/wasm_webkit.mm PROPERTIES COMPILE_OPTIONS "-fobjc-arc" COMPILE_FLAGS "-std=c++17")
    target_compile_definitions(operit_arm64 PRIVATE OPERIT_WASM_WEBKIT=1)
    target_link_libraries(operit_arm64 PRIVATE "-framework Foundation" "-framework WebKit")
endif()
target_compile_options(operit_arm64 PRIVATE -Wall -Wextra -Werror)
operit_host_pgo(operit_arm64)
if(OPERIT_ARM64_PGO_GENERATE)
    # Consumers of the instrumented static archive need the profiling runtime.
    target_link_options(operit_arm64 INTERFACE -fprofile-instr-generate)
endif()
set_target_properties(operit_arm64 PROPERTIES POSITION_INDEPENDENT_CODE ON)
find_package(Threads REQUIRED)
target_link_libraries(operit_arm64 PRIVATE resolv Threads::Threads)
if(NOT APPLE)
    find_package(OpenSSL REQUIRED COMPONENTS Crypto)
    target_link_libraries(operit_arm64 PRIVATE OpenSSL::Crypto)
endif()

option(OPERIT_ARM64_SANITIZERS "Enable host ASan and UBSan" OFF)
if(OPERIT_ARM64_SANITIZERS)
    target_compile_options(operit_arm64 PUBLIC -fsanitize=address,undefined -fno-omit-frame-pointer)
    target_link_options(operit_arm64 PUBLIC -fsanitize=address,undefined)
endif()

target_compile_definitions(operit_arm64 PUBLIC
    OPERIT_ARM64_GLOBAL_VALUES=$<BOOL:${OPERIT_ARM64_GLOBAL_VALUES}>
    OPERIT_ARM64_MEMORY_SCHEDULING=$<BOOL:${OPERIT_ARM64_MEMORY_SCHEDULING}>
    OPERIT_ARM64_DIRECT_CHAINS=$<BOOL:${OPERIT_ARM64_DIRECT_CHAINS}>
    OPERIT_ARM64_DEFER_CALL_TARGETS=$<BOOL:${OPERIT_ARM64_DEFER_CALL_TARGETS}>
    OPERIT_ARM64_RETAIN_LOOP_CALLS=$<BOOL:${OPERIT_ARM64_RETAIN_LOOP_CALLS}>
    OPERIT_ARM64_ADAPTIVE_CALLS=$<BOOL:${OPERIT_ARM64_ADAPTIVE_CALLS}>)

option(BUILD_TESTING "Build runtime regression tests" ON)
if(NOT CMAKE_SYSTEM_NAME STREQUAL "iOS")
    if(APPLE AND OPERIT_ARM64_WASM)
        add_library(operit-wasm-cli OBJECT cli/main.cpp)
        target_compile_features(operit-wasm-cli PRIVATE cxx_std_17)
        target_include_directories(operit-wasm-cli PRIVATE include)
        target_compile_definitions(operit-wasm-cli PRIVATE main=operit_wasm_cli_main)
        add_executable(operit-arm64-wasm tools/wasm_region/integration.mm $<TARGET_OBJECTS:operit-wasm-cli>)
        target_compile_features(operit-arm64-wasm PRIVATE cxx_std_17)
        target_include_directories(operit-arm64-wasm PRIVATE src)
        target_compile_options(operit-arm64-wasm PRIVATE -fobjc-arc)
        target_link_libraries(operit-arm64-wasm PRIVATE operit_arm64 "-framework Cocoa")
    endif()
    add_executable(operit-arm64 cli/main.cpp)
    target_compile_features(operit-arm64 PRIVATE cxx_std_17)
    target_link_libraries(operit-arm64 PRIVATE operit_arm64)
    target_compile_options(operit-arm64 PRIVATE -Wall -Wextra -Werror)
    operit_host_pgo(operit-arm64)
    add_executable(operit-arm64-execution-bench benchmarks/execution.cpp)
    target_compile_features(operit-arm64-execution-bench PRIVATE cxx_std_17)
    target_include_directories(operit-arm64-execution-bench PRIVATE src)
    target_link_libraries(operit-arm64-execution-bench PRIVATE operit_arm64)
    target_compile_options(operit-arm64-execution-bench PRIVATE -Wall -Wextra -Werror)
    add_executable(operit-arm64-global-opt-bench benchmarks/global_optimizer.cpp)
    target_compile_features(operit-arm64-global-opt-bench PRIVATE cxx_std_17)
    target_include_directories(operit-arm64-global-opt-bench PRIVATE src)
    target_link_libraries(operit-arm64-global-opt-bench PRIVATE operit_arm64)
    target_compile_options(operit-arm64-global-opt-bench PRIVATE -Wall -Wextra -Werror)
    include(CTest)
    if(BUILD_TESTING)
        if(APPLE AND OPERIT_ARM64_WASM)
            add_test(NAME arm64-wasm COMMAND operit-arm64-wasm)
            set_tests_properties(arm64-wasm PROPERTIES TIMEOUT 60)
        endif()
        find_package(Python3 COMPONENTS Interpreter REQUIRED)
        function(add_runtime_test target source test_name)
            add_executable(${target} ${source})
            target_compile_features(${target} PRIVATE cxx_std_17)
            target_include_directories(${target} PRIVATE src)
            target_link_libraries(${target} PRIVATE operit_arm64)
            add_test(NAME ${test_name} COMMAND ${target})
            set_tests_properties(${test_name} PROPERTIES TIMEOUT 15)
        endfunction()
        add_executable(operit-arm64-cpu-validation tests/cpu_validation.cpp)
        target_compile_features(operit-arm64-cpu-validation PRIVATE cxx_std_17)
        target_include_directories(operit-arm64-cpu-validation PRIVATE src)
        target_link_libraries(operit-arm64-cpu-validation PRIVATE operit_arm64)
        add_test(NAME arm64-cpu-validation COMMAND operit-arm64-cpu-validation)
        add_test(NAME arm64-cache COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_cache.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-cache PROPERTIES TIMEOUT 30)
        add_executable(operit-arm64-memory-concurrency-tests tests/memory_concurrency.cpp)
        target_compile_features(operit-arm64-memory-concurrency-tests PRIVATE cxx_std_17)
        target_include_directories(operit-arm64-memory-concurrency-tests PRIVATE src)
        target_link_libraries(operit-arm64-memory-concurrency-tests PRIVATE operit_arm64 Threads::Threads)
        add_test(NAME arm64-memory-concurrency COMMAND operit-arm64-memory-concurrency-tests)
        set_tests_properties(arm64-memory-concurrency PROPERTIES TIMEOUT 30)
        add_test(NAME arm64-worker-profile-validation
            COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_worker_profile.py")
        set_tests_properties(arm64-worker-profile-validation PROPERTIES TIMEOUT 10)
        add_test(NAME arm64-profile-superblock-generator
            COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_profile_superblocks.py")
        set_tests_properties(arm64-profile-superblock-generator PROPERTIES TIMEOUT 10)
        add_test(NAME arm64-guest-aot-generator
            COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_guest_aot.py")
        set_tests_properties(arm64-guest-aot-generator PROPERTIES TIMEOUT 10)
        add_runtime_test(operit-arm64-execution-tests tests/execution_backend.cpp arm64-execution)
        if(OPERIT_ARM64_GUEST_AOT_PROFILE OR OPERIT_ARM64_GUEST_AOT_EXTRA_PROFILES OR OPERIT_ARM64_GUEST_AOT_ELFS)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT=1)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT_BRANCHES=$<BOOL:${OPERIT_ARM64_GUEST_AOT_BRANCHES}>)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT_INDIRECT=$<BOOL:${OPERIT_ARM64_GUEST_AOT_INDIRECT}>)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT_REGISTER_MEMORY=$<BOOL:${OPERIT_ARM64_GUEST_AOT_REGISTER_MEMORY}>)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT_TRACE_REGISTER_MEMORY=$<BOOL:${OPERIT_ARM64_GUEST_AOT_TRACE_REGISTER_MEMORY}>)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT_TRACES=$<BOOL:${OPERIT_ARM64_GUEST_AOT_TRACES}>)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT_TRACE_MAX_BLOCKS=${OPERIT_ARM64_GUEST_AOT_TRACE_MAX_BLOCKS})
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_GUEST_AOT_FOLDED=$<BOOL:${OPERIT_ARM64_GUEST_AOT_FOLDED}>)
        endif()
        if(OPERIT_ARM64_SUPERBLOCK_PROFILE)
            target_compile_definitions(operit-arm64-execution-tests PRIVATE OPERIT_ARM64_PROFILE_SUPERBLOCKS=1)
        endif()
        add_runtime_test(operit-arm64-memory-code-validation-tests tests/memory_code_validation.cpp arm64-memory-code-validation)
        add_runtime_test(operit-arm64-filesystem-tests tests/filesystem.cpp arm64-filesystem)
        add_runtime_test(operit-arm64-descriptor-budget-tests tests/descriptor_budget.cpp arm64-descriptor-budget)
        if(APPLE)
            add_test(NAME arm64-bind-sandbox
                COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_bind_sandbox.py"
                --fixture "$<TARGET_FILE:operit-arm64-filesystem-tests>")
            set_tests_properties(arm64-bind-sandbox PROPERTIES TIMEOUT 30)
        endif()
        add_runtime_test(operit-arm64-terminal-discipline-tests tests/terminal_discipline.cpp arm64-terminal-discipline)
        add_runtime_test(operit-arm64-terminal-syscall-tests tests/terminal_syscalls.cpp arm64-terminal-syscalls)
        if(APPLE)
            add_test(NAME arm64-terminal-sandbox
                COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_terminal_sandbox.py"
                --fixture "$<TARGET_FILE:operit-arm64-terminal-discipline-tests>"
                --syscalls "$<TARGET_FILE:operit-arm64-terminal-syscall-tests>")
            set_tests_properties(arm64-terminal-sandbox PROPERTIES TIMEOUT 45)
        endif()
        add_runtime_test(operit-arm64-vfs-tests tests/virtual_filesystem.cpp arm64-virtual-filesystem)
        add_runtime_test(operit-arm64-inotify-tests tests/inotify.cpp arm64-inotify)
        add_runtime_test(operit-arm64-network-tests tests/network.cpp arm64-network)
        add_runtime_test(operit-arm64-futex-pi-tests tests/futex_pi.cpp arm64-futex-pi)
        add_runtime_test(operit-arm64-futex2-tests tests/futex2.cpp arm64-futex2)
        add_runtime_test(operit-arm64-rseq-tests tests/rseq.cpp arm64-rseq)
        add_runtime_test(operit-arm64-seccomp-tests tests/seccomp.cpp arm64-seccomp)
        add_runtime_test(operit-arm64-namespace-tests tests/namespaces.cpp arm64-namespaces)
        add_runtime_test(operit-arm64-io-uring-tests tests/io_uring.cpp arm64-io-uring)
        add_runtime_test(operit-arm64-memory-async-lease-tests tests/memory_async_lease.cpp
            arm64-memory-async-lease)
        add_test(NAME arm64-io-uring-async-guest
            COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/guest/io_uring_async_test.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-io-uring-async-guest PROPERTIES TIMEOUT 900)
        add_test(NAME arm64-io-uring-wait-guest
            COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/guest/io_uring_wait_test.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-io-uring-wait-guest PROPERTIES TIMEOUT 120)
        add_runtime_test(operit-arm64-sysv-msg-tests tests/sysv_msg.cpp arm64-sysv-msg)
        add_runtime_test(operit-arm64-sysv-sem-tests tests/sysv_sem.cpp arm64-sysv-sem)
        add_runtime_test(operit-arm64-sysv-shm-tests tests/sysv_shared_memory.cpp arm64-sysv-shm)
        add_runtime_test(operit-arm64-posix-mq-tests tests/posix_mq.cpp arm64-posix-mq)
        add_runtime_test(operit-arm64-posix-mq-epoll-tests tests/posix_mq_epoll.cpp arm64-posix-mq-epoll)
        add_runtime_test(operit-arm64-ptrace-tests tests/ptrace.cpp arm64-ptrace)
        add_runtime_test(operit-arm64-ptrace-memory-tests tests/ptrace_memory.cpp arm64-ptrace-memory)
        add_test(NAME arm64-ptrace-guest COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/ptrace_guest.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-ptrace-guest PROPERTIES TIMEOUT 90)
        add_test(NAME arm64-ptrace-lifecycle-guest COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/ptrace_lifecycle_guest.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-ptrace-lifecycle-guest PROPERTIES TIMEOUT 120)
        add_test(NAME arm64-ptrace-clone-x8-guest COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/ptrace_clone_x8_guest.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-ptrace-clone-x8-guest PROPERTIES TIMEOUT 120)
        add_test(NAME arm64-ptrace-threads-exit-guest COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/ptrace_threads_exit_guest.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-ptrace-threads-exit-guest PROPERTIES TIMEOUT 120)
        add_test(NAME arm64-seccomp-guest COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/seccomp_guest.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-seccomp-guest PROPERTIES TIMEOUT 120)
        add_test(NAME arm64-namespaces-guest COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/namespaces_integration.py"
            --runtime "$<TARGET_FILE:operit-arm64>")
        set_tests_properties(arm64-namespaces-guest PROPERTIES TIMEOUT 90)
        add_runtime_test(operit-arm64-process-vm-tests tests/process_vm.cpp arm64-process-vm)
        add_runtime_test(operit-arm64-path-extensions-tests tests/path_extensions.cpp arm64-path-extensions)
        add_runtime_test(operit-arm64-identity-tests tests/identity.cpp arm64-identity)
        add_runtime_test(operit-arm64-scheduling-tests tests/scheduling.cpp arm64-scheduling)
        add_test(NAME arm64-realtime COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_realtime.py"
            --api-tests $<TARGET_FILE:operit-arm64-scheduling-tests>)
        set_tests_properties(arm64-realtime PROPERTIES TIMEOUT 30)
        add_runtime_test(operit-arm64-task-tests tests/tasks.cpp arm64-tasks)
        if(CMAKE_HOST_SYSTEM_PROCESSOR MATCHES "^(arm64|aarch64)$")
            add_test(NAME arm64-isa-audit COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/isa/audit.py"
                --runtime "$<TARGET_FILE:operit-arm64>" --report "${CMAKE_CURRENT_BINARY_DIR}/isa-audit.json" --register-aliases --require-passing)
            set_tests_properties(arm64-isa-audit PROPERTIES TIMEOUT 120)
        endif()
        add_executable(operit-arm64-api-tests tests/api.cpp)
        target_compile_features(operit-arm64-api-tests PRIVATE cxx_std_17)
        target_link_libraries(operit-arm64-api-tests PRIVATE operit_arm64)
        add_test(NAME arm64-runtime COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_runtime.py"
            --runtime "$<TARGET_FILE:operit-arm64>" --api-tests "$<TARGET_FILE:operit-arm64-api-tests>")
        set_tests_properties(arm64-runtime PROPERTIES TIMEOUT 120)
        add_executable(operit-arm64-fixed-tests tests/fixed_components.cpp)
        target_compile_features(operit-arm64-fixed-tests PRIVATE cxx_std_17)
        target_include_directories(operit-arm64-fixed-tests PRIVATE src)
        target_link_libraries(operit-arm64-fixed-tests PRIVATE operit_arm64)
        add_executable(operit-arm64-terminal-tests tests/terminal.cpp)
        target_compile_features(operit-arm64-terminal-tests PRIVATE cxx_std_17)
        target_link_libraries(operit-arm64-terminal-tests PRIVATE operit_arm64)
        set(OPERIT_ARM64_ROOTFS "" CACHE PATH "Pinned Alpine AArch64 root for integration tests")
        if(OPERIT_ARM64_ROOTFS)
            add_test(NAME arm64-terminal COMMAND operit-arm64-terminal-tests "${OPERIT_ARM64_ROOTFS}")
            set_tests_properties(arm64-terminal PROPERTIES TIMEOUT 45)
            add_test(NAME arm64-terminal-descriptor-budget
                COMMAND operit-arm64-terminal-tests "${OPERIT_ARM64_ROOTFS}" --descriptor-budget)
            set_tests_properties(arm64-terminal-descriptor-budget PROPERTIES TIMEOUT 45)
            if(APPLE)
                add_test(NAME arm64-terminal-shell-sandbox
                    COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_terminal_sandbox.py"
                    --fixture "$<TARGET_FILE:operit-arm64-terminal-discipline-tests>"
                    --terminal "$<TARGET_FILE:operit-arm64-terminal-tests>" --root "${OPERIT_ARM64_ROOTFS}")
                set_tests_properties(arm64-terminal-shell-sandbox PROPERTIES TIMEOUT 60)
            endif()
            add_test(NAME arm64-alpine COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_alpine.py"
                --runtime "$<TARGET_FILE:operit-arm64>" --root "${OPERIT_ARM64_ROOTFS}"
                --fixed-tests "$<TARGET_FILE:operit-arm64-fixed-tests>")
            set_tests_properties(arm64-alpine PROPERTIES TIMEOUT 120)
        endif()
    endif()
endif()
