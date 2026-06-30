use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::ffi::CString;
use libc::{ioctl, TIOCSWINSZ, winsize, pid_t, SIGTERM, waitpid};
use pty::fork::{Fork};
use tauri::State;

/// State to hold the PTY process
pub struct PtyState {
    process: Arc<Mutex<Option<PtyProcess>>>,
}

impl PtyState {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for PtyState {
    fn default() -> Self {
        Self::new()
    }
}

struct PtyProcess {
    child_pid: pid_t,
    master_file: Arc<Mutex<File>>,
    reader_handle: Option<thread::JoinHandle<()>>,
    should_run: Arc<AtomicBool>,
    output_buffer: Arc<Mutex<Vec<u8>>>,
}

#[tauri::command]
pub fn start_pty(
    state: State<'_, PtyState>,
    shell: String,
) -> Result<(), String> {
    eprintln!("PTY: start_pty called with shell: {}", shell);
    let mut lock = state.process.lock().map_err(|e| e.to_string())?;
    if lock.is_some() {
        eprintln!("PTY: already started");
        return Err("PTY already started".into());
    }

    let fork = Fork::from_ptmx().map_err(|e| e.to_string())?;
    eprintln!("PTY: fork created");

    match &fork {
        Fork::Parent(pid, master) => {
            eprintln!("PTY: in parent process, pid: {}", pid);
            let master_raw_fd = master.as_raw_fd();
            let dup_fd = unsafe { libc::dup(master_raw_fd) };
            if dup_fd == -1 {
                return Err(format!("Failed to dup pty fd: {}", io::Error::last_os_error()));
            }
            let master_file = unsafe { File::from_raw_fd(dup_fd) };
            let reader_file = master_file.try_clone().map_err(|e| e.to_string())?;
            let master_file_arc = Arc::new(Mutex::new(master_file));

            let init_win_size = winsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let _ = unsafe {
                ioctl(
                    master_file_arc
                        .lock()
                        .map_err(|e| e.to_string())?
                        .as_raw_fd(),
                    TIOCSWINSZ,
                    &init_win_size as *const _ as *mut libc::c_void,
                )
            };
            eprintln!("PTY: window size set");

            let should_run = Arc::new(AtomicBool::new(true));
            let should_run_clone = should_run.clone();
            let output_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            let output_buffer_clone = output_buffer.clone();

            let reader_handle = thread::spawn(move || {
                eprintln!("PTY: reader thread started");
                let mut buf = reader_file;
                let mut buffer = [0; 1024];
                loop {
                    if !should_run_clone.load(Ordering::SeqCst) {
                        eprintln!("PTY: reader thread stopping");
                        break;
                    }
                    match buf.read(&mut buffer) {
                        Ok(0) => {
                            eprintln!("PTY: reader thread got EOF");
                            break;
                        }
                        Ok(n) => {
                            let mut out = output_buffer_clone.lock().unwrap();
                            let data = &buffer[..n];
                            out.extend_from_slice(data);
                            eprintln!("PTY: read {} bytes (buffer now {})", n, out.len());
                        }
                        Err(e) => {
                            eprintln!("Error reading from PTY: {}", e);
                            break;
                        }
                    }
                }
                eprintln!("PTY: reader thread exited");
            });

            *lock = Some(PtyProcess {
                child_pid: *pid,
                master_file: master_file_arc,
                reader_handle: Some(reader_handle),
                should_run,
                output_buffer,
            });
            eprintln!("PTY: process stored");
            Ok(())
        }
        Fork::Child(slave) => {
            eprintln!("PTY: in child process, preparing to exec shell: {}", shell);
            let slave_fd = slave.as_raw_fd();
            if unsafe { libc::dup2(slave_fd, 0) } == -1 {
                eprintln!("PTY: Failed to dup2 slave to stdin");
                std::process::exit(1);
            }
            if unsafe { libc::dup2(slave_fd, 1) } == -1 {
                eprintln!("PTY: Failed to dup2 slave to stdout");
                std::process::exit(1);
            }
            if unsafe { libc::dup2(slave_fd, 2) } == -1 {
                eprintln!("PTY: Failed to dup2 slave to stderr");
                std::process::exit(1);
            }
            let _ = unsafe { libc::close(slave_fd) };

            let shell_cstr = CString::new(shell.as_str()).map_err(|e| e.to_string())?;
            let argv = [shell_cstr.as_ptr(), std::ptr::null()];
            unsafe {
                libc::execvp(
                    shell_cstr.as_ptr(),
                    argv.as_ptr(),
                );
            }
            eprintln!("PTY: Failed to execute shell: {}", shell);
            std::process::exit(1);
        }
    }
}

#[tauri::command]
pub fn stop_pty(state: State<'_, PtyState>) -> Result<(), String> {
    let mut lock = state.process.lock().map_err(|e| e.to_string())?;
    if let Some(mut process) = lock.take() {
        process.should_run.store(false, Ordering::SeqCst);
        // Kill child first so the PTY slave closes, unblocking the reader thread
        let _ = unsafe { libc::kill(process.child_pid, SIGTERM) };
        // Spawn cleanup in background to avoid blocking the IPC handler
        let child_pid = process.child_pid;
        if let Some(handle) = process.reader_handle.take() {
            std::thread::spawn(move || {
                let _ = unsafe { waitpid(child_pid, std::ptr::null_mut(), 0) };
                let _ = handle.join();
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub fn pty_input(state: State<'_, PtyState>, input: String) -> Result<(), String> {
    let lock = state.process.lock().map_err(|e| e.to_string())?;
    if let Some(process) = lock.as_ref() {
        let mut master_file = process.master_file.lock().map_err(|e| e.to_string())?;
        master_file
            .write_all(input.as_bytes())
            .map_err(|e| e.to_string())?;
    } else {
        return Err("PTY not started".into());
    }
    Ok(())
}

#[tauri::command]
pub fn pty_resize(state: State<'_, PtyState>, rows: u16, cols: u16) -> Result<(), String> {
    let lock = state.process.lock().map_err(|e| e.to_string())?;
    if let Some(process) = lock.as_ref() {
        let master_file = process.master_file.lock().map_err(|e| e.to_string())?;
        let fd = master_file.as_raw_fd();

        #[cfg(target_os = "linux")]
        {
            let win_size = winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let result = unsafe {
                ioctl(
                    fd,
                    TIOCSWINSZ,
                    &win_size as *const _ as *mut libc::c_void,
                )
            };
            if result == -1 {
                return Err(format!(
                    "ioctl failed: {}",
                    io::Error::last_os_error()
                ));
            }
        }
    } else {
        return Err("PTY not started".into());
    }
    Ok(())
}

#[tauri::command]
pub fn read_pty_output(state: State<'_, PtyState>) -> Result<String, String> {
    let lock = state.process.lock().map_err(|e| e.to_string())?;
    if let Some(process) = lock.as_ref() {
        let mut buf = process.output_buffer.lock().map_err(|e| e.to_string())?;
        if buf.is_empty() {
            return Ok(String::new());
        }
        let data = std::mem::take(&mut *buf);
        drop(buf);
        match String::from_utf8(data) {
            Ok(s) => Ok(s),
            Err(e) => {
                Ok(String::from_utf8_lossy(&e.into_bytes()).to_string())
            }
        }
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub fn log_debug(msg: String) -> Result<(), String> {
    eprintln!("[FRONTEND] {}", msg);
    Ok(())
}