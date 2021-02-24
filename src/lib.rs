use std::cmp::{self, PartialOrd};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::error::Error;
use std::fmt::{self, Display};
use std::str;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};

pub mod tests;

pub fn get_terminal_size() -> (u16, u16) {
    use libc::{winsize, TIOCGWINSZ, ioctl};
    let mut output = (1, 1);

    let mut t = winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0
    };


    if unsafe { ioctl(libc::STDOUT_FILENO, TIOCGWINSZ.into(), &mut t) } != -1 {
        output = (t.ws_col, t.ws_row);
    }

    output
}

#[derive(PartialEq, Eq, Debug)]
pub enum WreckedError {
    AllGood,
    BadColor,
    InvalidUtf8,
    StringTooLong(usize, (isize, isize), String), // Rect_id, position, string
    NotFound(usize),
    NoParent(usize), // Rect has no parent id
    BadPosition(isize, isize),
    ParentNotFound(usize, usize), // rect has an associated parent id that does not exist in RectManager
    ChildNotFound(usize, usize),
    StdoutFailure(String),
    Disabled(usize)
}

impl Display for WreckedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //let name = format!("{:?}", self);
        write!(f, "{:?}", self)
    }
}

impl Error for WreckedError {}

/// enum versions of the ANSI color codes
#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Debug)]
pub enum Color {
    BLACK = 0,
    RED = 1,
    GREEN = 2,
    YELLOW = 3,
    BLUE = 4,
    MAGENTA = 5,
    CYAN = 6,
    WHITE = 7,
    BRIGHTBLACK = 8 | 0,
    BRIGHTRED = 8 | 1,
    BRIGHTGREEN = 8 | 2,
    BRIGHTYELLOW = 8 | 3,
    BRIGHTBLUE = 8 | 4,
    BRIGHTMAGENTA = 8 | 5,
    BRIGHTCYAN = 8 | 6,
    BRIGHTWHITE = 8 | 7
}

/// Structure to manage text effects instead of having disparate flags
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct EffectsHandler {
    bold: bool,
    underline: bool,
    invert: bool,
    italics: bool,
    strike: bool,
    blink: bool,
    background_color: Option<Color>,
    foreground_color: Option<Color>
}

impl fmt::Debug for EffectsHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EffectsHandler")
         .field("bold", &self.bold)
         .field("underline", &self.underline)
         .field("invert", &self.invert)
         .field("strike", &self.strike)
         .field("blink", &self.blink)
         .field("background_color", &self.background_color)
         .field("foreground_color", &self.foreground_color)
         .finish()
    }
}

impl EffectsHandler {
    pub fn new() -> EffectsHandler {
        EffectsHandler {
            bold: false,
            underline: false,
            invert: false,
            italics: false,
            strike: false,
            blink: false,
            background_color: None,
            foreground_color: None
        }
    }

    pub fn is_plain(&self) -> bool {
        !self.bold
        && !self.underline
        && !self.invert
        && !self.italics
        && !self.strike
        && !self.blink
        && self.background_color.is_none()
        && self.foreground_color.is_none()
    }

    pub fn clear(&mut self) {
        self.bold = false;
        self.underline = false;
        self.invert = false;
        self.italics = false;
        self.strike = false;
        self.blink = false;
        self.background_color = None;
        self.foreground_color = None;
    }

    pub fn len(&mut self) -> usize {
        let mut output = 0;
        if self.bold {
            output += 1;
        }
        if self.underline {
            output += 1;
        }
        if self.invert {
            output += 1;
        }
        if self.italics {
            output += 1;
        }
        if self.strike {
            output += 1;
        }
        if self.blink {
            output += 1;
        }
        if self.background_color.is_some() {
            output += 1;
        }
        if self.foreground_color.is_some() {
            output += 1;
        }

        output
    }
}


/// This is the id of the top-level rectangle that is instantiated when a new RectManager is created.
pub const ROOT: usize = 0;

/// An environment to manage and display character-based graphics in-console.
///
/// # Example
/// ```
/// use std::{thread, time};
/// use wrecked::{RectManager, ROOT};
///
/// let mut rectmanager = RectManager::new();
/// // rectmanager is initialized with top-level rect (id = ROOT) attached...
/// rectmanager.set_string(ROOT, 0, 0, "Hello World");
///
/// // draw the latest changes
/// rectmanager.render();
///
/// // wait 5 seconds (in order to see the screen)
/// let five_seconds = time::Duration::from_secs(5);
/// let now = time::Instant::now();
/// thread::sleep(five_seconds);
///
/// rectmanager.kill();
/// ```
pub struct RectManager {
    idgen: usize,
    recycle_ids: Vec<usize>,
    rects: HashMap<usize, Rect>,
    // top_cache is used to prevent redrawing the same
    // characters at the same coordinate.
    top_cache: HashMap<(isize, isize), (char, EffectsHandler)>,
    _termios: Option<Termios>,
    default_character: char
}

impl RectManager {
    /// Instantiate a new environment
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// // Initialize the console; turn off echo and enable non-canonical input.
    /// let mut rectmanager = RectManager::new();
    /// // turn echo back on and return input to normal.
    /// rectmanager.kill();
    /// ```
    pub fn new() -> RectManager {
        let termios = Termios::from_fd(libc::STDOUT_FILENO).ok();

        match termios.clone() {
            Some(mut new_termios) => {
                new_termios.c_lflag &= !(ICANON | ECHO);
                tcsetattr(0, TCSANOW, &mut new_termios).unwrap();

                RectManager::write("\x1B[?25l\x1B[?1049h").expect("Couldn't switch screen buffer"); // New screen
            }
            None => {

            }
        }


        let mut rectmanager = RectManager {
            idgen: ROOT,
            recycle_ids: Vec::new(),
            rects: HashMap::new(),
            top_cache: HashMap::new(),
            _termios: termios,
            default_character: ' '
        };

        rectmanager.new_orphan().expect("Couldn't Create ROOT rect");
        rectmanager.auto_resize();
        rectmanager
    }

