use std::{ptr, sync::Mutex};

use snafu::prelude::*;
use tokio::sync::{mpsc, oneshot};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

static mut HOOK: HHOOK = HHOOK(ptr::null_mut());
static SENDER: Mutex<Option<mpsc::UnboundedSender<char>>> = Mutex::new(None);

#[derive(Debug, Snafu)]
pub(crate) enum KBError {
    #[snafu(display("At {location}: Failed to set keyboard hook\n{source}"))]
    HookSet {
        source: windows::core::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Already listening for keys"))]
    AlreadyListening {
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

unsafe extern "system" fn keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode >= 0 {
        let kb = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };

        if matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
            let ch = match kb.vkCode {
                c @ 65..=90 => char::from_u32(c),
                c @ 48..=57 => char::from_u32(c),
                _ => None,
            };
            if let Some(ch) = ch {
                if let Some(tx) = &*SENDER.lock().unwrap_or_else(|p| p.into_inner()) {
                    let _ = tx.send(ch);
                }
            }
        }
    }

    unsafe { CallNextHookEx(Some(HOOK), ncode, wparam, lparam) }
}

pub async fn listen_keys() -> Result<mpsc::UnboundedReceiver<char>, KBError> {
    let mut s = SENDER.lock().unwrap_or_else(|p| p.into_inner());

    snafu::ensure!(
        s.as_ref().is_none_or(|s| s.is_closed()),
        AlreadyListeningSnafu
    );

    let uninit = s.as_ref().is_none();
    let (tx, rx) = mpsc::unbounded_channel();
    *s = Some(tx);
    drop(s);

    if uninit {
        let (init_tx, init_rx) = oneshot::channel();

        std::thread::spawn(move || unsafe {
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) {
                Ok(h) => {
                    HOOK = h;
                    let _ = init_tx.send(Ok(()));
                    h
                }
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    return;
                }
            };

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            UnhookWindowsHookEx(hook).unwrap();
        });

        init_rx.await.unwrap().context(HookSetSnafu)?;
    }

    Ok(rx)
}
