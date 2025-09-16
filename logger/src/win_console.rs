//
// quick hack for colors in windows consoles -> a very stripped down windows
// console coloring from the "term" crate (https://github.com/Stebalien/term)
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct WinConsole;
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::io;
use std::ops::Deref;
use std::ptr;

use winapi::shared::minwindef::WORD;
use winapi::um::fileapi::{CreateFileA, OPEN_EXISTING};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::wincon::SetConsoleTextAttribute;
use winapi::um::wincon::{GetConsoleScreenBufferInfo, CONSOLE_SCREEN_BUFFER_INFO};
use winapi::um::winnt::{FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE, HANDLE};

use super::ColorConsole as Console;
use super::ConsoleColor as Col;
// ----------------------------------------------------------------------------
fn color_to_bits(color: Col) -> u16 {
    match color {
        Col::Black => 0,
        Col::Blue => 0x1,
        Col::Green => 0x2,
        Col::Red => 0x4,
        Col::Yellow => 0x2 | 0x4,
        Col::Magenta => 0x1 | 0x4,
        Col::Cyan => 0x1 | 0x2,
        Col::White => 0x1 | 0x2 | 0x4,
        _ => unreachable!(),
    }
}
// ----------------------------------------------------------------------------
fn bits_to_color(bits: u16) -> Col {
    match bits & 0x7 {
        0 => Col::Black,
        0x1 => Col::Blue,
        0x2 => Col::Green,
        0x4 => Col::Red,
        0x6 => Col::Yellow,
        0x5 => Col::Magenta,
        0x3 => Col::Cyan,
        0x7 => Col::White,
        _ => unreachable!(),
    }
}
// ----------------------------------------------------------------------------
struct HandleWrapper {
    inner: HANDLE,
}
// ----------------------------------------------------------------------------
fn conout() -> io::Result<HandleWrapper> {
    let name = b"CONOUT$\0";
    let handle = unsafe {
        CreateFileA(
            name.as_ptr() as *const i8,
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_WRITE,
            ptr::null_mut(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(HandleWrapper::new(handle))
    }
}
// ----------------------------------------------------------------------------
unsafe fn get_console_screen_buffer_info(handle: HANDLE) -> io::Result<CONSOLE_SCREEN_BUFFER_INFO> {
    let mut buffer_info = ::std::mem::MaybeUninit::uninit();
    if GetConsoleScreenBufferInfo(handle, buffer_info.as_mut_ptr()) == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(buffer_info.assume_init())
    }
}
// ----------------------------------------------------------------------------
fn set_col(foreground: Col) -> io::Result<()> {
    let handle = conout()?;
    let mut accum: WORD = 0;
    accum |= color_to_bits(foreground);
    unsafe {
        if let Ok(buffer_info) = get_console_screen_buffer_info(*handle) {
            // leave background as it is
            let bg = bits_to_color(buffer_info.wAttributes >> 4);
            accum |= color_to_bits(bg) << 4;
        }

        SetConsoleTextAttribute(*handle, accum);
    }
    Ok(())
}
// ----------------------------------------------------------------------------
impl Console for WinConsole {
    fn set_color(col: Col) {
        let result = match col {
            Col::Yellow => set_col(Col::Yellow),
            Col::Red => set_col(Col::Red),
            Col::Blue => set_col(Col::Blue),
            Col::Green => set_col(Col::Green),
            Col::Magenta => set_col(Col::Magenta),
            Col::Cyan => set_col(Col::Cyan),
            _ => set_col(Col::White),
        };

        if let Err(err) = result {
            panic!("failed setting console text attribute: {}", err);
        }
    }
}
// ----------------------------------------------------------------------------
impl HandleWrapper {
    fn new(h: HANDLE) -> HandleWrapper {
        HandleWrapper { inner: h }
    }
}
// ----------------------------------------------------------------------------
impl Drop for HandleWrapper {
    fn drop(&mut self) {
        if self.inner != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.inner);
            }
        }
    }
}
// ----------------------------------------------------------------------------
impl Deref for HandleWrapper {
    type Target = HANDLE;
    fn deref(&self) -> &HANDLE {
        &self.inner
    }
}
// ----------------------------------------------------------------------------
