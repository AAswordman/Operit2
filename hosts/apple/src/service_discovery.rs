//! Uses Apple's Bonjour daemon so discovery follows system local-network permissions.

#[path = "service_discovery/ffi.rs"]
mod ffi;

use ffi::*;
use operit_host_api::ServiceDiscovery::{
    DiscoveredService, DiscoveryAdvertisement, DiscoveryCallback, DiscoverySubscription,
    ServiceAdvertisement, ServiceDiscoveryHost,
};
use operit_host_api::{HostError, HostResult};
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_void, CStr, CString},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
    sync::{mpsc, Arc, Mutex, Weak},
    time::Duration,
};

type ServiceKey = (String, u32);

#[derive(Default)]
struct BrowserState {
    records: BTreeMap<ServiceKey, DiscoveredService>,
    callbacks: BTreeMap<u64, DiscoveryCallback>,
    nextId: u64,
    error: Option<String>,
    ended: bool,
}

impl BrowserState {
    /// Preserves daemon failures instead of turning them into empty device lists.
    fn snapshot(&self) -> HostResult<Vec<DiscoveredService>> {
        if let Some(error) = &self.error {
            return Err(hostError(error));
        }
        if self.ended {
            return Err(hostError("Bonjour browser has stopped"));
        }
        Ok(self.records.values().cloned().collect())
    }
}

enum Command {
    Refresh(mpsc::SyncSender<HostResult<()>>),
    Stop,
}

struct Worker {
    commands: mpsc::Sender<Command>,
    done: Mutex<mpsc::Receiver<()>>,
}

impl Worker {
    /// Starts a Host-owned worker and waits for explicit operation initialization.
    fn start(
        task: impl FnOnce(mpsc::Receiver<Command>, mpsc::SyncSender<HostResult<()>>) -> HostResult<()>
            + Send
            + 'static,
    ) -> HostResult<Self> {
        let (commands, received) = mpsc::channel();
        let (ready, initialized) = mpsc::sync_channel(1);
        let (done, finished) = mpsc::sync_channel(1);
        operit_host_native_common::scheduleNativeServiceTask("operit-bonjour", move || {
            if let Err(error) = task(received, ready) {
                operit_host_api::tryLogHostConsole(6, "ServiceDiscovery", &error.to_string());
            }
            let _ = done.send(());
        })?;
        let worker = Self {
            commands,
            done: Mutex::new(finished),
        };
        initialized
            .recv_timeout(Duration::from_secs(3))
            .map_err(hostError)??;
        Ok(worker)
    }

    /// Restarts the query on its owning worker without replacing subscriber state.
    fn refresh(&self) -> HostResult<()> {
        let (reply, result) = mpsc::sync_channel(1);
        self.commands
            .send(Command::Refresh(reply))
            .map_err(hostError)?;
        result
            .recv_timeout(Duration::from_secs(3))
            .map_err(hostError)?
    }
}

impl Drop for Worker {
    /// Cancels polling and waits until the daemon handles have been released.
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Err(error) = self
            .done
            .lock()
            .expect("Bonjour worker lock poisoned")
            .recv_timeout(Duration::from_secs(3))
        {
            operit_host_api::tryLogHostConsole(
                6,
                "ServiceDiscovery",
                &format!("Bonjour shutdown: {error}"),
            );
        }
    }
}

struct Browser {
    state: Arc<Mutex<BrowserState>>,
    worker: Worker,
}
struct Subscription {
    state: Weak<Mutex<BrowserState>>,
    id: u64,
}
impl DiscoverySubscription for Subscription {}
impl Drop for Subscription {
    /// Removes only this runtime's callback from the shared browser.
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state
                .lock()
                .expect("Bonjour state poisoned")
                .callbacks
                .remove(&self.id);
        }
    }
}

/// Owns Bonjour browsing and registration through the platform-neutral Host contract.
#[derive(Default)]
pub struct AppleServiceDiscoveryHost {
    browsers: Mutex<BTreeMap<String, Arc<Browser>>>,
}

/// Adds operation context to OS, encoding, and worker failures.
fn hostError(error: impl std::fmt::Display) -> HostError {
    HostError::new(format!("Bonjour discovery: {error}"))
}

