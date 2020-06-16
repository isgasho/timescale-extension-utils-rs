
use std::{
    ffi::CString,
    fmt,
    os::raw::{c_char, c_int}
};

use crate::sys;

#[derive(Clone, Copy)]
pub enum Level {
    Debug5 = sys::DEBUG5 as isize,
    Debug4 = sys::DEBUG4 as isize,
    Debug3 = sys::DEBUG3 as isize,
    Debug2 = sys::DEBUG2 as isize,
    Debug1 = sys::DEBUG1 as isize,
    Log = sys::LOG as isize,
    LogServerOnly = sys::LOG_SERVER_ONLY as isize,
    Info = sys::INFO as isize,
    Notice = sys::NOTICE as isize,
    Warning = sys::WARNING as isize,
    Error = sys::ERROR as isize,
    Fatal = sys::FATAL as isize,
    Panic = sys::PANIC as isize,
}

impl From<Level> for c_int {
    fn from(level: Level) -> Self {
        level as isize as c_int
    }
}

/// [`Level` enum]: enum.Level.html
#[macro_export]
macro_rules! elog {
    ($lvl:expr, $($arg:tt)+) => ({
        $crate::elog::__private_api_log(
            format_args!($($arg)+),
            $lvl,
            &(
                // Construct zero-terminated strings at compile time.
                concat!(module_path!(), "\0") as *const str as *const ::std::os::raw::c_char,
                concat!(file!(), "\0") as *const str as *const ::std::os::raw::c_char,
                line!(),
            ),
        );
    });
}

// WARNING: this is not part of the crate's public API and is subject to change at any time
#[doc(hidden)]
pub fn __private_api_log(
    args: fmt::Arguments,
    level: Level,
    &(module_path, file, line): &(*const c_char, *const c_char, u32),
) {
    use std::sync::atomic::{compiler_fence, Ordering};

    let errlevel: c_int = c_int::from(level);
    let line = line as c_int;
    const LOG_DOMAIN: *const c_char = "RUST\0" as *const str as *const c_char;

    // Rust has no "function name" macro, for now we use module path instead.
    // See: https://github.com/rust-lang/rfcs/issues/1743
    // let do_log = unsafe {
    //     crate::guard_pg(|| sys::errstart(errlevel, file, line, module_path, LOG_DOMAIN))
    // };

    let do_log = unsafe { sys::errstart(errlevel, file, line, module_path, LOG_DOMAIN) };

    // If errstart returned false, the message won't be seen by anyone; logging will be skipped
    if do_log {
        // At this point we format the passed format string `args`; if the log level is suppressed,
        // no string processing needs to take place.
        let msg = format!("{}", args);
        let c_msg = CString::new(msg).or_else(
            |_| CString::new("failed to convert msg to a CString, check extension code for incompatible `CString` messages")
        ).expect("this should not fail: msg");

        unsafe {
            crate::guard_pg(|| {
                compiler_fence(Ordering::SeqCst);
                let msg_result = sys::errmsg(c_msg.as_ptr());
                sys::errfinish(msg_result);
            });
        }
    }
}
