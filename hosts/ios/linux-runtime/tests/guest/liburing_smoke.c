#include <liburing.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int complete(struct io_uring *ring, int expected) {
    struct io_uring_cqe *cqe = NULL;
    const int submitted = io_uring_submit_and_wait(ring, 1);
    if (submitted != 1)
        return fprintf(stderr, "submit: %d\n", submitted), 1;
    const int result = io_uring_wait_cqe(ring, &cqe);
    if (result < 0)
        return fprintf(stderr, "wait: %d\n", result), 1;
    const int actual = cqe->res;
    io_uring_cqe_seen(ring, cqe);
    if (actual != expected)
        return fprintf(stderr, "completion: %d expected %d\n", actual, expected), 1;
    return 0;
}

int main(void) {
    char path[] = "/tmp/operit-liburing-XXXXXX";
    int fd = mkstemp(path);
    if (fd < 0)
        return perror("mkstemp"), 1;
    if (unlink(path) < 0)
        return perror("unlink"), 1;

    struct io_uring ring;
    int result = io_uring_queue_init(8, &ring, 0);
    if (result < 0)
        return fprintf(stderr, "queue_init: %d\n", result), 1;

    char first[] = "hello ", second[] = "uring";
    struct iovec input[] = {{first, sizeof(first) - 1}, {second, sizeof(second) - 1}};
    struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
    if (!sqe)
        return 1;
    io_uring_prep_writev(sqe, fd, input, 2, 0);
    if (complete(&ring, 11))
        return 1;

    char output[12] = {0};
    sqe = io_uring_get_sqe(&ring);
    if (!sqe)
        return 1;
    io_uring_prep_read(sqe, fd, output, 11, 0);
    if (complete(&ring, 11) || strcmp(output, "hello uring"))
        return 1;

    sqe = io_uring_get_sqe(&ring);
    if (!sqe)
        return 1;
    io_uring_prep_fsync(sqe, fd, IORING_FSYNC_DATASYNC);
    if (complete(&ring, 0))
        return 1;

    io_uring_queue_exit(&ring);
    close(fd);
    puts("liburing-file-io-ok");
    return 0;
}
