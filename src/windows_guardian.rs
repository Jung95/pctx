//! Windows keeper for the existing runner lease service, not a second policy DB.
//! Parent integration must reserve its existing durable slots before starting this
//! keeper, verify the registered guardian executable, then persist the returned
//! ACK in that ledger before resuming the atomically assigned suspended child.
//! The keeper acquires native canonical locks itself; it adopts no external handle.
//!
//! LockFileEx locks belong to the locking process and are released on its death:
//! https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex
//! Therefore guardian death requires the existing ledger to block takeover; this
//! component alone cannot protect a legacy contender that ignores that ledger.
//! Parent death after attachment leaves this independent keeper running normally.
//! Missing/inaccessible job, unsealed admission, or invalid proof retains locks.
//! No heartbeat/TTL/clock-derived boot identity authorizes release.
#![cfg(windows)]
use crate::windows_process::{
    ContainmentObservation, JobIdentity, JobObserver, ProcessIdentity, current_user_sid,
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    ffi::{OsString, c_void},
    fs::{File, OpenOptions},
    io::{self, BufRead, Read, Write},
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Component, Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{
        FILETIME, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE, LocalFree, WAIT_OBJECT_0,
        WAIT_TIMEOUT,
    },
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetSecurityInfo, SDDL_REVISION_1, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, GetAce, GetSecurityDescriptorControl,
        OWNER_SECURITY_INFORMATION, PSID, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        CREATE_NEW, CreateDirectoryW, CreateFileW, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_WRITE_THROUGH,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    },
    System::{
        IO::CancelSynchronousIo,
        Threading::{
            GetCurrentProcessId, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            WaitForSingleObject,
        },
    },
};
const FRAME_LIMIT: u64 = 16384;
const PREFIX: &str = "PCTX-WIN-GUARDIAN-v1 ";
const AUTHORITY: &str = "live_native_handles_only_reboot_recovery_unverified";
const FILE_PRIVATE_ACCESS: u32 = 0x001f_01ff;
const SYNCHRONIZE_RIGHT: u32 = 0x0010_0000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IdentityReceipt {
    pub pid: u32,
    pub creation_filetime: u64,
}
impl From<ProcessIdentity> for IdentityReceipt {
    fn from(p: ProcessIdentity) -> Self {
        Self {
            pid: p.pid,
            creation_filetime: p.creation_filetime,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JobReceipt {
    pub name: String,
    pub owner_sid: String,
}
impl From<&JobIdentity> for JobReceipt {
    fn from(j: &JobIdentity) -> Self {
        Self {
            name: j.name.clone(),
            owner_sid: j.owner_sid.clone(),
        }
    }
}
impl JobReceipt {
    fn native(&self) -> JobIdentity {
        JobIdentity {
            name: self.name.clone(),
            owner_sid: self.owner_sid.clone(),
        }
    }
}
/// The secret nonce is transmitted only over the owned child's anonymous input
/// pipe. Do not format/log this configuration. Receipt files store only its hash.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardianConfiguration {
    pub schema_version: u32,
    pub protocol_dir: PathBuf,
    pub canonical_slots: Vec<PathBuf>,
    pub lease_id: String,
    pub profile_fingerprint: String,
    pub parent: IdentityReceipt,
    pub job: JobReceipt,
    nonce: String,
}
/// Unknown boot identity is explicit. This protocol relies on currently owned
/// kernel process/job objects plus fresh nonce, and grants no reboot recovery.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GuardianReceipt {
    pub schema_version: u32,
    pub phase: String,
    pub lease_id: String,
    pub profile_fingerprint: String,
    pub parent: IdentityReceipt,
    pub guardian: IdentityReceipt,
    pub child: Option<IdentityReceipt>,
    pub job: JobReceipt,
    pub nonce_hash: String,
    pub configuration_hash: String,
    pub admission_sealed: bool,
    pub containment_empty: bool,
    pub boot_id: Option<String>,
    pub authority: String,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum AdmissionMessage {
    Attach {
        nonce: String,
        ready_hash: String,
        parent: IdentityReceipt,
        guardian: IdentityReceipt,
        child: IdentityReceipt,
    },
    AbortBeforeSpawn {
        nonce: String,
        ready_hash: String,
        parent: IdentityReceipt,
        guardian: IdentityReceipt,
    },
}
fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
fn hash_bytes(bytes: &[u8]) -> String {
    crate::domain::hash(bytes)
}
fn receipt_hash(receipt: &GuardianReceipt) -> io::Result<String> {
    Ok(hash_bytes(&serde_json::to_vec(receipt)?))
}
fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut v: Vec<u16> = path.as_os_str().encode_wide().take(32768).collect();
    if v.len() >= 32768 || v.contains(&0) {
        return Err(invalid("invalid protocol path"));
    }
    v.push(0);
    Ok(v)
}
struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
fn descriptor(directory: bool) -> io::Result<LocalAllocation> {
    let sid = current_user_sid()?;
    let flags = if directory { "OICI" } else { "" };
    let sddl: Vec<u16> = format!("O:{sid}D:P(A;{flags};0x{FILE_PRIVATE_ACCESS:x};;;{sid})")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut sd = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(LocalAllocation(sd))
}
fn sid_text(sid: PSID) -> io::Result<String> {
    if sid.is_null() {
        return Err(invalid("missing object owner"));
    }
    let mut raw = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut raw) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let _memory = LocalAllocation(raw.cast());
    let mut len = 0;
    while len < 1024 && unsafe { *raw.add(len) } != 0 {
        len += 1;
    }
    if len == 1024 {
        return Err(invalid("invalid owner SID"));
    }
    String::from_utf16(unsafe { std::slice::from_raw_parts(raw, len) })
        .map_err(|_| invalid("invalid owner SID"))
}
fn private_security(file: &File, directory: bool) -> io::Result<()> {
    let sid = current_user_sid()?;
    let mut owner = ptr::null_mut();
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut sd = ptr::null_mut();
    let error = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut dacl,
            ptr::null_mut(),
            &mut sd,
        )
    };
    if error != 0 {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    let _memory = LocalAllocation(sd);
    let mut control = 0;
    let mut revision = 0;
    if unsafe { GetSecurityDescriptorControl(sd, &mut control, &mut revision) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if sid_text(owner)? != sid
        || control & SE_DACL_PROTECTED == 0
        || dacl.is_null()
        || unsafe { (*dacl).AceCount } != 1
    {
        return Err(invalid("protocol object is not SID-private"));
    }
    let mut raw = ptr::null_mut();
    if unsafe { GetAce(dacl, 0, &mut raw) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let header = unsafe { &*raw.cast::<ACE_HEADER>() };
    let flags = if directory { 3 } else { 0 };
    if header.AceType != 0
        || header.AceFlags != flags
        || usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>()
    {
        return Err(invalid("unexpected protocol ACE"));
    }
    let ace = unsafe { &*raw.cast::<ACCESS_ALLOWED_ACE>() };
    if ace.Mask != FILE_PRIVATE_ACCESS
        || sid_text(ptr::addr_of!(ace.SidStart).cast_mut().cast())? != sid
    {
        return Err(invalid("protocol object grants foreign access"));
    }
    Ok(())
}
/// Pins every existing directory component without FILE_SHARE_DELETE and rejects
/// native reparse attributes. These handles prevent concurrent ancestor rename.
struct PinnedPath {
    _directories: Vec<File>,
}
impl PinnedPath {
    fn directory(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() || path.components().count() > 128 {
            return Err(invalid("bounded absolute protocol directory required"));
        }
        let mut current = PathBuf::new();
        let mut directories = Vec::new();
        for component in path.components() {
            if !matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::Normal(_)
            ) {
                return Err(invalid("protocol path traversal"));
            }
            current.push(component.as_os_str());
            if matches!(component, Component::Prefix(_)) {
                continue;
            }
            let f = OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(&current)?;
            let m = f.metadata()?;
            if !m.is_dir() || m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(invalid("linked protocol ancestor"));
            }
            directories.push(f);
        }
        Ok(Self {
            _directories: directories,
        })
    }
}
fn create_private_directory(path: &Path) -> io::Result<PinnedPath> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("protocol parent unavailable"))?;
    let pin = PinnedPath::directory(parent)?;
    let sd = descriptor(true)?;
    let attrs = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: 0,
    };
    if unsafe { CreateDirectoryW(wide(path)?.as_ptr(), &attrs) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let new_pin = PinnedPath::directory(path)?;
    private_security(
        new_pin
            ._directories
            .last()
            .ok_or_else(|| invalid("protocol directory unavailable"))?,
        true,
    )?;
    drop(pin);
    Ok(new_pin)
}
fn private_create(path: &Path) -> io::Result<File> {
    let sd = descriptor(false)?;
    let attrs = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: 0,
    };
    let raw = unsafe {
        CreateFileW(
            wide(path)?.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ,
            &attrs,
            CREATE_NEW,
            FILE_FLAG_WRITE_THROUGH | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // Owned successful native constructor only; no public raw-handle adoption.
    let file = unsafe { File::from_raw_handle(raw) };
    private_security(&file, false)?;
    Ok(file)
}
fn write_receipt(
    config: &GuardianConfiguration,
    name: &str,
    receipt: &GuardianReceipt,
) -> io::Result<()> {
    let bytes = serde_json::to_vec(receipt)?;
    if bytes.len() as u64 > FRAME_LIMIT {
        return Err(invalid("receipt exceeds protocol bound"));
    }
    let mut file = private_create(&config.protocol_dir.join(name))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
fn read_receipt(config: &GuardianConfiguration, name: &str) -> io::Result<GuardianReceipt> {
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(config.protocol_dir.join(name))?;
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(invalid("linked receipt"));
    }
    private_security(&file, false)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(FRAME_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > FRAME_LIMIT {
        return Err(invalid("receipt exceeds protocol bound"));
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid("invalid durable receipt"))
}
fn read_input<T: for<'de> Deserialize<'de>>(input: &mut impl BufRead) -> io::Result<T> {
    let mut bytes = Vec::new();
    input.take(FRAME_LIMIT + 1).read_until(b'\n', &mut bytes)?;
    if bytes.len() as u64 > FRAME_LIMIT || bytes.last() != Some(&b'\n') {
        return Err(invalid("invalid bounded guardian input"));
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid("invalid guardian protocol input"))
}
fn send(output: &mut impl Write, receipt: &GuardianReceipt) -> io::Result<()> {
    let bytes = serde_json::to_vec(receipt)?;
    if bytes.len() as u64 > FRAME_LIMIT {
        return Err(invalid("receipt exceeds protocol bound"));
    }
    output.write_all(PREFIX.as_bytes())?;
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()
}
fn process_handle(expected: IdentityReceipt) -> io::Result<OwnedHandle> {
    if expected.pid == 0 || expected.creation_filetime == 0 {
        return Err(invalid("incomplete native process identity"));
    }
    let raw = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE_RIGHT,
            0,
            expected.pid,
        )
    };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    let owned = unsafe { OwnedHandle::from_raw_handle(raw) };
    if identity(&owned, expected.pid)? != expected {
        return Err(invalid("native process creation mismatch"));
    }
    Ok(owned)
}
fn identity(handle: &OwnedHandle, pid: u32) -> io::Result<IdentityReceipt> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    if unsafe {
        GetProcessTimes(
            handle.as_raw_handle(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(IdentityReceipt {
        pid,
        creation_filetime: (u64::from(creation.dwHighDateTime) << 32)
            | u64::from(creation.dwLowDateTime),
    })
}
fn exited(handle: &OwnedHandle) -> io::Result<bool> {
    match unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } {
        WAIT_OBJECT_0 => Ok(true),
        WAIT_TIMEOUT => Ok(false),
        _ => Err(io::Error::last_os_error()),
    }
}
fn native_job(receipt: &JobReceipt) -> io::Result<OwnedHandle> {
    let name: Vec<u16> = receipt.name.encode_utf16().chain(Some(0)).collect();
    let raw =
        unsafe { windows_sys::Win32::System::JobObjects::OpenJobObjectW(4, 0, name.as_ptr()) };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}
fn in_job(process: &OwnedHandle, job: &OwnedHandle) -> io::Result<bool> {
    let mut member = 0;
    if unsafe {
        windows_sys::Win32::System::JobObjects::IsProcessInJob(
            process.as_raw_handle(),
            job.as_raw_handle(),
            &mut member,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(member != 0)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
impl GuardianConfiguration {
    /// Create a new SID-private protocol directory underneath an existing trusted
    /// host/lease directory. The caller supplies the existing service's stable
    /// mutex files; this component creates no resource registry or policy database.
    pub fn prepare(
        parent_dir: &Path,
        canonical_slots: Vec<PathBuf>,
        lease_id: String,
        profile_fingerprint: String,
        parent: ProcessIdentity,
        job: &JobIdentity,
    ) -> io::Result<Self> {
        let protocol_dir = parent_dir.join(format!("WG-{}", uuid::Uuid::new_v4()));
        let _pin = create_private_directory(&protocol_dir)?;
        let config = Self {
            schema_version: 1,
            protocol_dir,
            canonical_slots,
            lease_id,
            profile_fingerprint,
            parent: parent.into(),
            job: job.into(),
            nonce: format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
        };
        config.validate()?;
        Ok(config)
    }
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != 1
            || !valid_identifier(&self.lease_id)
            || !valid_hash(&self.profile_fingerprint)
            || !valid_hash(&self.nonce)
            || self.canonical_slots.is_empty()
            || self.canonical_slots.len() > 32
            || self.canonical_slots.iter().any(|p| !p.is_absolute())
            || self.job.owner_sid != current_user_sid()?
        {
            return Err(invalid("invalid guardian configuration"));
        }
        let pin = PinnedPath::directory(&self.protocol_dir)?;
        private_security(
            pin._directories
                .last()
                .ok_or_else(|| invalid("protocol directory unavailable"))?,
            true,
        )?;
        Ok(())
    }
    fn digest(&self) -> io::Result<String> {
        Ok(hash_bytes(&serde_json::to_vec(self)?))
    }
}
/// Owner provisioning only: a new stable mutex file, separate from atomically
/// replaced slot JSON. Existing files are never overwritten or repaired here.
pub fn provision_canonical_slot(path: &Path) -> io::Result<()> {
    let _pin = PinnedPath::directory(
        path.parent()
            .ok_or_else(|| invalid("slot parent unavailable"))?,
    )?;
    private_create(path)?.sync_all()
}
struct CanonicalLocks {
    _files: Vec<File>,
    _pins: Vec<PinnedPath>,
}
impl CanonicalLocks {
    fn acquire(config: &GuardianConfiguration) -> io::Result<Self> {
        let mut paths = config.canonical_slots.clone();
        paths.sort();
        paths.dedup();
        let mut files = Vec::new();
        let mut pins = Vec::new();
        for path in paths {
            let pin = PinnedPath::directory(
                path.parent()
                    .ok_or_else(|| invalid("slot parent unavailable"))?,
            )?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .open(path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            {
                return Err(invalid("linked or non-file canonical slot"));
            }
            private_security(&file, false)?;
            file.try_lock_exclusive()?; // fs2 uses fail-immediately LockFileEx on Windows.
            files.push(file);
            pins.push(pin);
        }
        // Partial failures drop all locks before READY: no execution has been admitted.
        Ok(Self {
            _files: files,
            _pins: pins,
        })
    }
}
fn retain_unknown(output: &mut impl Write, reason: &'static str) -> ! {
    let _ = writeln!(output, "PCTX-WIN-GUARDIAN-UNKNOWN-v1 {reason}");
    let _ = output.flush();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
/// Entry point for the parent's fixed, registered guardian CLI. First input frame
/// is configuration; second is the single attach/abort proof. After sealing, no
/// stdin/parent heartbeat is needed. No arbitrary external handle is accepted.
pub fn run_from_stdio() -> io::Result<GuardianReceipt> {
    let mut input = io::BufReader::new(io::stdin().lock());
    let config: GuardianConfiguration = read_input(&mut input)?;
    let mut output = io::stdout().lock();
    run(config, &mut input, &mut output)
}
/// Pre-READY errors admit no child. After canonical acquisition, uncertain state
/// does not return: it retains locks until an explicit owner intervenes. Caller
/// must not kill this keeper as a timeout-based recovery mechanism.
pub fn run(
    config: GuardianConfiguration,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<GuardianReceipt> {
    config.validate()?;
    let _protocol_pin = PinnedPath::directory(&config.protocol_dir)?;
    let _parent = process_handle(config.parent)?;
    let observer = JobObserver::reopen(&config.job.native())
        .map_err(|_| invalid("job identity unavailable"))?;
    if observer.observe() != ContainmentObservation::EmptyProven {
        return Err(invalid("prepared job is not empty"));
    }
    let job = native_job(&config.job)?;
    let own =
        crate::windows_process::OwnedProcess::open(unsafe { GetCurrentProcessId() })?.identity()?;
    let mut receipt = GuardianReceipt {
        schema_version: 1,
        phase: "ready".into(),
        lease_id: config.lease_id.clone(),
        profile_fingerprint: config.profile_fingerprint.clone(),
        parent: config.parent,
        guardian: own.into(),
        child: None,
        job: config.job.clone(),
        nonce_hash: hash_bytes(config.nonce.as_bytes()),
        configuration_hash: config.digest()?,
        admission_sealed: false,
        containment_empty: true,
        boot_id: None,
        authority: AUTHORITY.into(),
    };
    let ready_hash = receipt_hash(&receipt)?;
    let locks = CanonicalLocks::acquire(&config)?;
    // From here on, every unproven outcome retains the locks in this scope.
    if observer.observe() != ContainmentObservation::EmptyProven {
        retain_unknown(output, "prepared_containment_changed");
    }
    if write_receipt(&config, "ready.json", &receipt)
        .and_then(|_| send(output, &receipt))
        .is_err()
    {
        retain_unknown(output, "ready_publication_failed");
    }
    let message: AdmissionMessage = match read_input(input) {
        Ok(m) => m,
        Err(_) => retain_unknown(output, "admission_proof_missing_or_invalid"),
    };
    let child = match message {
        AdmissionMessage::Attach {
            nonce,
            ready_hash: claimed_hash,
            parent,
            guardian,
            child,
        } => {
            if nonce != config.nonce
                || claimed_hash != ready_hash
                || parent != config.parent
                || guardian != receipt.guardian
            {
                retain_unknown(output, "admission_proof_mismatch");
            }
            let root = match process_handle(child) {
                Ok(p) => p,
                Err(_) => retain_unknown(output, "child_identity_unknown"),
            };
            if !matches!(in_job(&root, &job), Ok(true)) {
                retain_unknown(output, "child_containment_unknown");
            }
            receipt.child = Some(child);
            Some(root)
        }
        AdmissionMessage::AbortBeforeSpawn {
            nonce,
            ready_hash: claimed_hash,
            parent,
            guardian,
        } => {
            if nonce != config.nonce
                || claimed_hash != ready_hash
                || parent != config.parent
                || guardian != receipt.guardian
                || observer.observe() != ContainmentObservation::EmptyProven
            {
                retain_unknown(output, "abort_proof_unknown");
            }
            None
        }
    };
    receipt.admission_sealed = true;
    receipt.phase = if child.is_some() {
        "attached"
    } else {
        "aborted_before_spawn"
    }
    .into();
    receipt.containment_empty = child.is_none();
    // File is SID-private/create-new/WRITE_THROUGH and FlushFileBuffers(sync_all)
    // succeeds before any ACK reaches the caller. No directory-fsync/boot claim.
    if write_receipt(&config, "ack.json", &receipt).is_err() {
        retain_unknown(output, "durable_ack_failed");
    }
    // A dead parent cannot receive this ACK or resume its suspended child. The
    // valid proof is nevertheless sealed: keep observing until cancellation or
    // normal exit proves emptiness, independent of the now-broken parent pipe.
    let _ = send(output, &receipt);
    // Parent death / broken stdin does not stop or unlock a sealed keeper.
    loop {
        let root_gone = child
            .as_ref()
            .is_none_or(|root| matches!(exited(root), Ok(true)));
        if root_gone && observer.observe() == ContainmentObservation::EmptyProven {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    receipt.phase = "released".into();
    receipt.containment_empty = true;
    if write_receipt(&config, "finished.json", &receipt).is_err() {
        retain_unknown(output, "completion_publication_failed");
    }
    // Parent death must not retain a proven-empty sealed lease solely because
    // its reporting pipe has closed. Durable completion remains available.
    let _ = send(output, &receipt);
    drop(locks);
    Ok(receipt)
}

/// Fixed guardian program/entry arguments supplied by the registered runner.
/// The executable hash must be the current trusted registered guardian hash.
pub struct GuardianLaunch {
    pub executable: PathBuf,
    pub executable_hash: String,
    pub args: Vec<OsString>,
    pub environment: std::collections::BTreeMap<OsString, OsString>,
}
pub struct GuardianClient {
    child: Child,
    ipc: Option<ParentIpc>,
    config: GuardianConfiguration,
    ready: GuardianReceipt,
    message_sent: bool,
    sealed_ack: Option<GuardianReceipt>,
    observer: JobObserver,
    _protocol_pin: PinnedPath,
}
struct ParentIpc {
    input: ChildStdin,
    output: ChildStdout,
}
// A Windows cancellation request need not complete immediately. Bound residual
// workers globally rather than pretending cancellation completion is guaranteed.
static IPC_WORKERS: AtomicUsize = AtomicUsize::new(0);
const MAX_IPC_WORKERS: usize = 8;
struct WorkerPermit;
impl WorkerPermit {
    fn acquire() -> io::Result<Self> {
        let mut current = IPC_WORKERS.load(Ordering::Acquire);
        loop {
            if current >= MAX_IPC_WORKERS {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "guardian IPC workers unavailable; retain lease",
                ));
            }
            match IPC_WORKERS.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(Self),
                Err(observed) => current = observed,
            }
        }
    }
}
impl Drop for WorkerPermit {
    fn drop(&mut self) {
        IPC_WORKERS.fetch_sub(1, Ordering::AcqRel);
    }
}
fn ipc_timeout() -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        "guardian IPC timeout; retain lease and keeper",
    )
}
fn input_frame<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() as u64 + 1 > FRAME_LIMIT {
        return Err(invalid("guardian input exceeds bound"));
    }
    bytes.push(b'\n');
    Ok(bytes)
}
fn cancelled(flag: &AtomicBool, deadline: Instant) -> io::Result<()> {
    if flag.load(Ordering::Acquire) || Instant::now() >= deadline {
        Err(ipc_timeout())
    } else {
        Ok(())
    }
}
fn worker_frame(
    output: &mut ChildStdout,
    flag: &AtomicBool,
    deadline: Instant,
) -> io::Result<GuardianReceipt> {
    let mut line = Vec::new();
    let mut total = 0u64;
    loop {
        cancelled(flag, deadline)?;
        let mut one = [0];
        if output.read(&mut one)? == 0 {
            return Err(invalid("guardian ended before receipt"));
        }
        total += 1;
        line.push(one[0]);
        if total > FRAME_LIMIT {
            return Err(invalid("guardian receipt exceeds bound"));
        }
        if one[0] == b'\n' {
            let text = std::str::from_utf8(&line)
                .map_err(|_| invalid("invalid guardian receipt encoding"))?;
            // Isolated native libtest entries have a bounded preamble. Never log it.
            if let Some(offset) = text.find(PREFIX) {
                return serde_json::from_str(&text[offset + PREFIX.len()..])
                    .map_err(|_| invalid("invalid guardian receipt"));
            }
            if text.contains("PCTX-WIN-GUARDIAN-UNKNOWN-v1") {
                return Err(invalid("guardian retains unknown ownership"));
            }
            line.clear();
        }
    }
}
fn supervise_cancellation(worker: std::thread::JoinHandle<()>, flag: Arc<AtomicBool>) {
    flag.store(true, Ordering::Release);
    // CancelSynchronousIo does not wait for completion; ERROR_NOT_FOUND can race
    // with submission. The supervisor owns the worker handle, retries that race,
    // and NEVER performs a blocking join. It does not kill the keeper.
    // https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelsynchronousio
    unsafe {
        CancelSynchronousIo(AsRawHandle::as_raw_handle(&worker));
    }
    let _ = std::thread::Builder::new()
        .name("pctx-guardian-io-cancel".into())
        .spawn(move || {
            while !worker.is_finished() {
                unsafe {
                    CancelSynchronousIo(AsRawHandle::as_raw_handle(&worker));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = worker.join(); // is_finished already proved completion.
        });
    // If spawning the supervisor fails, the first cancellation was still issued.
    // The worker owns all handles and its permit until actual completion; no
    // dangling borrows or unbounded parent wait, and the global cap still applies.
}
fn exchange(
    slot: &mut Option<ParentIpc>,
    bytes: Option<Vec<u8>>,
    deadline: Instant,
) -> io::Result<GuardianReceipt> {
    let permit = WorkerPermit::acquire()?;
    if Instant::now() >= deadline {
        return Err(ipc_timeout());
    }
    let mut ipc = slot
        .take()
        .ok_or_else(|| invalid("guardian IPC unavailable; retain lease"))?;
    let flag = Arc::new(AtomicBool::new(false));
    let worker_flag = Arc::clone(&flag);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name("pctx-guardian-io".into())
        .spawn(move || {
            let _permit = permit;
            let result = (|| {
                if let Some(bytes) = bytes {
                    let mut rest = bytes.as_slice();
                    while !rest.is_empty() {
                        cancelled(&worker_flag, deadline)?;
                        let count = ipc.input.write(rest)?;
                        if count == 0 {
                            return Err(invalid("guardian input closed"));
                        }
                        rest = &rest[count..];
                    }
                    cancelled(&worker_flag, deadline)?;
                    ipc.input.flush()?;
                }
                worker_frame(&mut ipc.output, &worker_flag, deadline)
            })();
            let _ = sender.send((ipc, result));
        })?;
    match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok((ipc, result)) => {
            // Receipt parsing can win the channel race at the exact deadline.
            // Never accept proof after the agreed whole-exchange deadline.
            if Instant::now() >= deadline {
                drop(ipc);
                supervise_cancellation(worker, flag);
                return Err(ipc_timeout());
            }
            if result.is_ok() {
                *slot = Some(ipc);
            }
            // The sender owns only its worker permit now; joining is unnecessary.
            drop(worker);
            result
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            supervise_cancellation(worker, flag);
            Err(ipc_timeout())
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            supervise_cancellation(worker, flag);
            Err(invalid("guardian IPC worker failed; retain lease"))
        }
    }
}
impl GuardianClient {
    /// Starts a keeper outside the target job. The parent must already reserve
    /// its durable lease; startup errors never authorize deleting that reservation.
    pub fn start(config: GuardianConfiguration, launch: &GuardianLaunch) -> io::Result<Self> {
        config.validate()?;
        if !launch.executable.is_absolute()
            || !launch
                .executable
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
            || !valid_hash(&launch.executable_hash)
        {
            return Err(invalid("registered guardian fingerprint mismatch"));
        }
        let _executable_parent = PinnedPath::directory(
            launch
                .executable
                .parent()
                .ok_or_else(|| invalid("guardian executable parent unavailable"))?,
        )?;
        let mut executable_file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&launch.executable)?;
        let metadata = executable_file.metadata()?;
        if !metadata.is_file()
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || metadata.len() > 512 * 1024 * 1024
        {
            return Err(invalid("guardian executable unavailable"));
        }
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = executable_file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if format!("{:x}", digest.finalize()) != launch.executable_hash {
            return Err(invalid("registered guardian fingerprint mismatch"));
        }
        let pin = PinnedPath::directory(&config.protocol_dir)?;
        let observer = JobObserver::reopen(&config.job.native())
            .map_err(|_| invalid("job identity unavailable"))?;
        let mut child = Command::new(&launch.executable)
            .args(&launch.args)
            .env_clear()
            .envs(&launch.environment)
            .current_dir(&config.protocol_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| invalid("guardian input unavailable"))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| invalid("guardian output unavailable"))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut ipc = Some(ParentIpc { input, output });
        let ready = exchange(&mut ipc, Some(input_frame(&config)?), deadline)?;
        let native_guardian = crate::windows_process::OwnedProcess::open(child.id())?.identity()?;
        if ready.schema_version != 1
            || ready.phase != "ready"
            || ready.parent != config.parent
            || ready.guardian != IdentityReceipt::from(native_guardian)
            || ready.job != config.job
            || ready.lease_id != config.lease_id
            || ready.profile_fingerprint != config.profile_fingerprint
            || ready.nonce_hash != hash_bytes(config.nonce.as_bytes())
            || ready.configuration_hash != config.digest()?
            || ready.admission_sealed
            || !ready.containment_empty
            || ready.boot_id.is_some()
            || ready.authority != AUTHORITY
            || read_receipt(&config, "ready.json")? != ready
        {
            return Err(invalid("guardian READY proof mismatch"));
        }
        Ok(Self {
            child,
            ipc,
            config,
            ready,
            message_sent: false,
            sealed_ack: None,
            observer,
            _protocol_pin: pin,
        })
    }
    pub fn ready(&self) -> &GuardianReceipt {
        &self.ready
    }
    /// Accepts the native owned suspended child, never a CLI-supplied raw handle.
    /// Caller must persist this ACK in its existing shared lease ledger before
    /// invoking resume_after_guardian_ack on the child.
    pub fn attach(
        &mut self,
        child: &crate::windows_process::SuspendedChild,
    ) -> io::Result<GuardianReceipt> {
        if self.message_sent {
            return Err(invalid("guardian admission nonce already consumed"));
        }
        let identity = IdentityReceipt::from(child.identity()?);
        self.message_sent = true;
        let deadline = Instant::now() + Duration::from_secs(5);
        let bytes = input_frame(&AdmissionMessage::Attach {
            nonce: self.config.nonce.clone(),
            ready_hash: receipt_hash(&self.ready)?,
            parent: self.config.parent,
            guardian: self.ready.guardian,
            child: identity,
        })?;
        let ack = exchange(&mut self.ipc, Some(bytes), deadline)?;
        self.verify_ack(&ack, Some(identity))?;
        if self.child.try_wait()?.is_some() {
            return Err(invalid("guardian died before parent accepted attachment"));
        }
        self.sealed_ack = Some(ack.clone());
        Ok(ack)
    }
    fn verify_ack(&self, ack: &GuardianReceipt, child: Option<IdentityReceipt>) -> io::Result<()> {
        let expected = GuardianReceipt {
            phase: if child.is_some() {
                "attached"
            } else {
                "aborted_before_spawn"
            }
            .into(),
            child,
            admission_sealed: true,
            containment_empty: child.is_none(),
            ..self.ready.clone()
        };
        if *ack != expected || read_receipt(&self.config, "ack.json")? != *ack {
            return Err(invalid("durable guardian ACK mismatch"));
        }
        Ok(())
    }
    /// Consumes the sole PreparedJob admission capability before sealing an
    /// empty abort, preventing this native API from admitting a late root child.
    pub fn abort_before_spawn(
        &mut self,
        job: crate::windows_process::PreparedJob,
    ) -> io::Result<GuardianReceipt> {
        if self.message_sent
            || JobReceipt::from(job.identity()) != self.config.job
            || job.observe() != ContainmentObservation::EmptyProven
        {
            return Err(invalid("cannot prove empty pre-spawn abort"));
        }
        drop(job);
        self.message_sent = true;
        let deadline = Instant::now() + Duration::from_secs(5);
        let bytes = input_frame(&AdmissionMessage::AbortBeforeSpawn {
            nonce: self.config.nonce.clone(),
            ready_hash: receipt_hash(&self.ready)?,
            parent: self.config.parent,
            guardian: self.ready.guardian,
        })?;
        let ack = exchange(&mut self.ipc, Some(bytes), deadline)?;
        self.verify_ack(&ack, None)?;
        self.sealed_ack = Some(ack.clone());
        Ok(ack)
    }
    /// Success is release eligibility for the existing ledger, not direct ledger
    /// mutation. A dead keeper/live or inaccessible job always returns an error.
    pub fn wait_released(&mut self, timeout: Duration) -> io::Result<GuardianReceipt> {
        if self.sealed_ack.is_none() {
            return Err(invalid("admission is not sealed"));
        }
        let deadline = Instant::now() + timeout.min(Duration::from_secs(30));
        let finished = exchange(&mut self.ipc, None, deadline)?;
        let mut expected = self.sealed_ack.clone().expect("sealed admission");
        expected.phase = "released".into();
        expected.containment_empty = true;
        if finished != expected || read_receipt(&self.config, "finished.json")? != finished {
            return Err(invalid("guardian completion proof mismatch"));
        }
        while self.child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "guardian has not exited; retain lease",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        if self.observer.observe() != ContainmentObservation::EmptyProven {
            return Err(invalid("current containment is not proven empty"));
        }
        Ok(finished)
    }
    pub fn guardian_alive(&mut self) -> io::Result<bool> {
        Ok(self.child.try_wait()?.is_none())
    }
}
// No killing Drop implementation: dropping a parent client closes its pipes,
// while the independently running keeper retains canonical resource ownership.
