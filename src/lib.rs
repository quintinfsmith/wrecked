use std::collections::HashMap;
use std::collections::HashSet;
use std::str;
use std::io::{self, Write};
use std::cmp;
use std::cmp::PartialOrd;

use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
use std::fmt;

use std::error::Error;
use std::fmt::Display;

pub mod tests;
/*
    TODO
    Figure out why i made height/width of rect isize, change to usize or uN if not a good reason
*/

pub fn get_terminal_size() -> (u16, u16) {
    use libc::{winsize, TIOCGWINSZ, ioctl, isatty};
    let mut output = (1, 1);

    #[cfg(debug_assertions)]
    {
        output = (50, 25);
    }


    #[cfg(not(debug_assertions))]
    {
        let mut t = winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0
        };


        if unsafe { ioctl(libc::STDOUT_FILENO, TIOCGWINSZ.into(), &mut t) } != -1 {
            output = (t.ws_col, t.ws_row);
        }
    }

    output
}


#[derive(PartialEq, Eq, Debug)]
pub enum RectError {
    AllGood,
    BadColor,
    InvalidUtf8,
    StringTooLong,
    NotFound(usize),
    NoParent(usize), // Rect has no parent id
    BadPosition(isize, isize),
    ParentNotFound(usize, usize), // rect has an associated parent id that does not exist in RectManager
    ChildNotFound(usize, usize)
}
impl Display for RectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //let name = format!("{:?}", self);
        write!(f, "{:?}", self)
    }
}
impl Error for RectError {}

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Debug)]
pub enum RectColor {
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
struct RectEffectsHandler {
    bold: bool,
    underline: bool,
    invert: bool,
    italics: bool,
    strike: bool,
    blink: bool,
    background_color: Option<RectColor>,
    foreground_color: Option<RectColor>
}

impl fmt::Debug for RectEffectsHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RectEffectsHandler")
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

impl RectEffectsHandler {
    pub fn new() -> RectEffectsHandler {
        RectEffectsHandler {
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
    pub fn is_empty(&self) -> bool {
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
}

/// This is the id of the top-level rectangle that is instantiated when a new RectManager is created.
pub const TOP: usize = 0;

/// An environment to manage and display character-based graphics in-console.
///
/// # Example
/// ```
/// use std::{thread, time};
/// use wrecked::{RectManager, TOP};
///
/// let mut rectmanager = RectManager::new();
/// // rectmanager is initialized with top-level rect (id = TOP) attached...
/// rectmanager.set_string(TOP, 0, 0, "Hello World");
///
/// // draw the latest changes
/// rectmanager.draw();
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
    rects: HashMap<usize, Rect>,
    // top_cache is used to prevent redrawing the same
    // characters at the same coordinate.
    top_cache: HashMap<(isize, isize), (char, RectEffectsHandler)>,
    _termios: Option<Termios>,
    default_character: char
}

impl RectManager {
    /// Instantiate a new environment
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// // Initialize the console; turn off echo and enable non-canonical input.
    /// let mut rectmanager = RectManager::new();
    /// // turn echo back on and return input to normal.
    /// rectmanager.kill();
    /// ```
    pub fn new() -> RectManager {
        let termios = Termios::from_fd(libc::STDOUT_FILENO).ok();

        #[cfg(not(debug_assertions))]
        match termios.clone() {
            Some(mut new_termios) => {
                new_termios.c_lflag &= !(ICANON | ECHO);
                tcsetattr(0, TCSANOW, &mut new_termios).unwrap();

                RectManager::__write("\x1B[?25l\x1B[?1049h"); // New screen
            }
            None => {

            }
        }


        let mut rectmanager = RectManager {
            idgen: TOP,
            rects: HashMap::new(),
            top_cache: HashMap::new(),
            _termios: termios,
            default_character: ' '
        };

        rectmanager.new_orphan().expect("Couldn't Create TOP rect");
        rectmanager.auto_resize();
        rectmanager
    }

    fn __write(input: &str) -> io::Result<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(format!("{}\x0A", input).as_bytes())?;
        Ok(())
    }



    /// Add a new rectangle to the environment
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    ///
    /// // Create a rectangle and attach it as a child to the top-level rectangle.
    /// let first_rect_id = rectmanager.new_rect(TOP).ok().unwrap();
    ///
    /// // Create a child of the newly created rect...
    /// let second_rect_id = rectmanager.new_rect(first_rect_id).ok().unwrap();
    ///
    /// rectmanager.kill();
    /// ```
    pub fn new_rect(&mut self, parent_id: usize) -> Result<usize, RectError> {
        let new_id = self.idgen;
        self.idgen += 1;

        self.rects.entry(new_id).or_insert(Rect::new(new_id));

        self.attach(new_id, parent_id)?;

        self.flag_refresh(new_id)?;

        Ok(new_id)
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
    pub fn new_orphan(&mut self) -> Result<usize, RectError> {
        // For now, there's really no way to Result in an error here,
        // but future proofing and consistency and all that, we'll return a Result
        let new_id = self.idgen;
        self.idgen += 1;
        self.rects.entry(new_id).or_insert(Rect::new(new_id));

        Ok(new_id)
    }

    /// Render the rectangle (and all its children) specified.
    /// # Example
    /// ```
    /// // Use TOP to draw everything
    /// use std::{thread, time};
    /// use wrecked::{RectManager, TOP};
    ///
    /// let mut rectmanager = RectManager::new();
    /// let some_rect = rectmanager.new_rect(TOP).ok().unwrap();
    /// // Adjust the rectangle so it will fit the string
    /// rectmanager.resize(some_rect, 16, 1);
    /// // Give it some text
    /// rectmanager.set_string(some_rect, 0, 0, "Hello World");
    ///
    /// // draw the latest changes, but only those of some_rect
    /// rectmanager.draw_rect(some_rect);
    ///
    /// // wait 5 seconds (in order to see the screen)
    /// let five_seconds = time::Duration::from_secs(5);
    /// let now = time::Instant::now();
    /// thread::sleep(five_seconds);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn draw_rect(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.build_latest_rect_string(rect_id) {
            Some(renderstring) => {
                RectManager::__write(&format!("{}\x1B[0m\x1B[1;1H", renderstring));
            }
            None => ()
        }
        Ok(())
    }


