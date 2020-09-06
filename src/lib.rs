use terminal_size::{Width, Height, terminal_size};
use std::collections::HashMap;
use std::collections::HashSet;
use std::str;
use std::cmp;
use std::cmp::{PartialOrd,Ordering};
use std::fs::File;
use std::io::{Write};
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::ops::{BitOrAssign, BitAnd, Not};
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
use std::fmt;

pub mod tests;

/*
    TODO
    Figure out why i made height/width of rect isize, change to usize or uN if not a good reason
*/

pub fn logg(msg: String) {
    let path = "rlogg";

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)
        .unwrap();

    writeln!(file, "{}",  msg);
}

#[derive(PartialEq, Eq)]
pub enum RectError {
    AllGood = 0,
    BadPosition = 1,
    NotFound = 2,
    ParentNotFound = 3, // rect has an associated parent id that does not exist in RectManager
    NoParent = 4, // Rect has no parent id
    BadColor = 5,
    InvalidUtf8 = 6,
    ChildNotFound = 7
}
impl fmt::Debug for RectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            RectError::AllGood => { "Ok" }
            RectError::BadPosition => { "Position out of bounds" }
            RectError::NotFound => { "No Rect with given id" }
            RectError::ParentNotFound => { "The parent associated with this Rect doesn't exist" }
            RectError::NoParent => { "Rect doesn't have a parent" }
            RectError::BadColor => { "Not a valid RectColor" }
            RectError::InvalidUtf8 => { "Invalid utf8" }
            RectError::ChildNotFound => { "Child associated with this rect doesn't exist" }
            _ => { "???" }
        };

        write!(f, "{}", name)
    }
}


#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
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
    BRIGHTWHITE = 8 | 7,
    NONE = 255
}
impl fmt::Debug for RectColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            RectColor::BLACK => { "Black" }
            RectColor::RED => { "Red" }
            RectColor::GREEN => { "Green" }
            RectColor::YELLOW => { "Yellow" }
            RectColor::BLUE => { "Blue" }
            RectColor::MAGENTA => { "Magenta" }
            RectColor::CYAN => { "Cyan" }
            RectColor::WHITE => { "White" }
            RectColor::BRIGHTBLACK => { "BrightBlack" }
            RectColor::BRIGHTRED => { "BrightRed" }
            RectColor::BRIGHTGREEN => { "BrightGreen" }
            RectColor::BRIGHTYELLOW => { "BrightYellow" }
            RectColor::BRIGHTBLUE => { "BrightBlue" }
            RectColor::BRIGHTMAGENTA => { "BrightMagenta" }
            RectColor::BRIGHTCYAN => { "BrightCyan" }
            RectColor::BRIGHTWHITE => { "BrightWhite" }
            RectColor::NONE => { "None" }
            _ => { "Invalid Color" }
        };

        write!(f, "{}", name)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct RectEffectsHandler {
    bold: bool,
    underline: bool,
    invert: bool,
    italics: bool,
    strike: bool,
    background_color: RectColor,
    foreground_color: RectColor
}
impl fmt::Debug for RectEffectsHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RectEffectsHandler")
         .field("bold", &self.bold)
         .field("underline", &self.underline)
         .field("invert", &self.invert)
         .field("strike", &self.strike)
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
            background_color: RectColor::NONE,
            foreground_color: RectColor::NONE
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.bold
        && !self.underline
        && !self.invert
        && !self.italics
        && !self.strike
        && self.background_color != RectColor::NONE
        && self.foreground_color != RectColor::NONE
    }
}
pub struct RectManager {
    idgen: usize,
    rects: HashMap<usize, Rect>,
    draw_queue: Vec<usize>,
    // top_cache is used to prevent redrawing the same
    // characters at the same coordinate.
    top_cache: HashMap<(isize, isize), (char, RectEffectsHandler)>,
    _termios: Termios,
    default_character: char
}

pub struct Rect {
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

    character_space: HashMap<(isize,isize), char>,

    flag_full_refresh: bool,
    flags_pos_refresh: HashSet<(isize, isize)>,

    enabled: bool,

    effects: RectEffectsHandler,

    _cached_display: HashMap<(isize, isize), (char, RectEffectsHandler)>,
}

