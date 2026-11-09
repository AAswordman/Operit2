"""Execute real Linux ELF fixtures and compare A64 ALU results with Apple hardware."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(add_help=False)
parser.add_argument('--runtime', required=True)
parser.add_argument('--api-tests', required=True)
OPTIONS, UNITTEST_ARGS = parser.parse_known_args()


def tool(name, candidates):
    for candidate in [os.environ.get(name), *candidates]:
        if candidate and (Path(candidate).is_file() or shutil.which(candidate)):
            return os.path.abspath(shutil.which(candidate) or candidate)
    raise RuntimeError('missing tool: ' + name)


class RuntimeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='operit arm64 ')
        cls.addClassCleanup(cls.temp.cleanup)
        cls.directory = Path(cls.temp.name)
        cls.root = cls.directory / 'root'
        (cls.root / 'data').mkdir(parents=True)
        (cls.root / 'data/message').write_text('inner')
        (cls.root / 'secret').write_text('root-secret')
        (cls.directory / 'secret').write_text('outside-host-secret')
        (cls.root / 'absolute').symlink_to('/data')
        (cls.root / 'escape').symlink_to('../secret')
        (cls.root / 'loop').symlink_to('loop')
        cls.clang = tool('OPERIT_AARCH64_CLANG', ['/opt/homebrew/opt/llvm/bin/clang', 'clang'])
        cls.linker = tool('OPERIT_AARCH64_LLD', ['/opt/homebrew/opt/lld/bin/ld.lld', 'ld.lld'])
        flags = [cls.clang, '--target=aarch64-linux-gnu', '-O2', '-ffreestanding', '-nostdlib',
                 '-fno-builtin', '-fno-stack-protector', '-mno-outline-atomics',
                 '-fno-vectorize', '-fno-slp-vectorize', '--ld-path=' + cls.linker,
                 '-Wl,--build-id=none', '-Wl,-z,max-page-size=4096', '-I' + str(ROOT / 'guest')]
        cls.flags = flags
        def compile_fixture(name):
            sources = [str(ROOT / 'guest/start.S'), str(ROOT / ('guest/' + name + '.c'))]
            if name in ('alu', 'floating', 'threads', 'parallel', 'futex_requeue_pi', 'futex2', 'rseq'):
                assembly = 'threads' if name in ('parallel', 'futex_requeue_pi', 'futex2') else name
                sources.append(str(ROOT / ('guest/' + assembly + '.S')))
            return subprocess.run(flags + ['-static', '-no-pie', '-Wl,-Ttext=0x100004000', *sources,
                                   '-o', str(cls.root / name)], capture_output=True, timeout=30)
        with ThreadPoolExecutor(max_workers=2) as pool:
            compilations = list(pool.map(compile_fixture, ['hello', 'workload', 'alu', 'floating', 'threads', 'parallel', 'parallel_lifecycle', 'timers', 'inotify', 'resources', 'events', 'futex_requeue_pi', 'futex2', 'scheduling', 'rseq', 'shm_lifecycle', 'sysv_msg_lifecycle', 'sysv_sem_lifecycle', 'posix_mq_lifecycle', 'ipc_lifecycle', 'seccomp_lifecycle', 'io_uring_lifecycle']))
        failures = [result.stderr.decode() for result in compilations if result.returncode]
        if failures:
            raise RuntimeError('\n'.join(failures))
        cls.hello = (cls.root / 'hello').read_bytes()

    def run_guest(self, mode=None, *, executable='/workload', steps=10000000, memory=32, extra=(), cpus=None):
        command = [OPTIONS.runtime, '--root', str(self.root), '--stats', '--max-steps', str(steps),
                   '--memory-mib', str(memory), '--', executable]
        if cpus is not None:
            command[1:1] = ['--cpus', str(cpus)]
        if mode is not None: command.append(mode)
        command.extend(extra)
        result = subprocess.run(command, capture_output=True, timeout=20)
        lines = result.stderr.decode().splitlines()
        stats = json.loads(lines[-1]) if lines and lines[-1].startswith('{') else {}
        return result, stats

    def successful(self, mode, **kwargs):
        result, stats = self.run_guest(mode, **kwargs)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'workload-ok\n')
        self.assertEqual(stats['status'], 0)
        return stats

    def test_real_static_elf_above_four_gib(self):
        result, stats = self.run_guest(executable='/hello')
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'hello-arm64\n')
        self.assertGreater(stats['pc'], 1 << 32)

    def test_linux_initial_stack_and_auxv(self):
        self.successful('stack', extra=('argument with spaces',))

    def test_linux_rseq_auxv_after_exec(self):
        self.successful('exec-stack')

    def test_sparse_eight_gib_reservation_under_small_resident_limit(self):
        stats = self.successful('memory')
        self.assertLess(stats['resident_bytes'], 1 << 20)

    def test_file_io_stat_cwd_and_linux_errno_translation(self):
        self.successful('files')
        self.assertEqual((self.root / 'created').read_text(), 'persistent-arm64\n')

    def test_absolute_symlinks_and_parent_paths_stay_under_guest_root(self):
        self.successful('paths')
        self.assertEqual((self.directory / 'secret').read_text(), 'outside-host-secret')

    def test_modified_executable_pages_invalidate_decoded_blocks(self):
        self.successful('self-modify')

    def test_bad_syscall_pointers_return_efault_and_unknown_calls_return_enosys(self):
        stats = self.successful('bad-pointer')
        self.assertEqual(stats['unsupported_syscalls'], 1)
        self.assertEqual(stats['last_unsupported_syscall'], 999999)

    def test_linux_clock_and_random_layouts(self):
        self.successful('clock')

    def test_write_to_readonly_guest_memory_faults(self):
        result, stats = self.run_guest('fault')
        self.assertEqual(result.returncode, 128 + 11, result.stderr.decode())
        self.assertEqual(stats['status'], 0)
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_nonexecutable_guest_memory_cannot_execute(self):
        result, stats = self.run_guest('execute-denied')
        self.assertEqual(result.returncode, 128 + 11, result.stderr.decode())
        self.assertEqual(stats['status'], 0)
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_undefined_instruction_delivers_guest_sigill(self):
        result, stats = self.run_guest('undefined')
        self.assertEqual(result.returncode, 128 + 4, result.stderr.decode())
        self.assertEqual(stats['status'], 0)
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_wait_hint_still_requires_a_scheduler(self):
        result, stats = self.run_guest('wait-event')
        self.assertEqual(result.returncode, 125, result.stderr.decode())
        self.assertEqual(stats['status'], 3)
        self.assertIn(b'0xd503205f', result.stderr)

    def test_instruction_budget_bounds_loops_and_reuses_blocks(self):
        result, stats = self.run_guest('loop', steps=50000)
        self.assertEqual(result.returncode, 124, result.stderr.decode())
        self.assertEqual(stats['instructions'], 50000)
        self.assertGreater(stats['block_cache_hits'], stats['decoded_blocks'])
        # Regions can span many old basic blocks. Doubling executed work must
        # reuse compiled code rather than require proportionally more images.
        longer, long_stats = self.run_guest('loop', steps=100000)
        self.assertEqual(longer.returncode, 124, longer.stderr.decode())
        self.assertEqual(long_stats['instructions'], 100000)
        self.assertGreater(long_stats['block_cache_hits'], stats['block_cache_hits'])
        self.assertLessEqual(long_stats['decoded_blocks'], stats['decoded_blocks'] + 8)

    def test_vm_isolation_budget_resume_and_invalid_host_callback(self):
        result = subprocess.run([OPTIONS.api_tests, str(self.root)], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr.decode())

    def test_threads_share_memory_and_keep_tls_with_futex_join(self):
        result, stats = self.run_guest(executable='/threads')
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'threads-tls-futex-join-ok\n')
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_scheduling_attributes_validate_versions_limits_and_retained_state(self):
        result, stats = self.run_guest(executable='/scheduling')
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'scheduling-attributes-limits-versioning-ok\n')
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_pi_requeue_acquires_mutex_before_returning_on_one_and_four_cpus(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/futex_requeue_pi', steps=50000000, cpus=cpus)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'futex-requeue-pi-ownership-handoff-join-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_parallel_cpus_compute_and_synchronize_shared_memory(self):
        result, stats = self.run_guest(executable='/parallel', steps=50000000, cpus=4)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'parallel-compute-atomics-join-ok\n')
        self.assertEqual(stats['execution_threads'], 4)
        self.assertGreaterEqual(stats['maximum_parallel_tasks'], 2)
        self.assertEqual(stats['unsupported_syscalls'], 0)
        serial, baseline = self.run_guest(executable='/parallel', steps=50000000, cpus=1)
        self.assertEqual(serial.returncode, 0, serial.stderr.decode())
        self.assertEqual(serial.stdout, result.stdout)
        self.assertEqual(baseline['maximum_parallel_tasks'], 1)

    def test_process_timers_deliver_real_and_cpu_signals(self):
        result, stats = self.run_guest(executable='/timers', steps=50000000)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'process-timers-signals-cpu-ok\n')
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_epoll_pwait2_timespec_timer_wakeup_and_signal_mask(self):
        result, stats = self.run_guest(executable='/events')
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'epoll-pwait2-timespec-timer-mask-ok\n')
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_inotify_reports_files_rename_cookies_and_data_lifecycle(self):
        result, stats = self.run_guest(executable='/inotify')
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'inotify-files-rename-cookie-data-lifecycle-ok\n')
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_resource_limits_constrain_writes_tasks_and_queued_signals(self):
        result, stats = self.run_guest(executable='/resources')
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout, b'resource-filesize-nproc-sigpending-ok\n')
        self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_ipc_cross_process_blocking_and_resource_lifecycle(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/ipc_lifecycle', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'ipc-cross-process-lifecycle-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_sysv_message_queue_cross_process_lifecycle(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/sysv_msg_lifecycle', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'sysv-msg-cross-process-lifecycle-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_sysv_semaphore_undo_fork_clone_and_exec(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/sysv_sem_lifecycle', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'sysv-sem-undo-fork-clone-exec-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_posix_mq_poll_notification_and_exec(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/posix_mq_lifecycle', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'posix-mq-poll-notify-close-exec-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_seccomp_filter_inherits_across_fork_and_exec(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/seccomp_lifecycle', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'seccomp-filter-fork-exec-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_io_uring_mmap_submission_poll_and_file_io(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/io_uring_lifecycle', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'io-uring-mmap-submit-poll-file-io-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_rseq_registration_and_critical_section(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/rseq', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'rseq-guest-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_futex2_multicore_wait_wake_and_requeue(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/futex2', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'futex2-multicore-waitv-wake-requeue-race-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_sysv_shared_memory_fork_and_mapping_lifecycle(self):
        for cpus in (1, 4):
            with self.subTest(cpus=cpus):
                result, stats = self.run_guest(executable='/shm_lifecycle', cpus=cpus, steps=50000000)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, b'sysv-shm-fork-mapping-lifecycle-ok\n')
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_cpu_limit_terminates_actual_guest_computation(self):
        # Leave enough work to cross the one-second CPU limit on fast hosts.
        result, stats = self.run_guest('cpu', executable='/resources', steps=2000000000)
        self.assertEqual(result.returncode, 137, result.stderr.decode())
        self.assertEqual(stats['unsupported_syscalls'], 0)
        self.assertGreater(stats['instructions'], 0)

    def test_arithmetic_matches_native_apple_arm64_hardware(self):
        if platform.machine().lower() not in ('aarch64', 'arm64'):
            self.skipTest('requires native ARM64 hardware')
        native = self.directory / 'native-alu'
        host_clang = shutil.which('cc')
        subprocess.run([host_clang, '-O2', '-DOPERIT_NATIVE', str(ROOT / 'guest/alu.c'),
                        str(ROOT / 'guest/alu.S'), '-o', str(native)], check=True, capture_output=True, timeout=30)
        expected = subprocess.run([str(native)], check=True, capture_output=True, timeout=10).stdout
        result, _ = self.run_guest(executable='/alu')
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        for index, (actual, wanted) in enumerate(zip(struct.iter_unpack('<Q', result.stdout), struct.iter_unpack('<Q', expected))):
            self.assertEqual(actual, wanted, 'native ALU mismatch at result ' + str(index))
        self.assertEqual(len(result.stdout), len(expected))

    def test_floating_point_matches_hardware_rounding_exceptions_and_nan(self):
        if platform.machine().lower() not in ('aarch64', 'arm64'):
            self.skipTest('native ARM64 floating-point backend required')
        native = self.directory / 'native-floating'
        subprocess.run([shutil.which('cc'), '-O2', '-DOPERIT_NATIVE', str(ROOT / 'guest/floating.c'),
                        str(ROOT / 'guest/floating.S'), '-o', str(native)], check=True, capture_output=True, timeout=30)
        expected = subprocess.run([str(native)], check=True, capture_output=True, timeout=20).stdout
        result, _ = self.run_guest(executable='/floating', steps=10000000)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(len(result.stdout), len(expected))
        if result.stdout != expected:
            actual_words = struct.iter_unpack('<Q', result.stdout)
            expected_words = struct.iter_unpack('<Q', expected)
            for index, (actual, wanted) in enumerate(zip(actual_words, expected_words)):
                self.assertEqual(actual, wanted, 'FP mismatch at word ' + str(index))

    def test_bad_elf_headers_segments_and_entry_are_rejected(self):
        for name, offset, data in [
            ('x86',18,struct.pack('<H',3)), ('wrong-endian',5,b'\x02'),
            ('bad-phoff',32,struct.pack('<Q',(1<<64)-1)), ('bad-entry',24,struct.pack('<Q',0xdeadbeef)),
        ]:
            with self.subTest(name=name):
                damaged=bytearray(self.hello); damaged[offset:offset+len(data)]=data
                (self.root / name).write_bytes(damaged)
                result, _ = self.run_guest(executable='/' + name)
                self.assertEqual(result.returncode,126,result.stderr.decode())

    def test_truncated_elf_is_rejected(self):
        (self.root / 'truncated').write_bytes(self.hello[:63])
        result, _ = self.run_guest(executable='/truncated')
        self.assertEqual(result.returncode,126,result.stderr.decode())

    def test_guest_elf_cannot_escape_root_through_host_absolute_path(self):
        result, _ = self.run_guest(executable=str(self.root / 'hello'))
        self.assertEqual(result.returncode,126,result.stderr.decode())


if __name__ == '__main__':
    unittest.main(argv=[__file__, *UNITTEST_ARGS], verbosity=2)
