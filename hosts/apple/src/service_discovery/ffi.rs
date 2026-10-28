//! Minimal bindings to Apple's system DNS-SD daemon; no raw multicast sockets.

use std::ffi::{c_char, c_void};

pub type ServiceRef = *mut c_void;
pub const SHARE_CONNECTION: u32 = 0x4000;
pub const ADD: u32 = 0x2;

#[repr(C)]
pub struct SockAddr {
    pub len: u8,
    pub family: u8,
    pub data: [u8; 14],
}

#[repr(C)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

pub type BrowseReply = unsafe extern "C" fn(
    ServiceRef,
    u32,
    u32,
    i32,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
);
pub type ResolveReply = unsafe extern "C" fn(
    ServiceRef,
    u32,
    u32,
    i32,
    *const c_char,
    *const c_char,
    u16,
    u16,
    *const u8,
    *mut c_void,
);
pub type AddressReply = unsafe extern "C" fn(
    ServiceRef,
    u32,
    u32,
    i32,
    *const c_char,
    *const SockAddr,
    u32,
    *mut c_void,
);
pub type RegisterReply = unsafe extern "C" fn(
    ServiceRef,
    u32,
    i32,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
);

// DNS-SD symbols are exported by libSystem on Apple platforms. New SDKs no longer
// ship the separate libdns_sd linker stub.
#[cfg_attr(target_vendor = "apple", link(name = "System"))]
extern "C" {
    /// Opens one shared connection to the system Bonjour daemon.
    pub fn DNSServiceCreateConnection(service: *mut ServiceRef) -> i32;
    /// Releases a connection and all of its subordinate operations.
    pub fn DNSServiceRefDeallocate(service: ServiceRef);
    /// Returns the daemon connection's event descriptor.
    pub fn DNSServiceRefSockFD(service: ServiceRef) -> i32;
    /// Dispatches available replies on the owning worker thread.
    pub fn DNSServiceProcessResult(service: ServiceRef) -> i32;
    /// Starts a browse operation on all local interfaces.
    pub fn DNSServiceBrowse(
        service: *mut ServiceRef,
        flags: u32,
        interface: u32,
        kind: *const c_char,
        domain: *const c_char,
        callback: BrowseReply,
        context: *mut c_void,
    ) -> i32;
    /// Resolves an instance's SRV and TXT records on its discovered interface.
    pub fn DNSServiceResolve(
        service: *mut ServiceRef,
        flags: u32,
        interface: u32,
        name: *const c_char,
        kind: *const c_char,
        domain: *const c_char,
        callback: ResolveReply,
        context: *mut c_void,
    ) -> i32;
    /// Watches IPv4 and IPv6 addresses without opening a peer connection.
    pub fn DNSServiceGetAddrInfo(
        service: *mut ServiceRef,
        flags: u32,
        interface: u32,
        protocol: u32,
        hostname: *const c_char,
        callback: AddressReply,
        context: *mut c_void,
    ) -> i32;
    /// Publishes a service using the daemon's automatically managed local hostname.
    pub fn DNSServiceRegister(
        service: *mut ServiceRef,
        flags: u32,
        interface: u32,
        name: *const c_char,
        kind: *const c_char,
        domain: *const c_char,
        host: *const c_char,
        port: u16,
        txtLen: u16,
        txt: *const c_void,
        callback: RegisterReply,
        context: *mut c_void,
    ) -> i32;
    /// Escapes a DNS-SD instance name into its canonical full service name.
    pub fn DNSServiceConstructFullName(
        fullName: *mut c_char,
        name: *const c_char,
        kind: *const c_char,
        domain: *const c_char,
    ) -> i32;
    /// Waits for daemon events with a bounded cancellation latency.
    pub fn poll(fds: *mut PollFd, count: u32, timeoutMs: i32) -> i32;
}
