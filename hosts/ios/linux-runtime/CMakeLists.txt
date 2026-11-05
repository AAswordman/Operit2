cmake_minimum_required(VERSION 3.20)
project(OperitArm64Runtime VERSION 0.1.0 LANGUAGES C CXX ASM)

option(OPERIT_ARM64_LTO "Optimize Release builds across interpreter module boundaries" ON)
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

add_library(operit_arm64 STATIC
    src/acceleration/fixed_components.cpp
    src/api/c_api.cpp
    src/base/host_io.cpp
    src/base/random.cpp
    src/base/sha256.cpp
    src/cpu/aarch64/decoder.cpp
    src/cpu/aarch64/exclusive.cpp
    src/cpu/aarch64/execute.cpp
    src/cpu/aarch64/fixed_conversion.cpp
    src/cpu/aarch64/floating_point.cpp
    src/cpu/aarch64/interpreter.cpp
    src/cpu/aarch64/gadgets.cpp
    src/cpu/aarch64/gadgets.S
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
target_compile_features(operit_arm64 PRIVATE cxx_std_17)
target_include_directories(operit_arm64 PUBLIC include PRIVATE src)
target_compile_options(operit_arm64 PRIVATE -Wall -Wextra -Werror)
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

option(BUILD_TESTING "Build runtime regression tests" ON)
if(NOT CMAKE_SYSTEM_NAME STREQUAL "iOS")
    add_executable(operit-arm64 cli/main.cpp)
    target_compile_features(operit-arm64 PRIVATE cxx_std_17)
    target_link_libraries(operit-arm64 PRIVATE operit_arm64)
    target_compile_options(operit-arm64 PRIVATE -Wall -Wextra -Werror)
    include(CTest)
    if(BUILD_TESTING)
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
        add_runtime_test(operit-arm64-gadget-tests tests/gadgets.cpp arm64-gadgets)
        add_runtime_test(operit-arm64-filesystem-tests tests/filesystem.cpp arm64-filesystem)
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
            add_test(NAME arm64-alpine COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/test_alpine.py"
                --runtime "$<TARGET_FILE:operit-arm64>" --root "${OPERIT_ARM64_ROOTFS}"
                --fixed-tests "$<TARGET_FILE:operit-arm64-fixed-tests>")
            set_tests_properties(arm64-alpine PROPERTIES TIMEOUT 120)
        endif()
    endif()
endif()