    /// builds a string from the latest cached content of the
    /// given rect.
    /// It's been separated and made public for testing only, so if you're using it outside that case, you're likely doing something wrong.
    pub fn build_latest_rect_string(&mut self, rect_id: usize) -> Option<String> {
        let draw_map = self.build_draw_map(rect_id);
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

    /// Render the visible portion of the rectmanager environment
    /// # Example
    /// ```
    /// // Use TOP to draw everything
    /// use std::{thread, time};
    /// use wrecked::{RectManager, TOP};
    ///
    /// let mut rectmanager = RectManager::new();
    /// // rectmanager is initialized with top-level rect (id = TOP) attached...
    /// rectmanager.set_string(TOP, 0, 0, "Hello World");
    ///
    /// // draw the latest changes
    /// rectmanager.draw();
    ///
    /// // wait 5 seconds (in order to see the screen)
    /// let five_seconds = time::Duration::from_secs(5);
    /// let now = time::Instant::now();
    /// thread::sleep(five_seconds);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn draw(&mut self) -> Result<(), RectError> {
        self.draw_rect(TOP)
    }


    /// Get dimensions of specified rectangle, if it exists
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let rect_id = rectmanager.new_rect(TOP).ok().unwrap();
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


    /// Resize a rectangle
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let rect_id = rectmanager.new_rect(TOP).ok().unwrap();
    /// // Resizing to make sure we know the size
    /// rectmanager.resize(rect_id, 10, 10);
    /// assert_eq!((10, 10), rectmanager.get_rect_size(rect_id).unwrap());
    /// ```
    pub fn resize(&mut self, rect_id: usize, width: usize, height: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.resize(width, height);
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        };

