use std::{ffi::OsString, sync::{mpsc::Sender, Mutex, OnceLock}};
use std::fmt::Display;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::process;


use windows::Win32::{
    UI::{
        WindowsAndMessaging::{
            HHOOK, SetWindowsHookExW, WH_KEYBOARD_LL, UnhookWindowsHookEx, CallNextHookEx,
            GetForegroundWindow, GetWindowThreadProcessId, GetWindowRect,
        },
        Input::KeyboardAndMouse::GetAsyncKeyState,
    },
    Foundation::{HINSTANCE, WPARAM, LPARAM, CloseHandle, LRESULT, RECT, HANDLE},
    System::{
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PROCESS_ACCESS_RIGHTS},
        ProcessStatus::K32GetProcessImageFileNameW,
    },
};

use crate::app::message::{Message, ProcessInfo};

static SENDER: OnceLock<Mutex<Option<Sender<Message>>>> = OnceLock::new();
static HOOK: OnceLock<Mutex<Option<Hook>>> = OnceLock::new();

pub struct ProcessHandle {
    handle: HANDLE,
}

impl ProcessHandle {
    pub fn open(desired_access: PROCESS_ACCESS_RIGHTS, inherit_handle: bool, process_id: u32) -> windows::core::Result<Self> {
        unsafe {
            let handle = OpenProcess(desired_access, inherit_handle, process_id)?;
            Ok(ProcessHandle { handle })
        }
    }

    pub fn handle(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

pub struct Hook {
    hhook : HHOOK
}

// SAFETY: HHOOK is a handle and can be safely sent and shared between threads for this use case.
unsafe impl Send for Hook {}
unsafe impl Sync for Hook {}

impl Hook {
    pub fn new() -> Hook {
        unsafe {
            Hook {
                hhook: SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(hook_callback),
                    Some(HINSTANCE::default()),
                    0).expect("Create hook")
            }
        }
    }
    pub fn uninstall(&self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.hhook);
        }
    }
}

impl Drop for Hook {
    fn drop(&mut self) {
        self.uninstall();
    }
}

pub fn install(sender: Sender<Message>) {
    {
        let mut s = SENDER.get_or_init(|| Mutex::new(None)).lock().unwrap();
        *s = Some(sender);
    }

    let mut hook_lock = HOOK.get_or_init(|| Mutex::new(None)).lock().unwrap();

    if hook_lock.is_none() {
        let hook = Hook::new();
        *hook_lock = Some(hook);
    }
}

pub fn uninstall() {
    {
        let mut hook = HOOK.get_or_init(|| Mutex::new(None)).lock().unwrap();
        if let Some(h) = hook.take() {
            h.uninstall();
        }
    }

    {
        let mut sender = SENDER.get_or_init(|| Mutex::new(None)).lock().unwrap();
        sender.take();
    }
}

unsafe extern "system" fn hook_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if is_ctrl_alt_numpad0(code, wparam, lparam) {
            let fgproc = get_foreground_process();
            if fgproc.pid != process::id() {
                trigger_hook_event(fgproc);
                return LRESULT(1)
            }
        }
        return CallNextHookEx(Some(HHOOK::default()), code, wparam, lparam);
    }
}

fn is_ctrl_alt_numpad0(code: i32, wparam: WPARAM, lparam: LPARAM) -> bool {
    const WM_KEYDOWN : WPARAM = WPARAM(0x0100);

    unsafe {
        if code >= 0 && wparam == WM_KEYDOWN {
            let lparam = *(lparam.0 as *const i32);

            return lparam == 0x60 // Numpad 0
                    && GetAsyncKeyState(0x11).is_negative() // Ctrl
                    && GetAsyncKeyState(0x12).is_negative(); // Alt
        };
        return false;
    }
}

fn get_foreground_process() -> ProcessInfo {
    unsafe {
        let pid: Option<*mut u32> = Some(&mut 0);
        let fg_hwnd = GetForegroundWindow();

        let mut lprect = RECT::default();
        let _ = GetWindowRect(fg_hwnd, &mut lprect);

        GetWindowThreadProcessId(fg_hwnd, pid);

        let pid = *pid.unwrap();
        let process_handle = ProcessHandle::open(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).unwrap();

        let mut file_path: [u16; 500] = [0; 500];
        let file_path_len = K32GetProcessImageFileNameW(process_handle.handle(), &mut file_path) as usize;

        let proc_info = ProcessInfo { pid, name: file_name(file_path, file_path_len) };
        log::trace!("Foreground: {:?} {}", process_handle.handle(), proc_info);

        proc_info
    }
}


fn trigger_hook_event(fgproc: ProcessInfo) {
    let maybe_sender = {
        let sender_lock = SENDER.get_or_init(|| Mutex::new(None)).lock().unwrap();
        sender_lock.clone()
    };

    if let Some(tx) = maybe_sender {
        let _ = tx.send(Message::HookEvt(fgproc));
    }
}

fn file_name(file_path: [u16; 500], len: usize) -> String {
    Path::new(
        &OsString::from_wide(&file_path[0..len])
    ).file_name()
    .unwrap()
    .to_str()
    .unwrap()
    .to_lowercase()
}

impl Display for ProcessInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProcInfo {{ pid: {}, name: '{}' }}", self.pid, &self.name)
    }
}