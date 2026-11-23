#include <android/log.h>
#include <errno.h>
#include <signal.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <limits.h>
#include <fcntl.h>
#include <grp.h>
#include <dirent.h>
#include <mntent.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

/** Rejects a failed launch with a visible diagnostic and a nonzero exit status. */
static void fail(const char *operation) {
    __android_log_print(ANDROID_LOG_ERROR, "OperitShell", "%s: errno=%d", operation, errno);
    dprintf(STDERR_FILENO, "%s: %s\n", operation, strerror(errno));
    _exit(126);
}

/** Matches one complete mount option without assuming a fixed Android cgroup mount path. */
static bool memory_controller(const char *options) {
    char copy[PATH_MAX];
    snprintf(copy, sizeof(copy), "%s", options);
    char *state = nullptr;
    for (char *option = strtok_r(copy, ",", &state); option; option = strtok_r(nullptr, ",", &state)) {
        if (!strcmp(option, "memory")) return true;
    }
    return false;
}

/** Moves Root activation out of the app's memory cgroup before dropping its capabilities. */
static void detach_memory_cgroups() {
    FILE *mounts = setmntent("/proc/mounts", "r");
    if (!mounts) fail("read cgroup mounts");
    while (mntent *mount = getmntent(mounts)) {
        if (strcmp(mount->mnt_type, "cgroup2") &&
            (strcmp(mount->mnt_type, "cgroup") || !memory_controller(mount->mnt_opts))) continue;
        char path[PATH_MAX], pid[32];
        snprintf(path, sizeof(path), "%s/cgroup.procs", mount->mnt_dir);
        int fd = open(path, O_WRONLY | O_CLOEXEC);
        if (fd < 0) fail("open memory cgroup");
        int size = snprintf(pid, sizeof(pid), "%d\n", getpid());
        if (write(fd, pid, size) != size) fail("detach memory cgroup");
        close(fd);
    }
    endmntent(mounts);
}

/** Establishes shell identity once, before the supervisor, VM or child service exists. */
static void enter_shell() {
    if (geteuid() == 0) {
        detach_memory_cgroups();
        const gid_t groups[] = {2000, 1004, 1007, 1011, 1015, 1028, 3001, 3002, 3003, 3006, 3009, 3011};
        if (setgroups(sizeof(groups) / sizeof(groups[0]), groups) || setgid(2000) || setuid(2000))
            fail("enter shell identity");
        int fd = open("/proc/self/attr/current", O_WRONLY | O_CLOEXEC);
        const char context[] = "u:r:shell:s0";
        if (fd < 0) fail("open SELinux identity");
        if (write(fd, context, sizeof(context)) != sizeof(context)) fail("enter shell SELinux identity");
        close(fd);
    }
    if (getuid() != 2000 || geteuid() != 2000 || getgid() != 2000)
        fail("service requires shell uid/gid 2000");
}

/** Detaches the activation connection and waits for explicit service termination or crashes. */
int main(int argc, char **argv) {
    if (argc != 3) return 64;
    const char *apk = argv[1], *manager = argv[2];
    if (strcmp(manager, "app.operit") && strcmp(manager, "app.operit.module")) return 64;
    enter_shell();
    signal(SIGHUP, SIG_IGN);
    signal(SIGPIPE, SIG_IGN);
    char lock_path[PATH_MAX];
    snprintf(lock_path, sizeof(lock_path), "/data/local/tmp/%s.shell.lock", manager);
    int lock = open(lock_path, O_CREAT | O_RDWR, 0600);
    if (lock < 0) fail("open service lock");
    if (flock(lock, LOCK_EX | LOCK_NB)) {
        if (errno == EWOULDBLOCK) return 0;
        fail("lock service");
    }
    pid_t daemon = fork();
    if (daemon < 0) fail("fork supervisor");
    if (daemon > 0) return 0;
    if (setsid() < 0 || chdir("/") < 0) fail("detach supervisor");
    int nullfd = open("/dev/null", O_RDWR);
    if (nullfd < 0) fail("open /dev/null");
    for (int fd = 0; fd < 3; ++fd) if (dup2(nullfd, fd) < 0) fail("detach standard streams");
    if (nullfd > 2) close(nullfd);
    DIR *fds = opendir("/proc/self/fd");
    if (!fds) fail("list inherited descriptors");
    while (dirent *entry = readdir(fds)) {
        char *end = nullptr;
        long fd = strtol(entry->d_name, &end, 10);
        if (*end == '\0' && fd > 2 && fd != lock && fd != dirfd(fds)) close((int) fd);
    }
    closedir(fds);
    for (int crashes = 0; crashes < 5; ++crashes) {
        if (access(apk, R_OK)) return 50; // APK updates require a fresh resolved path.
        pid_t child = fork();
        if (child < 0) fail("fork service");
        if (child == 0) {
            close(lock);
            setenv("CLASSPATH", apk, 1);
            char manager_property[256];
            snprintf(manager_property, sizeof(manager_property), "-Doperit.shizuku.manager=%s", manager);
            execl("/system/bin/app_process", "app_process", manager_property,
                "/system/bin", "--nice-name=operit_shell", "rikka.shizuku.server.ShizukuService", nullptr);
            fail("exec service");
        }
        int status = 0;
        pid_t waited;
        do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
        if (waited < 0) fail("wait service");
        if (WIFEXITED(status) && (WEXITSTATUS(status) == 0 || WEXITSTATUS(status) == 50)) return 0;
        sleep(2);
    }
    return 1;
}
