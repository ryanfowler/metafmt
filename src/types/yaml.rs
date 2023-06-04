use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use super::Format;

#[derive(Clone, Copy, Default)]
pub(crate) struct Yaml {}

impl Format for Yaml {
    fn format(&self, input: &str) -> Result<String, String> {
        let c_in = match CString::new(input) {
            Ok(c_in) => c_in,
            Err(err) => return Err(err.to_string()),
        };
        let mut output: *const c_char = std::ptr::null();
        let mut perr: *const c_char = std::ptr::null();
        unsafe {
            format(c_in.as_ptr(), &mut output, &mut perr);
        }
        match unsafe { perr.as_ref() } {
            Some(perr) => {
                let c_str = unsafe { CStr::from_ptr(perr) };
                match c_str.to_str() {
                    Ok(perr) => Err(perr.to_string()),
                    Err(err) => Err(err.to_string()),
                }
            }
            None => match unsafe { output.as_ref() } {
                None => Err("unknown yaml error".to_owned()),
                Some(output) => {
                    let c_str = unsafe { CStr::from_ptr(output) };
                    match c_str.to_str() {
                        Ok(out) => Ok(out.to_owned()),
                        Err(err) => Err(err.to_string()),
                    }
                }
            },
        }
    }
}

extern "C" {
    fn format(input: *const c_char, output: *mut *const c_char, perr: *mut *const c_char);
}
