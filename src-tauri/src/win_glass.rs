#![allow(non_snake_case)]

use std::ffi::c_void;

#[repr(C)]
struct AccentPolicy {
    accent_state: i32,
    accent_flags: u32,
    gradient_color: u32,
    animation_id: i32,
}

#[repr(C)]
struct WindowCompositionAttribData {
    attrib: u32,
    pv_data: *mut c_void,
    cb_data: usize,
}

type EnumWindowsProc = unsafe extern "system" fn(hwnd: isize, lparam: isize) -> i32;

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleA(lpModuleName: *const u8) -> isize;
    fn GetProcAddress(hModule: isize, lpProcName: *const u8) -> *const c_void;
    fn GetCurrentProcessId() -> u32;
}

#[link(name = "user32")]
extern "system" {
    fn EnumWindows(lpEnumFunc: EnumWindowsProc, lParam: isize) -> i32;
    fn GetWindowThreadProcessId(hWnd: isize, lpdwProcessId: *mut u32) -> u32;
    fn IsWindowVisible(hWnd: isize) -> i32;
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: isize,
        dwAttribute: u32,
        pvAttribute: *const c_void,
        cbAttribute: u32,
    ) -> i32;
}

/// Ativa Per-Monitor V2 DPI Awareness antes de criar a janela para nitidez 1:1 nativa.
pub fn enable_per_monitor_dpi() {
    unsafe {
        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if user32 != 0 {
            let addr = GetProcAddress(user32, b"SetProcessDpiAwarenessContext\0".as_ptr());
            if !addr.is_null() {
                let func: unsafe extern "system" fn(isize) -> i32 = std::mem::transmute(addr);
                // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4
                let _ = func(-4);
            }
        }
    }
}

struct FindCtx {
    target_pid: u32,
    found_hwnd: isize,
}

unsafe extern "system" fn enum_proc(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut FindCtx);
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == ctx.target_pid && IsWindowVisible(hwnd) != 0 {
        ctx.found_hwnd = hwnd;
        return 0;
    }
    1
}

/// Aplica efeito translúcido DWM / BlurBehind nativo do Windows + modo escuro + cantos arredondados (Win11).
pub fn apply_liquid_glass_to_process_window() -> bool {
    unsafe {
        let mut ctx = FindCtx {
            target_pid: GetCurrentProcessId(),
            found_hwnd: 0,
        };
        EnumWindows(enum_proc, &mut ctx as *mut FindCtx as isize);
        let hwnd = ctx.found_hwnd;
        if hwnd == 0 {
            return false;
        }

        // Dark mode DWM (20 = DWMWA_USE_IMMERSIVE_DARK_MODE)
        let dark: i32 = 1;
        let _ = DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as *const c_void, 4);

        // Cantos arredondados nativos no Windows 11 (33 = DWMWA_WINDOW_CORNER_PREFERENCE, 2 = DWMWCP_ROUND)
        let round: i32 = 2;
        let _ = DwmSetWindowAttribute(hwnd, 33, &round as *const _ as *const c_void, 4);

        // BlurBehind real no Windows 10/11 via SetWindowCompositionAttribute (sem lag ao arrastar)
        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if user32 != 0 {
            let swca = GetProcAddress(user32, b"SetWindowCompositionAttribute\0".as_ptr());
            if !swca.is_null() {
                let set_comp: unsafe extern "system" fn(
                    isize,
                    *mut WindowCompositionAttribData,
                ) -> i32 = std::mem::transmute(swca);

                // ACCENT_ENABLE_BLURBEHIND = 3
                let mut policy = AccentPolicy {
                    accent_state: 3,
                    accent_flags: 2,
                    gradient_color: 0xCC1E1410,
                    animation_id: 0,
                };
                let mut data = WindowCompositionAttribData {
                    attrib: 19, // WCA_ACCENT_POLICY
                    pv_data: &mut policy as *mut _ as *mut c_void,
                    cb_data: std::mem::size_of::<AccentPolicy>(),
                };
                let _ = set_comp(hwnd, &mut data);
            }
        }

        true
    }
}
