#include "linux.h"

struct rseq_area {
    unsigned cpu_id_start, cpu_id;
    u64 rseq_cs;
    unsigned flags, node_id, mm_cid;
} __attribute__((aligned(32)));
struct rseq_cs {
    unsigned version, flags;
    u64 start_ip, post_commit_offset, abort_ip;
} __attribute__((aligned(32)));

extern char rseq_guest_start[], rseq_guest_commit[], rseq_guest_abort[];
extern unsigned rseq_guest_increment(struct rseq_area *, volatile unsigned *, struct rseq_cs *);
static struct rseq_area area;
static struct rseq_cs section;
static volatile unsigned counter;

static i64 rseq_call(u64 address, unsigned size, unsigned flags, unsigned signature) {
    return call6(293, address, size, flags, signature, 0, 0);
}

int guest_main(u64 *stack) {
    (void)stack;
    const unsigned signature = 0xd428bc00;
    area.cpu_id = (unsigned)-1;
    CHECK(rseq_call((u64)&area + 4, 32, 0, signature) == -22, 10);
    CHECK(rseq_call((u64)&area, 31, 0, signature) == -22, 11);
    CHECK(rseq_call((u64)&area, 32, 0, signature) == 0, 12);
    CHECK(area.cpu_id == area.cpu_id_start && area.cpu_id < 64 && area.node_id == 0 && area.mm_cid < 64, 13);
    CHECK(rseq_call((u64)&area, 32, 0, signature) == -16, 14);
    CHECK(rseq_call((u64)&area, 32, 0, signature + 1) == -1, 15);
    section.start_ip = (u64)rseq_guest_start;
    section.post_commit_offset = (u64)rseq_guest_commit - section.start_ip;
    section.abort_ip = (u64)rseq_guest_abort;
    unsigned committed = 0;
    for (unsigned i = 0; i != 64; ++i) {
        unsigned aborted = rseq_guest_increment(&area, &counter, &section);
        CHECK(aborted <= 1, 16);
        committed += !aborted;
        CHECK(counter == committed, 17);
    }
    CHECK(rseq_call((u64)&area, 32, 1, signature + 1) == -1, 18);
    CHECK(rseq_call((u64)&area, 32, 1, signature) == 0, 19);
    CHECK(area.cpu_id_start == 0 && area.cpu_id == (unsigned)-1 && area.node_id == 0 && area.mm_cid == 0,
          20);
    print("rseq-guest-ok\n");
    return 0;
}
