use std::{ffi::OsString, sync::{mpsc::Sender, Mutex, OnceLock}};
use std::fmt::Display;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::process;


use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, HINSTANCE, LPARAM, LRESULT, RECT, WPARAM}, System::{
        ProcessStatus::K32GetProcessImageFileNameW, Threading::{OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ}
    }, UI::{
        Input::KeyboardAndMouse::GetAsyncKeyState, WindowsAndMessaging::{
            CallNextHookEx, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, WH_KEYBOARD_LL
        }
    }
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

        // Get Window Rect
        let mut lprect = RECT::default();
        let _ = GetWindowRect(fg_hwnd, &mut lprect);

        GetWindowThreadProcessId(fg_hwnd, pid);

        let pid = *pid.unwrap();
        let process_handle = ProcessHandle::open(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).unwrap();

        let mut file_path: [u16; 500] = [0; 500];
        let file_path_len = K32GetProcessImageFileNameW(process_handle.handle(), &mut file_path) as usize;

        // Get window title
        let mut title: [u16; 500] = [0; 500];
        let title_len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(fg_hwnd, &mut title);


        let proc_info = ProcessInfo {
            pid,
            name: file_name(file_path, file_path_len),
            title: title_name(title, title_len),
            hwnd: fg_hwnd.0 as isize
        };
        log::trace!("Foreground: {:?} {}", process_handle.handle(), proc_info);

        proc_info
    }
}

fn title_name(title: [u16; 500], title_len: i32) -> String {
    OsString::from_wide(&title[0..title_len as usize]).to_string_lossy().to_string()
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


pub mod win_icon {

    use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_GETICON, ICON_BIG, ICON_SMALL, GetClassLongPtrW, GCL_HICON, GCL_HICONSM}; // FindWindowW,
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO, DrawIconEx, DI_NORMAL};
    use windows::Win32::Graphics::Gdi::{GetObjectW, DeleteObject, BITMAP, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, GetDC, ReleaseDC, DeleteDC, GetDIBits, BITMAPINFOHEADER, BITMAPINFO, DIB_RGB_COLORS};
    use windows::Win32::UI::WindowsAndMessaging::HICON;
    use windows::Win32::Foundation::{WPARAM, LPARAM};
    use std::{mem, path::PathBuf};
    use image::{RgbaImage}; // Rgba

    /// Get HICON dimensions, returns (width, height) or None if failed
    unsafe fn get_icon_dimensions(hico: HICON) -> Option<(i32, i32)> {
        let mut ii = ICONINFO::default();
        let f_result = GetIconInfo(hico, &mut ii);

        if f_result.is_ok() {
            let mut bm = BITMAP::default();
            let result = GetObjectW(
                ii.hbmMask.into(),
                mem::size_of::<BITMAP>() as i32,
                Some(&mut bm as *mut _ as *mut _)
            );

            let dimensions = if result == mem::size_of::<BITMAP>() as i32 {
                let width = bm.bmWidth;
                let height = if !ii.hbmColor.0.is_null() { bm.bmHeight } else { bm.bmHeight / 2 };
                Some((width, height))
            } else {
                None
            };

            // Clean up resources
            if !ii.hbmMask.0.is_null() {
                let _ = DeleteObject(ii.hbmMask.into());
            }
            if !ii.hbmColor.0.is_null() {
                let _ = DeleteObject(ii.hbmColor.into());
            }

            dimensions
        } else {
            None
        }
    }

    /// Convert HICON to RGBA image data
    unsafe fn hicon_to_rgba(hicon: HICON, width: i32, height: i32) -> Option<RgbaImage> {
        // Get desktop DC (null means desktop)
        let desktop_dc = GetDC(None);
        if desktop_dc.0.is_null() {
            return None;
        }

        // Create compatible DC and bitmap
        let mem_dc = CreateCompatibleDC(Some(desktop_dc));
        let bitmap = CreateCompatibleBitmap(desktop_dc, width, height);

        if mem_dc.0.is_null() || bitmap.0.is_null() {
            ReleaseDC(None, desktop_dc);
            return None;
        }

        let old_bitmap = SelectObject(mem_dc, bitmap.into());

        // Draw the icon onto the bitmap
        let result = DrawIconEx(
            mem_dc,
            0, 0,
            hicon,
            width, height,
            0,
            None, // No background brush
            DI_NORMAL
        );

        if result.is_err() {
            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, desktop_dc);
            return None;
        }

        // Prepare BITMAPINFO structure
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative for top-down bitmap
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };

        // Allocate buffer for pixel data
        let buffer_size = (width * height * 4) as usize;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Get bitmap bits
        let scanlines = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup GDI resources
        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, desktop_dc);

        if scanlines == 0 {
            return None;
        }

        // Convert BGRA to RGBA
        let mut rgba_buffer = Vec::with_capacity(buffer_size);
        for chunk in buffer.chunks_exact(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = chunk[3];
            rgba_buffer.extend_from_slice(&[r, g, b, a]);
        }

        RgbaImage::from_raw(width as u32, height as u32, rgba_buffer)
    }

    /// Get icon from window handle using various methods
    unsafe fn get_window_icon(hwnd: windows::Win32::Foundation::HWND, large: bool) -> Option<HICON> {
        let icon_type = if large { ICON_BIG } else { ICON_SMALL };

        // Try WM_GETICON first
        let icon = SendMessageW(hwnd, WM_GETICON, Some(WPARAM(icon_type as usize)), Some(LPARAM(0)));
        if icon.0 != 0 {
            return Some(HICON(icon.0 as *mut _));
        }

        // Try GetClassLongPtr
        let class_icon = if large {
            GetClassLongPtrW(hwnd, GCL_HICON)
        } else {
            GetClassLongPtrW(hwnd, GCL_HICONSM)
        };

        if class_icon != 0 {
            Some(HICON(class_icon as *mut _))
        } else {
            None
        }
    }

    pub fn save_window_icon(hwnd: windows::Win32::Foundation::HWND, png_path: &PathBuf, large: bool) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            if let Some(hicon) = get_window_icon(hwnd, large) {
                if let Some((width, height)) = get_icon_dimensions(hicon) {
                    log::debug!("Icon dimensions: {}x{}", width, height);
                    if let Some(rgba_img) = hicon_to_rgba(hicon, width, height) {
                        rgba_img.save(png_path)?;
                        log::debug!("Saved icon to {}", png_path.display());
                        // test_png_transparency(png_path.to_str().unwrap())?;
                        return Ok(());
                    } else {
                        log::error!("Failed to convert HICON to RGBA image");
                    }
                } else {
                    log::error!("Failed to get icon dimensions");
                }
            } else {
                log::error!("No icon found for window");
            }
        }
        Err("Failed to save window icon".into())
    }

    pub fn save_first_window_icon(hwnd: windows::Win32::Foundation::HWND, png_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Try large icon first, then small icon
        if save_window_icon(hwnd, png_path, true).is_ok() {
            return Ok(());
        }
        if save_window_icon(hwnd, png_path, false).is_ok() {
            return Ok(());
        }
        Err("No icon found for window".into())
    }

}