    fn write(input: &str) -> Result<(), WreckedError> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        match handle.write_all(format!("{}\x0A", input).as_bytes()) {
            Ok(_) => {
                Ok(())
            }
            Err(_e) => {
                Err(WreckedError::StdoutFailure(input.to_string()))
            }
        }
    }


    /// If the ROOT rectangle dimensions to not match up to the console dimensions, then resize to fit.
    /// Returns true if a resize was made.
    pub fn auto_resize(&mut self) -> bool {
        let mut did_resize = false;
        let (current_width, current_height) = self.get_rect_size(ROOT).unwrap();

        let (w, h) = get_terminal_size();
        if w as usize != current_width || h as usize != current_height {
            self.resize(ROOT, w as usize, h as usize).expect("Unable to fit ROOT rect to terminal");
            did_resize = true;
        }

        did_resize
    }

    /// Render the visible portion of the rectmanager environment
    /// # Example
    /// ```
    /// // Use ROOT to draw everything
    /// use std::{thread, time};
    /// use wrecked::{RectManager, ROOT};
    ///
    /// let mut rectmanager = RectManager::new();
    /// // rectmanager is initialized with top-level rect (id = ROOT) attached...
    /// rectmanager.set_string(ROOT, 0, 0, "Hello World");
    ///
    /// // draw the latest changes
    /// rectmanager.render();
    ///
    /// // wait 5 seconds (in order to see the screen)
    /// let five_seconds = time::Duration::from_secs(5);
    /// let now = time::Instant::now();
    /// thread::sleep(five_seconds);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn render(&mut self) -> Result<(), WreckedError> {
        self.draw(ROOT)
    }

    /// Return console state to normal
    /// # Example
    /// ```
    /// use wrecked::RectManager;
    /// // Initialize the console; turn off echo and enable non-canonical input.
    /// let mut rectmanager = RectManager::new();
    /// // turn echo back on and return input to normal.
    /// rectmanager.kill();
    /// ```
    pub fn kill(&mut self) -> Result<(), WreckedError> {
        let mut last_error = Ok(());
        match self.clear_children(ROOT) {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }
        match self.clear_characters(ROOT) {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }
        match self.clear_effects(ROOT) {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }
        match self.render() {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }

        // Even if it fails, we want to try clearing out all the rects
        // that are drawn, and reset the screen, to try to make failure
        // as easy to read as possible.
        match self._termios {
            Some(_termios) => {
                tcsetattr(0, TCSANOW, & _termios).unwrap();

                RectManager::write("\x1B[?25h\x1B[?1049l")?; // Return to previous screen
            }
            None => ()
        }

        last_error
    }

    /// Gets the height of the RectManager
    pub fn get_height(&self) -> usize {
        let (_, height) = self.get_rect_size(ROOT).unwrap();
        height
    }

    /// Gets the width of the RectManager
    pub fn get_width(&self) -> usize {
        let (width, _) = self.get_rect_size(ROOT).unwrap();
        width
    }

    /// Create a new rectangle, but don't add it to the environment yet.
    /// # Example
    /// ```
    /// use wrecked::RectManager;
    /// let mut rectmanager = RectManager::new();
    ///
    /// // Create a rectangle
    /// let orphan_id = rectmanager.new_orphan().ok().unwrap();
    ///
    /// assert!(!rectmanager.has_parent(orphan_id));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn new_orphan(&mut self) -> Result<usize, WreckedError> {
        // For now, there's really no way to Result in an error here,
        // but future proofing and consistency and all that, we'll return a Result
        let new_id = self.gen_id();
        self.rects.entry(new_id).or_insert(Rect::new(new_id));

        Ok(new_id)
    }

    fn build_ansi_string(&mut self, display_map: Vec<((isize, isize), (char, EffectsHandler))>) -> String {
        let mut renderstring = "".to_string();

        let mut val_a: &char;
        let mut active_effects = EffectsHandler::new();
        let mut new_effects;
        let mut current_col = -10;
        let mut current_row = -10;

        for (pos, val) in display_map.iter() {
            if pos.1 != current_row || pos.0 != current_col {
                renderstring += &format!("\x1B[{};{}H", pos.1 + 1, pos.0 + 1);
                current_col = pos.0;
                current_row = pos.1;
            }

            val_a = &val.0;
            new_effects = val.1;

            if new_effects != active_effects {
                let mut ansi_code_list: Vec<u8> = vec![];
                if new_effects.is_plain() {
                    ansi_code_list.push(0);
                } else {
                    let mut tmp_color_n;
                    // ForeGround
                    if new_effects.foreground_color != active_effects.foreground_color {
                        match new_effects.foreground_color {
                            Some(fg_color) => {
                                tmp_color_n = fg_color as u8;
                                if tmp_color_n & 8 == 8 {
                                    ansi_code_list.push(90 + (tmp_color_n & 7));
                                } else {
                                    ansi_code_list.push(30 + (tmp_color_n & 7));
                                }
                            }
                            None => {
                                ansi_code_list.push(39);
                            }
                        }
                    }

                    // BackGround
                    if new_effects.background_color != active_effects.background_color {
                        match new_effects.background_color {
                            Some(bg_color) => {
                                tmp_color_n = bg_color as u8;
                                if tmp_color_n & 8 == 8 {
                                    ansi_code_list.push(100 + (tmp_color_n & 7));
                                } else {
                                    ansi_code_list.push(40 + (tmp_color_n & 7));
                                }
                            }
                            None => {
                                ansi_code_list.push(49);
                            }
                        }
                    }

                    // Bold
                    if new_effects.bold != active_effects.bold {
                        if new_effects.bold {
                            ansi_code_list.push(1); // on
                        } else {
                            ansi_code_list.push(22); // off
                        }
                    }

                    // Underline
                    if new_effects.underline != active_effects.underline {
                        if new_effects.underline {
                            ansi_code_list.push(4); // on
                        } else {
                            ansi_code_list.push(24); // off
                        }
                    }

                    // Inverted
                    if new_effects.invert != active_effects.invert {
                        if new_effects.invert {
                            ansi_code_list.push(7); // on
                        } else {
                            ansi_code_list.push(27); // off
                        }
                    }

                    // Italics
                    if new_effects.italics != active_effects.italics {
                        if new_effects.italics {
                            ansi_code_list.push(3); // on
                        } else {
                            ansi_code_list.push(23); // off
                        }
                    }

                    // Strike
                    if new_effects.blink != active_effects.blink {
                        if new_effects.blink {
                            ansi_code_list.push(5); // on
                        } else {
                            ansi_code_list.push(25); // off
                        }
                    }
                }

                renderstring += "\x1B[";
                for (i, n) in ansi_code_list.iter().enumerate() {
                    if i > 0 {
                        renderstring += ";";
                    }
                    renderstring += &format!("{}", n);
                }
                renderstring += "m";
            }

            active_effects = new_effects;

            renderstring += &format!("{}", val_a);

            current_col += 1;
        }

        renderstring
    }

    fn filter_cached(&mut self, full_display_map: Vec<((isize, isize), (char, EffectsHandler))>) -> Vec<((isize, isize), (char, EffectsHandler))> {
        let mut filtered_map = Vec::new();

        let mut update_top_cache;
        for (pos, val) in full_display_map.iter() {
            update_top_cache = false;
            match self.top_cache.get(&pos) {
                Some(char_pair) => {
                    if *char_pair != *val {
                        update_top_cache = true;
                    }
                }
                None => {
                    update_top_cache = true;
                }
            }

            if update_top_cache {
                self.top_cache.entry(*pos)
                    .and_modify(|e| { *e = *val })
                    .or_insert(*val);

                filtered_map.push((*pos, *val));
            }
        }

        filtered_map
    }

    fn gen_id(&mut self) -> usize {
        if self.recycle_ids.len() > 0 {
            self.recycle_ids.pop().unwrap()
        } else {
            let new_id = self.idgen;
            self.idgen += 1;
            new_id
        }
    }

    /// Check if a given rectangle (and therefor its children) will be considered when drawing.
    pub fn is_rect_enabled(&self, rect_id: usize) -> bool {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.enabled
            }
            None => {
                false
            }
        }
    }

    /// Get the character at the given position of a rectangle.
    /// The rectangle's default character (usually ' ') is returned if no character is found.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_character(ROOT, 0, 0, 'X');
    /// assert_eq!(rectmanager.get_character(ROOT, 0, 0).ok().unwrap(), 'X');
    /// rectmanager.kill();
    /// ```
    pub fn get_character(&self, rect_id: usize, x: isize, y: isize) -> Result<char, WreckedError> {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.get_character(x, y)
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }
    }

    pub fn get_children(&self, rect_id: usize) -> Vec<usize> {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.children.clone()
            }
            None => {
                Vec::new()
            }
        }
    }

    /// Get the offset relative to the top-level rectangle in the RectManager.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_a = rectmanager.new_rect(ROOT).ok().unwrap();
    /// let mut rect_b = rectmanager.new_rect(rect_a).ok().unwrap();
    /// // Move parent rect ...
    /// rectmanager.set_position(rect_a, 5, 2);
    /// // Move child rect ...
    /// rectmanager.set_position(rect_b, 5, 2);
    ///
    /// assert_eq!(rectmanager.get_absolute_offset(rect_b).unwrap(), (10, 4));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn get_absolute_offset(&self, rect_id: usize) -> Option<(isize, isize)> {
        if self.has_rect(rect_id) {
            let mut x = 0;
            let mut y = 0;
            let mut working_id = rect_id;
            let mut broken = false;
            loop {
                match self.get_parent(working_id) {
                    Some(parent) => {
                        match parent.get_child_position(working_id) {
                            Some(pos) => {
                                x += pos.0;
                                y += pos.1;
                                working_id = parent.rect_id;
                            }
                            None => {
                                broken = true;
                                break;
                            }
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            if broken {
                None
            } else {
                Some((x, y))
            }
        } else {
            None
        }
    }

    /// Get the offset relative to the parent rectangle of the given rectangle.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_a = rectmanager.new_rect(ROOT).ok().unwrap();
    /// let mut rect_b = rectmanager.new_rect(rect_a).ok().unwrap();
    /// // Move parent rect ...
    /// rectmanager.set_position(rect_a, 10, 1);
    /// // Move child rect ...
    /// rectmanager.set_position(rect_b, 5, 2);
    ///
    /// assert_eq!(rectmanager.get_relative_offset(rect_b).unwrap(), (5, 2));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn get_relative_offset(&self, rect_id: usize) -> Option<(isize, isize)> {
        match self.get_parent(rect_id) {
            Some(parent) => {
                parent.get_child_position(rect_id)
            }
            None => {
                None
            }
        }
    }

    /// Get width of given rectangle
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect = rectmanager.new_rect(ROOT).ok().unwrap();
    /// rectmanager.resize(rect, 10, 10);
    /// assert_eq!(rectmanager.get_rect_height(rect), 10);
    /// rectmanager.kill();
    /// ```
    pub fn get_rect_width(&self, rect_id: usize) -> usize {
        let (width, _) = self.get_rect_size(rect_id).unwrap();
        width
    }

    /// Get height of given rectangle
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect = rectmanager.new_rect(ROOT).ok().unwrap();
    /// rectmanager.resize(rect, 10, 10);
    /// assert_eq!(rectmanager.get_rect_height(rect), 10);
    /// rectmanager.kill();
    /// ```
    pub fn get_rect_height(&self, rect_id: usize) -> usize {
        let (_, height) = self.get_rect_size(rect_id).unwrap();
        height
    }

    /// Get dimensions of specified rectangle, if it exists
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let rect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    /// // Resizing to make sure we know the size
    /// rectmanager.resize(rect_id, 10, 10);
    /// assert_eq!((10, 10), rectmanager.get_rect_size(rect_id).unwrap());
    /// ```
    pub fn get_rect_size(&self, rect_id: usize) -> Option<(usize, usize)> {
        match self.get_rect(rect_id) {
            Some(rect) => {
                Some((rect.width, rect.height))
            }
            None => {
                None
            }
        }
    }

    /// Add a new rectangle to the environment
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    ///
    /// // Create a rectangle and attach it as a child to the top-level rectangle.
    /// let first_rect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    ///
    /// // Create a child of the newly created rect...
    /// let second_rect_id = rectmanager.new_rect(first_rect_id).ok().unwrap();
    ///
    /// rectmanager.kill();
    /// ```
    pub fn new_rect(&mut self, parent_id: usize) -> Result<usize, WreckedError> {
        let new_id = self.gen_id();

        self.rects.entry(new_id).or_insert(Rect::new(new_id));

        self.attach(new_id, parent_id)?;

        self.flag_refresh(new_id)?;

        Ok(new_id)
    }


    /// Render the rectangle (and all its children) specified. This will not update the Rects at higher levels and can lead to artifacts.
    /// # Example
    /// ```
    /// // Use ROOT to draw everything
    /// use std::{thread, time};
    /// use wrecked::{RectManager, ROOT};
    ///
    /// let mut rectmanager = RectManager::new();
    /// let some_rect = rectmanager.new_rect(ROOT).ok().unwrap();
    /// // Adjust the rectangle so it will fit the string
    /// rectmanager.resize(some_rect, 16, 1);
    /// // Give it some text
    /// rectmanager.set_string(some_rect, 0, 0, "Hello World");
    ///
    /// // draw the latest changes, but only those of some_rect
    /// rectmanager.draw(some_rect);
    ///
    /// // wait 5 seconds (in order to see the screen)
    /// let five_seconds = time::Duration::from_secs(5);
    /// let now = time::Instant::now();
    /// thread::sleep(five_seconds);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn draw(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        match self.build_latest_rect_string(rect_id) {
            Some(renderstring) => {
                RectManager::write(&format!("{}\x1B[0m\x1B[1;1H", renderstring))?;
            }
            None => ()
        }
        Ok(())
    }

    /// Resize a rectangle
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let rect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    /// // Resizing to make sure we know the size
    /// rectmanager.resize(rect_id, 10, 10);
    /// assert_eq!((10, 10), rectmanager.get_rect_size(rect_id).unwrap());
    /// ```
    pub fn resize(&mut self, rect_id: usize, width: usize, height: usize) -> Result<(), WreckedError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.resize(width, height);
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        let mut position = None;
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                position = parent.get_child_position(rect_id);
            }
            None => ()
        }

        match position {
            Some((x, y)) => {
                self.set_position(rect_id, x, y)?;
            }
            None => ()
        }


        self.flag_refresh(rect_id)?;

        Ok(())
    }

    /// Move all child rectangles, but not characters by the offsets specified
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_parent = rectmanager.new_rect(ROOT).ok().unwrap();
    /// rectmanager.resize(rect_parent, 16, 5);
    ///
    /// // Put a string at (0, 0)
    /// rectmanager.set_string(rect_parent, 0, 0, "Hello world");
    /// // Put a rect at (0, 1)
    /// let rect_child = rectmanager.new_rect(rect_parent).ok().unwrap();
    /// rectmanager.set_position(rect_child, 0, 1);
    /// // Shift contents down one row ...
    /// rectmanager.shift_contents(rect_parent, 0, 1);
    ///
    /// assert_eq!(rectmanager.get_character(rect_parent, 0, 0).ok().unwrap(), 'H');
    /// assert_eq!(rectmanager.get_relative_offset(rect_child).unwrap(), (0, 2));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn shift_contents(&mut self, rect_id: usize, x_offset: isize, y_offset: isize) -> Result<(), WreckedError> {
        let mut child_ids = Vec::new();

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.shift_contents(x_offset, y_offset);
                for child_id in rect.children.iter() {
                    child_ids.push(*child_id);
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        for child_id in child_ids.iter() {
            self.update_child_space(*child_id)?;
        }

        self.flag_refresh(rect_id)?;

        Ok(())
    }

    /// Set relative offset of given rectangle.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    /// rectmanager.set_position(rect_id, 4, 4);
    /// assert_eq!(rectmanager.get_relative_offset(rect_id).unwrap(), (4, 4));
    /// ```
    pub fn set_position(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), WreckedError> {
        let mut has_parent = false;

        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                let did_move = match parent.child_positions.get(&rect_id) {
                    Some((xx, yy)) => {
                        !(*xx == x && *yy == y)
                    }
                    None => {
                        true
                    }
                };

                if did_move {
                    parent.set_child_position(rect_id, x, y);
                }
                has_parent = true;
            }
            None => {
                Err(WreckedError::NoParent(rect_id))?;
            }
        }

        if has_parent {
            self.update_child_space(rect_id)?;
            self.cached_to_queued(rect_id);
            self.flag_parent_refresh(rect_id)?;
        }

        Ok(())
    }

    /// Do not draw the given rectangle or is descendents when draw() is called.
    pub fn disable(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let mut was_enabled = false;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                was_enabled = rect.enabled;
                rect.disable();
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        let mut parent_id = ROOT;

        if was_enabled {
            match self.get_parent_mut(rect_id) {
                Some(parent) => {
                    parent.clear_child_space(rect_id);
                    parent_id = parent.rect_id;
                }
                None => {
                    Err(WreckedError::NotFound(rect_id))?;
                }
            }

            self.flag_refresh(parent_id)?;
        }

        Ok(())
    }

    /// If a rectangle has been disabled, enable it.
    pub fn enable(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let mut was_enabled = false;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                was_enabled = rect.enabled;
                rect.enable();
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }


        if ! was_enabled {
            self.update_child_space(rect_id)?;
        }

        Ok(())
    }

    /// Remove all the text added to a rectangle.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// // Add some Characters to ROOT rect
    /// for x in 0 .. 10 {
    ///     rectmanager.set_character(ROOT, x, 0, 'X');
    /// }
    /// // Now delete them all ...
    /// rectmanager.clear_characters(ROOT);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn clear_characters(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.clear_characters();
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        };
        self.flag_refresh(rect_id)
    }

    /// Remove all children from a rectangle, deleting them.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// // Add some children to ROOT rect
    /// for _ in 0 .. 10 {
    ///     rectmanager.new_rect(ROOT).ok().unwrap();
    /// }
    /// // Now delete them all ...
    /// rectmanager.clear_children(ROOT);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn clear_children(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let mut children = Vec::new();

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                for child_id in rect.children.iter() {
                    children.push(*child_id);
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        for child_id in children.iter() {
            self.delete_rect(*child_id)?;
        }

        Ok(())

    }

    /// Remove all effects from the rectangle's text. Does not apply recursively.
    pub fn clear_effects(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.effects.clear();
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        };

        self.flag_refresh(rect_id)?;

        Ok(())
    }


    /// Remove a rectangle from its parent without destroying it, so it can be reattached later.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// // Create a rectangle that is attached to ROOT.
    /// let rect_a = rectmanager.new_rect(ROOT).ok().unwrap();
    /// rectmanager.detach(rect_a);
    ///
    /// assert!(!rectmanager.has_parent(rect_a));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn detach(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        self.clear_child_space(rect_id)?;

        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.detach_child(rect_id);
            }
            None => ()
        }

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_parent();
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        Ok(())
    }

    /// Attach one rect as a child to another.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// // Create a rectangle that is attached to ROOT.
    /// let rect_a = rectmanager.new_rect(ROOT).ok().unwrap();
    /// // Create an orphan rectangle to switch in.
    /// let rect_b = rectmanager.new_orphan().ok().unwrap();
    /// rectmanager.attach(rect_b, rect_a);
    ///
    /// assert_eq!(rectmanager.get_parent_id(rect_b).unwrap(), rect_a);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn attach(&mut self, rect_id: usize, new_parent_id: usize) -> Result<(), WreckedError> {
        self.detach(rect_id)?;

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_parent(new_parent_id);
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        match self.get_rect_mut(new_parent_id) {
            Some(parent) => {
                parent.add_child(rect_id);
            }
            None => {
                Err(WreckedError::NoParent(rect_id))?;
            }
        }

        self.flag_parent_refresh(rect_id)?;

        Ok(())
    }

    /// Set a string of characters starting at the specified position of the given rectangle.
    /// Wraps automatically, but will throw error on y-overflow.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_string(ROOT, 0, 0, "This Some Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_string(&mut self, rect_id: usize, start_x: isize, start_y: isize, string: &str) -> Result<(), WreckedError> {
        let mut dimensions = (0, 0);

        match self.get_rect_size(rect_id) {
            Some(_dim) => {
                dimensions = (_dim.0 as isize, _dim.1 as isize);
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }


        let mut x;
        let mut y;
        let start_offset = (start_y * dimensions.0) + start_x;

        if start_offset + (string.len() as isize) > dimensions.0 * dimensions.1 {
            Err(WreckedError::StringTooLong(rect_id, (start_x, start_y), string.to_string()))?;
        }

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                let mut i = start_offset;
                for character in string.chars() {
                    x = i % dimensions.0;
                    y = i / dimensions.0;
                    rect.set_character(x, y, character)?;
                    i += 1;
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;

        Ok(())
    }

    /// Set the character at the given position of a rectangle.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_character(ROOT, 0, 0, 'X');
    /// assert_eq!(rectmanager.get_character(ROOT, 0, 0).ok().unwrap(), 'X');
    /// rectmanager.kill();
    /// ```
    pub fn set_character(&mut self, rect_id: usize, x: isize, y: isize, character: char) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_character(x, y, character)
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_pos_refresh(rect_id, x, y)
        } else {
            Ok(())
        }
    }

    /// Delete a set character of a given rectangle at specified point
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_character(ROOT, 0, 0, 'X');
    /// rectmanager.unset_character(ROOT, 0, 0);
    /// assert_eq!(rectmanager.get_character(ROOT, 0, 0).ok().unwrap(), rectmanager.get_default_character(ROOT));
    /// rectmanager.kill();
    /// ```
    pub fn unset_character(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_character(x, y)
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Completely erase a rectangle & remove it from the RectManager's tree.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect = rectmanager.new_rect(ROOT).ok().unwrap();
    ///
    /// rectmanager.delete_rect(rect);
    /// assert!(!rectmanager.has_rect(rect));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn delete_rect(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let mut to_delete = Vec::new();
        let mut stack = vec![rect_id];

        while stack.len() > 0 {
            match stack.pop() {
                Some(working_id) => {
                    match self.get_rect_mut(working_id) {
                        Some(rect) => {
                            stack.extend(rect.children.iter().copied());
                            to_delete.push(working_id);
                        }
                        // don't throw an error here. it may be the case that the
                        // parent still needs to be deleted even though the children are missing
                        None => ()
                    }
                }
                None => {
                    break;
                }
            }
        }

        self.clear_child_space(rect_id)?;
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.detach_child(rect_id);
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        for id in to_delete.iter() {
            self.rects.remove(&id);
            self.recycle_id(*id);
        }

        Ok(())
    }

    /// Swap out one rectangle with another.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// // Create a rectangle that is attached to ROOT.
    /// let rect_a = rectmanager.new_rect(ROOT).ok().unwrap();
    /// // Create an orphan rectangle to switch in.
    /// let rect_b = rectmanager.new_orphan().ok().unwrap();
    /// rectmanager.replace_with(rect_a, rect_b);
    ///
    /// assert!(rectmanager.has_parent(rect_b));
    /// assert!(!rectmanager.has_parent(rect_a));
    /// ```
    pub fn replace_with(&mut self, old_rect_id: usize, new_rect_id: usize) -> Result<(), WreckedError> {
        let mut parent_id = ROOT;
        let mut old_position = (0, 0);
        match self.get_parent_mut(old_rect_id) {
            Some(parent) => {
                parent_id = parent.rect_id;
                old_position = *parent.child_positions.get(&old_rect_id).unwrap();
            }
            None => {
                Err(WreckedError::NotFound(old_rect_id))?;
            }
        }

        self.detach(old_rect_id)?;
        self.attach(new_rect_id, parent_id)?;
        self.set_position(new_rect_id, old_position.0, old_position.1)?;

        Ok(())
    }

    /// Apply bold effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_bold_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Some Bold Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_bold_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_bold_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Disable bold text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_bold_flag(ROOT);
    /// rectmanager.unset_bold_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_bold_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_bold_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Apply underline effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_underline_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Some Underlined Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_underline_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_underline_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;
        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Disable underline text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_underline_flag(ROOT);
    /// rectmanager.unset_underline_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_underline_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_underline_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;
        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Invert the background and foreground colors of the text of the given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_invert_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Some Inverted Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_invert_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_invert_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;
        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Disable invert text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_invert_flag(ROOT);
    /// rectmanager.unset_invert_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_invert_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_invert_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;
        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Apply italics effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_italics_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Some Italicized Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_italics_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_italics_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;
        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Disable italics text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_italics_flag(ROOT);
    /// rectmanager.unset_italics_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_italics_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_italics_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;
        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Apply strike effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_strike_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Some Text With Strikethrough");
    /// rectmanager.kill();
    /// ```
    pub fn set_strike_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_strike_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Disable strike text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_strike_flag(ROOT);
    /// rectmanager.unset_strike_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_strike_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_strike_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;
        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Apply blink effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_blink_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Some Blinking Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_blink_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_blink_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Disable blink text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_blink_flag(ROOT);
    /// rectmanager.unset_blink_flag(ROOT);
    /// rectmanager.set_string(ROOT, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_blink_flag(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_blink_flag())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Set color of background of given rect (does not apply recursively)
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a Green background
    /// rectmanager.set_fg_color(ROOT, Color::GREEN);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn set_bg_color(&mut self, rect_id: usize, color: Color) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_bg_color(color))
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Return background color to default
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a Magenta background
    /// rectmanager.set_bg_color(ROOT, Color::MAGENTA);
    ///
    /// // Remove those colors...
    /// rectmanager.unset_bg_color(ROOT);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn unset_bg_color(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_bg_color())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Set color of foreground (text) of given rect (does not apply recursively)
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a YELLOW foreground
    /// rectmanager.set_fg_color(ROOT, Color::YELLOW);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn set_fg_color(&mut self, rect_id: usize, color: Color) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.set_fg_color(color))
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Return foreground color to default
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a White foreground
    /// rectmanager.set_fg_color(ROOT, Color::WHITE);
    ///
    /// // Remove those colors...
    /// rectmanager.unset_fg_color(ROOT);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn unset_fg_color(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_fg_color())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }

    /// Return both background and foreground colors to default
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a Blue background and a White foreground
    /// rectmanager.set_bg_color(ROOT, Color::BLUE);
    /// rectmanager.set_fg_color(ROOT, Color::WHITE);
    ///
    /// // Remove those colors...
    /// rectmanager.unset_color(ROOT);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn unset_color(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let changed = match self.get_rect_mut(rect_id) {
            Some(rect) => {
                Ok(rect.unset_color())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }?;

        if changed {
            self.flag_refresh(rect_id)
        } else {
            Ok(())
        }
    }
    /// Get the fallback character that would be displayed where no character is set.
    /// Defaults to ' '.
    pub fn get_default_character(&self, rect_id: usize) -> char {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.get_default_character()
            }
            None => {
                self.default_character
            }
        }
    }

    /// Get id of parent rectangle
    /// # Example
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    ///
    /// let mut rect = rectmanager.new_rect(ROOT);
    /// assert_eq!(rectmanager.get_parent_id(rect), Some(ROOT));
    ///
    /// rectmanager.detach(rect);
    /// assert_eq!(rectmanager.get_parent_id(rect), None);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn get_parent_id(&self, rect_id: usize) -> Option<usize> {
        let mut output = None;

        match self.get_rect(rect_id) {
            Some(rect) => {
                output = rect.parent;
            }
            None => ()
        };

        output
    }

    /// Check if given Rect is connected to a parent.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect = rectmanager.new_rect(ROOT).ok().unwrap();
    /// assert!(rectmanager.has_parent(rect));
    ///
    /// rectmanager.detach(rect);
    /// assert!(!rectmanager.has_parent(rect));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn has_parent(&self, rect_id: usize) -> bool {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.parent.is_some()
            }
            None => {
                false
            }
        }
    }

    /// Check if the given id has an associated Rect within the RectManager.
    pub fn has_rect(&self, rect_id: usize) -> bool {
        self.rects.contains_key(&rect_id)
    }


    /// Check if the given Rect displays its background where no characters are set.
    pub fn is_transparent(&self, rect_id: usize) -> bool {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.transparent
            }
            None => {
                false
            }
        }
    }


    /// Set the transparency of given Rect. Transparent Rects will show the content of the Rects behind them where no characters or children are set. Opaque Rects will display the default characters in the corresponding foreground and background colors.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// let rect = rectmanager.new_rect(ROOT).ok().unwrap();
    /// rectmanager.set_transparency(rect, true);
    /// rectmanager.kill();
    /// ```
    pub fn set_transparency(&mut self, rect_id: usize, transparent: bool) -> Result<(), WreckedError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.transparent = transparent;
                Ok(())
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }
    }


    /// Get foreground color of given Rect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// assert_eq!(rectmanager.get_fg_color(ROOT), None);
    /// rectmanager.set_fg_color(ROOT, Color::BLUE);
    /// assert_eq!(rectmanager.get_fg_color(ROOT), Some(Color::BLUE));
    /// // turn echo back on and return input to normal.
    /// rectmanager.kill();
    /// ```
    pub fn get_fg_color(&self, rect_id: usize) -> Option<Color> {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.get_fg_color()
            }
            None => {
                None
            }
        }
    }

    /// Get background color of given rectangle
    /// # Example
    /// ```
    /// use wrecked::{RectManager, ROOT, Color};
    /// let mut rectmanager = RectManager::new();
    /// assert_eq!(rectmanager.get_bg_color(ROOT), None);
    /// rectmanager.set_bg_color(ROOT, Color::BLUE);
    /// assert_eq!(rectmanager.get_bg_color(ROOT), Some(Color::BLUE));
    /// // turn echo back on and return input to normal.
    /// rectmanager.kill();
    /// ```
    pub fn get_bg_color(&self, rect_id: usize) -> Option<Color> {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.get_bg_color()
            }
            None => {
                None
            }
        }
    }


    fn flag_refresh(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.flag_refresh();
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        self.flag_parent_refresh(rect_id)?;

        Ok(())
    }

    fn get_rect(&self, rect_id: usize) -> Option<&Rect> {
        self.rects.get(&rect_id)
    }

    fn get_rect_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        self.rects.get_mut(&rect_id)
    }

    fn get_parent(&self, rect_id: usize) -> Option<&Rect> {
        let mut has_parent = false;
        let mut parent_id = ROOT;

        match self.get_rect(rect_id) {
            Some(rect) => {
                match rect.parent {
                    Some(pid) => {
                        has_parent = true;
                        parent_id = pid;
                    }
                    None => ()
                }
            }
            None => ()
        }


        if has_parent {
            self.get_rect(parent_id)
        } else {
            None
        }
    }

    fn get_parent_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        let mut has_parent = false;
        let mut parent_id = ROOT;

        match self.get_rect(rect_id) {
            Some(rect) => {
                match rect.parent {
                    Some(pid) => {
                        has_parent = true;
                        parent_id = pid;
                    }
                    None => ()
                }
            }
            None => ()
        }


        if has_parent {
            self.get_rect_mut(parent_id)
        } else {
            None
        }
    }

    fn _update_queue_by_positions(&mut self, rect_id: usize, positions: &HashSet<(isize, isize)>) -> Result<(), WreckedError> {
        let mut pos_stack: HashMap<(isize, isize), Vec<(usize, usize)>> = HashMap::new();
        let mut require_updates: HashSet<usize> = HashSet::new();

        let mut x: isize;
        let mut y: isize;
        let mut tmp_chr;
        let mut tmp_fx;

        let mut child_ids = Vec::new();
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                child_ids = rect.children.clone();
            }
            None => ()
        }

        let mut transparent_children = HashSet::new();
        for child_id in child_ids.iter() {
            if self.is_transparent(*child_id) {
                transparent_children.insert(*child_id);
            }
        }

        let mut child_positions: HashMap<usize, (isize, isize)> = HashMap::new();
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                child_positions = rect.child_positions.clone();
                for (_x, _y) in positions.iter() {
                    x = *_x;
                    y = *_y;

                    if x < 0 || x >= rect.width as isize || y < 0 || y >= rect.height as isize {
                        continue;
                    }

                    if !rect.child_space.contains_key(&(x, y)) || rect.child_space[&(x, y)].is_empty() {
                        // Make sure at least default character is present
                        if !rect.transparent {
                            tmp_fx = rect.effects;

                            tmp_chr = rect.character_space.entry((x, y))
                                .or_insert(rect.default_character);

                            rect._queued_display.entry((x,y))
                                .and_modify(|e| {*e = (*tmp_chr, tmp_fx, 0)})
                                .or_insert((*tmp_chr, tmp_fx, 0));
                        } else {
                            rect._queued_display.remove(&(x, y));
                        }
                    } else {
                        match rect.child_space.get(&(x, y)) {
                            Some(child_ids) => {
                                for (i, child_id) in child_ids.iter().rev().enumerate() {
                                    let rank = child_ids.len() - i;
                                    require_updates.insert(*child_id);
                                    pos_stack.entry((x, y))
                                        .and_modify(|e| e.push((*child_id, rank)))
                                        .or_insert(vec![(*child_id, rank)]);

                                    if !transparent_children.contains(child_id) {
                                        break;
                                    }
                                }
                            }
                            None => ()
                        }

                    }
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        for child_id in require_updates.iter() {
            self._update_queued_display(*child_id)?;
        }


        let mut new_values = Vec::new();
        let mut transparent_coords = HashSet::new();

        for ((x, y), child_ids) in pos_stack.iter() {
            for (child_id, rank) in child_ids.iter() {
                match child_positions.get(child_id) {
                    Some(child_position) => {
                        match self.get_rect_mut(*child_id) {
                            Some(child) => {

                                match child._queued_display.get(&(*x - child_position.0, *y - child_position.1)) {
                                    Some(new_value) => {
                                        new_values.push((*new_value, *rank, *x, *y));
                                        break;
                                    }
                                    None => {
                                        if child.transparent {
                                            transparent_coords.insert((*x, *y));
                                        }
                                    }
                                }
                            }
                            None => {
                                Err(WreckedError::NotFound(*child_id))?;
                            }
                        }
                    }
                    None => ()
                }
            }
        }

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                for (new_value, rank, x, y) in new_values.iter() {
                    rect._queued_display.entry((*x, *y))
                        .and_modify(|e| {
                            if e.2 <= *rank {
                                *e = (new_value.0, new_value.1, *rank);
                            }
                        })
                        .or_insert((new_value.0, new_value.1, *rank));

                    transparent_coords.remove(&(*x, *y));
                }

                for coord in transparent_coords.iter() {
                    if rect.transparent {
                        rect._queued_display.remove(coord);
                    } else {
                        tmp_fx = rect.effects;

                        tmp_chr = rect.character_space.entry(*coord)
                            .or_insert(rect.default_character);

                        rect._queued_display.entry(*coord)
                            .and_modify(|e| {*e = (*tmp_chr, tmp_fx, 0)})
                            .or_insert((tmp_chr.clone(), tmp_fx.clone(), 0));
                    }
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }
        Ok(())
    }

    fn _update_queued_display(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let mut flags_pos_refresh = HashSet::new();

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                if rect.enabled {
                    /*
                      If a full refresh is requested,
                      fill flags_pos_refresh with all potential coords
                    */
                    if rect.flag_full_refresh {
                        rect.flag_full_refresh = false;
                        rect._queued_display.clear();

                        for y in 0 .. rect.height {
                            for x in 0 .. rect.width {
                                flags_pos_refresh.insert((x as isize, y as isize));
                            }
                        }

                        rect.flags_pos_refresh.clear();
                    } else {
                        /*
                            Iterate through flags_pos_refresh and update
                            any children that cover the requested positions
                        */
                        for pos in rect.flags_pos_refresh.iter() {
                            if pos.0 >= 0 && pos.1 >= 0
                            && pos.0 < rect.width as isize
                            && pos.1 < rect.height as isize {
                                flags_pos_refresh.insert((pos.0 as isize, pos.1 as isize));
                            }
                        }
                    }
                    rect.flags_pos_refresh.clear();
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        self._update_queue_by_positions(rect_id, &flags_pos_refresh)?;

        Ok(())
    }

    fn get_visible_box(&self, rect_id: usize) -> Result<(isize, isize, isize, isize), WreckedError> {
        let mut rect_box = (0, 0, 0, 0);

        match self.get_rect_size(rect_id) {
            Some(_dim) => {
                rect_box.2 = _dim.0 as isize;
                rect_box.3 = _dim.1 as isize;
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        match self.get_absolute_offset(rect_id) {
            Some(offset) => {
                rect_box.0 = offset.0;
                rect_box.1 = offset.1;
            }
            None => ()
        }

        let mut working_id = rect_id;
        let mut parent_dim;
        loop {
            match self.get_parent(working_id) {
                Some(parent) => {
                    parent_dim = (parent.width, parent.height);
                    working_id = parent.rect_id;

                }
                None => {
                    break;
                }
            }

            match self.get_absolute_offset(working_id) {
                Some(offset) => {
                    rect_box.0 = cmp::max(rect_box.0, offset.0);
                    rect_box.1 = cmp::max(rect_box.1, offset.1);
                    rect_box.2 = cmp::min((offset.0 + parent_dim.0 as isize) - rect_box.0, rect_box.2);
                    rect_box.3 = cmp::min((offset.1 + parent_dim.1 as isize) - rect_box.1, rect_box.3);
                }
                None => {
                    Err(WreckedError::NotFound(working_id))?;
                }
            }
        }

        Ok(rect_box)
    }

    fn get_queued_display(&mut self, rect_id: usize) -> Result<&HashMap<(isize, isize), (char, EffectsHandler, usize)>, WreckedError> {
        self._update_queued_display(rect_id)?;

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                if rect.enabled {
                    Ok(&rect._queued_display)
                } else {
                    Err(WreckedError::Disabled(rect_id))
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))
            }
        }
    }

    fn get_queued_draw_map(&mut self, rect_id: usize) -> Vec<((isize, isize), (char, EffectsHandler))> {
        let mut to_draw = Vec::new();

        let mut offset = (0, 0);
        match self.get_absolute_offset(rect_id) {
            Some(_offset) => {
                offset = _offset;
            }
            None => ()
        }

        let mut boundry_box = (0, 0, 0, 0);
        match self.get_visible_box(rect_id) {
            Ok(_box) => {
                boundry_box = _box;
            }
            Err(_e) => { }
        }

        let mut to_cache = Vec::new();
        match self.get_queued_display(rect_id) {
            Ok(display_map) => {
                for (pos, val) in display_map.iter() {
                    if offset.0 + pos.0 < boundry_box.0
                    || offset.0 + pos.0 >= boundry_box.0 + boundry_box.2
                    || offset.1 + pos.1 < boundry_box.1
                    || offset.1 + pos.1 >= boundry_box.1 + boundry_box.3 {
                        // Ignore
                    } else {
                        to_draw.push(((offset.0 + pos.0, offset.1 + pos.1), (val.0, val.1)));
                        to_cache.push(*pos);
                    }
                }
            }
            Err(_e) => {}
        }

        // Cache the queued values
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                for key in to_cache.drain(..) {
                    let val = rect._queued_display.remove(&key).unwrap();
                    rect._cached_display.insert(key, val);
                }
            }
            None => {}
        }


        to_draw
    }

    // Flags the area of the parent of given rect covered by the given rect
    fn flag_parent_refresh(&mut self, rect_id: usize) -> Result<(), WreckedError> {
        let mut dimensions = (0, 0);
        match self.get_rect_size(rect_id) {
            Some(_dim) => {
                dimensions = _dim;
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        let mut working_id = rect_id;
        let mut offset = (0, 0);

        loop {
            match self.get_relative_offset(working_id) {
                Some(rel_offset) => {
                    offset = (
                        offset.0 + rel_offset.0,
                        offset.1 + rel_offset.1
                    );
                }
                None => ()
            }

            match self.get_parent_mut(working_id) {
                Some(parent) => {
                    for x in 0 .. dimensions.0 {
                        for y in 0 .. dimensions.1 {
                            parent.flags_pos_refresh.insert((offset.0 + x as isize, offset.1 + y as isize));
                        }
                    }
                    working_id = parent.rect_id;
                }
                None => {
                    break;
                }
            }
        }

        Ok(())
    }

    fn flag_pos_refresh(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), WreckedError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.flags_pos_refresh.insert((x, y));
                match rect._cached_display.remove(&(x, y)) {
                    Some(_) => {}
                    None => {}
                }
            }
            None => {
                Err(WreckedError::NotFound(rect_id))?;
            }
        }

        // loop top, setting requisite refresh flags
        let mut x_out = x;
        let mut y_out = y;
        let mut working_id = rect_id;
        loop {
            match self.get_relative_offset(rect_id) {
                Some(offs) => {
                    x_out += offs.0;
                    y_out += offs.1;
                }
                None => ()
            }

            match self.get_parent_mut(working_id) {
                Some(parent) => {
                    parent.flags_pos_refresh.insert((x_out, y_out));
                    working_id = parent.rect_id;
                }
                None => {
                    break;
                }
            }
        }

        Ok(())
    }

    fn update_child_space(&mut self, child_id: usize) -> Result<(), WreckedError> {
        let mut dimensions = (0, 0);
        let mut position = (0, 0);

        match self.get_rect_size(child_id) {
            Some(_dim) => {
                dimensions = (_dim.0 as isize, _dim.1 as isize);
            }
            None => {
                Err(WreckedError::NotFound(child_id))?;
            }
        }

        match self.get_relative_offset(child_id) {
            Some(_pos) => {
                position = _pos;
            }
            None => ()
        }

        match self.get_parent_mut(child_id) {
            Some(rect) => {
                rect.update_child_space(child_id, (
                    position.0,
                    position.1,
                    position.0 + dimensions.0,
                    position.1 + dimensions.1
                ), false);
            }
            None => ()
        }

        self.flag_parent_refresh(child_id)?;

        Ok(())
    }

    fn clear_child_space(&mut self, child_id: usize) -> Result<(), WreckedError> {
        self.flag_parent_refresh(child_id)?;

        match self.get_parent_mut(child_id) {
            Some(parent) => {
                parent.clear_child_space(child_id);
            }
            None => ()
        }

        Ok(())
    }

    /// builds a string from the latest cached content of the
    /// given rect.
    /// It's been separated to facilitate testing only, so if you're using it outside that case, you're likely doing something wrong.
    fn build_latest_rect_string(&mut self, rect_id: usize) -> Option<String> {
        let draw_map = self.get_queued_draw_map(rect_id);
        let mut filtered_map = self.filter_cached(draw_map);
        if filtered_map.len() > 0 {
            // Doesn't need to be sorted to work, but there're fewer ansi sequences if it is.
            filtered_map.sort();
            filtered_map.sort_by(|a,b|(a.0).1.cmp(&(b.0).1));

            Some(self.build_ansi_string(filtered_map))
        } else {
            None
        }
    }

    fn cached_to_queued(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                let mut tmp = Vec::new();
                for (key, val) in rect._cached_display.drain() {
                    tmp.push((key, val));
                }
                for (key, val) in tmp.drain(..) {
                    rect._queued_display.entry(key)
                        .and_modify(|e| {*e = val})
                        .or_insert(val);
                }

            }
            None => {}
        }
    }


    fn recycle_id(&mut self, old_id: usize) {
        // NOTE: Assumes 0 is reserved and can't be removed
        self.recycle_ids.push(old_id);
        self.recycle_ids.sort();

        while self.recycle_ids.len() > 0
        && self.recycle_ids.last().unwrap() == &(self.idgen - 1) {
            self.recycle_ids.pop();
            self.idgen -= 1;
        }
    }

}

