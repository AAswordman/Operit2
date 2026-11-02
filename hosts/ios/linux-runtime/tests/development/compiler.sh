set -eu
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
cat > main.c <<'C'
#include <pthread.h>
#include <stdio.h>
#include <sys/wait.h>
#include <unistd.h>
static void *worker(void *value) { return value; }
int main(void) {
    pthread_t task;
    void *result;
    if (pthread_create(&task, 0, worker, (void *)42) ||
        pthread_join(task, &result) || result != (void *)42)
        return 1;
    pid_t pid = fork();
    if (pid < 0) return 2;
    if (!pid) _exit(7);
    int status;
    if (waitpid(pid, &status, 0) != pid || !WIFEXITED(status) || WEXITSTATUS(status) != 7)
        return 3;
    puts("GCC_PTHREAD_FORK_OK");
    return 0;
}
C
gcc -O2 -pthread main.c -o main
./main