impl Rect {
    pub fn new(rect_id: usize) -> Rect {
        Rect {
            rect_id: rect_id,
            parent: None,
            width: 0,
            height: 0,
            children: Vec::new(),
            child_space: HashMap::new(),
            _inverse_child_space: HashMap::new(),
            child_positions: HashMap::new(),
            character_space: HashMap::new(),
            flag_full_refresh: true,
            flags_pos_refresh: HashSet::new(),
            enabled: true,

            effects: RectEffectsHandler::new(),

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

    fn update_child_space(&mut self, rect_id: usize, corners: (isize, isize, isize, isize)) {
        self.clear_child_space(rect_id);

        for y in corners.1 .. corners.3 {
            for x in corners.0 .. corners.2 {
                if x >= 0 && x < self.width as isize && y >= 0 && y < self.height as isize {
                    self.child_space.entry((x, y))
                        .or_insert(Vec::new())
                        .push(rect_id);

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
        let output;
        if y < self.height as isize && y >= 0 && x < self.width as isize && x >= 0 {
            output = match self.character_space.get(&(x, y)) {
                Some(character) => {
                    Ok(character.clone())
                }
                None => {
                    Ok(self.default_character)
                }
            };
        } else {
            output = Err(RectError::BadPosition);
        }

        output
    }

    fn set_character(&mut self, x: isize, y: isize, character: char) -> Result<(), RectError> {
        let output;
        if y < self.height as isize && y >= 0 && x < self.width as isize && x >= 0 {
            self.character_space.entry((x, y))
                .and_modify(|coord| { *coord = character })
                .or_insert(character);
            self.flags_pos_refresh.insert((x, y));
            output = Ok(());
        } else {
            output = Err(RectError::BadPosition);
        }

        output
    }

    fn unset_character(&mut self, x: isize, y: isize) -> Result<(), RectError> {
        self.set_character(x, y, self.default_character)
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

    fn unset_bg_color(&mut self) {
        self.set_bg_color(RectColor::NONE);
    }

    fn unset_fg_color(&mut self) {
        self.set_fg_color(RectColor::NONE);
    }

    fn unset_color(&mut self) {
        self.unset_bg_color();
        self.unset_fg_color();
    }

    fn set_bg_color(&mut self, color: RectColor) {
        if self.effects.background_color != color {
            self.flag_full_refresh = true;
        }

        self.effects.background_color = color;
    }

    fn set_fg_color(&mut self, color: RectColor) {
        if self.effects.foreground_color != color {
            self.flag_full_refresh = true;
        }

        self.effects.foreground_color = color;
    }

    fn add_child(&mut self, child_id: usize) {
        self.children.push(child_id);
        self._inverse_child_space.insert(child_id, Vec::new());
        self.set_child_position(child_id, 0, 0);
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

    fn clear(&mut self) {
        self.character_space.clear();
    }

    fn get_fg_color(&self) -> RectColor {
        self.effects.foreground_color
    }
    fn get_bg_color(&self) -> RectColor {
        self.effects.background_color
    }
}

impl RectManager {
    pub fn new() -> RectManager {
        let termios = Termios::from_fd(0).unwrap();

        let mut new_termios = termios.clone();

        let mut rectmanager = RectManager {
            idgen: 0,
            rects: HashMap::new(),
            draw_queue: Vec::new(),
            top_cache: HashMap::new(),
            _termios: termios,
            default_character: ' '
        };

        new_termios.c_lflag &= !(ICANON | ECHO);
        tcsetattr(0, TCSANOW, &mut new_termios).unwrap();

        print!("\x1B[?25l"); // Hide Cursor
        println!("\x1B[?1049h"); // New screen


        rectmanager.new_rect(None);
        rectmanager.auto_resize();


        rectmanager
    }

    pub fn new_rect(&mut self, parent_id: Option<usize>) -> usize {
        let new_id = self.idgen;
        self.idgen += 1;

        self.rects.entry(new_id).or_insert(Rect::new(new_id));

        match parent_id {
            Some(unpacked) => {
                self.attach(new_id, unpacked);
            }
            None => ()
        };
        self.flag_refresh(new_id);


        new_id
    }

    pub fn get_rect(&self, rect_id: usize) -> Result<&Rect, RectError> {
        match self.rects.get(&rect_id) {
            Some(rect) => {
                Ok(rect)
            }
            None => {
                Err(RectError::NotFound)
            }
        }
    }

    pub fn get_rect_mut(&mut self, rect_id: usize) -> Result<&mut Rect, RectError> {
        match self.rects.get_mut(&rect_id) {
            Some(rect) => {
                Ok(rect)
            }
            None => {
                Err(RectError::NotFound)
            }
        }
    }

    pub fn get_parent(&self, rect_id: usize) -> Result<&Rect, RectError> {
        let mut output = Err(RectError::NotFound);
        let mut has_parent = false;
        let mut parent_id = 0;

        match self.get_rect(rect_id) {
            Ok(rect) => {
                match rect.parent {
                    Some(pid) => {
                        has_parent = true;
                        parent_id = pid;
                    }
                    None => {
                        output = Err(RectError::NoParent);
                    }
                };
            },
            Err(e) => {
                output = Err(e);
            }
        };


        if has_parent {
            match self.get_rect(parent_id) {
                Ok(parent) => {
                    output = Ok(parent);
                },
                Err(e) => {
                    output = Err(RectError::ParentNotFound);
                }
            }
        }

        output
    }

    pub fn get_parent_mut(&mut self, rect_id: usize) -> Result<&mut Rect, RectError> {
        let mut output = Err(RectError::NotFound);
        let mut has_parent = false;
        let mut parent_id = 0;

        match self.get_rect(rect_id) {
            Ok(rect) => {
                match rect.parent {
                    Some(pid) => {
                        has_parent = true;
                        parent_id = pid;
                    }
                    None => {
                        output = Err(RectError::NoParent);
                    }
                };
            },
            Err(e) => {
                output = Err(e);
            }
        };


        if has_parent {
            match self.get_rect_mut(parent_id) {
                Ok(parent) => {
                    output = Ok(parent);
                },
                Err(e) => {
                    output = Err(RectError::ParentNotFound);
                }
            }
        }

        output
    }

    // Top can be the same as the given rect
    pub fn get_top(&self, rect_id: usize) -> Result<&Rect, RectError> {
        let mut current_id = rect_id;
        let mut output = Err(RectError::NotFound);
        let mut rect_defined = false;

        loop {
            match self.get_rect(current_id) {
                Ok(current_rect) => {
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
                Err(e) => {
                    // Should only happen on first loop, indicating that the
                    // queried rect doesn't exist
                    output = Err(e);
                    break;
                }
            }
        }

        if rect_defined {
            output = self.get_rect(current_id);
        }

        output
    }

    // Top can be the same as the given rect
    pub fn get_top_mut(&mut self, rect_id: usize) -> Result<&mut Rect, RectError> {
        let mut current_id = rect_id;
        let mut output = Err(RectError::NotFound);
        let mut rect_defined = false;

        loop {
            match self.get_rect_mut(current_id) {
                Ok(current_rect) => {
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
                Err(e) => {
                    // Should only happen on first loop, indicating that the
                    // queried rect doesn't exist
                    output = Err(e);
                    break;
                }
            }
        }

        if rect_defined {
            output = self.get_rect_mut(current_id);
        }

        output
    }

    fn _update_cached_by_positions(&mut self, rect_id: usize, positions: &HashSet<(isize, isize)>) -> Result<(), RectError> {
        // TODO: Double Check the logic in this function. I may have biffed it when refactoring
        /*
            child_recache items are:
                child_id,
                Vector of positions,
                has parent?
                offset (if has parent)
        */
        let mut child_recache: HashMap<usize, (Vec<(isize, isize)>, bool, (isize, isize))> = HashMap::new();
        let mut x: isize;
        let mut y: isize;
        let mut tmp_chr;
        let mut tmp_fx;
        let mut new_values = Vec::new();

        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                for (_x, _y) in positions.iter() {
                    x = *_x;
                    y = *_y;

                    if !rect.child_space.contains_key(&(x, y)) || rect.child_space[&(x, y)].is_empty() {
                        // Make sure at least default character is present
                        tmp_fx = rect.effects;

                        tmp_chr = rect.character_space.entry((x, y))
                            .or_insert(rect.default_character);

                        rect._cached_display.entry((x,y))
                            .and_modify(|e| {*e = (*tmp_chr, tmp_fx)})
                            .or_insert((*tmp_chr, tmp_fx));

                    } else {
                        match rect.child_space.get(&(x, y)) {
                            Some(child_ids) => {
                                match child_ids.last() {
                                    Some(child_id) => {
                                        child_recache.entry(*child_id)
                                            .or_insert((Vec::new(), false, (0, 0)));
                                    }
                                    None => ()
                                }
                            }
                            None => ()
                        }

                        for (child_id, value) in child_recache.iter_mut() {
                            match rect.child_positions.get_mut(&child_id) {
                                Some(pos) => {
                                    value.1 = true;
                                    value.2 = *pos;
                                }
                                None => ()
                            }
                            value.0.push((x, y));
                        }
                    }
                }
            }
            Err(e) => {
                output = Err(e);
            }
        }

        if output.is_ok() {
            for (child_id, (coords, child_has_position, child_position)) in child_recache.iter_mut() {
                if *child_has_position {
                    if output.is_ok() {
                        output = self._update_cached_display(*child_id);
                    }

                    if output.is_ok() {
                        match self.get_rect_mut(*child_id) {
                            Ok(child) => {
                                for (x, y) in coords.iter() {
                                    match child._cached_display.get(&(*x - child_position.0, *y - child_position.1)) {
                                        Some(new_value) => {
                                            new_values.push((*new_value, *x, *y));
                                        }
                                        None => ()
                                    };
                                }
                            }
                            Err(e) => {
                                output = Err(e);
                                break;
                            }
                        }
                    }
                }
            }
        }


        if output.is_ok() {
            match self.get_rect_mut(rect_id) {
                Ok(rect) => {
                    for (new_value, x, y) in new_values.iter() {
                        rect._cached_display.entry((*x, *y))
                            .and_modify(|e| { *e = *new_value })
                            .or_insert(*new_value);
                    }
                }
                Err(e) => {
                    output = Err(e);
                }
            };
        }

        output
    }

    fn _update_cached_display(&mut self, rect_id: usize) -> Result<(), RectError> {
        /*
           //TODO
            Since Children indicate to parents that a refresh is requested,
            if no flag is set, there is no need to delve down
        */
        let mut flags_pos_refresh = HashSet::new();
        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                if rect.enabled {

                    /*
                      If a full refresh is requested,
                      fill flags_pos_refresh with all potential coords
                    */
                    if rect.flag_full_refresh {
                        rect.flag_full_refresh = false;

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
                            flags_pos_refresh.insert((pos.0 as isize, pos.1 as isize));
                        }
                    }
                    rect.flags_pos_refresh.clear();
                }
            }
            Err(e) => {
                output = Err(e);
            }
        }

        if output.is_ok() {
            output = self._update_cached_by_positions(rect_id, &flags_pos_refresh);
        }

        output
    }

    pub fn get_visible_box(&self, rect_id: usize) -> Result<(isize, isize, isize, isize), RectError> {
        let mut output = Ok((0, 0, 0, 0));
        let mut rect_box = (0, 0, 0, 0);

        match self.get_rect_size(rect_id) {
            Ok(_dim) => {
                rect_box.2 = _dim.0 as isize;
                rect_box.3 = _dim.1 as isize;
            }
            Err(e) => {
                output = Err(e);
            }
        };

        if output.is_ok() {
            match self.get_absolute_offset(rect_id) {
                Ok(offset) => {
                    rect_box.0 = offset.0;
                    rect_box.1 = offset.1;
                }
                Err(e) => {
                    output = Err(e);
                }
            }

            let mut working_id = rect_id;
            let mut parent_dim = (0, 0);
            loop {
                match self.get_parent(working_id) {
                    Ok(parent) => {
                        parent_dim = (parent.width, parent.height);
                        working_id = parent.rect_id;

                    }
                    Err(error) => {
                        if error != RectError::NoParent {
                            output = Err(error);
                        }
                        break;
                    }
                }

                if output.is_ok() {
                    match self.get_absolute_offset(working_id) {
                        Ok(offset) => {
                            rect_box.0 = cmp::max(rect_box.0, offset.0);
                            rect_box.1 = cmp::max(rect_box.1, offset.1);
                            rect_box.2 = cmp::min((offset.0 + parent_dim.0 as isize) - rect_box.0, rect_box.2);
                            rect_box.3 = cmp::min((offset.1 + parent_dim.1 as isize) - rect_box.1, rect_box.3);
                        }
                        Err(e) => {
                            output = Err(e);
                        }
                    }
                }
            }
            if output.is_ok() {
                output = Ok(rect_box);
            }
        }

        output
    }

    fn get_display(&mut self, rect_id: usize) -> Result<HashMap<(isize, isize), (char, RectEffectsHandler)>, RectError> {
        let mut output = Ok(HashMap::new());
        let mut outhash = HashMap::new();

        match self._update_cached_display(rect_id) {
            Ok(_) => {}
            Err(e) => {
                output = Err(e);
            }
        }

        if output.is_ok() {
            match self.get_rect(rect_id) {
                Ok(rect) => {
                    if rect.enabled {
                        for ((x, y), (new_c, effects)) in rect._cached_display.iter() {
                            outhash.insert((*x, *y), (*new_c, *effects));
                        }
                    }
                }
                Err(e) => {
                    output = Err(e);
                }
            }
        }

        if output.is_ok() {
            output = Ok(outhash);
        }

        output
    }

    fn build_ansi_string(&mut self, display_map: Vec<((isize, isize), (char, RectEffectsHandler))>) -> String {
        let mut renderstring = "".to_string();
        let mut width = self.get_width();

        let mut val_a: &char;
        let mut utf_char: &[u8];
        let mut active_effects = RectEffectsHandler::new();
        let mut new_effects;
        let mut current_col = -10;
        let mut current_row = -10;

        // THEN build then ANSI string
        for (pos, val) in display_map.iter() {
            renderstring += &format!("\x1B[{};{}H", pos.1 + 1, pos.0 + 1);
            if pos.1 != current_row || pos.0 != current_col {
                current_col = pos.0;
                current_row = pos.1;
            }

            val_a = &val.0;
            new_effects = val.1;

            if new_effects.is_empty() && ! active_effects.is_empty() {
                renderstring += "\x1B[0m";
            } else if !new_effects.is_empty() {
                let mut tmp_color_n;
                // ForeGround
                if new_effects.foreground_color != active_effects.foreground_color {
                    if new_effects.foreground_color != RectColor::NONE {
                        tmp_color_n = new_effects.foreground_color as u8; 
                        if tmp_color_n & 8 == 8 {
                            renderstring += &format!("\x1B[9{}m", tmp_color_n & 7);
                        } else {
                            renderstring += &format!("\x1B[3{}m", tmp_color_n & 7);
                        }
                    } else {
                        renderstring += &format!("\x1B[39m");
                    }
                }

                // BackGround
                if new_effects.background_color != active_effects.background_color {
                    if new_effects.background_color != RectColor::NONE {
                        tmp_color_n = new_effects.background_color as u8;
                        if tmp_color_n & 8 == 8 {
                            renderstring += &format!("\x1B[10{}m", (tmp_color_n & 7));
                        } else {
                            renderstring += &format!("\x1B[4{}m", (tmp_color_n & 7));
                        }
                    } else {
                        renderstring += &format!("\x1B[49m");
                    }
                }

                // Bold
                if new_effects.bold != active_effects.bold {
                    if new_effects.bold {
                        renderstring += &format!("\x1B[1m"); // On
                    } else {
                        renderstring += &format!("\x1B[21m"); // Off
                    }
                }

                // Underline
                if new_effects.underline != active_effects.underline {
                    if new_effects.underline {
                        renderstring += &format!("\x1B[4m");
                    } else {
                        renderstring += &format!("\x1B[24m"); // Off
                    }
                }

                // Inverted
                if new_effects.invert != active_effects.invert {
                    if new_effects.invert {
                        renderstring += &format!("\x1B[7m");
                    } else {
                        renderstring += &format!("\x1B[27m"); // Off
                    }
                }

                // Italics
                if new_effects.italics != active_effects.italics {
                    if new_effects.italics {
                        renderstring += &format!("\x1B[3m");
                    } else {
                        renderstring += &format!("\x1B[23m"); // Off
                    }
                }

                // Strike
                if new_effects.strike != active_effects.strike {
                    if new_effects.strike {
                        renderstring += &format!("\x1B[9m");
                    } else {
                        renderstring += &format!("\x1B[29m"); // Off
                    }
                }
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
                    if (*char_pair != *val) {
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

    pub fn build_draw_map(&mut self, rect_id: usize) -> Vec<((isize, isize), (char, RectEffectsHandler))> {
        let mut to_draw = Vec::new();

        let mut offset = (0, 0);
        match self.get_absolute_offset(rect_id) {
            Ok(_offset) => {
                offset = _offset;
            }
            Err(e)=> {
            }
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
            Err(e) => {
            }
        }

        to_draw
    }

    pub fn draw(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut draw_map = self.build_draw_map(rect_id);

        let mut filtered_map = self.filter_cached(draw_map);

        if (filtered_map.len() > 0) {
            // Doesn't need to be sorted to work, but there're fewer ansi sequences if it is.
            filtered_map.sort();

            let renderstring = self.build_ansi_string(filtered_map);
            print!("{}\x1B[0m", renderstring);
            println!("\x1B[1;1H");
        }

        Ok(())
    }

    // Get n where n is the position in sibling array
    fn get_rank(&self, rect_id: usize) -> Result<usize, RectError> {
        let mut output = Ok(0);
        let mut rank = 0;
        match self.get_parent(rect_id) {
            Ok(parent) => {
                let mut _rank = 0;
                for i in parent.children.iter() {
                    if *i == rect_id {
                        rank = _rank;
                        break;
                    }
                    _rank += 1;
                }

                if _rank == parent.children.len() {
                    output = Err(RectError::ChildNotFound);
                }

            }
            Err(error) => {
                if error != RectError::NoParent {
                    output = Err(error);
                }
            }
        }

        if output.is_ok() {
            output = Ok(rank);
        }

        output
    }

    fn get_depth(&self, rect_id: usize) -> Result<usize, RectError> {
        let mut output = Ok(0);
        let mut depth = 0;
        let mut working_id = rect_id;
        loop {
            match self.get_parent(working_id) {
                Ok(parent) => {
                    working_id = parent.rect_id;
                    depth += 1
                }
                Err(error) => {
                    if error != RectError::NoParent {
                        output = Err(error);
                    }
                    break;
                }
            }
        }

        if output.is_ok() {
            output = Ok(depth);
        }

        output
    }

    fn trace_lineage(&self, rect_id: usize) -> Vec<usize> {
        let mut lineage = Vec::new();
        let mut working_id = rect_id;
        loop {
            match self.get_parent(working_id) {
                Ok(parent) => {
                    lineage.push(parent.rect_id);
                    working_id = parent.rect_id;
                }
                Err(error) => {
                    break;
                }
            }
        }

        lineage
    }

    pub fn draw_queued(&mut self) -> Result<(), RectError> {
        let mut output = Ok(());


        let mut to_draw = Vec::new();
        let mut depth_tracker: HashMap<(isize, isize), usize> = HashMap::new();

        let mut offset = (0, 0);

        let mut draw_queue = Vec::new();
        let mut done_ = Vec::new();
        for rect_id in self.draw_queue.iter() {
            if ! done_.contains(rect_id) {
                draw_queue.push((0, 0, *rect_id));
                done_.push(*rect_id);
            }
        }

        self.draw_queue.clear();

        let mut dimensions = (0, 0);
        match self.get_rect(0) {
            Ok(top) => {
                dimensions = (top.width, top.height);
            }
            Err(e) => {
                output = Err(e);
            }
        };

        let mut skip_rect;
        let mut is_attached;
        for (depth, rank, rect_id) in draw_queue.iter_mut() {
            skip_rect = false;
            is_attached = false;
            for ancestor_id in self.trace_lineage(*rect_id).iter() {
                if done_.contains(ancestor_id) {
                    skip_rect = true;
                    break;
                }
                if *ancestor_id == 0 {
                    is_attached = true;
                }
            }
            if ! is_attached {
                skip_rect = true;
            }
            if skip_rect {
                continue;
            }

            match self.get_depth(*rect_id) {
                Ok(real_depth) => {
                    *depth = real_depth;
                }
                Err(error) => {
                    output = Err(error);
                    break;
                }
            }
            match self.get_rank(*rect_id) {
                Ok(real_rank) => {
                    *rank = real_rank;
                }
                Err(error) => {
                    output = Err(error);
                    break;
                }
            }
        }

        if output.is_ok() && draw_queue.len() > 0 {

            draw_queue.sort();
            draw_queue.reverse();

            let mut boundry_box = (0, 0, 0, 0);
            for (depth, _rank, rect_id) in draw_queue {
                match self.get_absolute_offset(rect_id) {
                    Ok(_offset) => {
                        offset = _offset;
                    }
                    Err(e)=> {
                        output = Err(e);
                    }
                };

                if output.is_ok() {

                    match self.get_visible_box(rect_id) {
                        Ok(_box) => {
                            boundry_box = _box;
                        }
                        Err(e) => { }
                    };

                    match self.get_display(rect_id) {
                        Ok(display_map) => {
                            for (pos, val) in display_map.iter() {
                                if ! depth_tracker.contains_key(pos) || *depth_tracker.get(pos).unwrap() <= depth {
                                    if offset.0 + pos.0 < boundry_box.0
                                    || offset.0 + pos.0 >= boundry_box.0 + boundry_box.2
                                    || offset.1 + pos.1 < boundry_box.1
                                    || offset.1 + pos.1 >= boundry_box.1 + boundry_box.3 {
                                        // pass
                                    } else {
                                        to_draw.push(((offset.0 + pos.0, offset.1 + pos.1), *val));
                                        depth_tracker.entry(*pos)
                                            .and_modify(|e| { *e = depth })
                                            .or_insert(depth);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            output = Err(e);
                            break;
                        }
                    }

                    self.flag_parent_refresh(rect_id);


                } else {
                    break;
                }
            }

            self._draw(&mut to_draw);
        }

        output
    }

    pub fn get_rect_size(&self, rect_id: usize) -> Result<(usize, usize), RectError> {
        let output;

        match self.get_rect(rect_id) {
            Ok(rect) => {
                output = Ok((rect.width, rect.height));
            }
            Err(e) => {
                output = Err(e);
            }
        }

        output
    }

    fn get_relative_offset(&self, rect_id: usize) -> Result<(isize, isize), RectError> {
        let mut x = 0;
        let mut y = 0;
        let mut output = Ok((0, 0));
        let pos;


        match self.get_parent(rect_id) {
            Ok(parent) => {
                pos = parent.get_child_position(rect_id);
                x += pos.0;
                y += pos.1;
            },
            Err(error) => {
                output = Err(error);
            }
        };

        if output.is_ok() {
           output = Ok((x, y));
        }

        output
    }

    fn get_absolute_offset(&self, rect_id: usize) -> Result<(isize, isize), RectError> {
        let mut x = 0;
        let mut y = 0;
        let mut working_id = rect_id;
        let mut pos;
        let mut output = Ok((0, 0));


        loop {
            match self.get_parent(working_id) {
                Ok(parent) => {
                    pos = parent.get_child_position(working_id);
                    x += pos.0;
                    y += pos.1;
                    working_id = parent.rect_id;
                },
                Err(error) => {
                    if error != RectError::NoParent {
                        output = Err(error);
                    }
                    break;
                }
            };
        }

        if output.is_ok() {
           output = Ok((x, y));
        }

        output
    }

    pub fn resize(&mut self, rect_id: usize, width: usize, height: usize) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut pos = (0, 0);

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.resize(width, height);
            },
            Err(e) => {
                output = Err(e);
            }
        };


        if output.is_ok() {
            match self.get_parent_mut(rect_id) {
                Ok(parent) => {
                    pos = parent.get_child_position(rect_id);
                }
                Err(error) => {
                    if error != RectError::NoParent {
                        output = Err(error);
                    }
                }
            };
        }

        if output.is_ok() {
            output = self.set_position(rect_id, pos.0, pos.1);
            self.flag_refresh(rect_id);
        }

        if output.is_err() {
            logg("Resize fail".to_string());
        }

        output
    }

    pub fn shift_contents(&mut self, rect_id: usize, x_offset: isize, y_offset: isize) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut child_ids = Vec::new();
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.shift_contents(x_offset, y_offset);
                for child_id in rect.children.iter() {
                    child_ids.push(*child_id);
                }
            }
            Err(error) => {
                output = Err(error);
            }
        }
        for child_id in child_ids.iter() {
            self.update_child_space(*child_id);
        }

        self.flag_refresh(rect_id);

        output
    }

    pub fn set_position(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
        let mut output = Ok(());

        let mut has_parent = false;
        let mut did_move = true;

        match self.get_parent_mut(rect_id) {
            Ok(parent) => {
                match parent.child_positions.get(&rect_id) {
                    Some((xx, yy)) => {
                        if *xx == x && *yy == y {
                            did_move = false;
                        }
                    }
                    None => ()
                };
                if (did_move) {
                    parent.set_child_position(rect_id, x, y);
                }
                has_parent = true;
            }
            Err(error) => {
                did_move = false;
                if error != RectError::NoParent {
                    output = Err(error);
                }
            }
        };

        if has_parent {
            output = self.update_child_space(rect_id);
        }

        if output.is_ok() {
            self.flag_parent_refresh(rect_id);
        }

        if output.is_err() {
            logg("Move fail".to_string());
        }

        output
    }

    // Flags the area of the parent of given rect covered by the given rect
    fn flag_parent_refresh(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        let mut dimensions = (0, 0);
        match self.get_rect_size(rect_id) {
            Ok(_dim) => {
                dimensions = _dim;
            }
            Err(e) => {
                output = Err(e);
            }
        };

        let mut working_id = rect_id;
        let mut offset = (0, 0);

        if output.is_ok() {
            loop {
                match self.get_relative_offset(working_id) {
                    Ok(rel_offset) => {
                        offset = (
                            offset.0 + rel_offset.0,
                            offset.1 + rel_offset.1
                        );
                    }
                    Err(error) => {
                        if error != RectError::NoParent {
                            output = Err(error);
                        }
                        break;
                    }
                };

                match self.get_parent_mut(working_id) {
                    Ok(parent) => {
                        for x in 0 .. dimensions.0 {
                            for y in 0 .. dimensions.1 {
                                parent.flags_pos_refresh.insert((offset.0 + x as isize, offset.1 + y as isize));
                            }
                        }
                        working_id = parent.rect_id;
                    }
                    Err(error) => {
                        if error != RectError::NoParent {
                            output = Err(error);
                        }
                        break;
                    }
                };
            }
        }

        output
    }

    fn flag_pos_refresh(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.flags_pos_refresh.insert((x, y));
            }
            Err(e) => {
                output = Err(e);
            }
        };

        if output.is_ok() {
            // loop top, setting requisite refresh flags
            let mut x_out = x;
            let mut y_out = y;
            let mut working_id = rect_id;
            loop {
                match self.get_relative_offset(rect_id) {
                    Ok(offs) => {
                        x_out += offs.0;
                        y_out += offs.1;
                    }
                    Err(error) => {
                        if error != RectError::NoParent {
                            output = Err(error);
                        }
                        break;
                    }
                };

                match self.get_parent_mut(working_id) {
                    Ok(parent) => {
                        parent.flags_pos_refresh.insert((x_out, y_out));
                        working_id = parent.rect_id;
                    }
                    Err(error) => {
                        if error != RectError::NoParent {
                            output = Err(error);
                        }
                        break;
                    }
                };
            }
        }

        output
    }

    pub fn flag_refresh(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.flag_refresh();
            }
            Err(e) => {
                output = Err(e);
            }
        };

        if output.is_ok() {
            output = self.flag_parent_refresh(rect_id);
        }

        output
    }

    pub fn disable(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut was_enabled = false;
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                was_enabled = rect.enabled;
                rect.disable();
            }
            Err(error) => {
                output = Err(error);
            }
        };