#[derive(Debug)]
struct Rect {
    rect_id: usize,

    width: usize,
    height: usize,
    default_character: char,
    parent: Option<usize>, // RectId

    children: Vec<usize>,
    // Used to find a box by position
    child_space: HashMap<(isize, isize), Vec<usize>>,
    _inverse_child_space: HashMap<usize, Vec<(isize, isize)>>,
    // Used to find a position of a box
    child_positions: HashMap<usize, (isize, isize)>,
    _child_ranks: HashMap<usize, usize>,

    character_space: HashMap<(isize,isize), char>,

    flag_full_refresh: bool,
    flags_pos_refresh: HashSet<(isize, isize)>,

    enabled: bool,
    transparent: bool,

    effects: EffectsHandler,

    _queued_display: HashMap<(isize, isize), (char, EffectsHandler, usize)>,
    _cached_display: HashMap<(isize, isize), (char, EffectsHandler, usize)>
}

impl Rect {
    pub fn new(rect_id: usize) -> Rect {
        Rect {
            rect_id,
            parent: None,
            width: 1,
            height: 1,
            children: Vec::new(),
            child_space: HashMap::new(),
            _inverse_child_space: HashMap::new(),
            child_positions: HashMap::new(),
            _child_ranks: HashMap::new(),
            character_space: HashMap::new(),
            flag_full_refresh: true,
            flags_pos_refresh: HashSet::new(),
            enabled: true,
            transparent: false,

            effects: EffectsHandler::new(),

            _queued_display: HashMap::new(),
            _cached_display: HashMap::new(),
            default_character: ' ' // Space
        }
    }