/// Translates typed DNS-SD errors, including explicit local-network denial.
fn check(code: i32) -> HostResult<()> {
    match code {
        0 => Ok(()),
        -65570 => Err(hostError(
            "local-network access was denied; enable Local Network for Operit2 in system settings",
        )),
        _ => Err(hostError(format!("DNS-SD error {code}"))),
    }
}

/// Converts the local Host service type to Bonjour's registration type.
fn serviceKind(serviceType: &str) -> HostResult<CString> {
    let kind = serviceType
        .strip_suffix(".local.")
        .ok_or_else(|| hostError("service type must end in .local."))?;
    CString::new(kind).map_err(hostError)
}

/// Copies UTF-8 strings provided by the DNS-SD callback ABI.
unsafe fn string(value: *const c_char) -> HostResult<String> {
    if value.is_null() {
        return Err(hostError("DNS-SD returned a null string"));
    }
    CStr::from_ptr(value)
        .to_str()
        .map(str::to_owned)
        .map_err(hostError)
}

/// Reads length-prefixed DNS TXT entries without guessing at protocol contents.
fn decodeTxt(bytes: &[u8]) -> HostResult<BTreeMap<String, String>> {
    let mut properties = BTreeMap::new();
    let mut rest = bytes;
    while let Some((&length, tail)) = rest.split_first() {
        if tail.len() < length as usize {
            return Err(hostError("truncated TXT record"));
        }
        let (entry, tail) = tail.split_at(length as usize);
        rest = tail;
        if entry.is_empty() {
            continue;
        }
        let entry = std::str::from_utf8(entry).map_err(hostError)?;
        let (key, value) = entry
            .split_once('=')
            .ok_or_else(|| hostError("TXT property must have a value"))?;
        properties.insert(key.to_owned(), value.to_owned());
    }
    Ok(properties)
}

/// Encodes the Host's TXT properties with DNS-SD's per-entry size constraint.
fn encodeTxt(properties: BTreeMap<String, String>) -> HostResult<Vec<u8>> {
    let mut txt = Vec::new();
    for (key, value) in properties {
        if key.is_empty() || key.as_bytes().contains(&b'=') {
            return Err(hostError("invalid TXT property key"));
        }
        let entry = format!("{key}={value}");
        txt.push(u8::try_from(entry.len()).map_err(hostError)?);
        txt.extend_from_slice(entry.as_bytes());
    }
    u16::try_from(txt.len()).map_err(hostError)?;
    Ok(txt)
}

/// Decodes Darwin sockaddr bytes while retaining both supported address families.
unsafe fn ipAddress(address: *const SockAddr) -> HostResult<IpAddr> {
    let address = address
        .as_ref()
        .ok_or_else(|| hostError("DNS-SD returned a null address"))?;
    let bytes = std::slice::from_raw_parts(
        address as *const SockAddr as *const u8,
        address.len as usize,
    );
    match (address.family, bytes.len()) {
        (2, 16..) => Ok(IpAddr::V4(Ipv4Addr::from(
            <[u8; 4]>::try_from(&bytes[4..8]).map_err(hostError)?,
        ))),
        (30, 28..) => Ok(IpAddr::V6(Ipv6Addr::from(
            <[u8; 16]>::try_from(&bytes[8..24]).map_err(hostError)?,
        ))),
        _ => Err(hostError("unsupported DNS-SD address family or size")),
    }
}

struct Connection(ServiceRef);
impl Connection {
    /// Opens the system daemon connection on its owning worker thread.
    fn new() -> HostResult<Self> {
        let mut service = ptr::null_mut();
        check(unsafe { DNSServiceCreateConnection(&mut service) })?;
        Ok(Self(service))
    }

    /// Dispatches daemon replies with a 100 ms upper bound on cancellation latency.
    fn poll(&self) -> HostResult<()> {
        let mut fd = PollFd {
            fd: unsafe { DNSServiceRefSockFD(self.0) },
            events: 1,
            revents: 0,
        };
        if fd.fd < 0 {
            return Err(hostError("DNS-SD connection has no event descriptor"));
        }
        let result = unsafe { poll(&mut fd, 1, 100) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                return Ok(());
            }
            return Err(hostError(error));
        }
        if fd.revents & (8 | 16 | 32) != 0 {
            return Err(hostError("DNS-SD event descriptor closed"));
        }
        if fd.revents & 1 != 0 {
            check(unsafe { DNSServiceProcessResult(self.0) })?;
        }
        Ok(())
    }
}
impl Drop for Connection {
    /// Releases the parent connection and every subordinate operation exactly once.
    fn drop(&mut self) {
        unsafe {
            DNSServiceRefDeallocate(self.0);
        }
    }
}

