use super::*;

use rxqlite_notification::{Action, Notification};
//use flume::Sender;
use libsqlite3_sys as ffi;
use std::os::raw::{c_char, c_int, c_void};

//https://docs.rs/rusqlite/latest/src/rusqlite/hooks.rs.html
unsafe fn expect_utf8<'a>(p_str: *const c_char, description: &'static str) -> &'a str {
    expect_optional_utf8(p_str, description)
        .unwrap_or_else(|| panic!("received empty {description}"))
}

unsafe fn expect_optional_utf8<'a>(
    p_str: *const c_char,
    description: &'static str,
) -> Option<&'a str> {
    if p_str.is_null() {
        return None;
    }
    std::str::from_utf8(std::ffi::CStr::from_ptr(p_str).to_bytes())
        .unwrap_or_else(|_| panic!("received non-utf8 string as {description}"))
        .into()
}

pub fn init_update_handler(handle: &ConnectionHandle) {
    unsafe extern "C" fn call_boxed_closure(
        _p_arg: *mut c_void,
        action_code: c_int,
        p_db_name: *const c_char,
        p_table_name: *const c_char,
        row_id: i64,
    ) {
        let action = Action::from(action_code);
        drop(catch_unwind(|| {
            let tx = &crate::notifications::NOTIFICATION_DISPATCHER
                .get_or_init(Default::default)
                .tx;
            let notification = Notification::Update {
                action,
                database: String::from(expect_utf8(p_db_name, "database name")),
                table: String::from(expect_utf8(p_table_name, "table name")),
                row_id,
            };
            tracing::debug!("dispatching notification: {:?}", notification);
            let _ = tx.send(notification);
        }));
    }
    /*
    let tx = crate::notifications::NOTIFICATION_DISPATCHER.get_or_init(Default::default).tx.clone();
    let hook = move ||->&'static Sender<Notification> {
      &tx
    };
    let free_update_hook = free_boxed_hook::<F> as unsafe fn(*mut c_void);


    let boxed_hook: *mut F = Box::into_raw(Box::new(hook));
    */
    unsafe {
        ffi::sqlite3_update_hook(handle.as_ptr(), Some(call_boxed_closure), 0 as _);
    }
    /*
    free_update_hook
    */
}