    fn disable(&mut self) {
        self.enabled = false;
    }

    fn enable(&mut self) {
        self.enabled = true;
    }

    fn flag_refresh(&mut self) {
        self.flag_full_refresh = true;
        self._cached_display.drain();
    }

    fn get_default_character(&self) -> char {
        self.default_character
    }

    fn shift_contents(&mut self, x_offset: isize, y_offset: isize) {
        for (_child_id, position) in self.child_positions.iter_mut() {
            *position = (
                position.0 + x_offset,
                position.1 + y_offset
            )
        }
    }

    fn get_child_position(&self, child_id: usize) -> Option<(isize, isize)> {
        match self.child_positions.get(&child_id) {
            Some((x, y)) => {
                Some((*x, *y))
            }
            None => {
                None
            }
        }
    }

    fn update_child_space(&mut self, rect_id: usize, corners: (isize, isize, isize, isize), keep_cached: bool) {
        if !keep_cached {
            self.clear_child_space(rect_id);
        }

        let child_ranks = &self._child_ranks;
        for y in corners.1 .. corners.3 {
            for x in corners.0 .. corners.2 {
                if x >= 0 && x < self.width as isize && y >= 0 && y < self.height as isize {
                    self.child_space.entry((x, y))
                        .or_insert(Vec::new());

                    match self.child_space.get_mut(&(x, y)) {
                        Some(child_list) => {
                            child_list.push(rect_id);
                            child_list.sort_by(|a, b| {
                                child_ranks[a].cmp(&child_ranks[b])
                            });
                        }
                        None => ()
                    }


                    self._inverse_child_space.entry(rect_id)
                        .or_insert(Vec::new())
                        .push((x, y));
                }
            }
        }
    }