        let mut position = None;
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                position = Some(parent.get_child_position(rect_id));
            }
            None => ()
        };

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
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_parent = rectmanager.new_rect(TOP).ok().unwrap();
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
    pub fn shift_contents(&mut self, rect_id: usize, x_offset: isize, y_offset: isize) -> Result<(), RectError> {
        let mut child_ids = Vec::new();

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.shift_contents(x_offset, y_offset);
                for child_id in rect.children.iter() {
                    child_ids.push(*child_id);
                }
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
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
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_id = rectmanager.new_rect(TOP).ok().unwrap();
    /// rectmanager.set_position(rect_id, 4, 4);
    /// assert_eq!(rectmanager.get_relative_offset(rect_id).unwrap(), (4, 4));
    /// ```
    pub fn set_position(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
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
                Err(RectError::NoParent(rect_id))?;
            }
        }

        if has_parent {
            self.update_child_space(rect_id)?;
            self.flag_parent_refresh(rect_id)?;
        }

        Ok(())
    }

    /// Do not draw the given rectangle or is descendents when draw() is called.
    pub fn disable(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut was_enabled = false;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                was_enabled = rect.enabled;
                rect.disable();
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        let mut parent_id = TOP;

        if was_enabled {
            match self.get_parent_mut(rect_id) {
                Some(parent) => {
                    parent.clear_child_space(rect_id);
                    parent_id = parent.rect_id;
                }
                None => {
                    Err(RectError::NotFound(rect_id))?;
                }
            }

            self.flag_refresh(parent_id)?;
        }

        Ok(())
    }

    /// If a rectangle has been disabled, enable it.
    pub fn enable(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut was_enabled = false;
        let mut parent = None;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                was_enabled = rect.enabled;
                rect.enable();
                parent = rect.parent;
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }


        if ! was_enabled {
            self.update_child_space(rect_id);
        }

        Ok(())
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

    /// Remove all the text added to a rectangle.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// // Add some Characters to TOP rect
    /// for x in 0 .. 10 {
    ///     rectmanager.set_character(TOP, x, 0, 'X');
    /// }
    /// // Now delete them all ...
    /// rectmanager.clear_characters(TOP);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn clear_characters(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.clear_characters();
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        };
        self.flag_refresh(rect_id)
    }

    /// Remove all children from a rectangle, deleting them.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// // Add some children to TOP rect
    /// for _ in 0 .. 10 {
    ///     rectmanager.new_rect(TOP).ok().unwrap();
    /// }
    /// // Now delete them all ...
    /// rectmanager.clear_children(TOP);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn clear_children(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut children = Vec::new();

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                for child_id in rect.children.iter() {
                    children.push(*child_id);
                }
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        for child_id in children.iter() {
            self.delete_rect(*child_id)?;
        }

        Ok(())

    }

    /// Remove all effects from the rectangle's text. Does not apply recursively.
    pub fn clear_effects(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.effects.clear();
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        };

        self.flag_refresh(rect_id)?;

        Ok(())
    }


    /// Remove a rectangle from its parent without destroying it, so it can be reattached later.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// // Create a rectangle that is attached to TOP.
    /// let rect_a = rectmanager.new_rect(TOP).ok().unwrap();
    /// rectmanager.detach(rect_a);
    ///
    /// assert!(!rectmanager.has_parent(rect_a));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn detach(&mut self, rect_id: usize) -> Result<(), RectError> {
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
                Err(RectError::NotFound(rect_id))?;
            }
        }

        Ok(())
    }

    /// Attach one rect as a child to another.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// // Create a rectangle that is attached to TOP.
    /// let rect_a = rectmanager.new_rect(TOP).ok().unwrap();
    /// // Create an orphan rectangle to switch in.
    /// let rect_b = rectmanager.new_orphan().ok().unwrap();
    /// rectmanager.attach(rect_b, rect_a);
    ///
    /// assert_eq!(rectmanager.get_parent_id(rect_b).unwrap(), rect_a);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn attach(&mut self, rect_id: usize, new_parent_id: usize) -> Result<(), RectError> {
        self.detach(rect_id)?;

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_parent(new_parent_id);
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        match self.get_rect_mut(new_parent_id) {
            Some(parent) => {
                parent.add_child(rect_id);
            }
            None => {
                Err(RectError::NoParent(rect_id))?;
            }
        }


        // TODO: This SHOULD only need flag_parent_refresh. but for some reason that break.
        self.flag_refresh(rect_id)?;

        Ok(())
    }

    /// Set a string of characters starting at the specified position of the given rectangle.
    /// Wraps automatically, but will throw error on y-overflow.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_string(TOP, 0, 0, "This Some Text");
    /// rectmanager.kill();
    /// ```

    pub fn set_string(&mut self, rect_id: usize, start_x: isize, start_y: isize, string: &str) -> Result<(), RectError> {
        let mut dimensions = (0, 0);

        match self.get_rect_size(rect_id) {
            Some(_dim) => {
                dimensions = (_dim.0 as isize, _dim.1 as isize);
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }


        let mut x;
        let mut y;
        let start_offset = (start_y * dimensions.0) + start_x;

        if start_offset + (string.len() as isize) > dimensions.0 * dimensions.1 {
            Err(RectError::StringTooLong)?;
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
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;

        Ok(())
    }


    /// Get the offset relative to the parent rectangle of the given rectangle.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_a = rectmanager.new_rect(TOP).ok().unwrap();
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
                let pos = parent.get_child_position(rect_id);
                Some((pos.0, pos.1))
            },
            None => {
               Some((0, 0))
            }
        }
    }

    /// Get the offset relative to the top-level rectangle in the RectManager.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect_a = rectmanager.new_rect(TOP).ok().unwrap();
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
        let mut output = None;

        let mut found = true;
        match self.get_rect(rect_id) {
            Some(_) => {}
            None => {
                found = false;
            }
        }

        if found {
            let mut x = 0;
            let mut y = 0;
            let mut working_id = rect_id;
            let mut pos;
            loop {
                match self.get_parent(working_id) {
                    Some(parent) => {
                        pos = parent.get_child_position(working_id);
                        x += pos.0;
                        y += pos.1;
                        working_id = parent.rect_id;
                    },
                    None => {
                        break;
                    }
                };
            }

            output = Some((x, y));
        }

        output
    }

    /// Get the character at the given position of a rectangle.
    /// The rectangle's default character (usually ' ') is returned if no character is found.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_character(TOP, 0, 0, 'X');
    /// assert_eq!(rectmanager.get_character(TOP, 0, 0).ok().unwrap(), 'X');
    /// rectmanager.kill();
    /// ```
    pub fn get_character(&self, rect_id: usize, x: isize, y: isize) -> Result<char, RectError> {
        match self.get_rect(rect_id) {
            Some(rect) => {
                rect.get_character(x, y)
            }
            None => {
                Err(RectError::NotFound(rect_id))
            }
        }
    }

    /// Set the character at the given position of a rectangle.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_character(TOP, 0, 0, 'X');
    /// assert_eq!(rectmanager.get_character(TOP, 0, 0).ok().unwrap(), 'X');
    /// rectmanager.kill();
    /// ```
    pub fn set_character(&mut self, rect_id: usize, x: isize, y: isize, character: char) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_character(x, y, character)?;
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_pos_refresh(rect_id, x, y)?;

        Ok(())
    }

    /// Delete a set character of a given rectangle at specified point
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_character(TOP, 0, 0, 'X');
    /// rectmanager.unset_character(TOP, 0, 0);
    /// assert_eq!(rectmanager.get_character(TOP, 0, 0).ok().unwrap(), rectmanager.get_default_character(TOP));
    /// rectmanager.kill();
    /// ```
    pub fn unset_character(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_character(x, y)?;
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;

        Ok(())
    }

    /// Completely erase a rectangle & remove it from the RectManager's tree.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect = rectmanager.new_rect(TOP).ok().unwrap();
    ///
    /// rectmanager.delete_rect(rect);
    /// assert!(!rectmanager.has_rect(rect));
    ///
    /// rectmanager.kill();
    /// ```
    pub fn delete_rect(&mut self, rect_id: usize) -> Result<(), RectError> {

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
                Err(RectError::NotFound(rect_id))?;
            }
        }

        for id in to_delete.iter() {
            self.rects.remove(&id);
        }

        Ok(())
    }

    /// Swap out one rectangle with another.
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// // Create a rectangle that is attached to TOP.
    /// let rect_a = rectmanager.new_rect(TOP).ok().unwrap();
    /// // Create an orphan rectangle to switch in.
    /// let rect_b = rectmanager.new_orphan().ok().unwrap();
    /// rectmanager.replace_with(rect_a, rect_b);
    ///
    /// assert!(rectmanager.has_parent(rect_b));
    /// assert!(!rectmanager.has_parent(rect_a));
    /// ```
    pub fn replace_with(&mut self, old_rect_id: usize, new_rect_id: usize) -> Result<(), RectError> {
        let mut parent_id = TOP;
        let mut old_position = (0, 0);
        match self.get_parent_mut(old_rect_id) {
            Some(parent) => {
                parent_id = parent.rect_id;
                old_position = *parent.child_positions.get(&old_rect_id).unwrap();
            }
            None => {
                Err(RectError::NotFound(old_rect_id))?;
            }
        }

        self.detach(old_rect_id)?;
        self.attach(new_rect_id, parent_id)?;
        self.set_position(new_rect_id, old_position.0, old_position.1)?;

        Ok(())
    }

    /// Get width of given rectangle
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect = rectmanager.new_rect(TOP).ok().unwrap();
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
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// let mut rect = rectmanager.new_rect(TOP).ok().unwrap();
    /// rectmanager.resize(rect, 10, 10);
    /// assert_eq!(rectmanager.get_rect_height(rect), 10);
    /// rectmanager.kill();
    /// ```
    pub fn get_rect_height(&self, rect_id: usize) -> usize {
        let (_, height) = self.get_rect_size(rect_id).unwrap();
        height
    }

    /// Gets the width of the RectManager
    pub fn get_width(&self) -> usize {
        let (width, _) = self.get_rect_size(TOP).unwrap();
        width
    }

    /// Gets the height of the RectManager
    pub fn get_height(&self) -> usize {
        let (_, height) = self.get_rect_size(TOP).unwrap();
        height
    }

    /// If the TOP rectangle dimensions to not match up to the console dimensions, then resize to fit.
    /// Returns true if a resize was made.
    pub fn auto_resize(&mut self) -> bool {
        let mut did_resize = false;
        let (current_width, current_height) = self.get_rect_size(TOP).unwrap();

        let (w, h) = get_terminal_size();
        if w as usize != current_width || h as usize != current_height {
            self.resize(TOP, w as usize, h as usize).expect("Unable to fit TOP rect to terminal");
            did_resize = true;
        }

        did_resize
    }

    /// Apply bold effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_bold_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Some Bold Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_bold_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_bold_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Disable bold text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_bold_flag(TOP);
    /// rectmanager.unset_bold_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_bold_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_bold_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Apply underline effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_underline_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Some Underlined Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_underline_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_underline_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Disable underline text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_underline_flag(TOP);
    /// rectmanager.unset_underline_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_underline_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_underline_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Invert the background and foreground colors of the text of the given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_invert_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Some Inverted Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_invert_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_invert_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Disable invert text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_invert_flag(TOP);
    /// rectmanager.unset_invert_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_invert_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_invert_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Apply italics effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_italics_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Some Italicized Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_italics_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_italics_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Disable italics text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_italics_flag(TOP);
    /// rectmanager.unset_italics_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_italics_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_italics_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Apply strike effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_strike_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Some Text With Strikethrough");
    /// rectmanager.kill();
    /// ```
    pub fn set_strike_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_strike_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Disable strike text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_strike_flag(TOP);
    /// rectmanager.unset_strike_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_strike_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_strike_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Apply blink effect to text of given rect (does not apply recursively).
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_blink_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Some Blinking Text");
    /// rectmanager.kill();
    /// ```
    pub fn set_blink_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_blink_flag();
            }
            None => ()
        }
        self.flag_refresh(rect_id)
    }

    /// Disable blink text effect
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP};
    /// let mut rectmanager = RectManager::new();
    /// rectmanager.set_blink_flag(TOP);
    /// rectmanager.unset_blink_flag(TOP);
    /// rectmanager.set_string(TOP, 0, 0, "Normal Text");
    /// rectmanager.kill();
    /// ```
    pub fn unset_blink_flag(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_blink_flag();
            }
            None => ()
        }

        self.flag_refresh(rect_id)?;

        Ok(())
    }

    /// Set color of background of given rect (does not apply recursively)
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP, RectColor};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a Green background
    /// rectmanager.set_fg_color(TOP, RectColor::GREEN);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn set_bg_color(&mut self, rect_id: usize, color: RectColor) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_bg_color(color);
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;

        Ok(())
    }

    /// Return background color to default
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP, RectColor};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a Magenta background
    /// rectmanager.set_bg_color(TOP, RectColor::MAGENTA);
    ///
    /// // Remove those colors...
    /// rectmanager.unset_bg_color(TOP);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn unset_bg_color(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_bg_color();
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;
        Ok(())
    }

    /// Set color of foreground (text) of given rect (does not apply recursively)
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP, RectColor};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a YELLOW foreground
    /// rectmanager.set_fg_color(TOP, RectColor::YELLOW);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn set_fg_color(&mut self, rect_id: usize, color: RectColor) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_fg_color(color);
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;
        Ok(())
    }

    /// Return foreground color to default
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP, RectColor};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a White foreground
    /// rectmanager.set_fg_color(TOP, RectColor::WHITE);
    ///
    /// // Remove those colors...
    /// rectmanager.unset_fg_color(TOP);
    ///
    /// rectmanager.kill();
    /// ```
    pub fn unset_fg_color(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_fg_color();
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;
        Ok(())
    }

    /// Return both background and foreground colors to default
    /// # Example
    /// ```
    /// use wrecked::{RectManager, TOP, RectColor};
    /// let mut rectmanager = RectManager::new();
    /// // Give Top a Blue background and a White foreground
    /// rectmanager.set_bg_color(TOP, RectColor::BLUE);
    /// rectmanager.set_fg_color(TOP, RectColor::WHITE);
    ///
    /// // Remove those colors...
    /// rectmanager.unset_color(TOP);
    ///
    /// rectmanager.kill();
    /// ```

    pub fn unset_color(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_color();
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self.flag_refresh(rect_id)?;
        Ok(())
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
    pub fn kill(&mut self) -> Result<(), RectError> {
        let mut last_error = Ok(());
        match self.clear_children(TOP) {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }
        match self.clear_characters(TOP) {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }
        match self.clear_effects(TOP) {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }
        match self.draw() {
            Ok(_) => {}
            Err(e) => { last_error = Err(e); }
        }

        // Even if it fails, we want to try clearing out all the rects
        // that are drawn, and reset the screen, to try to make failure
        // as easy to read as possible.
        #[cfg(not(debug_assertions))]
        match self._termios {
            Some(_termios) => {
                tcsetattr(0, TCSANOW, & _termios).unwrap();

                RectManager::__write("\x1B[?25h\x1B[?1049l"); // Return to previous screen
            }
            None => ()
        }

        last_error
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

    pub fn has_rect(&self, rect_id: usize) -> bool {
        self.rects.contains_key(&rect_id)
    }

    fn flag_refresh(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.flag_refresh();
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
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
        let mut parent_id = TOP;

        match self.get_rect(rect_id) {
            Some(rect) => {
                match rect.parent {
                    Some(pid) => {
                        has_parent = true;
                        parent_id = pid;
                    }
                    None => ()
                };
            }
            None => ()
        };


        if has_parent {
            self.get_rect(parent_id)
        } else {
            None
        }
    }

    fn get_parent_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        let mut has_parent = false;
        let mut parent_id = TOP;

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

    // Top can be the same as the given rect
    fn get_top(&self, rect_id: usize) -> Option<&Rect> {
        let mut current_id = rect_id;
        let mut rect_defined = false;

        loop {
            match self.get_rect(current_id) {
                Some(current_rect) => {
                    rect_defined = true;
                    match current_rect.parent {
                        Some(parent_id) => {
                            current_id = parent_id
                        }
                        None => {
                            break;
                        }
                    }
                }
                None => {
                    // Should only happen on first loop, indicating that the
                    // queried rect doesn't exist
                    break;
                }
            }
        }

        if rect_defined {
            self.get_rect(current_id)
        } else {
            None
        }
    }

    // Top can be the same as the given rect
    fn get_top_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        let mut current_id = rect_id;
        let mut rect_defined = false;

        loop {
            match self.get_rect_mut(current_id) {
                Some(current_rect) => {
                    rect_defined = true;
                    match current_rect.parent {
                        Some(parent_id) => {
                            current_id = parent_id
                        }
                        None => {
                            break;
                        }
                    }
                }
                None => {
                    // Should only happen on first loop, indicating that the
                    // queried rect doesn't exist
                    break;
                }
            }
        }

        if rect_defined {
            self.get_rect_mut(current_id)
        } else {
            None
        }
    }

    fn _update_cached_by_positions(&mut self, rect_id: usize, positions: &HashSet<(isize, isize)>) -> Result<(), RectError> {
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
        let mut max_cache_ranks: HashMap<(isize, isize), usize> = HashMap::new();
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

                            rect._cached_display.entry((x,y))
                                .and_modify(|e| {*e = (*tmp_chr, tmp_fx, 0)})
                                .or_insert((*tmp_chr, tmp_fx, 0));
                        } else {
                            rect._cached_display.remove(&(x, y));
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
                Err(RectError::NotFound(rect_id))?;
            }
        }

        for child_id in require_updates.iter() {
            self._update_cached_display(*child_id);
        }


        let mut new_values = Vec::new();
        let mut transparent_coords = HashSet::new();

        for ((x, y), child_ids) in pos_stack.iter() {
            for (child_id, rank) in child_ids.iter() {
                match child_positions.get(child_id) {
                    Some(child_position) => {
                        match self.get_rect_mut(*child_id) {
                            Some(child) => {

                                match child._cached_display.get(&(*x - child_position.0, *y - child_position.1)) {
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
                                Err(RectError::NotFound(*child_id))?;
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
                    let max_cache_rank = match max_cache_ranks.get(&(*x, *y)) {
                        Some(m) => *m,
                        None => 0
                    };

                    rect._cached_display.entry((*x, *y))
                        .and_modify(|e| {
                            if e.2 <= *rank || max_cache_rank <= e.2 {
                                *e = (new_value.0, new_value.1, *rank);
                            }
                        })
                        .or_insert((new_value.0, new_value.1, *rank));

                    transparent_coords.remove(&(*x, *y));
                }

                for coord in transparent_coords.iter() {
                    if rect.transparent {
                        rect._cached_display.remove(coord);
                    } else {
                        tmp_fx = rect.effects;

                        tmp_chr = rect.character_space.entry(*coord)
                            .or_insert(rect.default_character);

                        rect._cached_display.entry(*coord)
                            .and_modify(|e| {*e = (*tmp_chr, tmp_fx, 0)})
                            .or_insert((tmp_chr.clone(), tmp_fx.clone(), 0));
                    }
                }
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }
        Ok(())
    }

    fn _update_cached_display(&mut self, rect_id: usize) -> Result<(), RectError> {
        /*
           //TODO
            Since Children indicate to parents that a refresh is requested,
            if no flag is set, there is no need to delve down
        */
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
                        rect._cached_display.clear();

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
                            if pos.0 >= 0 && pos.1 >= 0 && pos.0 < rect.width as isize && pos.1 < rect.height as isize {
                                flags_pos_refresh.insert((pos.0 as isize, pos.1 as isize));
                            }
                        }
                    }
                    rect.flags_pos_refresh.clear();
                }
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        self._update_cached_by_positions(rect_id, &flags_pos_refresh)?;

        Ok(())
    }

    fn get_visible_box(&self, rect_id: usize) -> Result<(isize, isize, isize, isize), RectError> {
        let mut rect_box = (0, 0, 0, 0);

        match self.get_rect_size(rect_id) {
            Some(_dim) => {
                rect_box.2 = _dim.0 as isize;
                rect_box.3 = _dim.1 as isize;
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        };

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
                    Err(RectError::NotFound(working_id))?;
                }
            }
        }

        Ok(rect_box)
    }

    fn get_display(&mut self, rect_id: usize) -> Result<HashMap<(isize, isize), (char, RectEffectsHandler)>, RectError> {
        let mut outhash = HashMap::new();

        self._update_cached_display(rect_id)?;

        match self.get_rect(rect_id) {
            Some(rect) => {
                if rect.enabled {
                    for ((x, y), (new_c, effects, _)) in rect._cached_display.iter() {
                        outhash.insert((*x, *y), (*new_c, *effects));
                    }
                }
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        Ok(outhash)
    }

    fn build_ansi_string(&mut self, display_map: Vec<((isize, isize), (char, RectEffectsHandler))>) -> String {
        let mut renderstring = "".to_string();

        let mut val_a: &char;
        let mut active_effects = RectEffectsHandler::new();
        let mut new_effects;
        let mut current_col = -10;
        let mut current_row = -10;

        // THEN build then ANSI string
        for (pos, val) in display_map.iter() {
            if pos.1 != current_row || pos.0 != current_col {
                renderstring += &format!("\x1B[{};{}H", pos.1 + 1, pos.0 + 1);
                current_col = pos.0;
                current_row = pos.1;
            }

            val_a = &val.0;
            new_effects = val.1;

            if new_effects != active_effects {
                let mut tmp_color_n;
                let mut ansi_code_list: Vec<u8> = vec![];
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
                        ansi_code_list.push(21); // off
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

    fn filter_cached(&mut self, full_display_map: Vec<((isize, isize), (char, RectEffectsHandler))>) -> Vec<((isize, isize), (char, RectEffectsHandler))> {
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

    fn build_draw_map(&mut self, rect_id: usize) -> Vec<((isize, isize), (char, RectEffectsHandler))> {
        let mut to_draw = Vec::new();

        let mut offset = (0, 0);
        match self.get_absolute_offset(rect_id) {
            Some(_offset) => {
                offset = _offset;
            }
            None => ()
        };

        let mut boundry_box = (0, 0, 0, 0);
        match self.get_visible_box(rect_id) {
            Ok(_box) => {
                boundry_box = _box;
            }
            Err(_e) => { }
        };

        match self.get_display(rect_id) {
            Ok(display_map) => {
                for (pos, val) in display_map.iter() {
                    if offset.0 + pos.0 < boundry_box.0
                    || offset.0 + pos.0 >= boundry_box.0 + boundry_box.2
                    || offset.1 + pos.1 < boundry_box.1
                    || offset.1 + pos.1 >= boundry_box.1 + boundry_box.3 {
                        // pass
                    } else {
                        to_draw.push(((offset.0 + pos.0, offset.1 + pos.1), *val));
                    }
                }
            }
            Err(_e) => {
            }
        }

        to_draw
    }

    // Get n where n is the position in sibling array
    fn get_rank(&self, rect_id: usize) -> Result<usize, RectError> {
        let mut rank = 0;
        match self.get_parent(rect_id) {
            Some(parent) => {
                let mut _rank = 0;
                for i in parent.children.iter() {
                    if *i == rect_id {
                        rank = _rank;
                        break;
                    }
                    _rank += 1;
                }

                if _rank == parent.children.len() {
                    Err(RectError::ChildNotFound(parent.rect_id, rect_id))?;
                }

            }
            None => {
                Err(RectError::NotFound(rect_id))?;
            }
        }

        Ok(rank)
    }

    fn get_depth(&self, rect_id: usize) -> Option<usize> {
        let mut output = match self.get_rect(rect_id) {
            Some(_) => {
                Some(0)
            }
            None => {
                None
            }
        };

        if output.is_some() {
            let mut depth = 0;
            let mut working_id = rect_id;
            loop {
                match self.get_parent(working_id) {
                    Some(parent) => {
                        working_id = parent.rect_id;
                        depth += 1
                    }
                    None => {
                        break;
                    }
                }
            }

            output = Some(depth);
        }

        output
    }

    fn trace_lineage(&self, rect_id: usize) -> Vec<usize> {
        let mut lineage = Vec::new();
        let mut working_id = rect_id;
        loop {
            match self.get_parent(working_id) {
                Some(parent) => {
                    lineage.push(parent.rect_id);
                    working_id = parent.rect_id;
                }
                None => {
                    break;
                }
            }
        }

        lineage
    }


    // Flags the area of the parent of given rect covered by the given rect
    fn flag_parent_refresh(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut dimensions = (0, 0);
        match self.get_rect_size(rect_id) {
            Some(_dim) => {
                dimensions = _dim;
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
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
            };

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
            };
        }

        Ok(())
    }

    fn flag_pos_refresh(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.flags_pos_refresh.insert((x, y));
            }
            None => {
                Err(RectError::NotFound(rect_id))?;
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

    fn update_child_space(&mut self, child_id: usize) -> Result<(), RectError> {
        let mut dimensions = (0, 0);
        let mut position = (0, 0);

        match self.get_rect_size(child_id) {
            Some(_dim) => {
                dimensions = (_dim.0 as isize, _dim.1 as isize);
            }
            None => {
                Err(RectError::NotFound(child_id))?;
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

    fn clear_child_space(&mut self, child_id: usize) -> Result<(), RectError> {
        self.flag_parent_refresh(child_id)?;

        match self.get_parent_mut(child_id) {
            Some(parent) => {
                parent.clear_child_space(child_id);
            }
            None => ()
        }

        Ok(())
    }

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

    pub fn set_transparency(&mut self, rect_id: usize, transparent: bool) -> Result<(), RectError> {
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.transparent = transparent;
                Ok(())
            }
            None => {
                Err(RectError::NotFound(rect_id))
            }
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

    effects: RectEffectsHandler,

    _cached_display: HashMap<(isize, isize), (char, RectEffectsHandler, usize)>,
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

            effects: RectEffectsHandler::new(),

            _cached_display: HashMap::new(),
            default_character: ' ' // Space
        }
    }

    fn get_rect_id(&self) -> usize {
        self.rect_id
    }

    fn disable(&mut self) {
        self.enabled = false;
    }

    fn enable(&mut self) {
        self.enabled = true;
    }

    fn flag_refresh(&mut self) {
        self.flag_full_refresh = true;
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

    fn get_child_position(&self, child_id: usize) -> (isize, isize) {
        let x;
        let y;

        match self.child_positions.get(&child_id) {
            Some(position) => {
                x = position.0;
                y = position.1;
            }
            // TODO: Throw Error
            None => {
                x = 0;
                y = 0;
            }
        }

        (x, y)
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
        // TODO: Implement Copy for Vec<(isize, isize)> ?
        let mut new_positions = Vec::new();
        match self._inverse_child_space.get(&rect_id) {
            Some(positions) => {
                for position in positions.iter() {
                    new_positions.push((position.0, position.1));
                }
            }
            None => ()
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

    fn get_character(&self, x: isize, y: isize) -> Result<char, RectError> {
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
            Err(RectError::BadPosition(x, y))
        }
    }

    fn set_character(&mut self, x: isize, y: isize, character: char) -> Result<(), RectError> {
        if y < self.height as isize && y >= 0 && x < self.width as isize && x >= 0 {
            self.character_space.entry((x, y))
                .and_modify(|coord| { *coord = character })
                .or_insert(character);
            self.flags_pos_refresh.insert((x, y));
        } else {
            Err(RectError::BadPosition(x, y))?;
        }

        Ok(())
    }

    fn unset_character(&mut self, x: isize, y: isize) -> Result<(), RectError> {
        self.set_character(x, y, self.default_character)
    }

    pub fn is_plain(&self) -> bool {
        self.effects.is_empty()
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

    fn set_bold_flag(&mut self) {
        if ! self.effects.bold {
            self.flag_full_refresh = true;
        }

        self.effects.bold = true;
    }

    fn unset_bold_flag(&mut self) {
        if self.effects.bold {
            self.flag_full_refresh = true;
        }

        self.effects.bold = false;
    }
    fn set_underline_flag(&mut self) {
        if ! self.effects.underline {
            self.flag_full_refresh = true;
        }

        self.effects.underline = true;
    }

    fn unset_underline_flag(&mut self) {
        if self.effects.underline {
            self.flag_full_refresh = true;
        }

        self.effects.underline = false;
    }

    fn set_invert_flag(&mut self) {
        if ! self.effects.invert {
            self.flag_full_refresh = true;
        }

        self.effects.invert = true;
    }

    fn unset_invert_flag(&mut self) {
        if self.effects.invert {
            self.flag_full_refresh = true;
        }

        self.effects.invert = false;
    }

    fn set_italics_flag(&mut self) {
        if ! self.effects.italics {
            self.flag_full_refresh = true;
        }

        self.effects.italics = true;
    }

    fn unset_italics_flag(&mut self) {
        if self.effects.italics {
            self.flag_full_refresh = true;
        }

        self.effects.italics = false;
    }

    fn set_strike_flag(&mut self) {
        if ! self.effects.strike {
            self.flag_full_refresh = true;
        }

        self.effects.strike = true;
    }

    fn unset_strike_flag(&mut self) {
        if self.effects.strike {
            self.flag_full_refresh = true;
        }

        self.effects.strike = false;
    }

    fn set_blink_flag(&mut self) {
        if ! self.effects.blink {
            self.flag_full_refresh = true;
        }

        self.effects.blink = true;
    }

    fn unset_blink_flag(&mut self) {
        if self.effects.blink {
            self.flag_full_refresh = true;
        }

        self.effects.blink = false;
    }

    fn unset_bg_color(&mut self) {
        if self.effects.background_color.is_some() {
            self.flag_full_refresh = true;
        }

        self.effects.background_color = None;
    }

    fn unset_fg_color(&mut self) {
        if self.effects.foreground_color.is_some() {
            self.flag_full_refresh = true;
        }

        self.effects.foreground_color = None;
    }

    fn unset_color(&mut self) {
        self.unset_bg_color();
        self.unset_fg_color();
    }

    fn set_bg_color(&mut self, color: RectColor) {
        if self.effects.background_color != Some(color) {
            self.flag_full_refresh = true;
        }

        self.effects.background_color = Some(color);
    }

    fn set_fg_color(&mut self, color: RectColor) {
        if self.effects.foreground_color != Some(color) {
            self.flag_full_refresh = true;
        }

        self.effects.foreground_color = Some(color);
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

    pub fn has_child(&self, child_id: usize) -> bool {
        let mut output = false;
        for connected_child_id in self.children.iter() {
            if *connected_child_id == child_id {
                output = true;
                break;
            }
        }
        output
    }

    pub fn clear_characters(&mut self) {
        self.character_space.clear();
        self._cached_display.clear();
    }

    pub fn get_fg_color(&self) -> Option<RectColor> {
        self.effects.foreground_color
    }
    pub fn get_bg_color(&self) -> Option<RectColor> {
        self.effects.background_color
    }
}