struct Resolution {
    state: Arc<Mutex<BrowserState>>,
    parent: ServiceRef,
    key: ServiceKey,
    resolve: Option<ServiceRef>,
    address: Option<ServiceRef>,
    service: Option<DiscoveredService>,
}
impl Drop for Resolution {
    /// Stops address and SRV/TXT watchers before releasing their callback context.
    fn drop(&mut self) {
        unsafe {
            if let Some(address) = self.address {
                DNSServiceRefDeallocate(address);
            }
            if let Some(resolve) = self.resolve {
                DNSServiceRefDeallocate(resolve);
            }
        }
    }
}

struct Query {
    resolutions: BTreeMap<ServiceKey, Box<Resolution>>,
    state: Arc<Mutex<BrowserState>>,
    connection: Connection,
}

impl Query {
    /// Starts browsing through one shared system connection for all resolutions.
    fn new(kind: &CString, state: Arc<Mutex<BrowserState>>) -> HostResult<Box<Self>> {
        let mut query = Box::new(Self {
            resolutions: BTreeMap::new(),
            state,
            connection: Connection::new()?,
        });
        let mut browse = query.connection.0;
        check(unsafe {
            DNSServiceBrowse(
                &mut browse,
                SHARE_CONNECTION,
                0,
                kind.as_ptr(),
                c"local.".as_ptr(),
                browseReply,
                query.as_mut() as *mut Self as *mut c_void,
            )
        })?;
        Ok(query)
    }
}

/// Records callback failures so the interactive scan returns the actual OS error.
fn failed(state: &Mutex<BrowserState>, result: HostResult<()>) {
    if let Err(error) = result {
        operit_host_api::tryLogHostConsole(6, "ServiceDiscovery", &error.to_string());
        state.lock().expect("Bonjour state poisoned").error = Some(error.to_string());
    }
}

/// Adds and removes per-interface resolutions using escaped DNS-SD instance names.
unsafe extern "C" fn browseReply(
    _: ServiceRef,
    flags: u32,
    interface: u32,
    code: i32,
    name: *const c_char,
    kind: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    let query = &mut *(context as *mut Query);
    let result = (|| {
        check(code)?;
        let mut fullName = [0 as c_char; 1009];
        check(DNSServiceConstructFullName(
            fullName.as_mut_ptr(),
            name,
            kind,
            domain,
        ))?;
        let key = (string(fullName.as_ptr())?, interface);
        if flags & ADD == 0 {
            query.resolutions.remove(&key);
            query
                .state
                .lock()
                .expect("Bonjour state poisoned")
                .records
                .remove(&key);
            return Ok(());
        }
        if query.resolutions.contains_key(&key) {
            return Ok(());
        }
        let mut resolution = Box::new(Resolution {
            state: query.state.clone(),
            parent: query.connection.0,
            key: key.clone(),
            resolve: None,
            address: None,
            service: None,
        });
        let mut service = query.connection.0;
        check(DNSServiceResolve(
            &mut service,
            SHARE_CONNECTION,
            interface,
            name,
            kind,
            domain,
            resolveReply,
            resolution.as_mut() as *mut Resolution as *mut c_void,
        ))?;
        resolution.resolve = Some(service);
        query.resolutions.insert(key, resolution);
        Ok(())
    })();
    failed(&query.state, result);
}