    fn clear_child_space(&mut self, rect_id: usize) {
        let new_positions = match self._inverse_child_space.get(&rect_id) {
            Some(positions) => {
                positions.clone()
            }
            None => {
                vec![]
            }
        };

        for position in new_positions.iter() {
            self.flags_pos_refresh.insert(*position);

            match self.child_space.get_mut(&position) {
                Some(child_ids) => {
                    child_ids.retain(|&x| x != rect_id);
                }
                None => ()
            }

        }

        self._inverse_child_space.entry(rect_id)
            .or_insert(Vec::new())
            .clear();
    }

    fn get_character(&self, x: isize, y: isize) -> Result<char, WreckedError> {
        if y < self.height as isize && y >= 0 && x < self.width as isize && x >= 0 {
            match self.character_space.get(&(x, y)) {
                Some(character) => {
                    Ok(character.clone())
                }
                None => {
                    Ok(self.default_character)
                }
            }
        } else {
            Err(WreckedError::BadPosition(x, y))
        }
    }

    fn set_character(&mut self, x: isize, y: isize, character: char) -> Result<bool, WreckedError> {
        if y < self.height as isize && y >= 0 && x < self.width as isize && x >= 0 {
            let mut changed = true;
            match self.character_space.get(&(x,y)) {
                Some(existing_char) => {
                    changed = *existing_char != character;
                }
                None => {
                    changed = character != self.default_character && ! self.transparent;
                }
            }

            if changed {
                self.character_space.entry((x, y))
                    .and_modify(|coord| { *coord = character })
                    .or_insert(character);
            }
            Ok(changed)
        } else {
            Err(WreckedError::BadPosition(x, y))
        }

    }

