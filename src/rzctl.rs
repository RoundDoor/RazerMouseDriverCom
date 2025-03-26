#![allow(dead_code)]

use std::cell::RefCell;
use std::ffi::OsStr;
use std::iter::once;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::shared::ntdef::HANDLE;
use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE};
use crate::nt::find_sym_link;

const IOCTL_MOUSE: DWORD = 0x88883020;
const MAX_VAL: i32 = 65536;
const RZCONTROL_MOUSE: i32 = 2;
const RZCONTROL_KEYBOARD: i32 = 1;

#[repr(C)]
struct RzcontrolIoctlStruct {
    unk0: i32,                     // 0x0000
    unk1: i32,                     // 0x0004
    max_val_or_scan_code: i32,     // 0x0008
    click_mask: u32,               // 0x000C - Using u32 for mouse click mask
    unk3: i32,                     // 0x0010
    x: i32,                        // 0x0014
    y: i32,                        // 0x0018
    unk4: i32,                     // 0x001C
}

#[repr(u32)]
pub enum MouseClick {
    LeftDown = 1,
    LeftUp = 2,
    RightDown = 4,
    RightUp = 8,
    ScrollClickDown = 16,
    ScrollClickUp = 32,
    BackDown = 64,
    BackUp = 128,
    ForwardDown = 256,
    ForwardUp = 512,
    ScrollDown = 4287104000,
    ScrollUp = 7865344,
}

#[repr(i32)]
pub enum KeyboardInputType {
    KeyboardDown = 0,
    KeyboardUp = 1,
}

thread_local! {
    static DEVICE_HANDLE: RefCell<HANDLE> = RefCell::new(INVALID_HANDLE_VALUE);
}

// Helper function to convert Rust string to Windows wide string
fn to_wide_chars(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

/// Initialize the connection to the Razer device
pub fn init() -> bool {
    DEVICE_HANDLE.with(|handle| {
        let mut handle = handle.borrow_mut();
        if *handle != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(*handle) };
        }

        // The symlink approach that worked in tests
        let mut sym_name = String::new();
        let found = find_sym_link("\\GLOBAL??", "RZCONTROL", &mut sym_name);
        
        if found {
            let sym_path = format!("\\\\?\\{}", sym_name);
            let path = to_wide_chars(&sym_path);
            
            let device_handle = unsafe {
                CreateFileW(
                    path.as_ptr(),
                    0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    ptr::null_mut(),
                )
            };
            
            if device_handle != INVALID_HANDLE_VALUE {
                *handle = device_handle;
                return true;
            }
        }
        
        false
    })
}

/// Moves the mouse to a specified position
///
/// If from_start_point is true, x and y will be the offset from current mouse position.
/// Otherwise it will be a number in range of 1 to 65536, where 1,1 is top left of screen.
/// Note: x and/or y can not be 0 unless going from start point.
pub fn mouse_move(x: i32, y: i32, from_start_point: bool) {
    let mut max_val = 0;

    // Apply bounds if not moving from start point
    let (x, y) = if !from_start_point {
        max_val = MAX_VAL;
        (
            x.clamp(1, max_val),
            y.clamp(1, max_val),
        )
    } else {
        (x, y)
    };

    let mm = RzcontrolIoctlStruct {
        unk0: 0,
        unk1: RZCONTROL_MOUSE,
        max_val_or_scan_code: max_val,
        click_mask: 0,
        unk3: 0,
        x,
        y,
        unk4: 0,
    };

    _impl_mouse_ioctl(&mm);
}

/// Sends a mouse click event
pub fn mouse_click(click_mask: MouseClick) {
    let click_mask_value = click_mask as u32;
    
    let mm = RzcontrolIoctlStruct {
        unk0: 0,
        unk1: RZCONTROL_MOUSE,
        max_val_or_scan_code: 0,
        click_mask: click_mask_value,
        unk3: 0,
        x: 0,
        y: 0,
        unk4: 0,
    };

    _impl_mouse_ioctl(&mm);
}

/// Sends a keyboard input event
pub fn keyboard_input(scan_code: i32, up_down: KeyboardInputType) {
    let up_down_value = up_down as u32;
    
    let mm = RzcontrolIoctlStruct {
        unk0: 0,
        unk1: RZCONTROL_KEYBOARD,
        max_val_or_scan_code: scan_code << 16,
        click_mask: up_down_value,
        unk3: 0,
        x: 0,
        y: 0,
        unk4: 0,
    };

    _impl_mouse_ioctl(&mm);
}

/// Communicates with the device
fn _impl_mouse_ioctl(ioctl: &RzcontrolIoctlStruct) {
    DEVICE_HANDLE.with(|handle| {
        let handle = *handle.borrow();
        if handle != INVALID_HANDLE_VALUE {
            let mut junk: DWORD = 0;
            let result = unsafe {
                DeviceIoControl(
                    handle,
                    IOCTL_MOUSE,
                    ioctl as *const RzcontrolIoctlStruct as *mut _,
                    mem::size_of::<RzcontrolIoctlStruct>() as DWORD,
                    ptr::null_mut(),
                    0,
                    &mut junk,
                    ptr::null_mut(),
                )
            };

            // Re-open handle in case of failure
            if result == FALSE {
                init();
            }
        }
    });
}