/// Resolves the service metadata before starting its IPv4 and IPv6 address watcher.
unsafe extern "C" fn resolveReply(
    _: ServiceRef,
    _: u32,
    interface: u32,
    code: i32,
    fullName: *const c_char,
    hostname: *const c_char,
    port: u16,
    txtLen: u16,
    txt: *const u8,
    context: *mut c_void,
) {
    let resolution = &mut *(context as *mut Resolution);
    // A service can disappear between its browse event and its SRV/TXT reply.
    if matches!(code, -65538 | -65554) {
        resolution
            .state
            .lock()
            .expect("Bonjour state poisoned")
            .records
            .remove(&resolution.key);
        return;
    }
    let result = (|| {
        check(code)?;
        let properties = if txtLen == 0 {
            BTreeMap::new()
        } else {
            if txt.is_null() {
                return Err(hostError("DNS-SD returned null TXT data"));
            }
            decodeTxt(std::slice::from_raw_parts(txt, txtLen as usize))?
        };
        if let Some(address) = resolution.address.take() {
            DNSServiceRefDeallocate(address);
        }
        resolution.service = Some(DiscoveredService {
            fullName: string(fullName)?,
            hostname: string(hostname)?,
            addresses: Vec::new(),
            port: u16::from_be(port),
            properties,
        });
        resolution
            .state
            .lock()
            .expect("Bonjour state poisoned")
            .records
            .remove(&resolution.key);
        let mut address = resolution.parent;
        check(DNSServiceGetAddrInfo(
            &mut address,
            SHARE_CONNECTION,
            interface,
            3,
            hostname,
            addressReply,
            resolution as *mut Resolution as *mut c_void,
        ))?;
        resolution.address = Some(address);
        Ok(())
    })();
    failed(&resolution.state, result);
}

/// Publishes complete records and removes expired addresses on the same interface.
unsafe extern "C" fn addressReply(
    _: ServiceRef,
    flags: u32,
    _: u32,
    code: i32,
    _: *const c_char,
    address: *const SockAddr,
    _: u32,
    context: *mut c_void,
) {
    let resolution = &mut *(context as *mut Resolution);
    // An absent A or AAAA record is a negative DNS answer, not a broken browser.
    if matches!(code, -65538 | -65554) {
        return;
    }
    let result = (|| {
        check(code)?;
        let ip = ipAddress(address)?;
        let service = resolution
            .service
            .as_mut()
            .ok_or_else(|| hostError("address arrived before service metadata"))?;
        service.addresses.retain(|old| old != &ip);
        if flags & ADD != 0 {
            service.addresses.push(ip);
        }
        let callbacks = {
            let mut state = resolution.state.lock().expect("Bonjour state poisoned");
            if service.addresses.is_empty() {
                state.records.remove(&resolution.key);
                return Ok(());
            }
            state
                .records
                .insert(resolution.key.clone(), service.clone());
            state.callbacks.values().cloned().collect::<Vec<_>>()
        };
        operit_host_api::tryLogHostConsole(
            4,
            "ServiceDiscovery",
            &format!(
                "Bonjour resolved service={} interface={} addresses={:?} port={}",
                service.fullName, resolution.key.1, service.addresses, service.port
            ),
        );
        for callback in callbacks {
            callback(service.clone());
        }
        Ok(())
    })();
    failed(&resolution.state, result);
}

