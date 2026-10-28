//! Keep the usable stack exact even when macOS has 16 KiB VM pages.
use corosensei::stack::{Stack, StackPointer};

pub(super) struct BoundedStack {
    allocation: *mut libc::c_void,
    mapping_bytes: usize,
    base: StackPointer,
}
impl BoundedStack {
    pub(super) fn new(bytes: usize) -> std::io::Result<Self> {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page <= 0 || bytes < corosensei::stack::MIN_STACK_SIZE || bytes % 16 != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid stack budget",
            ));
        }
        let page = page as usize;
        let writable = bytes
            .checked_add(page - 1)
            .map(|value| value / page * page)
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Stack size overflow")
            })?;
        let mapping_bytes = writable.checked_add(page).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Stack mapping overflow")
        })?;
        let allocation = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mapping_bytes,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if allocation == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        // The base is deliberately below the top of the rounded VM allocation.
        // Only `bytes` are available below the initial stack pointer.
        let stack = Self {
            allocation,
            mapping_bytes,
            base: StackPointer::new(allocation as usize + page + bytes).unwrap(),
        };
        if unsafe {
            libc::mprotect(
                (allocation as *mut u8).add(page).cast(),
                writable,
                libc::PROT_READ | libc::PROT_WRITE,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(stack)
    }
}
// This mapping is exclusive, guarded below and alive until on_stack returns.
unsafe impl Stack for BoundedStack {
    fn base(&self) -> StackPointer {
        self.base
    }
    fn limit(&self) -> StackPointer {
        StackPointer::new(self.allocation as usize).unwrap()
    }
}
impl Drop for BoundedStack {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.allocation, self.mapping_bytes);
        }
    }
}