    fn unset_character(&mut self, x: isize, y: isize) -> Result<bool, WreckedError> {
        self.set_character(x, y, self.default_character)
    }

    fn set_bold_flag(&mut self) -> bool {
        if ! self.effects.bold {
            self.effects.bold = true;
            true
        } else {
            false
        }
    }

    fn unset_bold_flag(&mut self) -> bool {
        if self.effects.bold {
            self.effects.bold = false;
            true
        } else {
            false
        }
    }

    fn set_underline_flag(&mut self) -> bool {
        if ! self.effects.underline {
            self.effects.underline = true;
            true
        } else {
            false
        }
    }

    fn unset_underline_flag(&mut self) -> bool {
        if self.effects.underline {
            self.effects.underline = false;
            true
        } else {
            false
        }

    }

    fn set_invert_flag(&mut self) -> bool {
        if ! self.effects.invert {
            self.effects.invert = true;
            true
        } else {
            false
        }
    }

    fn unset_invert_flag(&mut self) -> bool {
        if self.effects.invert {
            self.effects.invert = false;
            true
        } else {
            false
        }

    }

    fn set_italics_flag(&mut self) -> bool {
        if ! self.effects.italics {
            self.effects.italics = true;
            true
        } else {
            false
        }

    }

    fn unset_italics_flag(&mut self) -> bool {
        if self.effects.italics {
            self.effects.italics = false;
            true
        } else {
            false
        }

    }