impl AppleServiceDiscoveryHost {
    /// Starts or refreshes the shared browser while preserving runtime subscriptions.
    fn browser(&self, serviceType: &str, refresh: bool) -> HostResult<Arc<Browser>> {
        let mut browsers = self.browsers.lock().map_err(hostError)?;
        if let Some(browser) = browsers.get(serviceType) {
            browser.state.lock().map_err(hostError)?.snapshot()?;
            if refresh {
                browser.worker.refresh()?;
            }
            return Ok(browser.clone());
        }
        let kind = serviceKind(serviceType)?;
        let state = Arc::new(Mutex::new(BrowserState::default()));
        let shared = state.clone();
        let worker = Worker::start(move |commands, ready| {
            let result = (|| {
                let query = Query::new(&kind, shared.clone());
                let _ = ready.send(query.as_ref().map(|_| ()).map_err(hostError));
                let mut query = query?;
                loop {
                    match commands.try_recv() {
                        Ok(Command::Stop) | Err(mpsc::TryRecvError::Disconnected) => break,
                        Ok(Command::Refresh(reply)) => {
                            // Drop subordinate callback contexts before creating the replacement query.
                            drop(query);
                            shared.lock().map_err(hostError)?.records.clear();
                            let next = Query::new(&kind, shared.clone());
                            let _ = reply.send(next.as_ref().map(|_| ()).map_err(hostError));
                            query = next?;
                        }
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                    query.connection.poll()?;
                }
                Ok(())
            })();
            failed(&shared, result.as_ref().map(|_| ()).map_err(hostError));
            shared.lock().expect("Bonjour state poisoned").ended = true;
            result
        })?;
        let browser = Arc::new(Browser { state, worker });
        browsers.insert(serviceType.to_owned(), browser.clone());
        Ok(browser)
    }
}

struct Advertisement {
    _worker: Worker,
}
impl DiscoveryAdvertisement for Advertisement {}

struct Registration {
    connection: Connection,
    status: Box<Option<HostResult<()>>>,
}

impl Registration {
    /// Registers on the system daemon while keeping callback storage behind a stable pointer.
    fn new(name: &CString, kind: &CString, port: u16, txt: &[u8]) -> HostResult<Self> {
        let mut registration = Self {
            connection: Connection::new()?,
            status: Box::new(None),
        };
        let mut service = registration.connection.0;
        check(unsafe {
            DNSServiceRegister(
                &mut service,
                SHARE_CONNECTION,
                0,
                name.as_ptr(),
                kind.as_ptr(),
                c"local.".as_ptr(),
                ptr::null(),
                port.to_be(),
                txt.len() as u16,
                txt.as_ptr().cast(),
                registerReply,
                registration.status.as_mut() as *mut Option<HostResult<()>> as *mut c_void,
            )
        })?;
        Ok(registration)
    }
}

/// Receives system registration status without constructing raw multicast advertisements.
unsafe extern "C" fn registerReply(
    _: ServiceRef,
    _: u32,
    code: i32,
    name: *const c_char,
    _: *const c_char,
    _: *const c_char,
    context: *mut c_void,
) {
    let result = &mut *(context as *mut Option<HostResult<()>>);
    *result = Some(check(code).and_then(|_| {
        operit_host_api::tryLogHostConsole(
            4,
            "ServiceDiscovery",
            &format!("Bonjour advertised {}", string(name)?),
        );
        Ok(())
    }));
}

impl ServiceDiscoveryHost for AppleServiceDiscoveryHost {
    fn localAddresses(&self) -> HostResult<Vec<std::net::IpAddr>> {
        operit_host_native_common::ServiceDiscovery::localNetworkAddresses()
    }

    /// Declares system Bonjour registration support.
    fn supportsAdvertisement(&self) -> bool {
        true
    }

