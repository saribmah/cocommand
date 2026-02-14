//! macOS FSEvents FFI wrapper.
//!
//! Provides a safe, RAII wrapper around the Core Services FSEvents API
//! that exposes per-file event IDs — enabling resumable file watching
//! without full rebuilds on restart.

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use bitflags::bitflags;
use core_foundation_sys::array::{CFArrayCreate, CFArrayRef};
use core_foundation_sys::base::{kCFAllocatorDefault, CFIndex, CFRelease};
use core_foundation_sys::runloop::{
    kCFRunLoopDefaultMode, CFRunLoopGetCurrent, CFRunLoopRef, CFRunLoopRun, CFRunLoopStop,
};
use core_foundation_sys::string::{kCFStringEncodingUTF8, CFStringCreateWithBytes, CFStringRef};

// ---------------------------------------------------------------------------
// FSEvents C types and constants
// ---------------------------------------------------------------------------

type FSEventStreamRef = *mut c_void;
type FSEventStreamEventId = u64;

#[repr(C)]
struct FSEventStreamContext {
    version: CFIndex,
    info: *mut c_void,
    retain: Option<extern "C" fn(*const c_void) -> *const c_void>,
    release: Option<extern "C" fn(*const c_void)>,
    copy_description: Option<extern "C" fn(*const c_void) -> CFStringRef>,
}

type FSEventStreamCallback = extern "C" fn(
    stream_ref: FSEventStreamRef,
    client_callback_info: *mut c_void,
    num_events: usize,
    event_paths: *mut c_void,
    event_flags: *const u32,
    event_ids: *const FSEventStreamEventId,
);

// Create flags
const K_FS_EVENT_STREAM_CREATE_FLAG_NO_DEFER: u32 = 0x02;
const K_FS_EVENT_STREAM_CREATE_FLAG_WATCH_ROOT: u32 = 0x04;
const K_FS_EVENT_STREAM_CREATE_FLAG_FILE_EVENTS: u32 = 0x10;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn FSEventStreamCreate(
        allocator: *const c_void,
        callback: FSEventStreamCallback,
        context: *mut FSEventStreamContext,
        paths_to_watch: CFArrayRef,
        since_when: FSEventStreamEventId,
        latency: f64,
        flags: u32,
    ) -> FSEventStreamRef;

    fn FSEventStreamScheduleWithRunLoop(
        stream: FSEventStreamRef,
        run_loop: CFRunLoopRef,
        run_loop_mode: CFStringRef,
    );

    fn FSEventStreamStart(stream: FSEventStreamRef) -> bool;
    fn FSEventStreamStop(stream: FSEventStreamRef);
    fn FSEventStreamInvalidate(stream: FSEventStreamRef);
    fn FSEventStreamRelease(stream: FSEventStreamRef);

    fn FSEventStreamSetExclusionPaths(stream: FSEventStreamRef, paths: CFArrayRef) -> bool;

    fn FSEventsGetCurrentEventId() -> FSEventStreamEventId;
}

// ---------------------------------------------------------------------------
// Event flags
// ---------------------------------------------------------------------------

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FsEventFlags: u32 {
        const MUST_SCAN_SUBDIRS  = 0x0000_0001;
        const EVENT_IDS_WRAPPED  = 0x0000_0008;
        const HISTORY_DONE       = 0x0000_0010;
        const ROOT_CHANGED       = 0x0000_0020;
        const ITEM_CREATED       = 0x0000_0100;
        const ITEM_REMOVED       = 0x0000_0200;
        const ITEM_RENAMED       = 0x0000_1000;
        const ITEM_MODIFIED      = 0x0000_2000;
        const ITEM_IS_FILE       = 0x0001_0000;
        const ITEM_IS_DIR        = 0x0002_0000;
        const ITEM_IS_SYMLINK    = 0x0004_0000;
    }
}

// ---------------------------------------------------------------------------
// Scan type classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsEventScanType {
    Nop,
    ReScan,
    Folder,
    SingleNode,
}

impl FsEventScanType {
    fn classify(flags: FsEventFlags) -> Self {
        if flags.contains(FsEventFlags::HISTORY_DONE)
            || flags.contains(FsEventFlags::EVENT_IDS_WRAPPED)
        {
            return Self::Nop;
        }
        if flags.contains(FsEventFlags::ROOT_CHANGED)
            || flags.contains(FsEventFlags::MUST_SCAN_SUBDIRS)
        {
            return Self::ReScan;
        }
        if flags.contains(FsEventFlags::ITEM_IS_DIR) {
            return Self::Folder;
        }
        Self::SingleNode
    }
}

// ---------------------------------------------------------------------------
// Parsed event
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct FsEvent {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub flags: FsEventFlags,
    pub event_id: u64,
    pub scan_type: FsEventScanType,
}

// ---------------------------------------------------------------------------
// RAII FsEventStream wrapper
// ---------------------------------------------------------------------------

/// A wrapper around `CFRunLoopRef` that is `Send` + `Sync`.
///
/// Safety: `CFRunLoopStop` is documented as thread-safe — it may be called
/// from any thread to stop a run loop running on another thread.
#[derive(Clone, Copy)]
struct SendableRunLoop(CFRunLoopRef);
unsafe impl Send for SendableRunLoop {}
unsafe impl Sync for SendableRunLoop {}

pub struct FsEventStream {
    run_loop: SendableRunLoop,
    _thread: JoinHandle<()>,
}

