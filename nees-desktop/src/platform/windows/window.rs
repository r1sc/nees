use std::collections::VecDeque;
use std::ffi::{c_int, CString};
use std::ptr::null_mut;

use windows::core::{s, PCSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{GetDC, GetSysColorBrush, COLOR_BACKGROUND, HDC};
use windows::Win32::Graphics::OpenGL::*;
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
use windows::Win32::UI::WindowsAndMessaging::*;

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let ptr = GetWindowLongPtrA(hwnd, GWLP_USERDATA);
    let this = if ptr == 0 {
        None
    } else {
        Some(ptr as *mut Window)
    };

    match (msg, this) {
        (WM_DESTROY, _) => {
            PostQuitMessage(0);
        }
        (WM_COMMAND, Some(this)) => {
            (*this).publish_event(WindowEvents::Command {
                which: wparam.0 as u16,
            });
        }
        (WM_KEYDOWN, Some(this)) => {
            (*this).publish_event(WindowEvents::Key(wparam.0 as u8, true));
        }
        (WM_KEYUP, Some(this)) => {
            (*this).publish_event(WindowEvents::Key(wparam.0 as u8, false));
        }
        (WM_SIZE, Some(this)) => {
            let new_width = lparam.0 & 0xFFFF;
            let new_height = lparam.0 >> 16;

            let new_size = if new_width > new_height {
                new_height
            } else {
                new_width
            };

            (*this).publish_event(WindowEvents::Resize(
                new_width as i32,
                new_height as i32,
                new_size as i32,
            ));
        }
        _ => {
            return DefWindowProcA(hwnd, msg, wparam, lparam);
        }
    }
    LRESULT(0)
}

#[derive(Clone, Copy)]
pub enum WindowEvents {
    Close,
    Key(u8, bool),
    Resize(i32, i32, i32),
    Command { which: u16 },
}
pub struct Window {
    hwnd: HWND,
    hdc: Option<HDC>,
    event_queue: VecDeque<WindowEvents>,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

pub enum Menu<'a> {
    Popout {
        title: &'a str,
        children: &'a [Menu<'a>],
    },
    Separator,
    Item {
        id: usize,
        title: &'a str,
    },
}

impl Window {
    pub fn new() -> Box<Self> {
        let h_instance = unsafe { GetModuleHandleA(None).unwrap() };

        let wc = WNDCLASSA {
            lpfnWndProc: Some(wndproc),
            hInstance: h_instance.into(),
            hbrBackground: unsafe { GetSysColorBrush(COLOR_BACKGROUND) },
            lpszClassName: s!("WinNES"),
            style: CS_OWNDC,
            ..Default::default()
        };

        unsafe {
            RegisterClassA(&wc);
        }

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 512,
            bottom: 512,
        };
        unsafe {
            AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, true).unwrap();
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        let hwnd = unsafe {
            CreateWindowExA(
                WINDOW_EX_STYLE(0),
                wc.lpszClassName,
                s!("WinNES"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                GetSystemMetrics(SM_CXSCREEN) / 2 - width / 2,
                GetSystemMetrics(SM_CYSCREEN) / 2 - height / 2,
                width,
                height,
                None,
                None,
                h_instance,
                None,
            )
            .unwrap()
        };

        let this = Box::new(Self {
            hwnd,
            hdc: None,
            event_queue: VecDeque::new(),
        });

        unsafe {
            SetWindowLongPtrA(hwnd, GWLP_USERDATA, &*this as *const Self as isize);
        }

        this
    }

    pub fn create_gl_surface(&mut self) -> glow::Context {
        let pfd = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 24,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 0,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 24,
            cStencilBits: 0,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE.0 as u8,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };

        let window_dc = unsafe { GetDC(self.hwnd) };
        self.hdc = Some(window_dc);

