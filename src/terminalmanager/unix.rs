use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};

pub struct TerminalManager {
    _termios: Option<Termios>
}

impl TerminalManager {
    pub fn new() -> TerminalManager {
        let termios = Termios::from_fd(libc::STDOUT_FILENO).ok();

        match termios.clone() {
            Some(mut new_termios) => {
                new_termios.c_lflag &= !(ICANON | ECHO);
                tcsetattr(0, TCSANOW, &mut new_termios).unwrap();
                //RectManager::write("\x1B[?25l\x1B[?1049h").expect("Couldn't switch screen buffer"); // New screen
            }
            None => {

            }
        }

        TerminalManager {
            _termios: termios
        }
    }

    pub fn tear_down(&mut self) {
        // Even if it fails, we want to try clearing out all the rects
        // that are drawn, and reset the screen, to try to make failure
        // as easy to read as possible.
        match self._termios {
            Some(_termios) => {
                tcsetattr(0, TCSANOW, & _termios).unwrap();

                //RectManager::write("\x1B[?25h\x1B[?1049l")?; // Return to previous screen
                //RectManager::write("\x1B[2A").expect("Couldn't restore cursor position");
            }
            None => ()
        }
    }
}


