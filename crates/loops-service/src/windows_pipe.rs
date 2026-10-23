use music_core::error::Result;
use std::{os::windows::ffi::OsStrExt, path::Path, ptr};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        },
        GetTokenInformation, TokenUser, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

/// Limit local IPC to the current user, including machines with shared sessions.
pub fn create(root: &Path, first: bool) -> Result<NamedPipeServer> {
    let mut hash = 14695981039346656037_u64;
    for unit in root.as_os_str().encode_wide() {
        hash = (hash ^ u64::from(unit)).wrapping_mul(1099511628211);
    }
    let name = format!(r"\\.\pipe\MusicGeneratorLoops-v1-{hash:016x}");
    let descriptor = user_descriptor()?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    // SAFETY: Windows copies the descriptor during CreateNamedPipeW. The aligned
    // attributes and descriptor remain valid until the call returns.
    let result = unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(&name, &attributes as *const _ as *mut _)
    };
    unsafe { LocalFree(descriptor) };
    Ok(result?)
}
fn user_descriptor() -> Result<*mut std::ffi::c_void> {
    // SAFETY: All Win32 output pointers address valid storage; token, SID string
    // and descriptor have distinct ownership. Only the descriptor escapes.
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut required = 0;
        GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut required);
        let mut buffer = vec![0_u64; (required as usize).div_ceil(8)];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            required,
            &mut required,
        );
        CloseHandle(token);
        if ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let mut sid = ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut length = 0;
        while *sid.add(length) != 0 {
            length += 1;
        }
        let identity = String::from_utf16_lossy(std::slice::from_raw_parts(sid, length));
        LocalFree(sid.cast());
        let sddl: Vec<u16> = format!("O:{identity}D:P(A;;GA;;;{identity})")
            .encode_utf16()
            .chain([0])
            .collect();
        let mut descriptor = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(descriptor)
    }
}
