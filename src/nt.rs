#![allow(dead_code)]

use ntapi::ntobapi::{
    NtClose, NtOpenDirectoryObject, NtQueryDirectoryObject, DIRECTORY_QUERY,
    POBJECT_DIRECTORY_INFORMATION,
};
use std::alloc::{alloc, dealloc, Layout};
use std::mem;
use std::ptr;
use winapi::shared::minwindef::DWORD;
use winapi::shared::ntdef::{HANDLE, NULL, OBJECT_ATTRIBUTES, UNICODE_STRING};
use widestring::{U16CStr, U16CString};

// Define constants that aren't exported by the crates
const STATUS_BUFFER_TOO_SMALL: i32 = 0xC0000023u32 as i32;
const OBJECT_NAME_INFORMATION: u32 = 1;

/// Gets the NT path of a file handle
pub fn get_path(h_file: HANDLE, ps_nt_path: &mut String) -> DWORD {
    if h_file == NULL || h_file == INVALID_HANDLE_VALUE {
        return winapi::shared::winerror::ERROR_INVALID_HANDLE;
    }

    #[repr(C)]
    struct OBJECT_NAME_INFO {
        #[allow(non_snake_case)]
        Name: UNICODE_STRING,
    }

    unsafe {
        // Allocate buffer for path info
        let buffer_size = 2000;
        let layout = Layout::from_size_align(buffer_size, 8).unwrap();
        let buffer = alloc(layout) as *mut u8;
        let pk_info = &mut (*(buffer as *mut OBJECT_NAME_INFO)).Name;
        pk_info.Buffer = ptr::null_mut();
        pk_info.Length = 0;

        let mut req_length: DWORD = 0;
        
        // Use ntapi directly with extern
        extern "system" {
            fn NtQueryObject(
                ObjectHandle: HANDLE,
                ObjectInformationClass: u32,
                ObjectInformation: *mut std::ffi::c_void,
                Length: u32,
                ResultLength: *mut u32,
            ) -> i32;
        }
        
        NtQueryObject(
            h_file,
            OBJECT_NAME_INFORMATION,
            buffer as *mut _,
            buffer_size as u32,
            &mut req_length,
        );

        // Check if buffer is valid
        if pk_info.Buffer.is_null() || pk_info.Length == 0 {
            dealloc(buffer, layout);
            return winapi::shared::winerror::ERROR_FILE_NOT_FOUND;
        }

        // Null-terminate the string (Length in Bytes!)
        *pk_info.Buffer.add(pk_info.Length as usize / 2) = 0;

        // Convert to Rust String
        let path = U16CStr::from_ptr_str(pk_info.Buffer).to_string().unwrap_or_default();
        *ps_nt_path = path;

        dealloc(buffer, layout);
        0
    }
}

/// Finds a symbolic link containing a given name
pub fn find_sym_link(dir: &str, name: &str, out: &mut String) -> bool {
    let wide_dir = U16CString::from_str(dir).unwrap();
    let dir_handle = open_directory(ptr::null_mut(), &wide_dir, DIRECTORY_QUERY);
    
    if dir_handle.is_null() {
        return false;
    }

    let mut found = false;

    unsafe {
        let mut query_context: u32 = 0;
        let mut length: u32;

        loop {
            // Query required buffer size
            length = 0;
            let status = NtQueryDirectoryObject(
                dir_handle,
                ptr::null_mut(),
                0,
                1u8, // TRUE
                0u8, // FALSE
                &mut query_context,
                &mut length,
            );
            
            if status != STATUS_BUFFER_TOO_SMALL {
                break;
            }

            // Allocate buffer
            let layout = Layout::from_size_align(length as usize, 8).unwrap();
            let buffer = alloc(layout);
            
            let status = NtQueryDirectoryObject(
                dir_handle,
                buffer as *mut _,
                length,
                1u8, // TRUE
                0u8, // FALSE
                &mut query_context,
                &mut length,
            );
            
            if status < 0 {
                dealloc(buffer, layout);
                break;
            }

            // Extract name and check if it contains our target
            let obj_inf = buffer as POBJECT_DIRECTORY_INFORMATION;
            
            // Just read Buffer directly before converting to avoid any type mismatch
            let obj_name_buffer = (*obj_inf).Name.Buffer;
            
            if !obj_name_buffer.is_null() {
                let obj_name = U16CStr::from_ptr_str(obj_name_buffer)
                    .to_string()
                    .unwrap_or_default();
                
                if obj_name.contains(name) {
                    found = true;
                    *out = obj_name;
                    dealloc(buffer, layout);
                    break;
                }
            }

            dealloc(buffer, layout);
        }

        NtClose(dir_handle);
    }

    found
}

/// Opens a directory object
fn open_directory(root_handle: HANDLE, dir: &U16CStr, desired_access: u32) -> HANDLE {
    unsafe {
        // Define function pointers for the functions we need
        extern "system" {
            fn RtlInitUnicodeString(
                DestinationString: *mut UNICODE_STRING,
                SourceString: *const u16,
            );
        }
        
        // Initialize UNICODE_STRING directly
        let mut us_dir: UNICODE_STRING = mem::zeroed();
        RtlInitUnicodeString(&mut us_dir, dir.as_ptr());
        
        // Initialize OBJECT_ATTRIBUTES directly
        const OBJ_CASE_INSENSITIVE: u32 = 0x00000040;
        
        let mut obj_attr: OBJECT_ATTRIBUTES = mem::zeroed();
        obj_attr.Length = std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32;
        obj_attr.RootDirectory = root_handle;
        obj_attr.ObjectName = &mut us_dir;
        obj_attr.Attributes = OBJ_CASE_INSENSITIVE;
        obj_attr.SecurityDescriptor = ptr::null_mut();
        obj_attr.SecurityQualityOfService = ptr::null_mut();
        
        let mut dir_handle = ptr::null_mut();
        let status = NtOpenDirectoryObject(&mut dir_handle, desired_access, &mut obj_attr);
        
        if status < 0 {
            return ptr::null_mut();
        }
        
        dir_handle
    }
}

// Constants that may be needed
const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;