        let offset = (0, 0);
        let mut parent_id = 0;

        if was_enabled {
            match self.get_parent_mut(rect_id) {
                Ok(parent) => {
                    parent.clear_child_space(rect_id);
                    parent_id = parent.rect_id;
                }
                Err(error) => {
                    if error != RectError::NoParent {
                        output = Err(error);
                    }
                }
            }

            if output.is_ok() {
                output = self.flag_refresh(parent_id);
            }
        }

        output
    }

    pub fn enable(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut was_enabled = false;
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                was_enabled = rect.enabled;
                rect.enable();
            }
            Err(error) => {
                output = Err(error);
            }
        };


        if ! was_enabled {
            match self.get_parent_mut(rect_id) {
                Ok(parent) => {
                    parent.clear_child_space(rect_id);
                }
                Err(error) => {
                    if error != RectError::NoParent {
                        output = Err(error);
                    }
                }
            }
            if output.is_ok() {
                output = self.flag_refresh(rect_id);
            }
        }

        output
    }

    fn is_rect_enabled(&self, rect_id: usize) -> bool {
        match self.get_rect(rect_id) {
            Ok(rect) => {
                rect.enabled
            }
            Err(e) => {
                false
            }
        }
    }

    // Remove all characters
    pub fn clear(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.clear();
            }
            Err(error) => {
                output = Err(error);
            }
        };
        self.flag_refresh(rect_id);

        output
    }

    // Remove All Children
    pub fn empty(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut children = Vec::new();
        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                for child_id in rect.children.iter() {
                    children.push(*child_id);
                }
            }
            Err(error) => {
                output = Err(error);
            }
        };

        if (output.is_ok()) {
            for child_id in children.iter() {
                output = self.detach(*child_id);
                if (output.is_err()) {
                    break;
                }
                output = self.delete_rect(*child_id);
                if (output.is_err()) {
                    break;
                }
            }
        }

        output
    }

    pub fn detach(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut parent_id = 0;
        let mut has_parent = false;
        let mut output = Ok(());

        output = self.clear_child_space(rect_id);

        match self.get_parent_mut(rect_id) {
            Ok(parent) => {
                parent.detach_child(rect_id);
                parent_id = parent.rect_id;
                has_parent = true;
            }
            Err(error) => {
                if error != RectError::NoParent {
                    output = Err(error);
                }
            }
        }


        if output.is_ok() {
            match self.get_rect_mut(rect_id) {
                Ok(rect) => {
                    rect.unset_parent();
                },
                Err(error) => {
                    output = Err(error);
                }
            };
        }

        output
    }

    pub fn attach(&mut self, rect_id: usize, new_parent_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        output = self.detach(rect_id);


        if output.is_ok() {
            match self.get_rect_mut(rect_id) {
                Ok(rect) => {
                    rect.set_parent(new_parent_id);
                },
                Err(error) => {
                    output = Err(error);
                }
            };
        }

        if output.is_ok() {
            match self.get_rect_mut(new_parent_id) {
                Ok(parent) => {
                    parent.add_child(rect_id);
                }
                Err(error) => {
                    output = Err(error);
                }
            };
        }


        // TODO: This SHOULD only need flag_parent_refresh. but for some reason that break.
        if output.is_ok() {
            output = self.flag_refresh(rect_id);
        }

        output
    }

    pub fn set_string(&mut self, rect_id: usize, start_x: isize, start_y: isize, string: &str) -> Result<(), RectError> {
        let mut output = Ok(());

        let mut dimensions = (0, 0);
        match self.get_rect_size(rect_id) {
            Ok(_dim) => {
                dimensions = (_dim.0 as isize, _dim.1 as isize);
            }
            Err(e) => {
                output = Err(e);
            }
        };

        let mut x;
        let mut y;
        let start_offset = (start_y * dimensions.0) + start_x;

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                let mut i = start_offset;
                for character in string.chars() {
                    x = i % dimensions.0;
                    y = i / dimensions.0;
                    rect.set_character(x, y, character);
                    i += 1;
                }
            }
            Err(e) => {
                output = Err(e);
            }
        };

        if output.is_ok() {
            output = self.flag_refresh(rect_id);
        }

        output
    }

    fn get_default_character(&self, rect_id: usize) -> char {
        match self.get_rect(rect_id) {
            Ok(rect) => {
                rect.get_default_character()
            }
            Err(e) => {
                self.default_character
            }
        }
    }

    pub fn get_character(&self, rect_id: usize, x: isize, y: isize) -> Result<char, RectError> {
        match self.get_rect(rect_id) {
            Ok(rect) => {
                rect.get_character(x, y)
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn set_character(&mut self, rect_id: usize, x: isize, y: isize, character: char) -> Result<(), RectError> {
        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                output = rect.set_character(x, y, character);
            }
            Err(e) => {
                output = Err(e);
            }
        };

        if output.is_ok() {
            output = self.flag_pos_refresh(rect_id, x, y);
        }

        output
    }

    pub fn unset_character(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                output = rect.unset_character(x, y);
            }
            Err(e) => {
                output = Err(e);
            }
        };

        if output.is_ok() {
            output = self.flag_refresh(rect_id);
        }

        output
    }

    pub fn delete_rect(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut to_delete = Vec::new();
        let mut stack = vec![rect_id];
        while stack.len() > 0 {
            match stack.pop() {
                Some(working_id) => {
                    match self.get_rect_mut(working_id) {
                        Ok(rect) => {
                            stack.extend(rect.children.iter().copied());
                        },
                        Err(e) => {}
                    };
                    to_delete.push(working_id);
                }
                None => {
                    break;
                }
            }
        }

        match self.get_parent_mut(rect_id) {
            Ok(parent) => {
                parent.detach_child(rect_id);
            }
            Err(e) => {
                output = Err(e);
            }
        };

        for id in to_delete.iter() {
            self.rects.remove(&id);
        }

        output
    }

    fn update_child_space(&mut self, child_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        let mut dimensions = (0, 0);
        match self.get_rect_size(child_id) {
            Ok(_dim) => {
                dimensions = (_dim.0 as isize, _dim.1 as isize);
            }
            Err(e) => {
                output = Err(e);
            }
        };

        let mut position = (0, 0);
        if output.is_ok() {
            match self.get_relative_offset(child_id) {
                Ok(_pos) => {
                    position = _pos;
                }
                Err(e) => {
                    output = Err(e);
                }
            };
        }

        if output.is_ok() {
            match self.get_parent_mut(child_id) {
                Ok(rect) => {
                    rect.update_child_space(child_id, (
                        position.0,
                        position.1,
                        position.0 + dimensions.0,
                        position.1 + dimensions.1
                    ));
                }
                Err(error) => {
                    if error != RectError::NoParent {
                        output = Err(error);
                    }
                }
            };

        }
        if output.is_ok() {
            self.flag_parent_refresh(child_id);
        }

        output
    }

    fn clear_child_space(&mut self, child_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        self.flag_parent_refresh(child_id);

        match self.get_parent_mut(child_id) {
            Ok(parent) => {
                parent.clear_child_space(child_id);
            }
            Err(error) => {
                if error != RectError::NoParent {
                    output = Err(error);
                }
            }
        }

        output
    }

    pub fn queue_draw(&mut self, rect_id: usize) -> Result<(), RectError> {
        match self.get_rect(rect_id) {
            Ok(_) => {
                self.draw_queue.push(rect_id);
                Ok(())
            }
            Err(error) => {
                Err(error)
            }
        }
    }

    pub fn replace_with(&mut self, old_rect_id: usize, new_rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut parent_id = 0;
        let mut old_position = (0, 0);
        match self.get_parent_mut(old_rect_id) {
            Ok(parent) => {
                parent_id = parent.rect_id;
                old_position = *parent.child_positions.get(&old_rect_id).unwrap();
            }
            Err(error) => {
                output = Err(error);
            }
        }

        if output.is_ok() {
            output = self.detach(old_rect_id);
        }

        if output.is_ok() {
            output = self.attach(new_rect_id, parent_id);
            self.set_position(new_rect_id, old_position.0, old_position.1);
        }

        output
    }

    pub fn get_rect_width(&mut self, rect_id: usize) -> usize {
        let (width, _) = self.get_rect_size(rect_id).ok().unwrap();
        width
    }

    pub fn get_rect_height(&mut self, rect_id: usize) -> usize {
        let (_, height) = self.get_rect_size(rect_id).ok().unwrap();
        height
    }

    pub fn get_width(&mut self) -> usize {
        let (width, _) = self.get_rect_size(0).ok().unwrap();
        width
    }

    pub fn get_height(&mut self) -> usize {
        let (_, height) = self.get_rect_size(0).ok().unwrap();
        height
    }

    pub fn auto_resize(&mut self) -> bool {
        let mut did_resize = false;
        let (current_width, current_height) = self.get_rect_size(0).ok().unwrap();

        match terminal_size() {
            Some((Width(w), Height(h))) => {
                if w as usize != current_width || h as usize != current_height {
                    self.resize(0, w as usize, h as usize);
                    did_resize = true;
                }
            }
            None => ()
        }

        did_resize
    }

    pub fn set_bold_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.set_bold_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }

    pub fn unset_bold_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_bold_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }

    pub fn set_underline_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.set_underline_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }
    pub fn unset_underline_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_underline_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }
    pub fn set_invert_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.set_invert_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }

    pub fn unset_invert_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_invert_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }

    pub fn set_italics_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.set_italics_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }

    pub fn unset_italics_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_italics_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }
    pub fn set_strike_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.set_strike_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }

    pub fn unset_strike_flag(&mut self, rect_id: usize) {
        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_strike_flag();
            }
            Err(e) => {}
        }
        self.flag_refresh(rect_id);
    }



    pub fn set_bg_color(&mut self, rect_id: usize, color: RectColor) -> Result<(), RectError> {
        let mut result = match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.set_bg_color(color);
                Ok(())
            },
            Err(e) => {
                Err(e)
            }
        };

        if (result.is_ok()) {
            result = self.flag_refresh(rect_id);
        }

        result
    }

    pub fn unset_bg_color(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut result = match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_bg_color();
                Ok(())
            },
            Err(e) => {
                Err(e)
            }
        };

        if (result.is_ok()) {
            result = self.flag_refresh(rect_id);
        }

        result
    }

    pub fn set_fg_color(&mut self, rect_id: usize, color: RectColor) -> Result<(), RectError> {
        let mut result = match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.set_fg_color(color);
                Ok(())
            },
            Err(e) => {
                Err(e)
            }
        };

        if (result.is_ok()) {
            result = self.flag_refresh(rect_id);
        }

        result
    }

    pub fn unset_fg_color(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut result = match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_fg_color();
                Ok(())
            },
            Err(e) => {
                Err(e)
            }
        };

        if (result.is_ok()) {
            result = self.flag_refresh(rect_id);
        }

        result
    }

    pub fn unset_color(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut result = match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                rect.unset_color();
                Ok(())
            },
            Err(e) => {
                Err(e)
            }
        };

        if (result.is_ok()) {
            result = self.flag_refresh(rect_id);
        }

        result
    }

    pub fn kill(&mut self) {
        self.empty(0);

        let (w, h) = self.get_rect_size(0).ok().unwrap();
        for x in 0 .. w {
            for y in 0 .. h {
                self.set_character(0, x as isize, y as isize, ' ');
            }
        }

        self.draw(0);
        tcsetattr(0, TCSANOW, & self._termios).unwrap();
        print!("\x1B[?25h"); // Show Cursor
        println!("\x1B[?1049l"); // Return to previous screen
    }
}

