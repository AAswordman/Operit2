//! A fixed Rust heap and guarded task stacks; desktop memory cannot extend these budgets.
use linked_list_allocator::LockedHeap;
use operit_host_native_scheduler::{DEVICE_ASYNC_STACK_BYTES, DEVICE_BLOCKING_STACK_BYTES};
use operit_edge_sdk::runtime::HostRuntimePeerService::PeerRuntimeLimits;
use std::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::{self, NonNull},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Once,
    },
};

pub const MAIN_STACK_BYTES: usize = 16 * 1024;
pub const HEAP_BYTES: usize = 256 * 1024;

#[repr(align(64))]
struct Arena(UnsafeCell<[u8; HEAP_BYTES]>);

struct DeviceHeap {
    arena: Arena,
    initialized: Once,
    heap: LockedHeap,
    peak: AtomicUsize,
    failures: AtomicUsize,
    lastFailure: AtomicUsize,
}

// The allocator lock is the only access to the arena after initialization.
unsafe impl Sync for DeviceHeap {}

impl DeviceHeap {
    /// Initializes the private arena without allocating through the global allocator.
    fn initialize(&self) {
        self.initialized.call_once(|| unsafe {
            self.heap
                .lock()
                .init(self.arena.0.get().cast::<u8>(), HEAP_BYTES);
        });
    }
}

unsafe impl GlobalAlloc for DeviceHeap {
    /// Allocates only from the fixed arena and exposes real exhaustion to the caller.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.initialize();
        let mut heap = self.heap.lock();
        match heap.allocate_first_fit(layout) {
            Ok(pointer) => {
                self.peak.fetch_max(heap.used(), Ordering::Relaxed);
                pointer.as_ptr()
            }
            Err(()) => {
                self.failures.fetch_add(1, Ordering::Relaxed);
                self.lastFailure.store(layout.size(), Ordering::Relaxed);
                ptr::null_mut()
            }
        }
    }

    /// Returns freed blocks to the same arena for reuse and coalescing.
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        self.heap
            .lock()
            .deallocate(NonNull::new_unchecked(pointer), layout);
    }
}

#[cfg_attr(not(test), global_allocator)]
static ALLOCATOR: DeviceHeap = DeviceHeap {
    arena: Arena(UnsafeCell::new([0; HEAP_BYTES])),
    initialized: Once::new(),
    heap: LockedHeap::empty(),
    peak: AtomicUsize::new(0),
    failures: AtomicUsize::new(0),
    lastFailure: AtomicUsize::new(0),
};

/// Reports enforced simulator budgets and allocator measurements from this process.
pub fn snapshot() -> serde_json::Value {
    ALLOCATOR.initialize();
    let (used, free) = {
        let heap = ALLOCATOR.heap.lock();
        (heap.used(), heap.free())
    };
    let limits = PeerRuntimeLimits::constrained();
    serde_json::json!({
        "profile": "esp32-2432s028", "kind": "enforced-budgets", "liveTelemetry": !cfg!(test),
        "heapCapacityBytes": HEAP_BYTES, "heapUsedBytes": used, "heapFreeBytes": free,
        "heapPeakBytes": ALLOCATOR.peak.load(Ordering::Relaxed),
        "allocationFailures": ALLOCATOR.failures.load(Ordering::Relaxed),
        "lastFailedAllocationBytes": ALLOCATOR.lastFailure.load(Ordering::Relaxed),
        "mainStackBytes": MAIN_STACK_BYTES, "asyncStackBytes": DEVICE_ASYNC_STACK_BYTES, "blockingStackBytes": DEVICE_BLOCKING_STACK_BYTES,
        "stackEnforcement": "fixed-mapping-with-guard-page",
        "maxPeerMessageBytes": limits.maxMessageBytes, "incomingSessions": limits.incomingSessions,
        "pendingPairings": limits.pendingPairings, "concurrentProbes": limits.concurrentProbes,
        "enforcedOperations": ["rust-allocation", "main-stack", "async-worker-stack", "blocking-worker-stack", "framed-peer-send", "framed-peer-receive"],
        "note": "Simulator Rust allocations only. Task stacks are separate fixed mappings. C/Wasm UI memory is reported by the renderer; these are not board measurements."
    })
}

/// Touches each frame so overflow probes cannot be optimized into a tail call.
#[inline(never)]
fn consumeStack(depth: usize) -> usize {
    let mut frame = [0u8; 1024];
    unsafe { ptr::write_volatile(frame.as_mut_ptr(), (depth % 251) as u8) };
    std::hint::black_box(&mut frame);
    let child = if depth == 0 {
        0
    } else {
        consumeStack(depth - 1)
    };
    child + unsafe { ptr::read_volatile(frame.as_ptr()) as usize }
}

