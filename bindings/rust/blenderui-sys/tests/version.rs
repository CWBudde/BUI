//! The kit's version is the crate's version: the header, the library and Cargo agree.

use std::ffi::CStr;

#[test]
fn kit_version_is_the_crate_version() {
    // SAFETY: the kit returns a static, NUL-terminated string.
    let s = unsafe { CStr::from_ptr(blenderui_sys::BK_version_string()) }.to_str().unwrap();
    assert_eq!(s, env!("CARGO_PKG_VERSION"));
    let mut parts = s.split('.').map(|p| p.parse::<i32>().unwrap());
    let number = parts.next().unwrap() * 10000 + parts.next().unwrap() * 100 + parts.next().unwrap();
    assert_eq!(unsafe { blenderui_sys::BK_version() }, number);
}
