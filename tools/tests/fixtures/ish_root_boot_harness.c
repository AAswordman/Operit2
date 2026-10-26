#include <assert.h>
#include <setjmp.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define __KERNEL__ 1
#define __init
#define rootfs_initcall(fn)
#define MS_SILENT 1
#define MS_MOVE 2
#define EINPROGRESS 115
#define DECLARE_COMPLETION(name) int name = 0
#define pr_err(...) ((void)0)

static int fault, stage, notified, initialized;
static jmp_buf parked;
bool rootfs_mounted;
static int next_stage(void) { return ++stage == fault ? -77 : 0; }
static int do_mount(const char *a, const char *b, const char *c, int d, void *e) {
    (void)a; (void)b; (void)c; (void)d; (void)e;
    return next_stage();
}
static int init_chdir(const char *path) { (void)path; return next_stage(); }
static int devtmpfs_mount(void) { return next_stage(); }
static int init_chroot(const char *path) { (void)path; return next_stage(); }
static void wait_for_completion(int *completion) { (void)completion; longjmp(parked, 1); }
int notify_startup(void) { notified++; return 0; }
const char *DefaultRootPath(void) { return "/prepared-root"; }
void FsInitialize(void) { initialized++; }

/* ROOT_BOOT_IMPLEMENTATION */

static char boot_command_line[128];
static void run_kernel(void) {
    if (setjmp(parked) == 0) ish_rootfs();
}

/* ACTUATE_KERNEL_IMPLEMENTATION */

int main(int argc, char **argv) {
    assert(argc == 2);
    fault = atoi(argv[1]);
    int result = actuate_kernel("earlyprintk");
    assert(strcmp(boot_command_line, "earlyprintk") == 0);
    assert(result == (fault ? -77 : 0));
    assert(rootfs_mounted == !fault);
    assert(notified == !!fault);
    assert(initialized == !fault);
    assert(stage == (fault ? fault : 6));
    return 0;
}
