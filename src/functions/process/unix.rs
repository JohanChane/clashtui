use anyhow::{Context, Result};
use std::{
    io::{self, IsTerminal},
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Child, Command, ExitStatus},
};

pub struct PreparedGroup;

pub trait Pipe: io::Read + std::os::fd::AsRawFd {}
impl<T: io::Read + std::os::fd::AsRawFd> Pipe for T {}

pub fn prepare_pipe(pipe: &impl std::os::fd::AsRawFd) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn read_pipe(pipe: &mut impl Pipe, buffer: &mut [u8]) -> io::Result<usize> {
    pipe.read(buffer)
}

pub fn pending_bytes(pipe: &impl Pipe) -> io::Result<usize> {
    let mut count: libc::c_int = 0;
    if unsafe { libc::ioctl(pipe.as_raw_fd(), libc::FIONREAD, &mut count) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(count as usize)
}

pub fn configure(command: &mut Command) -> Result<PreparedGroup> {
    command.process_group(0);
    Ok(PreparedGroup)
}

pub struct ProcessGroup {
    pid: libc::pid_t,
    previous_foreground: Option<libc::pid_t>,
    terminated: bool,
}

impl ProcessGroup {
    pub fn attach(child: &mut Child, foreground: bool, _prepared: PreparedGroup) -> Result<Self> {
        let mut group = Self {
            pid: child.id() as libc::pid_t,
            previous_foreground: None,
            terminated: false,
        };
        if foreground && io::stdin().is_terminal() {
            let previous = unsafe { libc::tcgetpgrp(libc::STDIN_FILENO) };
            if previous >= 0 {
                set_foreground(group.pid)
                    .context("Failed to give command the foreground terminal")?;
                group.previous_foreground = Some(previous);
                // A child may have attempted a terminal read before tcsetpgrp.
                unsafe {
                    libc::kill(-group.pid, libc::SIGCONT);
                }
            }
        }
        Ok(group)
    }

    pub fn terminate(&mut self) {
        if self.terminated {
            return;
        }
        self.terminated = true;
        unsafe {
            // SIGTERM lets sudo relay cancellation to its privileged command.
            libc::kill(-self.pid, libc::SIGTERM);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        unsafe {
            libc::kill(-self.pid, libc::SIGKILL);
        }
    }

    pub fn keep_children(&mut self) -> Result<()> {
        // The command explicitly transfers ownership of its surviving children.
        self.terminated = true;
        Ok(())
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        self.terminate();
        if let Some(previous) = self.previous_foreground {
            let _ = set_foreground(previous);
        }
    }
}

fn set_foreground(group: libc::pid_t) -> io::Result<()> {
    unsafe {
        let mut blocked = std::mem::zeroed::<libc::sigset_t>();
        let mut old = std::mem::zeroed::<libc::sigset_t>();
        libc::sigemptyset(&mut blocked);
        libc::sigaddset(&mut blocked, libc::SIGTTOU);
        let error = libc::pthread_sigmask(libc::SIG_BLOCK, &blocked, &mut old);
        if error != 0 {
            return Err(io::Error::from_raw_os_error(error));
        }
        let result = libc::tcsetpgrp(libc::STDIN_FILENO, group);
        let error = io::Error::last_os_error();
        libc::pthread_sigmask(libc::SIG_SETMASK, &old, std::ptr::null_mut());
        if result == -1 { Err(error) } else { Ok(()) }
    }
}

pub fn was_interrupted(status: ExitStatus) -> bool {
    status.signal() == Some(libc::SIGINT) || status.code() == Some(128 + libc::SIGINT)
}

pub fn install_console_handler() -> Result<()> {
    Ok(())
}

pub fn remove_console_handler() {}

pub fn wait_for_enter() -> Result<()> {
    if !io::stdin().is_terminal() {
        return Ok(());
    }
    while !super::foreground_cancelled() {
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut fd, 1, 50) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if ready > 0 {
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(_) => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(())
}
