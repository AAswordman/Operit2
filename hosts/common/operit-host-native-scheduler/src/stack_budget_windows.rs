//! Windows commits exactly the usable stack and cannot grow it through its guard page.
use corosensei::stack::{Stack, StackPointer, StackTebFields};
use windows_sys::Win32::System::{
    Memory::{
        VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
        PAGE_NOACCESS, PAGE_READWRITE,
    },
    SystemInformation::{GetSystemInfo, SYSTEM_INFO},
};

pub(super) struct BoundedStack {
    allocation: *mut std::ffi::c_void,
    base: StackPointer,
    bottom: usize,
}

impl BoundedStack {
    /// Reserves a fixed writable stack above one permanently inaccessible guard page.
    pub(super) fn new(bytes: usize) -> std::io::Result<Self> {
        let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        unsafe { GetSystemInfo(&mut info) };
        let page = info.dwPageSize as usize;
        if bytes < corosensei::stack::MIN_STACK_SIZE || bytes % page != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Stack budget must be page aligned",
            ));
        }
        let allocation = unsafe {
            VirtualAlloc(
                std::ptr::null(),
                bytes + page,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if allocation.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let bottom = allocation as usize + page;
        let stack = Self {
            allocation,
            base: StackPointer::new(bottom + bytes).unwrap(),
            bottom,
        };
        let mut previous = 0;
        if unsafe { VirtualProtect(allocation, page, PAGE_NOACCESS, &mut previous) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(stack)
    }
}

// The mapping is exclusive, has a hard guard and stays mapped until on_stack returns.
unsafe impl Stack for BoundedStack {
    /// Returns the high address of the fixed writable stack.
    fn base(&self) -> StackPointer {
        self.base
    }
    /// Includes the inaccessible guard page in the owned mapping.
    fn limit(&self) -> StackPointer {
        StackPointer::new(self.allocation as usize).unwrap()
    }
    /// Prevents Windows from extending this stack beyond its committed budget.
    fn teb_fields(&self) -> StackTebFields {
        StackTebFields {
            StackBase: self.base.get(),
            StackLimit: self.bottom,
            DeallocationStack: self.allocation as usize,
            GuaranteedStackBytes: 0,
        }
    }
    /// Requires the fixed mapping to retain its original bounds.
    fn update_teb_fields(&mut self, stack_limit: usize, _guaranteed_stack_bytes: usize) {
        assert_eq!(stack_limit, self.bottom, "Device stack bounds changed");
    }
}

impl Drop for BoundedStack {
    /// Releases the complete mapping after execution has returned to the native stack.
    fn drop(&mut self) {
        unsafe { VirtualFree(self.allocation, 0, MEM_RELEASE) };
    }
}
