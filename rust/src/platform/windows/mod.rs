#![cfg(target_os = "windows")]
use windows::Win32::System::Console;
use windows::Win32::Foundation;

use crate::RectManager;

pub type TermType = ();

#[inline]
pub fn prepare_console() -> Option<TermType> {
    unsafe {
        match Console::GetStdHandle(Console::STD_INPUT_HANDLE) {
            Ok(handle) => {
                let mut mode: Console::CONSOLE_MODE = Console::CONSOLE_MODE(0);
                Console::GetConsoleMode(handle, &mut mode);
                Console::SetConsoleMode(handle, mode & !Console::ENABLE_ECHO_INPUT & !Console::ENABLE_LINE_INPUT);

                RectManager::write("\x1B[?1049h").expect("Couldn't switch screen buffer"); // New screen
            }
            Err(_) => {}
        }
        // Hide cursor
        match Console::GetStdHandle(Console::STD_OUTPUT_HANDLE) {
            Ok(handle) => {
                let mut cursorInfo = Console::CONSOLE_CURSOR_INFO::default();
                Console::GetConsoleCursorInfo(handle, &mut cursorInfo);
                cursorInfo.bVisible = Foundation::BOOL::from(false);
                Console::SetConsoleCursorInfo(handle, &mut cursorInfo);
            }
            Err(_) => {}
        }
    }

   None
}

impl RectManager {
    pub fn restore_console_state(&mut self) {
        RectManager::write("\x1B[?1049l").expect("Couldn't switch screen buffer"); // Back to original screen
        unsafe {
            match Console::GetStdHandle(Console::STD_OUTPUT_HANDLE) {
                Ok(handle) => {
                    let mut cursorInfo = Console::CONSOLE_CURSOR_INFO::default();
                    Console::GetConsoleCursorInfo(handle, &mut cursorInfo);
                    cursorInfo.bVisible = Foundation::BOOL::from(true);
                    Console::SetConsoleCursorInfo(handle, &mut cursorInfo);
                }
                Err(_) => {}
            }
        }
    }
}