/// Executes destructive boundary probes in a separate simulator process.
pub fn probe(arguments: &[String]) -> Option<i32> {
    match arguments.first().map(String::as_str) {
        Some("--probe-heap-limit") => {
            let mut bytes = Vec::<u8>::new();
            Some(if bytes.try_reserve_exact(HEAP_BYTES + 1).is_err() {
                0
            } else {
                1
            })
        }
        Some("--probe-heap-fragmentation") => {
            // Fill the actual arena, then leave isolated 8 KiB holes. Total
            // free memory must not permit a larger contiguous allocation.
            const BLOCK: usize = 8 * 1024;
            let layout = Layout::from_size_align(BLOCK, 16).unwrap();
            let larger = Layout::from_size_align(BLOCK + BLOCK / 2, 16).unwrap();
            let mut blocks = [ptr::null_mut(); HEAP_BYTES / BLOCK];
            let mut count: usize = 0;
            for slot in &mut blocks {
                let block = unsafe { ALLOCATOR.alloc(layout) };
                if block.is_null() {
                    break;
                }
                *slot = block;
                count += 1;
            }
            // Retain the final block so no hole joins the trailing free space.
            for slot in blocks[..count.saturating_sub(1)].iter_mut().step_by(2) {
                unsafe { ALLOCATOR.dealloc(*slot, layout) };
                *slot = ptr::null_mut();
            }
            let free = ALLOCATOR.heap.lock().free();
            let fragmented = unsafe { ALLOCATOR.alloc(larger) };
            let rejected = fragmented.is_null();
            if !fragmented.is_null() {
                unsafe { ALLOCATOR.dealloc(fragmented, larger) };
            }
            for block in blocks {
                if !block.is_null() {
                    unsafe { ALLOCATOR.dealloc(block, layout) };
                }
            }
            // The same request must work once neighboring holes coalesce.
            let recovered = unsafe { ALLOCATOR.alloc(larger) };
            let reused = !recovered.is_null();
            if reused {
                unsafe { ALLOCATOR.dealloc(recovered, larger) };
            }
            eprintln!(
                "Fragmentation probe: blocks={count}, free={free}, request={}, rejected={rejected}, reused={reused}",
                larger.size()
            );
            Some(
                if count >= 4 && free > larger.size() && rejected && reused {
                    0
                } else {
                    1
                },
            )
        }
        Some("--probe-heap-fit") => {
            let mut bytes = Vec::new();
            if bytes.try_reserve_exact(HEAP_BYTES / 2).is_err() {
                return Some(1);
            }
            bytes.resize(HEAP_BYTES / 2, 0x5au8);
            std::hint::black_box(&bytes);
            let used = ALLOCATOR.heap.lock().used();
            drop(bytes);
            Some(
                if used >= HEAP_BYTES / 2 && ALLOCATOR.heap.lock().used() < used {
                    0
                } else {
                    1
                },
            )
        }
        Some(mode @ ("--probe-stack-limit" | "--probe-stack-fit")) => {
            use operit_host_api::HostRuntimeTaskSchedulerHost;
            let budget = match arguments.get(1).map(String::as_str) {
                Some("main") => MAIN_STACK_BYTES,
                Some("async") => DEVICE_ASYNC_STACK_BYTES,
                Some("blocking") => DEVICE_BLOCKING_STACK_BYTES,
                _ => return Some(2),
            };
            let overflow = mode == "--probe-stack-limit";
            let depth = if overflow {
                budget / 1024 + 1
            } else {
                budget / 2048
            };
            let task = Box::new(move || {
                std::hint::black_box(consumeStack(depth));
            });
            match arguments.get(1).map(String::as_str) {
                Some("main") => {
                    operit_host_native_scheduler::withStackBudget(MAIN_STACK_BYTES, task).unwrap();
                }
                Some(worker @ ("async" | "blocking")) => {
                    let scheduler =
                        operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost::new()
                            .unwrap();
                    let (sent, received) = std::sync::mpsc::sync_channel(1);
                    let task: operit_host_api::HostRuntimeAsyncTask = Box::new(move || {
                        Box::pin(async move {
                            task();
                            sent.send(()).unwrap();
                        })
                    });
                    match worker {
                        "async" => scheduler
                            .scheduleHostRuntimeAsyncTask("stack-limit-probe", task)
                            .unwrap(),
                        "blocking" => scheduler
                            .scheduleHostRuntimeBlockingAsyncTask("stack-limit-probe", task)
                            .unwrap(),
                        _ => unreachable!(),
                    }
                    received.recv().unwrap();
                }
                _ => return Some(2),
            }
            Some(if overflow { 1 } else { 0 })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    /// The reported capacity describes a fixed allocator, not configured frame limits alone.
    #[test]
    fn reports_enforced_heap_and_stack_budgets() {
        let snapshot = super::snapshot();
        assert_eq!(snapshot["heapCapacityBytes"], 262144);
        assert_eq!(snapshot["mainStackBytes"], 16384);
        assert_eq!(snapshot["maxPeerMessageBytes"], 8192);
        assert_eq!(snapshot["kind"], "enforced-budgets");
        assert!(snapshot.get("largest8BitBlock").is_none());
    }
}
