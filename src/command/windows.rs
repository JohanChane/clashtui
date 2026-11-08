//! Console/job handling, independent of the configured service manager.

use anyhow::{Context, Result, bail};
use std::{
    io::{self},
    os::windows::{io::AsRawHandle, process::CommandExt},
    process::{Child, Command, ExitStatus},
    ptr,
};

use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, ERROR_BROKEN_PIPE, FILETIME, GetLastError, HANDLE,
        INVALID_HANDLE_VALUE,
    },
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
        Pipes::PeekNamedPipe,
        Threading::{
            CREATE_SUSPENDED, GetThreadTimes, OpenThread, ResumeThread,
            THREAD_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
        },
    },
};

pub trait Pipe: io::Read + AsRawHandle {}
impl<T: io::Read + AsRawHandle> Pipe for T {}

pub fn prepare_pipe(_pipe: &impl AsRawHandle) -> io::Result<()> {
    Ok(())
}

pub fn pending_bytes(pipe: &impl Pipe) -> io::Result<usize> {
    let mut available = 0;
    // Each pipe has a single reader. Read only bytes already available, so a
    // background child retaining the write handle cannot make us wait for EOF.
    if unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &mut available,
            ptr::null_mut(),
        )
    } == 0
    {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
            Ok(0)
        } else {
            Err(error)
        };
    }
    Ok(available as usize)
}

pub fn read_pipe(pipe: &mut impl Pipe, buffer: &mut [u8]) -> io::Result<usize> {
    let available = pending_bytes(pipe)?;
    if available == 0 {
        return Err(io::ErrorKind::WouldBlock.into());
    }
    let count = buffer.len().min(available);
    pipe.read(&mut buffer[..count])
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE) -> io::Result<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn configure(command: &mut Command) -> Result<ProcessGroup> {
    // std encodes argv. Suspension closes the spawn/Job-assignment race.
    // Keep the inherited console and Ctrl-C behavior (no NEW_PROCESS_GROUP).
    command.creation_flags(CREATE_SUSPENDED);
    let name = format!(
        "Local\\ClashtuiProcess-{}-{:016x}{:016x}",
        std::process::id(),
        fastrand::u64(..),
        fastrand::u64(..),
    );
    let wide_name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let job = OwnedHandle::new(unsafe { CreateJobObjectW(ptr::null(), wide_name.as_ptr()) })?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        bail!("Managed command Job name already exists");
    }
    let mut limits = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error().into());
    }
    // Bundled UAC scripts pass this name to their elevated process, which must
    // join the same Job before any service operation. No shell wrapping occurs.
    command.env("CLASHTUI_PROCESS_JOB", name);
    Ok(ProcessGroup { job: Some(job) })
}

pub struct ProcessGroup {
    job: Option<OwnedHandle>,
}

impl ProcessGroup {
    pub fn attach(child: &mut Child, _foreground: bool, group: Self) -> Result<Self> {
        if unsafe { AssignProcessToJobObject(group.job.as_ref().unwrap().0, child.as_raw_handle()) }
            == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        resume_main_thread(child.id()).context("Failed to resume managed command")?;
        Ok(group)
    }

    pub fn terminate(&mut self) {
        if let Some(job) = self.job.take() {
            unsafe {
                TerminateJobObject(job.0, 1);
            }
            // Close immediately. An elevated script still waiting for UAC then
            // cannot open the Job; an already-open handle closes before work.
        }
    }

    pub fn keep_children(&mut self) -> Result<()> {
        if let Some(job) = &self.job {
            let limits = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
            if unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&limits) as u32,
                )
            } == 0
            {
                return Err(io::Error::last_os_error().into());
            }
        }
        // Clearing KILL_ON_JOB_CLOSE before closing releases surviving children.
        self.job.take();
        Ok(())
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn resume_main_thread(process_id: u32) -> Result<()> {
    let snapshot = OwnedHandle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })?;
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut found = unsafe { Thread32First(snapshot.0, &mut entry) };
    let mut candidates = Vec::new();
    while found != 0 {
        if entry.th32OwnerProcessID == process_id {
            let thread = OwnedHandle::new(unsafe {
                OpenThread(
                    THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                    0,
                    entry.th32ThreadID,
                )
            })?;
            let mut created = FILETIME::default();
            let mut exited = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            if unsafe {
                GetThreadTimes(thread.0, &mut created, &mut exited, &mut kernel, &mut user)
            } == 0
            {
                return Err(io::Error::last_os_error().into());
            }
            let created =
                (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
            candidates.push((created, thread));
        }
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        found = unsafe { Thread32Next(snapshot.0, &mut entry) };
    }
    // ToolHelp ordering is unspecified. The initial thread is the oldest;
    // an already-running injected thread must not count as a successful resume.
    candidates.sort_by_key(|(created, _)| *created);
    for (_, thread) in candidates {
        match unsafe { ResumeThread(thread.0) } {
            u32::MAX => return Err(io::Error::last_os_error().into()),
            0 => continue,
            _ => return Ok(()),
        }
    }
    bail!("Suspended command has no accessible main thread")
}

pub fn was_interrupted(status: ExitStatus) -> bool {
    status.code() == Some(0xC000_013Au32 as i32)
}

unsafe extern "system" fn console_control(event: u32) -> i32 {
    use std::sync::atomic::Ordering;
    match event {
        0 | 1 => {
            super::INTERRUPTED.store(true, Ordering::Release);
            1
        }
        2 | 5 | 6 => {
            super::SHUTDOWN.store(true, Ordering::Release);
            1
        }
        _ => 0,
    }
}
pub struct Signals;
impl Signals {
    pub fn install() -> Result<Self> {
        if unsafe {
            windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_control), 1)
        } == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        Ok(Self)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_control), 0);
        }
    }
}

/// A descendant outside the Job may retain stdin. Cancel the dedicated writer's
/// synchronous I/O as well as setting its token; retry to close the check/write race.
pub fn cancel_writer(writer: &std::thread::JoinHandle<()>) {
    while !writer.is_finished() {
        unsafe {
            windows_sys::Win32::System::IO::CancelSynchronousIo(writer.as_raw_handle());
        }
        std::thread::sleep(super::POLL_INTERVAL);
    }
}