        unsafe {
            let let_windows_choose_this_pixel_format = ChoosePixelFormat(window_dc, &pfd);
            SetPixelFormat(window_dc, let_windows_choose_this_pixel_format, &pfd).unwrap();

            let fake_context = wglCreateContext(window_dc).unwrap();
            wglMakeCurrent(window_dc, fake_context).unwrap();

            const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
            const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;
            const WGL_CONTEXT_FLAGS_ARB: i32 = 0x2094;
            const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;
            const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x00000001;

            #[rustfmt::skip]
            let attribs: [i32; 9] = [
                WGL_CONTEXT_MAJOR_VERSION_ARB, 3,
                WGL_CONTEXT_MINOR_VERSION_ARB, 2,
                WGL_CONTEXT_FLAGS_ARB, 0,
                WGL_CONTEXT_PROFILE_MASK_ARB, WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
                0,
            ];

            #[allow(non_snake_case)]
            let wglCreateContextAttribsARB: unsafe extern "system" fn(
                HDC,
                HGLRC,
                *const c_int,
            )
                -> HGLRC =
                core::mem::transmute(wglGetProcAddress(s!("wglCreateContextAttribsARB")).unwrap());

            #[allow(non_snake_case)]
            let ourOpenGLRenderingContext =
                wglCreateContextAttribsARB(window_dc, HGLRC(null_mut()), attribs.as_ptr());

            wglMakeCurrent(None, None).unwrap();
            wglDeleteContext(fake_context).unwrap();
            wglMakeCurrent(window_dc, ourOpenGLRenderingContext).unwrap();

            #[allow(non_snake_case)]
            let wglSwapIntervalEXT: unsafe extern "system" fn(c_int) -> BOOL =
                core::mem::transmute(wglGetProcAddress(s!("wglSwapIntervalEXT")).unwrap());
            _ = wglSwapIntervalEXT(0);
        }

        let gl_lib = unsafe { LoadLibraryA(s!("opengl32.dll")).unwrap() };

        unsafe {
            glow::Context::from_loader_function(|name| {
                match wglGetProcAddress(PCSTR::from_raw(name.as_ptr())) {
                    Some(ptr) => ptr as *const std::os::raw::c_void,
                    None => match GetProcAddress(gl_lib, PCSTR::from_raw(name.as_ptr())) {
                        Some(ptr) => ptr as *const std::os::raw::c_void,
                        None => null_mut(),
                    },
                }
            })
        }
    }

    pub fn swap_buffers(&self) {
        unsafe {
            let _ = SwapBuffers(self.hdc.unwrap());
        }
    }

    fn publish_event(&mut self, event: WindowEvents) {
        self.event_queue.push_back(event);
    }

    /// Pump Windows messages - this generate events in the event queue
    pub fn pump_events(&mut self) {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageA(&mut msg, None, 0, 0, PM_NOYIELD | PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    self.publish_event(WindowEvents::Close);
                }

                _ = TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }
        }
    }

    /// Get the next event from the event queue
    pub fn get_event(&mut self) -> Option<WindowEvents> {
        self.event_queue.pop_front()
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let cstr = CString::new(title).unwrap();
            _ = SetWindowTextA(
                self.hwnd,
                PCSTR::from_raw(cstr.as_bytes_with_nul().as_ptr()),
            );
        }
    }

    fn add_menu_item(parent_menu: HMENU, menu_spec: &Menu) {
        match menu_spec {
            Menu::Popout { title, children } => {
                let flags = MF_POPUP | MF_STRING;
                let cstr = CString::new(*title).unwrap();
                let menu = unsafe { CreateMenu().unwrap() };

                unsafe {
                    AppendMenuA(
                        parent_menu,
                        flags,
                        menu.0 as usize,
                        PCSTR::from_raw(cstr.as_ptr() as *const u8),
                    )
                    .unwrap();
                }

                for child in *children {
                    Self::add_menu_item(menu, child);
                }
            }
            Menu::Separator => unsafe {
                AppendMenuA(parent_menu, MF_SEPARATOR, 0, PCSTR::null()).unwrap();
            },
            Menu::Item { id, title } => {
                let flags = MF_STRING;
                let cstr = CString::new(*title).unwrap();

                unsafe {
                    AppendMenuA(
                        parent_menu,
                        flags,
                        *id,
                        PCSTR::from_raw(cstr.as_ptr() as *const u8),
                    )
                    .unwrap();
                }
            }
        }
    }

    pub fn add_menus(&self, menus: &[Menu]) {
        let window_menu = unsafe { CreateMenu().unwrap() };

        for menu in menus {
            Self::add_menu_item(window_menu, menu);
        }

        unsafe {
            SetMenu(self.hwnd, window_menu).unwrap();
            DrawMenuBar(self.hwnd).unwrap();
        }
    }
}
