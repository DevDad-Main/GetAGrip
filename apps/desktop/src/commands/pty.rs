use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::process::{Command};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use libc::{ioctl, TIOCSWINSZ, winsize, pid_t, SIGTERM, waitpid};
use pty::fork::{Fork};
use tauri::{Emitter, State};

/// State to hold the PTY process
pub struct PtyState {
    process: Arc<Mutex<Option<PtyProcess>>>,
}

struct PtyProcess {
    child_pid: pid_t,
    // We keep the master side of the pty as a File for reading/writing
    master_file: Arc<Mutex<File>>,
    // Handle for the reader thread
    reader_handle: Option<thread::JoinHandle<()>>,
    // Flag to signal the reader thread to stop
    should_run: Arc<AtomicBool>,
}

#[tauri::command]
pub fn start_pty(
    app_handle: tauri::AppHandle,
    state: State<'_, PtyState>,
    shell: String,
) -> Result<(), String> {
    let mut lock = state.process.lock().map_err(|e| e.to_string())?;
    if lock.is_some() {
        return Err("PTY already started".into());
    }

    // Create a new PTY pair using fork
    let fork = Fork::from_ptmx().map_err(|e| e.to_string())?;

    // Match on a reference to the fork to avoid moving it
    match &fork {
        // Parent process: we keep the master PTY
        Fork::Parent(pid, master) => {
            // Convert the master FD into a File for reading/writing and wrap in Arc<Mutex>
            let master_file = unsafe { File::from_raw_fd(master.as_raw_fd()) };
            let master_file_arc = Arc::new(Mutex::new(master_file));

            // Set initial window size (24 rows, 80 cols) - common terminal default
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

            // Clone the app handle for the reader thread
            let app_handle_clone = app_handle.clone();
            // We'll share the master file between threads (clone the Arc)
            let master_file_clone = master_file_arc.clone();
            // Flag to signal the reader thread to stop
            let should_run = Arc::new(AtomicBool::new(true));
            let should_run_clone = should_run.clone();

            // Spawn a thread to read from the PTY master and emit events
            let reader_handle = thread::spawn(move || {
                let mut master_file = master_file_clone.lock().unwrap();
                let mut buffer = [0; 1024];
                loop {
                    // Check if we should stop
                    if !should_run_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    match master_file.read(&mut buffer) {
                        Ok(0) => {
                            // EOF
                            break;
                        }
                        Ok(n) => {
                            let data = String::from_utf8_lossy(&buffer[..n]);
                            // Emit event to frontend
                            let _ = app_handle_clone.emit("pty-output", data.as_ref());
                        }
                        Err(e) => {
                            eprintln!("Error reading from PTY: {}", e);
                            break;
                        }
                    }
                }
            });

            // Store the process
            *lock = Some(PtyProcess {
                child_pid: *pid,
                master_file: master_file_arc,
                reader_handle: Some(reader_handle),
                should_run,
            });

            Ok(())
        }
        // Child process: we set up the slave PTY as stdio and exec the shell
        Fork::Child(slave) => {
            // Convert the slave FD into Files for stdin, stdout, stderr
            let slave_file = unsafe { File::from_raw_fd(slave.as_raw_fd()) };
            let stdin = slave_file
                .try_clone()
                .map_err(|e| e.to_string())?;
            let stdout = slave_file
                .try_clone()
                .map_err(|e| e.to_string())?;
            let stderr = slave_file
                .try_clone()
                .map_err(|e| e.to_string())?;

            // Execute the shell
            Command::new(shell)
                .stdin(stdin)
                .stdout(stdout)
                .stderr(stderr)
                .status()
                .map_err(|e| e.to_string())?;

            // If we reach here, the exec failed
            eprintln!("Failed to execute shell");
            std::process::exit(1);
        }
    }
}

#[tauri::command]
pub fn stop_pty(state: State<'_, PtyState>) -> Result<(), String> {
    let mut lock = state.process.lock().map_err(|e| e.to_string())?;
    if let Some(mut process) = lock.take() {
        // Signal the reader thread to stop
        process.should_run.store(false, Ordering::SeqCst);
        // Wait for the reader thread to finish
        if let Some(handle) = process.reader_handle.take() {
            let _ = handle.join();
        }
        // The reader thread has finished and dropped its clones of the Arcs.
        // Now drop the process (which will drop the master_file Arc and close the FD).
        // Then terminate the child process.
        let _ = unsafe { libc::kill(process.child_pid, SIGTERM) };
        let _ = unsafe { waitpid(process.child_pid, std::ptr::null_mut(), 0) };
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
        let mut master_file = process.master_file.lock().map_err(|e| e.to_string())?;
        let fd = master_file.as_raw_fd();

        #[cfg(target_os = "linux")]
        {
            let win_size = winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };

            // Safe ioctl call with proper error handling
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

        #[cfg(not(target_os = "linux"))]
        {
            // For non-Linux platforms, we could implement platform-specific resizing
            // For now, we'll just return Ok since we're focusing on Linux
            // In a real implementation, we would handle other platforms appropriately
        }
    } else {
        return Err("PTY not started".into());
    }
    Ok(())
}