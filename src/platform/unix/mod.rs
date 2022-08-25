#![cfg(unix)]
use crate::RectManager;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
pub type TermType = Termios;

#[inline]
pub fn prepare_console() -> Option<Termios> {
    let stdout_fileno = libc::STDOUT_FILENO;

    let termref = Termios::from_fd(stdout_fileno).ok();

    match termref.clone() {
        Some(mut new_termref) => {
            new_termref.c_lflag &= !(ICANON | ECHO);
            tcsetattr(0, TCSANOW, &mut new_termref).unwrap();
            RectManager::write("\x1B[?25l\x1B[?1049h").expect("Couldn't switch screen buffer"); // New screen
        }
        None => { }
    }
    termref
}

impl RectManager {
    pub fn restore_console_state(&mut self) {
        match self._termref {
            Some(_termref) => {
                tcsetattr(0, TCSANOW, & _termref).unwrap();
                RectManager::write("\x1B[?25h\x1B[?1049l").ok(); // Return to previous screen

            }
            None => ()
        }
    }
}