    /// Publishes through the OS daemon without blocking startup on a system permission dialog.
    fn advertise(
        &self,
        service: ServiceAdvertisement,
    ) -> HostResult<Box<dyn DiscoveryAdvertisement>> {
        let kind = serviceKind(&service.serviceType)?;
        let name = CString::new(service.instance).map_err(hostError)?;
        let txt = encodeTxt(service.properties)?;
        let worker = Worker::start(move |commands, ready| {
            let registration = Registration::new(&name, &kind, service.port, &txt);
            let _ = ready.send(registration.as_ref().map(|_| ()).map_err(hostError));
            let mut registration = registration?;
            loop {
                match commands.try_recv() {
                    Ok(Command::Stop) | Err(mpsc::TryRecvError::Disconnected) => break,
                    Ok(Command::Refresh(reply)) => {
                        let _ = reply.send(Err(hostError("advertisements cannot be browsed")));
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                registration.connection.poll()?;
                if let Some(result) = registration.status.take() {
                    result?;
                }
            }
            Ok(())
        })?;
        Ok(Box::new(Advertisement { _worker: worker }))
    }

    /// Actively refreshes Bonjour and returns a bounded discovery snapshot.
    fn discover(&self, serviceType: &str, timeoutMs: u64) -> HostResult<Vec<DiscoveredService>> {
        if timeoutMs == 0 {
            return Err(hostError("service discovery timeout must be positive"));
        }
        let browser = self.browser(serviceType, true)?;
        std::thread::sleep(Duration::from_millis(timeoutMs));
        let records = browser.state.lock().map_err(hostError)?.snapshot()?;
        operit_host_api::tryLogHostConsole(
            4,
            "ServiceDiscovery",
            &format!(
                "Bonjour scan service={serviceType} records={}",
                records.len()
            ),
        );
        Ok(records)
    }

    /// Shares one system browser across discovery and availability subscribers.
    fn subscribe(
        &self,
        serviceType: &str,
        callback: DiscoveryCallback,
    ) -> HostResult<Box<dyn DiscoverySubscription>> {
        let browser = self.browser(serviceType, false)?;
        let (id, records) = {
            let mut state = browser.state.lock().map_err(hostError)?;
            let records = state.snapshot()?;
            let id = state.nextId;
            state.nextId = id
                .checked_add(1)
                .ok_or_else(|| hostError("subscription identifier overflow"))?;
            state.callbacks.insert(id, callback.clone());
            (id, records)
        };
        for record in records {
            callback(record);
        }
        Ok(Box::new(Subscription {
            state: Arc::downgrade(&browser.state),
            id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies TXT encoding and strict length validation against the Host contract.
    #[test]
    fn txtRoundTripAndMalformedRecords() {
        let properties = BTreeMap::from([
            ("nodeId".into(), "peer=1".into()),
            ("transports".into(), "http,ws".into()),
        ]);
        assert_eq!(
            decodeTxt(&encodeTxt(properties.clone()).unwrap()).unwrap(),
            properties
        );
        assert!(decodeTxt(&[5, b'a']).is_err());
        assert!(encodeTxt(BTreeMap::from([("key".into(), "a".repeat(256))])).is_err());
    }

    /// Verifies the system permission denial is not represented as an empty scan.
    #[test]
    fn reportsLocalNetworkDenial() {
        assert!(check(-65570)
            .unwrap_err()
            .to_string()
            .starts_with("Bonjour discovery: local-network access was denied"));
        assert!(serviceKind("_operit-link._tcp.local.").is_ok());
        assert!(serviceKind("_operit-link._tcp.example.").is_err());
    }

    /// Decodes Darwin IPv4 and IPv6 socket layouts without applying address heuristics.
    #[test]
    fn decodesBothDarwinAddressFamilies() {
        let mut v4 = [0u8; 16];
        v4[0] = 16;
        v4[1] = 2;
        v4[4..8].copy_from_slice(&[192, 168, 8, 2]);
        let mut v6 = [0u8; 28];
        v6[0] = 28;
        v6[1] = 30;
        v6[8..24].copy_from_slice(&"fd00::2".parse::<Ipv6Addr>().unwrap().octets());
        assert_eq!(
            unsafe { ipAddress(v4.as_ptr().cast()) }.unwrap(),
            "192.168.8.2".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            unsafe { ipAddress(v6.as_ptr().cast()) }.unwrap(),
            "fd00::2".parse::<IpAddr>().unwrap()
        );
    }

    /// Keeps the other address family usable when Bonjour reports a negative DNS answer.
    #[test]
    fn addressCallbacksPreserveFamiliesAndReportPermissionErrors() {
        let state = Arc::new(Mutex::new(BrowserState::default()));
        let weak = Arc::downgrade(&state);
        state.lock().unwrap().callbacks.insert(
            0,
            Arc::new(move |_| {
                assert!(weak.upgrade().unwrap().try_lock().is_ok());
            }),
        );
        let mut resolution = Resolution {
            state: state.clone(),
            parent: ptr::null_mut(),
            resolve: None,
            address: None,
            key: ("peer._operit-link._tcp.local.".into(), 4),
            service: Some(DiscoveredService {
                fullName: "peer._operit-link._tcp.local.".into(),
                hostname: "peer.local.".into(),
                addresses: Vec::new(),
                port: 37195,
                properties: BTreeMap::new(),
            }),
        };
        let context = &mut resolution as *mut Resolution as *mut c_void;
        let mut address = [0u8; 16];
        address[0] = 16;
        address[1] = 2;
        address[4..8].copy_from_slice(&[192, 168, 8, 2]);
        unsafe {
            addressReply(
                ptr::null_mut(),
                ADD,
                4,
                0,
                ptr::null(),
                address.as_ptr().cast(),
                120,
                context,
            );
        }
        assert_eq!(state.lock().unwrap().snapshot().unwrap().len(), 1);
        unsafe {
            addressReply(
                ptr::null_mut(),
                0,
                4,
                -65554,
                ptr::null(),
                ptr::null(),
                0,
                context,
            );
        }
        assert_eq!(state.lock().unwrap().snapshot().unwrap().len(), 1);
        unsafe {
            addressReply(
                ptr::null_mut(),
                0,
                4,
                0,
                ptr::null(),
                address.as_ptr().cast(),
                0,
                context,
            );
        }
        assert!(state.lock().unwrap().snapshot().unwrap().is_empty());
        unsafe {
            addressReply(
                ptr::null_mut(),
                0,
                4,
                -65570,
                ptr::null(),
                ptr::null(),
                0,
                context,
            );
        }
        assert!(state.lock().unwrap().snapshot().is_err());
    }
}