    fn set_strike_flag(&mut self) -> bool {
        if ! self.effects.strike {
            self.effects.strike = true;
            true
        } else {
            false
        }

    }

    fn unset_strike_flag(&mut self) -> bool {
        if self.effects.strike {
            self.effects.strike = false;
            true
        } else {
            false
        }

    }

    fn set_blink_flag(&mut self) -> bool {
        if ! self.effects.blink {
            self.effects.blink = true;
            true
        } else {
            false
        }
    }

    fn unset_blink_flag(&mut self) -> bool {
        if self.effects.blink {
            self.effects.blink = false;
            true
        } else {
            false
        }
    }

    fn unset_bg_color(&mut self) -> bool {
        if self.effects.background_color.is_some() {
            self.effects.background_color = None;
            true
        } else {
            false
        }
    }

    fn unset_fg_color(&mut self) -> bool {
        if self.effects.foreground_color.is_some() {
            self.effects.foreground_color = None;
            true
        } else {
            false
        }
    }

    fn unset_color(&mut self) -> bool {
        let mut changed = self.unset_bg_color();
        changed |= self.unset_fg_color();

        changed
    }

    fn set_bg_color(&mut self, color: Color) -> bool {
        if self.effects.background_color != Some(color) {
            self.effects.background_color = Some(color);
            true
        } else {
            false
        }
    }

