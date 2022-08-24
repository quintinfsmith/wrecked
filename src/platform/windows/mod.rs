#![cfg(target_os = "windows")]
use windows::Win32::System::Console;
use windows::Win32::Foundation;

pub type TermType = None;

#[inline]
pub unsafe fn prepare_console() -> () {
    match Console::GetStdHandle(Console::STD_INPUT_HANDLE) {
        Ok(handle) => {
            let mut mode: Console::CONSOLE_MODE = Console::CONSOLE_MODE(0);
            Console::GetConsoleMode(handle, &mut mode);
            Console::SetConsoleMode(handle, mode & !Console::ENABLE_ECHO_INPUT & !Console::ENABLE_LINE_INPUT);
            RectManager::write("\x1B[?25l\x1B[?1049h").expect("Couldn't switch screen buffer"); // New screen
        }
        Err(_) => {}
    }
}

impl RectManager {
    pub fn restore_console_state(&mut self) {}
}
