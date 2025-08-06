use windows::{
    Win32::{
        Foundation::{ HWND, LPARAM, LRESULT, WPARAM },
        UI::WindowsAndMessaging::{
                DefWindowProcW, GetWindowLongPtrW,
                SetWindowLongPtrW, CREATESTRUCTW, GWLP_USERDATA, WM_NCCREATE, WM_NCDESTROY,
            },
    }
};

use super::traits::Window;
use super::app_registry::with_app_handler;

pub unsafe extern "system" fn wnd_proc_router<T: Window>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {

    if should_log_message(msg) {
        log_message(hwnd, msg, wparam, lparam);
    }

    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const CREATESTRUCTW);
        let this = cs.lpCreateParams as *mut T;
        if !this.is_null() {
            (*(this as *mut dyn Window)).set_hwnd(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, this as isize);
        }
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let this_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut T;
    if this_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let this = &mut *this_ptr;
    let result = if let Some(result) = this.handle_message(hwnd, msg, wparam, lparam) {
        result
    } else if let Some(result) = with_app_handler(|app| {
        (*app).app_wnd_proc(hwnd, msg, wparam, lparam)
    }) {
        result
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    };

    if should_log_message(msg) {
        log_response(hwnd, msg, result);
    }

    result
}

fn should_log_message(_msg: u32) -> bool {
    false
}

fn log_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) {
    log::trace!("Got Message ({})({:?}): {} wp {:#x} lp {:#x}", msg, hwnd, msg_name(msg), wparam.0, lparam.0);
}

fn log_response(hwnd: HWND, msg: u32, lresult: LRESULT) {
    log::info!("Got Response ({}): {:?} @ {:?}", msg, lresult, hwnd);
}

