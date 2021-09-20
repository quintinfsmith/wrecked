use std::os::windows::io::RawHandle;

pub struct TerminalManager {}

impl TerminalManager {
    pub fn new() -> TerminalManager {
        TerminalManager {}
    }
    pub fn tear_down(&mut self) {}

    pub fn get_size() -> (u16, u16) {
        use winapi::um::processenv::GetStdHandle;
        use winapi::um::winbase::STD_OUTPUT_HANDLE;
        use winapi::um::handleapi::INVALID_HANDLE_VALUE;
        use winapi::um::wincon::{CONSOLE_SCREEN_BUFFER_INFO, COORD, SMALL_RECT, GetConsoleScreenBufferInfo};

        let mut width = 0;
        let mut height = 0;
        match unsafe { GetStdHandle(STD_OUTPUT_HANDLE) } as winapi::um::winnt::HANDLE {
            INVALID_HANDLE_VALUE => {}
            handle => {
                let zc = COORD { X: 0, Y: 0 };
                let mut csbi = CONSOLE_SCREEN_BUFFER_INFO {
                    dwSize: zc,
                    dwCursorPosition: zc,
                    wAttributes: 0,
                    srWindow: SMALL_RECT {
                        Left: 0,
                        Top: 0,
                        Right: 0,
                        Bottom: 0,
                    },
                    dwMaximumWindowSize: zc,
                };

                if unsafe { GetConsoleScreenBufferInfo(handle, &mut csbi) } != 0 {
                    width = (csbi.srWindow.Right - csbi.srWindow.Left + 1) as u16;
                    height = (csbi.srWindow.Bottom - csbi.srWindow.Top + 1) as u16;
                }
            }
        }

        (width, height)
    }
}


