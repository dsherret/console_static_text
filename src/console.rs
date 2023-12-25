use terminal_size::Height;
use terminal_size::Width;

// terminal_size uses stdout by default, but we want to use stderr because
// that's what we output to by default

#[cfg(windows)]
pub fn size() -> Option<(Width, Height)> {
  use std::os::windows::io::RawHandle;
  use windows_sys::Win32::System::Console::GetStdHandle;
  use windows_sys::Win32::System::Console::STD_ERROR_HANDLE;

  let handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) as RawHandle };

  terminal_size::terminal_size_using_handle(handle)
}

#[cfg(not(windows))]
pub fn size() -> Option<(Width, Height)> {
  terminal_size::terminal_size_using_fd(rustix::stdio::raw_stderr())
}
