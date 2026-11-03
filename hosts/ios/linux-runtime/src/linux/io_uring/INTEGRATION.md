# io_uring integration contract

The implementation is intentionally limited to a synchronous regular-file
subset. See `io_uring.h` for exact C++ signatures. Linux v6.12 UAPI layouts and
queue behavior are the reference; the accepted subset is not full Linux
io_uring compatibility.

## Required hooks

- Add `std::shared_ptr<io_uring::Ring>` to the open description. `setup()`
  returns a close-on-exec guest fd and the ring to attach to this description.
  Duplicated descriptors and copied/shared descriptor tables then preserve the
  same open description. Do not destroy the ring eagerly on close or exec.
- Route AArch64 syscalls 425, 426 and 427 to `setup`, `enter` and `register_op`.
  For `enter`, forward `ExecuteIo` to the existing scalar `Syscalls::io` with
  `may_block=false`, and `InterruptPending` to the runtime's signal check.
- After normal mmap validation, address selection and mapping-limit checks,
  route ring descriptors to `map_ring`. SQ and CQ share one underlying file;
  each mapping retains a `shared_ptr<Ring>` through `Memory::map_file`.
- Treat the descriptor as an anonymous inode. Its host fd is only a readiness
  pipe: reject normal scalar and vector read/write, splice, and other transfer
  paths before they can consume its bytes. Otherwise the guest can drain the
  marker and corrupt readiness. Do not mistake this pipe for a guest pipe.
- Refresh each retained ring description before every poll/select attempt,
  including scheduler retries and epoll watches (also nested watches). This
  clears the marker after the guest advances CQ head and flushes overflow.
- In `Scheduler::interrupt`, when interruption is `-EINTR`, call
  `interrupt_wait(task)` before completing the task and use its result if
  present. A wait that submitted SQEs returns the submission count even if
  interrupted. The wait's cleanup also runs on task exit; destroying a
  scheduler destroys the registration without dereferencing a dead task.

The readiness marker reports CQ readability. Correct `POLLOUT`/`EPOLLOUT`
tracking as the SQ fills and drains is not implemented; callers must not claim
full io_uring poll semantics. Temporary signal masks and extended-argument
waits are rejected.

## Implemented ABI

- Setup flags: `IORING_SETUP_CQSIZE`, `IORING_SETUP_CLAMP`. SQ and CQ sizes are
  powers of two, with Linux v6.12 limits. SQ/CQ use `IORING_FEAT_SINGLE_MMAP`;
  SQEs use a separate mapping. Only shared mappings are accepted.
- Entries: `NOP` (including result injection), `READ`, `WRITE`, `READV`,
  `WRITEV` and `FSYNC` for native regular files. Read/write requests support
  current-position (`off == -1`) and explicit offsets. Vector requests import
  the Linux iovec array, use the runtime's iovec and file-write-limit helpers,
  and perform actual host file I/O. The runtime's 1 MiB per-request I/O cap
  also applies to vectors. `FSYNC` calls host `fsync`; `IORING_FSYNC_DATASYNC`
  uses host `fdatasync` on Linux and the stronger `fsync` fallback on Darwin.
  A specified sync range is conservatively widened to the entire file.
  Operations run synchronously; no background workers or deferred nonblocking
  operations are implemented. Host filesystem calls, including sync, may block.
  Current-position vector I/O and sync on standard-stream descriptions are
  rejected because an embedding callback may own their data or durability;
  positioned vector I/O follows the existing runtime's native-file behavior.
- SQ and CQ heads/tails, index array, invalid-index drops, CQE results and
  bounded FIFO overflow are backed by actual guest-shared memory. When the
  overflow bound is reached, further submissions fail with `EBUSY` instead of
  dropping completions. Allocation failure is reported before executing the
  affected operation.
- `IORING_REGISTER_PROBE` reports the implemented opcodes. All other register
  operations return `EINVAL`.
- Only `IORING_ENTER_GETEVENTS` is accepted. Unsupported enter/setup flags,
  registration operations, SQE flags, file types and opcodes are rejected.
  Unsupported links are failed and their following linked operations canceled.

In particular there are no registered files/buffers/eventfds,
polling, timers, cancellation, sockets, asynchronous workers, SQPOLL,
IOPOLL, personalities, linked execution, buffer selection, multishot, CQE skip,
or eventfd notifications. `READ`/`WRITE`/`READV`/`WRITEV` reject nonzero rw
flags and metadata not supported by this subset. Positioned writes to files
opened with `O_APPEND` use the runtime's existing save/write/restore behavior;
for ordinary files that sequence is not atomic against other shared-offset I/O.
A completed request carries its Linux-style
negative errno in the CQE; an unsupported configuration fails the syscall.
