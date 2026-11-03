#include "linux/syscalls.h"
#include "memory/fault.h"
#include "runtime/process/controller.h"
#include <fcntl.h>

namespace operit::arm64 {
Syscalls::Syscalls(Memory &memory, Filesystem &filesystem, const ort_arm64_config &config,
                   Scheduler &scheduler)
    : memory_(memory), filesystem_(filesystem), config_(config), terminals_(std::make_shared<TerminalManager>()),
      scheduler_(scheduler), message_queue_namespace_(ipc::make_message_queue_namespace(scheduler)),
      semaphore_namespace_(ipc::make_semaphore_namespace(scheduler)) {
    for (int fd = 0; fd < 3; ++fd) {
        int duplicate = ::fcntl(fd, F_DUPFD_CLOEXEC, 0);
        if (duplicate < 0 && !((fd == 0 && config.read) || (fd != 0 && config.write)))
            throw Failure("cannot duplicate standard descriptor");
        descriptors_.standard(fd, FD(duplicate));
    }
    mount_process_filesystems();
    information_ = std::make_shared<system::SamplingState>(system::start_information(scheduler_));
}
std::shared_ptr<SocketState> Syscalls::new_socket_state(int domain, int type, int protocol) {
    auto state = std::make_shared<SocketState>(domain, type, protocol);
    network_->sockets.push_back(state);
    return state;
}
void Syscalls::check_buffer(uint64_t address, uint64_t length, unsigned permissions) {
    if (!memory_.accessible(address, length, permissions))
        guest_error(14);
}
void Syscalls::handle(CpuState &cpu) {
    ++calls;
    system::observe_information(*information_, scheduler_);
    const auto arguments = cpu.x;
    try {
        auto &task = scheduler_.current();
        const auto decision = security::seccomp_evaluate(task.seccomp, cpu);
        switch (decision.action) {
        case security::SeccompAction::Allow:
        case security::SeccompAction::Log:
            if (decision.action == security::SeccompAction::Log && config_.trace_syscalls)
                std::fprintf(stderr, "seccomp log tid=%u syscall=%llu\n", task.tid,
                             static_cast<unsigned long long>(cpu.x[8]));
            cpu.x[0] = dispatch(cpu);
            break;
        case security::SeccompAction::Errno:
            cpu.x[0] = uint64_t(decision.value);
            break;
        case security::SeccompAction::Trap: {
            auto *controller = scheduler_.processes();
            if (!controller)
                throw Failure("seccomp SIGSYS requires a process controller");
            auto process = controller->signal_hooks().process(task.pid);
            security::seccomp_force_trap(*process.signals, task.signals, task.signal_mask, cpu, decision.data);
            break;
        }
        case security::SeccompAction::KillThread:
        case security::SeccompAction::KillProcess: {
            auto *controller = scheduler_.processes();
            if (!controller)
                throw Failure("seccomp termination requires a process controller");
            if (decision.action == security::SeccompAction::KillProcess ||
                scheduler_.live_tids(task.pid).size() == 1) {
                controller->terminate(task.pid, 31, true);
            } else {
                if (!controller->task_exiting(task.pid, task.tid, false, 31 | 128)) {
                    release_task_resources(task);
                    scheduler_.exit(128 + 31, false);
                }
            }
            break;
        }
        }
    } catch (const GuestError &error) {
        cpu.x[0] = uint64_t(-int64_t(error.number));
    } catch (const MemoryFault &) {
        cpu.x[0] = uint64_t(-int64_t(14));
    }
    if (config_.trace_syscalls) {
        std::fprintf(
            stderr, "syscall %llu(%#llx,%#llx,%#llx,%#llx,%#llx,%#llx) = %lld\n",
            static_cast<unsigned long long>(arguments[8]), static_cast<unsigned long long>(arguments[0]),
            static_cast<unsigned long long>(arguments[1]), static_cast<unsigned long long>(arguments[2]),
            static_cast<unsigned long long>(arguments[3]), static_cast<unsigned long long>(arguments[4]),
            static_cast<unsigned long long>(arguments[5]), static_cast<long long>(cpu.x[0]));
    }
}
int64_t Syscalls::dispatch(CpuState &cpu) {
    switch (cpu.x[8]) {
    case 277: // seccomp
        return security::seccomp_syscall(scheduler_.current().seccomp, memory_, cpu.x[0], cpu.x[1], cpu.x[2],
                                         &scheduler_);
    case 97:  // unshare
    case 268: // setns
        return dispatch_namespace(cpu);
    case 180: // mq_open
    case 181: // mq_unlink
    case 182: // mq_timedsend
    case 183: // mq_timedreceive
    case 184: // mq_notify
    case 185: // mq_getsetattr
    case 186: // msgget
    case 187: // msgctl
    case 188: // msgrcv
    case 189: // msgsnd
    case 190: // semget
    case 191: // semctl
    case 192: // semtimedop
    case 193: // semop
    case 194: // shmget
    case 195: // shmctl
    case 196: // shmat
    case 197: // shmdt
        return dispatch_ipc(cpu);
    case 293: // rseq
        return threads::rseq_syscall(memory_, scheduler_.current().rseq, cpu.x[0], uint32_t(cpu.x[1]),
                                     uint32_t(cpu.x[2]), uint32_t(cpu.x[3]));
    case 270: // process_vm_readv
    case 271: // process_vm_writev
        return dispatch_process_vm(cpu);
    case 425: // io_uring_setup
    case 426: // io_uring_enter
    case 427: // io_uring_register
        return dispatch_io_uring(cpu);
    case 143: // setregid
    case 144: // setgid
    case 145: // setreuid
    case 146: // setuid
    case 147: // setresuid
    case 148: // getresuid
    case 149: // setresgid
    case 150: // getresgid
    case 151: // setfsuid
    case 152: // setfsgid
    case 174: // getuid
    case 175: // geteuid
    case 176: // getgid
    case 177: // getegid
        return dispatch_identity(cpu);
    case 26:
    case 27:
    case 28:
        return dispatch_inotify(cpu);
    case 47:
    case 84:
    case 213:
    case 223:
    case 436:
        return dispatch_file_control(cpu);
    case 56:
    case 437:
        return dispatch_open(cpu);
    case 25:
    case 279:
        return descriptor_control(cpu);
    case 167:
        return dispatch_control(cpu);
    case 211:
    case 212:
    case 243:
    case 269:
        return socket_message(cpu);
    case 198:
    case 199:
    case 200:
    case 201:
    case 202:
    case 203:
    case 204:
    case 205:
    case 206:
    case 207:
    case 208:
    case 209:
    case 210:
    case 242:
        return dispatch_socket(cpu);
    case 102:
    case 103:
    case 107:
    case 108:
    case 109:
    case 110:
    case 111:
    case 85:
    case 86:
    case 87:
        return dispatch_timer(cpu);
    case 72:
    case 73:
        return dispatch_poll(cpu);
    case 95:
    case 117: // ptrace
    case 154:
    case 155:
    case 156:
    case 157:
    case 172:
    case 173:
    case 220:
    case 221:
    case 260:
    case 281:
    case 424:
    case 434:
    case 438:
    case 435:
        return dispatch_process(cpu);
    case 74:
    case 129:
    case 130:
    case 131:
    case 132:
    case 133:
    case 134:
    case 135:
    case 136:
    case 137:
    case 138:
    case 139:
    case 240:
        return dispatch_signal(cpu);
    case 93:
    case 94:
    case 96:
    case 98:
    case 99:
    case 100:
    case 118: // sched_setparam
    case 119: // sched_setscheduler
    case 120:
    case 121:
    case 122:
    case 123:
    case 124: // sched_yield
    case 127: // sched_rr_get_interval
    case 125: // sched_get_priority_max
    case 126: // sched_get_priority_min
    case 140: // setpriority
    case 141: // getpriority
    case 274: // sched_setattr
    case 275: // sched_getattr
    case 101:
    case 115:
    case 178:
    case 283:
    case 449: // futex_waitv
    case 454: // futex_wake
    case 455: // futex_wait
    case 456: // futex_requeue
        return dispatch_threads(cpu);
    case 19:
    case 20:
    case 21:
    case 22:
    case 59:
    case 441:
        return dispatch_events(cpu);
    case 39:
    case 40:
        return dispatch_mount(cpu);
    case 36:
    case 37:
    case 45:
    case 48:
    case 52:
    case 53:
    case 54:
    case 55:
    case 82:
    case 83:
    case 88:
    case 276: // renameat2
    case 439:
    case 452: // fchmodat2
        return dispatch_path(cpu);
    case 71:
    case 76:
    case 77:
    case 285:
        return dispatch_file_transfer(cpu);
    case 5:
    case 6:
    case 7:
    case 8:
    case 9:
    case 10:
    case 11:
    case 12:
    case 13:
    case 14:
    case 15:
    case 16:
    case 17:
    case 23:
    case 24:
    case 29:
    case 32:
    case 34:
    case 35:
    case 38:
    case 43:
    case 44:
    case 46:
    case 49:
    case 50:
    case 57:
    case 61:
    case 62:
    case 63:
    case 64:
    case 65:
    case 66:
    case 67:
    case 68:
    case 69:
    case 70:
    case 78:
    case 79:
    case 80:
    case 286:
    case 287:
    case 291:
        return dispatch_file(cpu);
    case 214:
    case 215:
    case 216:
    case 222:
    case 226:
    case 227:
    case 232:
    case 233:
        return dispatch_memory(cpu);
    case 163:
    case 164:
    case 261:
        return dispatch_limits(cpu);
    case 90:
    case 91:
    case 113:
    case 114:
    case 153:
    case 165:
    case 168:
    case 179:
    case 158:
    case 159:
    case 160:
    case 166:
    case 169:
    case 278:
        return dispatch_system(cpu);
    default:
        return unsupported(cpu);
    }
}
int64_t Syscalls::unsupported(const CpuState &cpu) {
    ++unsupported_count;
    last_unsupported = cpu.x[8];
    return -38;
}
} // namespace operit::arm64
