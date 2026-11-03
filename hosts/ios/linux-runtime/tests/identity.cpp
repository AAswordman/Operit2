#include "linux/process/pidfd.h"
#include "linux/syscalls.h"
#include "runtime/machine.h"
#include <algorithm>
#include <array>
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <initializer_list>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <vector>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>
#if defined(__linux__)
#include <sys/syscall.h>
#elif defined(__APPLE__)
#include <sys/sysctl.h>
#endif

using namespace operit::arm64;
namespace {
constexpr uint64_t out = 0x11000, second = out + 16, third = out + 32;
constexpr uint64_t sentinel = UINT64_MAX, upper_bits = uint64_t{0x12345678} << 32;
constexpr uint32_t marker = 0xa5a5a5a5;
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
uint32_t different(uint32_t a, uint32_t b = UINT32_MAX, uint32_t c = UINT32_MAX) {
    for (uint32_t value = 0;; ++value)
        if (value != a && value != b && value != c)
            return value;
}
std::optional<std::array<uint32_t, 3>> native_res(bool group) {
#if defined(__linux__)
    if (group) {
        gid_t real, effective, saved;
        if (::syscall(SYS_getresgid, &real, &effective, &saved) == 0)
            return std::array<uint32_t, 3>{uint32_t(real), uint32_t(effective), uint32_t(saved)};
    } else {
        uid_t real, effective, saved;
        if (::syscall(SYS_getresuid, &real, &effective, &saved) == 0)
            return std::array<uint32_t, 3>{uint32_t(real), uint32_t(effective), uint32_t(saved)};
    }
#elif defined(__APPLE__)
    uid_t uid;
    gid_t gid;
    errno = 0;
    const int overridden = ::pthread_getugid_np(&uid, &gid);
    if (!((overridden == -1 && errno == ESRCH) || overridden == ESRCH))
        return std::nullopt;
    int mib[] = {CTL_KERN, KERN_PROC, KERN_PROC_PID, ::getpid()};
    struct kinfo_proc info{};
    size_t size = sizeof(info);
    if (::sysctl(mib, 4, &info, &size, nullptr, 0) == 0 && size == sizeof(info)) {
        const auto &p = info.kp_eproc.e_pcred;
        const auto &u = info.kp_eproc.e_ucred;
        if (u.cr_ngroups > 0 && p.p_ruid == ::getuid() && u.cr_uid == ::geteuid() &&
            p.p_rgid == ::getgid() && u.cr_groups[0] == ::getegid())
            return group ? std::array<uint32_t, 3>{p.p_rgid, u.cr_groups[0], p.p_svgid}
                         : std::array<uint32_t, 3>{p.p_ruid, u.cr_uid, p.p_svuid};
    }
#else
    (void)group;
#endif
    return std::nullopt;
}
std::optional<uint32_t> native_fs(bool group) {
#if defined(__linux__)
    std::ifstream stream("/proc/thread-self/status");
    for (std::string line; std::getline(stream, line);) {
        if (line.compare(0, 4, group ? "Gid:" : "Uid:") != 0)
            continue;
        std::istringstream fields(line.substr(4));
        uint32_t real, effective, saved, filesystem;
        if (!(fields >> real >> effective >> saved >> filesystem))
            break;
        const auto ids = native_res(group);
        if (ids && *ids == std::array<uint32_t, 3>{real, effective, saved})
            return filesystem;
        break;
    }
    return std::nullopt;
#elif defined(__APPLE__)
    return group ? uint32_t(::getegid()) : uint32_t(::geteuid());
#else
    (void)group;
    return std::nullopt;
#endif
}
struct HostSnapshot {
    uint32_t uid = ::getuid(), euid = ::geteuid(), gid = ::getgid(), egid = ::getegid();
    std::optional<std::array<uint32_t, 3>> users = native_res(false), groups = native_res(true);
    std::optional<uint32_t> fsuid = native_fs(false), fsgid = native_fs(true);
    void unchanged() const {
        require(uid == ::getuid() && euid == ::geteuid() && gid == ::getgid() && egid == ::getegid(),
                "guest identity syscall changed host real/effective credentials");
        require(users == native_res(false) && groups == native_res(true) &&
                    fsuid == native_fs(false) && fsgid == native_fs(true),
                "guest identity syscall changed host saved/filesystem credentials");
    }
};
struct Fixture {
    std::string root;
    Scheduler standalone_scheduler;
    std::unique_ptr<Memory> standalone_memory;
    std::unique_ptr<Filesystem> standalone_fs;
    std::unique_ptr<Syscalls> standalone_sys;
    std::unique_ptr<Machine> machine;
    Memory *memory;
    Scheduler *scheduler;
    Syscalls *sys;
    explicit Fixture(bool managed = true) {
        char pattern[] = "/tmp/operit-identity-XXXXXX";
        const auto created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        root = created;
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        config.execution_threads = 1;
        if (managed) {
            machine = std::make_unique<Machine>(config);
            memory = machine->find_process(1)->image.memory.get();
            scheduler = &machine->scheduler();
            sys = machine->process_syscalls(1);
        } else {
            standalone_memory = std::make_unique<Memory>(8 << 20);
            standalone_fs = std::make_unique<Filesystem>(root.c_str());
            standalone_sys = std::make_unique<Syscalls>(*standalone_memory, *standalone_fs,
                                                       config, standalone_scheduler);
            memory = standalone_memory.get();
            scheduler = &standalone_scheduler;
            sys = standalone_sys.get();
        }
        memory->map(0x10000, 0x4000, read_perm | write_perm);
    }
    ~Fixture() {
        machine.reset();
        standalone_sys.reset();
        standalone_fs.reset();
        std::filesystem::remove_all(root);
    }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> arguments = std::initializer_list<uint64_t>{}) {
        auto &cpu = scheduler->current().cpu;
        cpu.x.fill(0);
        std::copy(arguments.begin(), arguments.end(), cpu.x.begin());
        cpu.x[8] = number;
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    process::PidfdCredentials credentials() {
        return process::pidfd_credentials(*machine, 1);
    }
    void credentials(const process::PidfdCredentials &value) {
        process::pidfd_update_credentials(*machine, 1, value);
    }
    void fill() {
        for (uint64_t p = out; p < out + 48; p += 4)
            memory->store(p, marker, 4);
    }
    std::array<uint32_t, 3> result() {
        return {uint32_t(memory->load(out, 4)), uint32_t(memory->load(second, 4)),
                uint32_t(memory->load(third, 4))};
    }
};
void registry_getters_and_ordered_faults() {
    Fixture f;
    auto ids = f.credentials();
    ids.real_uid = 101; ids.effective_uid = 202; ids.saved_uid = 303;
    ids.real_gid = 404; ids.effective_gid = 505; ids.saved_gid = 606;
    f.credentials(ids); // Test-only registry setup; no native credential mutation.
    require(f.call(174) == 101 && f.call(175) == 202 && f.call(176) == 404 && f.call(177) == 505,
            "getuid/euid/gid/egid must use the same registry as getres");
    for (bool group : {false, true}) {
        const uint64_t number = group ? 150 : 148;
        const std::array<uint32_t, 3> expected = group ? std::array<uint32_t, 3>{404, 505, 606}
                                                      : std::array<uint32_t, 3>{101, 202, 303};
        f.fill();
        require(f.call(number, {out, second, third}) == 0 && f.result() == expected,
                "getres must return distinct guest real/effective/saved IDs");
        require(f.memory->load(out + 4, 4) == marker && f.memory->load(third + 4, 4) == marker,
                "getres wrote more than four bytes per ID");
        for (unsigned bad = 0; bad < 3; ++bad) {
            for (const uint64_t invalid : {uint64_t{0}, uint64_t{0x50000}, UINT64_MAX - 1,
                                           upper_bits | out}) {
                f.fill();
                std::array<uint64_t, 3> pointers{out, second, third};
                pointers[bad] = invalid;
                require(f.call(number, {pointers[0], pointers[1], pointers[2]}) == -14,
                        "getres invalid pointer must return EFAULT");
                const auto actual = f.result();
                for (unsigned index = 0; index < 3; ++index)
                    require(actual[index] == (index < bad ? expected[index] : marker),
                            "getres fault lost earlier writes or touched later outputs");
            }
        }
        f.fill();
        require(f.call(number, {out, out, out}) == 0 && f.memory->load(out, 4) == expected[2],
                "aliased getres pointers must be written real then effective then saved");
        require(f.call(number, {out + 1, second + 1, third + 1}) == 0 &&
                    f.memory->load(third + 1, 4) == expected[2], "unaligned getres output");
        f.fill();
        f.memory->protect(0x13000, 4096, read_perm);
        require(f.call(number, {out, 0x13000, third}) == -14 &&
                    f.memory->load(out, 4) == expected[0] && f.memory->load(third, 4) == marker,
                "read-only second output must preserve real and leave saved untouched");
        require(f.call(number, {0x13ffe, second, third}) == -14,
                "four-byte output crossing into an unmapped page must fault");
        f.memory->protect(0x13000, 4096, read_perm | write_perm);
    }
    require(!f.sys->unsupported_count, "identity syscalls were not routed");
}
void setters_retain_every_identity_component() {
    Fixture f;
    for (bool group : {false, true}) {
        auto ids = f.credentials();
        const auto fs = native_fs(group);
        const uint32_t effective = group ? ids.effective_gid : ids.effective_uid;
        const uint32_t real = different(effective), saved = different(real, effective);
        if (group) { ids.real_gid = real; ids.saved_gid = saved; }
        else { ids.real_uid = real; ids.saved_uid = saved; }
        f.credentials(ids);
        const uint64_t re = group ? 143 : 145, res = group ? 149 : 147, single = group ? 144 : 146;
        const int64_t fs_result = !fs ? -38 : (*fs == effective ? 0 : -1);
        require(f.call(re, {sentinel, sentinel}) == fs_result, "setre(-1,-1) filesystem reset");
        require(f.call(re, {real, sentinel}) == -1 && f.call(re, {sentinel, effective}) == -1 &&
                    f.call(re, {real, effective}) == -1,
                "same-valued setre must reject an implicit saved-ID change");
        require(f.call(res, {sentinel, sentinel, sentinel}) == 0 &&
                    f.call(res, {real, sentinel, saved}) == 0,
                "setres no-op with omitted effective ID must retain filesystem ID");
        require(f.call(res, {real, effective, saved}) == fs_result,
                "setres of existing IDs must also check an explicit effective/fs ID");
        const uint32_t change = different(real, effective, saved);
        require(f.call(re, {change, sentinel}) == -1 && f.call(re, {sentinel, change}) == -1 &&
                    f.call(res, {change, sentinel, sentinel}) == -1 &&
                    f.call(res, {sentinel, change, sentinel}) == -1 &&
                    f.call(res, {sentinel, sentinel, change}) == -1 &&
                    f.call(single, {change}) == -1,
                "identity changes must fail even under a privileged host");
        require(f.call(single, {effective}) == -1, "setuid/gid target must be real or saved without caps");
        require(f.call(single, {sentinel}) == -22 &&
                    f.call(single, {upper_bits | UINT32_MAX}) == -22,
                "setuid/gid -1 is invalid, not a leave-unchanged sentinel");
        require(f.call(res, {upper_bits | real, sentinel, upper_bits | saved}) == 0 &&
                    f.call(res, {upper_bits | UINT32_MAX, uint64_t{UINT32_MAX},
                                 upper_bits | UINT32_MAX}) == 0,
                "setres arguments and -1 must be truncated to 32 bits");
        require(f.call(res, {upper_bits | change, sentinel, sentinel}) == -1,
                "truncated actual identity changes must still fail");
        const auto after = f.credentials();
        require(after.real_uid == ids.real_uid && after.effective_uid == ids.effective_uid &&
                    after.saved_uid == ids.saved_uid && after.real_gid == ids.real_gid &&
                    after.effective_gid == ids.effective_gid && after.saved_gid == ids.saved_gid &&
                    after.permitted_capabilities == ids.permitted_capabilities,
                "rejected/no-op setters mutated guest registry");
        // Keeping a saved ID already equal to effective removes the setre
        // hidden change, and makes setuid/setgid a permitted no-op.
        if (group) ids.saved_gid = effective;
        else ids.saved_uid = effective;
        f.credentials(ids);
        require(f.call(re, {upper_bits | real, upper_bits | effective}) == fs_result &&
                    f.call(single, {upper_bits | effective}) == fs_result,
                "safe existing-identity setters rejected after low-32-bit conversion");
    }
}
void filesystem_ids_are_silent_and_native() {
    Fixture f;
    for (bool group : {false, true}) {
        const auto old = native_fs(group);
        const uint64_t number = group ? 152 : 151;
        const uint32_t current = old.value_or(1000), change = different(current);
        for (uint64_t argument : {uint64_t(current), uint64_t(change), sentinel,
                                  uint64_t(UINT32_MAX), upper_bits | current, upper_bits | change}) {
            require(f.call(number, {argument}) == (old ? int64_t(*old) : -38),
                    "setfs must silently refuse changes and return the unsigned old native ID");
            require(native_fs(group) == old, "setfs changed host file permission identity");
        }
        // Registry effective is not a native filesystem credential.
        auto ids = f.credentials();
        if (group) ids.effective_gid = change;
        else ids.effective_uid = change;
        f.credentials(ids);
        require(f.call(number, {change}) == (old ? int64_t(*old) : -38),
                "setfs fabricated filesystem ID from guest effective ID");
        if (old) {
            require(f.call(group ? 143 : 145, {sentinel, sentinel}) == -1 &&
                        f.call(group ? 149 : 147, {sentinel, change, sentinel}) == -1,
                    "setter claimed success despite an implicit native filesystem-ID change");
        }
    }
    // A real backing file remains owned by the native fs identity after all
    // rejected requests. This also exercises the real guest open/write path.
    constexpr uint64_t path = 0x10000, data = 0x10100;
    const char name[] = "/owned";
    f.memory->copy_in(path, name, sizeof(name));
    f.memory->store(data, 'x', 1);
    const auto fd = f.call(56, {uint64_t(-100), path, 0x42, 0600}); // O_RDWR | O_CREAT
    require(fd >= 0 && f.call(64, {uint64_t(fd), data, 1}) == 1 &&
                f.call(57, {uint64_t(fd)}) == 0, "native backing-file create/write/close");
    struct stat info{};
    require(::stat((f.root + "/owned").c_str(), &info) == 0 && info.st_size == 1,
            "backing file was not created");
    if (const auto uid = native_fs(false))
        require(uint32_t(info.st_uid) == *uid, "guest identity setters changed backing-file ownership");
    const char locked[] = "/locked";
    f.memory->copy_in(path, locked, sizeof(locked));
    const auto native_path = f.root + "/locked";
    const int created = ::open(native_path.c_str(), O_WRONLY | O_CREAT | O_EXCL, 0000);
    require(created >= 0, "create native permission fixture");
    ::close(created);
    const int native = ::open(native_path.c_str(), O_RDONLY);
    const int native_error = errno;
    const auto guest = f.call(56, {uint64_t(-100), path, 0, 0});
    if (native >= 0) {
        ::close(native);
        require(guest >= 0, "guest lost native backing-file access");
        require(f.call(57, {uint64_t(guest)}) == 0, "close permission fixture");
    } else {
        require(native_error == EACCES && guest == -13,
                "guest registry identity must not bypass native backing-file permissions");
    }
}
void no_controller_falls_back_without_guessing_saved() {
    Fixture f(false);
    require(f.call(174) == ::getuid() && f.call(175) == ::geteuid() &&
                f.call(176) == ::getgid() && f.call(177) == ::getegid(),
            "no-controller real/effective fallback disagrees with native getters");
    for (bool group : {false, true}) {
        const auto native = native_res(group);
        f.fill();
        const auto result = f.call(group ? 150 : 148, {out, second, third});
        if (native) {
            require(result == 0 && f.result() == *native,
                    "no-controller getres must use actual native saved ID");
        } else {
            require(result == -38 && f.result() == std::array<uint32_t, 3>{marker, marker, marker},
                    "unavailable native saved ID must return ENOSYS without fabricating outputs");
        }
        require(f.call(group ? 149 : 147, {sentinel, sentinel, sentinel}) == 0,
                "no-controller all-minus-one setres must be a no-op");
    }
}
void registered_high_ids_and_missing_registration() {
    Fixture f;
    auto ids = f.credentials();
    ids.real_uid = 0x80000000u; ids.effective_uid = 0x80000001u; ids.saved_uid = 0xfffffffeu;
    ids.real_gid = 0x80000002u; ids.effective_gid = 0x80000003u; ids.saved_gid = 0xfffffffdu;
    f.credentials(ids);
    require(f.call(174) == int64_t(ids.real_uid) && f.call(175) == int64_t(ids.effective_uid) &&
                f.call(176) == int64_t(ids.real_gid) && f.call(177) == int64_t(ids.effective_gid),
            "32-bit unsigned IDs were sign-extended into negative syscall results");
    require(f.call(148, {out, second, third}) == 0 && f.result()[2] == ids.saved_uid &&
                f.call(150, {out, second, third}) == 0 && f.result()[2] == ids.saved_gid,
            "getres truncated high unsigned IDs");
    require(f.call(147, {upper_bits | ids.real_uid, sentinel, upper_bits | ids.saved_uid}) == 0,
            "high-bit UID no-op was confused with the -1 sentinel");
    f.sys->set_process_identity(9999, 0);
    require(f.call(148, {out, second, third}) == -3 && f.call(174) == -3,
            "controller missing registration must not silently fall back to native identity");
    f.sys->set_process_identity(1, 0);
}
} // namespace
int main() {
    try {
        HostSnapshot before;
        registry_getters_and_ordered_faults();
        setters_retain_every_identity_component();
        filesystem_ids_are_silent_and_native();
        no_controller_falls_back_without_guessing_saved();
        registered_high_ids_and_missing_registration();
        before.unchanged();
        std::puts("identity registry, no-op permissions, native filesystem IDs, ABI and ordered faults passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "Linux errno %d\n", error.number);
    }
    return 1;
}