fn msg_name(msg:u32) -> &'static str {
    match msg {
        000u32    => "WM_NULL",
        001u32    => "WM_CREATE",
        002u32    => "WM_DESTROY",
        003u32    => "WM_MOVE",
        004u32    => "WMSZ_TOPLEFT",
        005u32    => "WM_SIZE",
        006u32    => "WM_ACTIVATE",
        007u32    => "WM_SETFOCUS",
        008u32    => "WM_KILLFOCUS",
        010u32    => "WM_ENABLE",
        011u32    => "WM_SETREDRAW",
        012u32    => "WM_SETTEXT",
        013u32    => "WM_GETTEXT",
        014u32    => "WM_GETTEXTLENGTH",
        015u32    => "WM_PAINT",
        016u32    => "WM_CLOSE",
        017u32    => "WM_QUERYENDSESSION",
        018u32    => "WM_QUIT",
        019u32    => "WM_QUERYOPEN",
        020u32    => "WM_ERASEBKGND",
        021u32    => "WM_SYSCOLORCHANGE",
        022u32    => "WM_ENDSESSION",
        024u32    => "WM_SHOWWINDOW",
        026u32    => "WM_SETTINGCHANGE",
        027u32    => "WM_DEVMODECHANGE",
        028u32    => "WM_ACTIVATEAPP",
        029u32    => "WM_FONTCHANGE",
        030u32    => "WM_TIMECHANGE",
        031u32    => "WM_CANCELMODE",
        032u32    => "WM_SETCURSOR",
        033u32    => "WM_MOUSEACTIVATE",
        034u32    => "WM_CHILDACTIVATE",
        035u32    => "WM_QUEUESYNC",
        036u32    => "WM_GETMINMAXINFO",
        038u32    => "WM_PAINTICON",
        039u32    => "WM_ICONERASEBKGND",
        040u32    => "WM_NEXTDLGCTL",
        042u32    => "WM_SPOOLERSTATUS",
        043u32    => "WM_DRAWITEM",
        044u32    => "WM_MEASUREITEM",
        045u32    => "WM_DELETEITEM",
        046u32    => "WM_VKEYTOITEM",
        047u32    => "WM_CHARTOITEM",
        048u32    => "WM_SETFONT",
        049u32    => "WM_GETFONT",
        050u32    => "WM_SETHOTKEY",
        051u32    => "WM_GETHOTKEY",
        055u32    => "WM_QUERYDRAGICON",
        057u32    => "WM_COMPAREITEM",
        061u32    => "WM_GETOBJECT",
        065u32    => "WM_COMPACTING",
        068u32    => "WM_COMMNOTIFY",
        070u32    => "WM_WINDOWPOSCHANGING",
        071u32    => "WM_WINDOWPOSCHANGED",
        072u32    => "WM_POWER",
        074u32    => "WM_COPYDATA",
        075u32    => "WM_CANCELJOURNAL",
        078u32    => "WM_NOTIFY",
        080u32    => "WM_INPUTLANGCHANGEREQUEST",
        081u32    => "WM_INPUTLANGCHANGE",
        082u32    => "WM_TCARD",
        083u32    => "WM_HELP",
        084u32    => "WM_USERCHANGED",
        085u32    => "WM_NOTIFYFORMAT",
        1024u32   => "WM_USER",
        123u32    => "WM_CONTEXTMENU",
        124u32    => "WM_STYLECHANGING",
        125u32    => "WM_STYLECHANGED",
        126u32    => "WM_DISPLAYCHANGE",
        127u32    => "WM_GETICON",
        128u32    => "WM_SETICON",
        129u32    => "WM_NCCREATE",
        130u32    => "WM_NCDESTROY",
        131u32    => "WM_NCCALCSIZE",
        132u32    => "WM_NCHITTEST",
        133u32    => "WM_NCPAINT",
        134u32    => "WM_NCACTIVATE",
        135u32    => "WM_GETDLGCODE",
        136u32    => "WM_SYNCPAINT",
        160u32    => "WM_NCMOUSEMOVE",
        161u32    => "WM_NCLBUTTONDOWN",
        162u32    => "WM_NCLBUTTONUP",
        163u32    => "WM_NCLBUTTONDBLCLK",
        164u32    => "WM_NCRBUTTONDOWN",
        165u32    => "WM_NCRBUTTONUP",
        166u32    => "WM_NCRBUTTONDBLCLK",
        167u32    => "WM_NCMBUTTONDOWN",
        168u32    => "WM_NCMBUTTONUP",
        169u32    => "WM_NCMBUTTONDBLCLK",
        171u32    => "WM_NCXBUTTONDOWN",
        172u32    => "WM_NCXBUTTONUP",
        173u32    => "WM_NCXBUTTONDBLCLK",
        254u32    => "WM_INPUT_DEVICE_CHANGE",
        255u32    => "WM_INPUT",
        256u32    => "WM_KEYDOWN",
        257u32    => "WM_KEYUP",
        258u32    => "WM_CHAR",
        259u32    => "WM_DEADCHAR",
        260u32    => "WM_SYSKEYDOWN",
        261u32    => "WM_SYSKEYUP",
        262u32    => "WM_SYSCHAR",
        263u32    => "WM_SYSDEADCHAR",
        265u32    => "WM_KEYLAST",
        269u32    => "WM_IME_STARTCOMPOSITION",
        270u32    => "WM_IME_ENDCOMPOSITION",
        271u32    => "WM_IME_COMPOSITION",
        272u32    => "WM_INITDIALOG",
        273u32    => "WM_COMMAND",
        274u32    => "WM_SYSCOMMAND",
        275u32    => "WM_TIMER",
        276u32    => "WM_HSCROLL",
        277u32    => "WM_VSCROLL",
        278u32    => "WM_INITMENU",
        279u32    => "WM_INITMENUPOPUP",
        281u32    => "WM_GESTURE",
        282u32    => "WM_GESTURENOTIFY",
        287u32    => "WM_MENUSELECT",
        288u32    => "WM_MENUCHAR",
        289u32    => "WM_ENTERIDLE",
        290u32    => "WM_MENURBUTTONUP",
        291u32    => "WM_MENUDRAG",
        292u32    => "WM_MENUGETOBJECT",
        293u32    => "WM_UNINITMENUPOPUP",
        294u32    => "WM_MENUCOMMAND",
        295u32    => "WM_CHANGEUISTATE",
        296u32    => "WM_UPDATEUISTATE",
        297u32    => "WM_QUERYUISTATE",
        306u32    => "WM_CTLCOLORMSGBOX",
        307u32    => "WM_CTLCOLOREDIT",
        308u32    => "WM_CTLCOLORLISTBOX",
        309u32    => "WM_CTLCOLORBTN",
        310u32    => "WM_CTLCOLORDLG",
        311u32    => "WM_CTLCOLORSCROLLBAR",
        312u32    => "WM_CTLCOLORSTATIC",
        32768u32  => "WM_APP",
        512u32    => "WM_MOUSEMOVE",
        513u32    => "WM_LBUTTONDOWN",
        514u32    => "WM_LBUTTONUP",
        515u32    => "WM_LBUTTONDBLCLK",
        516u32    => "WM_RBUTTONDOWN",
        517u32    => "WM_RBUTTONUP",
        518u32    => "WM_RBUTTONDBLCLK",
        519u32    => "WM_MBUTTONDOWN",
        520u32    => "WM_MBUTTONUP",
        521u32    => "WM_MBUTTONDBLCLK",
        522u32    => "WM_MOUSEWHEEL",
        523u32    => "WM_XBUTTONDOWN",
        524u32    => "WM_XBUTTONUP",
        525u32    => "WM_XBUTTONDBLCLK",
        526u32    => "WM_MOUSEHWHEEL",
        528u32    => "WM_PARENTNOTIFY",
        529u32    => "WM_ENTERMENULOOP",
        530u32    => "WM_EXITMENULOOP",
        531u32    => "WM_NEXTMENU",
        532u32    => "WM_SIZING",
        533u32    => "WM_CAPTURECHANGED",
        534u32    => "WM_MOVING",
        536u32    => "WM_POWERBROADCAST",
        537u32    => "WM_DEVICECHANGE",
        544u32    => "WM_MDICREATE",
        545u32    => "WM_MDIDESTROY",
        546u32    => "WM_MDIACTIVATE",
        547u32    => "WM_MDIRESTORE",
        548u32    => "WM_MDINEXT",
        549u32    => "WM_MDIMAXIMIZE",
        550u32    => "WM_MDITILE",
        551u32    => "WM_MDICASCADE",
        552u32    => "WM_MDIICONARRANGE",
        553u32    => "WM_MDIGETACTIVE",
        560u32    => "WM_MDISETMENU",
        561u32    => "WM_ENTERSIZEMOVE",
        562u32    => "WM_EXITSIZEMOVE",
        563u32    => "WM_DROPFILES",
        564u32    => "WM_MDIREFRESHMENU",
        568u32    => "WM_POINTERDEVICECHANGE",
        569u32    => "WM_POINTERDEVICEINRANGE",
        570u32    => "WM_POINTERDEVICEOUTOFRANGE",
        576u32    => "WM_TOUCH",
        577u32    => "WM_NCPOINTERUPDATE",
        578u32    => "WM_NCPOINTERDOWN",
        579u32    => "WM_NCPOINTERUP",
        581u32    => "WM_POINTERUPDATE",
        582u32    => "WM_POINTERDOWN",
        583u32    => "WM_POINTERUP",
        585u32    => "WM_POINTERENTER",
        586u32    => "WM_POINTERLEAVE",
        587u32    => "WM_POINTERACTIVATE",
        588u32    => "WM_POINTERCAPTURECHANGED",
        589u32    => "WM_TOUCHHITTESTING",
        590u32    => "WM_POINTERWHEEL",
        591u32    => "WM_POINTERHWHEEL",
        593u32    => "WM_POINTERROUTEDTO",
        594u32    => "WM_POINTERROUTEDAWAY",
        595u32    => "WM_POINTERROUTEDRELEASED",
        641u32    => "WM_IME_SETCONTEXT",
        642u32    => "WM_IME_NOTIFY",
        643u32    => "WM_IME_CONTROL",
        644u32    => "WM_IME_COMPOSITIONFULL",
        645u32    => "WM_IME_SELECT",
        646u32    => "WM_IME_CHAR",
        648u32    => "WM_IME_REQUEST",
        656u32    => "WM_IME_KEYDOWN",
        657u32    => "WM_IME_KEYUP",
        672u32    => "WM_NCMOUSEHOVER",
        674u32    => "WM_NCMOUSELEAVE",
        689u32    => "WM_WTSSESSION_CHANGE",
        704u32    => "WM_TABLET_FIRST",
        735u32    => "WM_TABLET_LAST",
        736u32    => "WM_DPICHANGED",
        738u32    => "WM_DPICHANGED_BEFOREPARENT",
        739u32    => "WM_DPICHANGED_AFTERPARENT",
        740u32    => "WM_GETDPISCALEDSIZE",
        768u32    => "WM_CUT",
        769u32    => "WM_COPY",
        770u32    => "WM_PASTE",
        771u32    => "WM_CLEAR",
        772u32    => "WM_UNDO",
        773u32    => "WM_RENDERFORMAT",
        774u32    => "WM_RENDERALLFORMATS",
        775u32    => "WM_DESTROYCLIPBOARD",
        776u32    => "WM_DRAWCLIPBOARD",
        777u32    => "WM_PAINTCLIPBOARD",
        778u32    => "WM_VSCROLLCLIPBOARD",
        779u32    => "WM_SIZECLIPBOARD",
        780u32    => "WM_ASKCBFORMATNAME",
        781u32    => "WM_CHANGECBCHAIN",
        782u32    => "WM_HSCROLLCLIPBOARD",
        783u32    => "WM_QUERYNEWPALETTE",
        784u32    => "WM_PALETTEISCHANGING",
        785u32    => "WM_PALETTECHANGED",
        786u32    => "WM_HOTKEY",
        791u32    => "WM_PRINT",
        792u32    => "WM_PRINTCLIENT",
        793u32    => "WM_APPCOMMAND",
        794u32    => "WM_THEMECHANGED",
        797u32    => "WM_CLIPBOARDUPDATE",
        798u32    => "WM_DWMCOMPOSITIONCHANGED",
        799u32    => "WM_DWMNCRENDERINGCHANGED",
        800u32    => "WM_DWMCOLORIZATIONCOLORCHANGED",
        801u32    => "WM_DWMWINDOWMAXIMIZEDCHANGE",
        803u32    => "WM_DWMSENDICONICTHUMBNAIL",
        806u32    => "WM_DWMSENDICONICLIVEPREVIEWBITMAP",
        831u32    => "WM_GETTITLEBARINFOEX",
        856u32    => "WM_HANDHELDFIRST",
        863u32    => "WM_HANDHELDLAST",
        864u32    => "WM_AFXFIRST",
        895u32    => "WM_AFXLAST",
        896u32    => "WM_PENWINFIRST",
        911u32    => "WM_PENWINLAST",
        _         => "UNKNOWN"
    }
}