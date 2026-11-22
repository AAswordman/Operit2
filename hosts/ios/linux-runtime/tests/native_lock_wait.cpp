#include "linux/files/native_lock_wait.h"
#include <cstdio>
#include <stdexcept>
#include <poll.h>

using namespace operit::arm64;
namespace {
// Assert kernel-observable lock behavior, including cancellation before producer release.
void require(bool condition, const char *message) {
    if (!condition) throw std::runtime_error(message);
}
// Independently opened descriptions represent an external locker and its guest contender.
void exercise(bool whole_file) {
    char path[] = "/tmp/operit-lock-wait-XXXXXX";
    FD first(::mkstemp(path));
    require(first.get() >= 0, "create lock file");
    auto second = std::make_shared<FD>(::open(path, O_RDWR | O_CLOEXEC));
    FD third(::open(path, O_RDWR | O_CLOEXEC));
    ::unlink(path);
    require(second->get() >= 0 && third.get() >= 0, "open independent lock descriptions");
    struct flock range{};
    range.l_type = F_WRLCK;
    range.l_whence = SEEK_SET;
    range.l_len = 8;
    auto change = [&](int fd, bool acquire) {
        range.l_type = acquire ? F_WRLCK : F_UNLCK;
        return whole_file ? ::flock(fd, acquire ? LOCK_EX | LOCK_NB : LOCK_UN) :
            ::fcntl(fd, F_OFD_SETLK, &range);
    };
    require(change(first.get(), true) == 0, "acquire external lock");
    range.l_type = F_WRLCK;
    {
        files::NativeLockWait wait(second, whole_file, LOCK_EX, range);
        pollfd source{wait.native_handle(), POLLIN, 0};
        require(::poll(&source, 1, 0) == 0 && !wait.result(), "blocked lock signalled before release");
        require(change(first.get(), false) == 0, "release external lock");
        require(::poll(&source, 1, 2000) == 1, "lock completion did not notify its descriptor");
        require(wait.result() == 0, "native lock acquisition failed");
    }
    require(change(second->get(), false) == 0, "release completed guest lock");
    require(change(first.get(), true) == 0, "reacquire external lock");
    range.l_type = F_WRLCK;
    {
        files::NativeLockWait wait(second, whole_file, LOCK_EX, range);
        wait.stop();
        require(!wait.result(), "cancelled lock unexpectedly completed");
    }
    require(change(first.get(), false) == 0, "release after cancellation");
    require(change(third.get(), true) == 0, "cancelled worker acquired a late lock");
}
} // namespace
// Run both kernel lock families without a scheduler polling loop or timing sleeps.
int main() {
    try {
        exercise(true);
        exercise(false);
        std::puts("native lock waits: flock/OFD completion, cancellation and no late acquisition passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    }
}