    fn set_fg_color(&mut self, color: Color) -> bool {
        if self.effects.foreground_color != Some(color) {
            self.effects.foreground_color = Some(color);
            true
        } else {
            false
        }
    }

    fn add_child(&mut self, child_id: usize) {
        self.children.push(child_id);
        self._inverse_child_space.insert(child_id, Vec::new());
        self.update_child_ranks();
        self.set_child_position(child_id, 0, 0);
    }

    // Needed for quick access to child ranks
    fn update_child_ranks(&mut self) {
        self._child_ranks.drain();
        for (new_rank, child_id) in self.children.iter().enumerate() {
            self._child_ranks.insert(*child_id, new_rank);
        }
    }

    fn set_parent(&mut self, rect_id: usize) {
        self.parent = Some(rect_id);
    }

    fn unset_parent(&mut self) {
        self.parent = None;
    }

    fn detach_child(&mut self, rect_id: usize) {
        self.clear_child_space(rect_id);
        self.child_positions.remove(&rect_id);

        let mut new_children = Vec::new();
        for child_id in self.children.iter() {
            if *child_id != rect_id {
                new_children.push(*child_id);
            }
        }
        self.children = new_children;
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    // Can't update child_space here, need child width and height
    fn set_child_position(&mut self, rect_id: usize, x: isize, y: isize) {
        self.child_positions.entry(rect_id)
            .and_modify(|e| { *e = (x, y) })
            .or_insert((x, y));
    }

    fn clear_characters(&mut self) {
        self.character_space.clear();
        self._queued_display.clear();
    }

    fn get_fg_color(&self) -> Option<Color> {
        self.effects.foreground_color
    }
    fn get_bg_color(&self) -> Option<Color> {
        self.effects.background_color
    }

    // Everything below here is exclusively for testing
    fn is_plain(&self) -> bool {
        self.effects.is_plain()
    }

    fn is_bold(&self) -> bool {
        self.effects.bold
    }

    fn is_underlined(&self) -> bool {
        self.effects.underline
    }

    fn is_inverted(&self) -> bool {
        self.effects.invert
    }

    fn is_italicized(&self) -> bool {
        self.effects.italics
    }

    fn is_striken(&self) -> bool {
        self.effects.strike
    }

    fn is_blinking(&self) -> bool {
        self.effects.blink
    }

    fn has_child(&self, child_id: usize) -> bool {
        let mut output = false;
        for connected_child_id in self.children.iter() {
            if *connected_child_id == child_id {
                output = true;
                break;
            }
        }
        output
    }
}