impl FsEventStream {
    pub fn new<F>(
        path: &Path,
        exclusion_paths: &[PathBuf],
        since_event_id: u64,
        latency: f64,
        handler: F,
    ) -> Self
    where
        F: Fn(Vec<FsEvent>) + Send + Sync + 'static,
    {
        let path_string = path.to_string_lossy().to_string();
        let exclusion_strings: Vec<String> = exclusion_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let handler = Arc::new(handler);
        let run_loop_slot: Arc<std::sync::Mutex<Option<SendableRunLoop>>> =
            Arc::new(std::sync::Mutex::new(None));
        let run_loop_ready = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));

        let run_loop_slot_clone = run_loop_slot.clone();
        let run_loop_ready_clone = run_loop_ready.clone();

        let thread = thread::spawn(move || {
            // Safety: all FFI calls below follow the documented CoreServices API contract.
            unsafe {
                let cf_path = str_to_cfstring(&path_string);
                let path_array = CFArrayCreate(
                    kCFAllocatorDefault,
                    &cf_path as *const _ as *const *const c_void,
                    1,
                    std::ptr::null(),
                );

                let handler_ptr = Arc::into_raw(handler.clone()) as *mut c_void;

                let mut context = FSEventStreamContext {
                    version: 0,
                    info: handler_ptr,
                    retain: None,
                    release: None,
                    copy_description: None,
                };

                let flags = K_FS_EVENT_STREAM_CREATE_FLAG_NO_DEFER
                    | K_FS_EVENT_STREAM_CREATE_FLAG_FILE_EVENTS
                    | K_FS_EVENT_STREAM_CREATE_FLAG_WATCH_ROOT;

                let stream = FSEventStreamCreate(
                    kCFAllocatorDefault,
                    fsevent_callback::<F>,
                    &mut context,
                    path_array,
                    since_event_id,
                    latency,
                    flags,
                );

                // Set exclusion paths if any
                if !exclusion_strings.is_empty() {
                    let cf_exclusions: Vec<CFStringRef> = exclusion_strings
                        .iter()
                        .map(|s| str_to_cfstring(s))
                        .collect();
                    let exclusion_array = CFArrayCreate(
                        kCFAllocatorDefault,
                        cf_exclusions.as_ptr() as *const *const c_void,
                        cf_exclusions.len() as CFIndex,
                        std::ptr::null(),
                    );
                    FSEventStreamSetExclusionPaths(stream, exclusion_array);
                    CFRelease(exclusion_array as *const c_void);
                    for cf_str in cf_exclusions {
                        CFRelease(cf_str as *const c_void);
                    }
                }

                let current_run_loop = CFRunLoopGetCurrent();
                FSEventStreamScheduleWithRunLoop(stream, current_run_loop, kCFRunLoopDefaultMode);

                FSEventStreamStart(stream);

                // Publish run loop ref so the owner can stop it
                {
                    let mut slot = run_loop_slot_clone.lock().unwrap();
                    *slot = Some(SendableRunLoop(current_run_loop));
                    let (lock, cvar) = &*run_loop_ready_clone;
                    let mut ready = lock.lock().unwrap();
                    *ready = true;
                    cvar.notify_all();
                }

                CFRunLoopRun();

                // Cleanup
                FSEventStreamStop(stream);
                FSEventStreamInvalidate(stream);
                FSEventStreamRelease(stream);
                CFRelease(path_array as *const c_void);
                CFRelease(cf_path as *const c_void);

                // Drop the handler Arc we leaked into the context
                drop(Arc::from_raw(handler_ptr as *const F));
            }
        });

        // Wait for the run loop to be ready
        let (lock, cvar) = &*run_loop_ready;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }

        let run_loop = run_loop_slot.lock().unwrap().unwrap();

        Self {
            run_loop,
            _thread: thread,
        }
    }

    pub fn current_event_id() -> u64 {
        unsafe { FSEventsGetCurrentEventId() }
    }
}

impl Drop for FsEventStream {
    fn drop(&mut self) {
        unsafe {
            CFRunLoopStop(self.run_loop.0);
        }
        // The thread will exit after CFRunLoopRun returns, clean up the stream,
        // and terminate. We don't join here to avoid blocking the drop.
    }
}

// ---------------------------------------------------------------------------
// FFI callback
// ---------------------------------------------------------------------------

extern "C" fn fsevent_callback<F>(
    _stream_ref: FSEventStreamRef,
    client_callback_info: *mut c_void,
    num_events: usize,
    event_paths: *mut c_void,
    event_flags: *const u32,
    event_ids: *const FSEventStreamEventId,
) where
    F: Fn(Vec<FsEvent>) + Send + 'static,
{
    let mut events = Vec::with_capacity(num_events);

    unsafe {
        let paths_ptr = event_paths as *const *const c_char;
        for i in 0..num_events {
            let c_path = *paths_ptr.add(i);
            let path_str = CStr::from_ptr(c_path).to_string_lossy();
            let flags_raw = *event_flags.add(i);
            let event_id = *event_ids.add(i);
            let flags = FsEventFlags::from_bits_truncate(flags_raw);
            let scan_type = FsEventScanType::classify(flags);

            events.push(FsEvent {
                path: PathBuf::from(path_str.as_ref()),
                flags,
                event_id,
                scan_type,
            });
        }

        let handler = &*(client_callback_info as *const F);
        handler(events);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

unsafe fn str_to_cfstring(s: &str) -> CFStringRef {
    CFStringCreateWithBytes(
        kCFAllocatorDefault,
        s.as_ptr(),
        s.len() as CFIndex,
        kCFStringEncodingUTF8,
        false as u8,
    )
}
