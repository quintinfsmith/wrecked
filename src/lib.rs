use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str;
use std::cmp;
use std::fs::OpenOptions;
use std::io::prelude::*;


/*
    TODO
    Maybe change [u8; 4] to a struct like "Character"

    Drawing gets SLOOW with many layers. look for optimizations.
*/

fn logg(mut msg: String) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("rlogg")
        .unwrap();

    writeln!(file, "{}\n", msg);
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
    InvalidChild = 7,
    ChildNotFound = 8
}

pub struct RectManager {
    idgen: usize,
    rects: HashMap<usize, Rect>,
    draw_queue: Vec<usize>
}

pub struct Rect {
    rect_id: usize,

    width: isize,
    height: isize,
    default_character: [u8; 4],
    parent: Option<usize>, // RectId

    children: Vec<usize>,
    // Used to find a box by position
    child_space: HashMap<(isize, isize), Vec<usize>>,
    _inverse_child_space: HashMap<usize, Vec<(isize, isize)>>,
    // Used to find a position of a box
    child_positions: HashMap<usize, (isize, isize)>,

    character_space: HashMap<(isize,isize), [u8; 4]>,

    flag_full_refresh: bool,
    flags_pos_refresh: HashSet<(isize, isize)>,

    enabled: bool,
    has_been_drawn: bool,

    color: u16, // { 7: USEFG, 6-4: FG, 3: USEBG, 2-0: BG }

    _cached_display: HashMap<(isize, isize), ([u8; 4], u16)>,

}

impl Rect {
    fn new(rect_id: usize) -> Rect {
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
            has_been_drawn: false,
            color: 0u16,
            _cached_display: HashMap::new(),
            default_character: [0, 0, 0, 32]
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

    fn flag_child_rect_refresh(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        if (! self.has_child(rect_id)) {
            output = Err(RectError::InvalidChild);
        } else {

            let positions = self._inverse_child_space.entry(rect_id)
                .or_insert(Vec::new());

            for (x, y) in positions.iter() {
                self.flags_pos_refresh.insert((*x, *y));
            }

        }

        output
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
                if x >= 0 && x < self.width && y >= 0 && y <= self.height {

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
        // Works around borrowing
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

    fn set_character(&mut self, x: isize, y: isize, character: [u8;4]) -> Result<(), RectError> {
        let output;
        if y < self.height && y >= 0 && x < self.width && x >= 0 {
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

    fn unset_bg_color(&mut self) {
        let orig_color = self.color;
        self.color &= 0b1111111111100000;

        if self.color != orig_color {
            self.flag_full_refresh = true;
        }
    }

    fn unset_fg_color(&mut self) {
        let orig_color = self.color;
        self.color &= 0b1111110000011111;

        if self.color != orig_color {
            self.flag_full_refresh = true;
        }
    }

    fn unset_color(&mut self) {
        let orig_color = self.color;
        self.color &= 0;
        if orig_color == 0 {
            self.flag_full_refresh = true;
        }
    }

    fn set_bg_color(&mut self, n: u8) {
        let orig_color = self.color;
        let mut modded_n: u16 = n as u16;
        modded_n &= 0b01111;
        modded_n |= 0b10000;
        self.color &= 0b1111111111100000;
        self.color |= modded_n;

        if self.color != orig_color {
            self.flag_full_refresh = true;
        }
    }

    fn set_fg_color(&mut self, n: u8) {
        let orig_color = self.color;
        let mut modded_n: u16 = n as u16;
        modded_n &= 0b01111;
        modded_n |= 0b10000;
        self.color &= 0b1111110000011111;
        self.color |= modded_n << 5;

        if self.color != orig_color {
            self.flag_full_refresh = true;
        }
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

    fn resize(&mut self, width: isize, height: isize) {
        self.width = width;
        self.height = height;
    }

    // Can't update child_space here, need child width and height
    fn set_child_position(&mut self, rect_id: usize, x: isize, y: isize) {
        self.child_positions.entry(rect_id)
            .and_modify(|e| { *e = (x, y) })
            .or_insert((x, y));
    }

    fn has_child(&mut self, child_id: usize) -> bool {
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

impl RectManager {
    fn new() -> RectManager {
        let mut rectmanager = RectManager {
            idgen: 0,
            rects: HashMap::new(),
            draw_queue: Vec::new()
        };
        rectmanager.new_rect(None);

        rectmanager
    }

    fn new_rect(&mut self, parent_id: Option<usize>) -> usize {
        let new_id = self.idgen;
        self.idgen += 1;

        let rect = self.rects.entry(new_id)
            .or_insert(Rect::new(new_id));

        match parent_id {
            Some(unpacked) => {
                self.attach(new_id, unpacked);
            }
            None => ()
        };
        self.flag_refresh(new_id);


        new_id
    }

    fn get_rect(&self, rect_id: usize) -> Result<&Rect, RectError> {
        match self.rects.get(&rect_id) {
            Some(rect) => {
                Ok(rect)
            }
            None => {
                Err(RectError::NotFound)
            }
        }
    }

    fn get_rect_mut(&mut self, rect_id: usize) -> Result<&mut Rect, RectError> {
        match self.rects.get_mut(&rect_id) {
            Some(rect) => {
                Ok(rect)
            }
            None => {
                Err(RectError::NotFound)
            }
        }
    }

    fn get_parent(&self, rect_id: usize) -> Result<&Rect, RectError> {
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

    fn get_parent_mut(&mut self, rect_id: usize) -> Result<&mut Rect, RectError> {
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
    fn get_top(&self, rect_id: usize) -> Result<&Rect, RectError> {
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
    fn get_top_mut(&mut self, rect_id: usize) -> Result<&mut Rect, RectError> {
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

    fn has_parent(&self, rect_id: usize) -> Result<bool, RectError> {
        let mut output;
        match self.get_rect(rect_id) {
            Ok(rect) => {
                match rect.parent {
                    Some(_) => {
                        output = Ok(true);
                    }
                    None => {
                        output = Ok(false);
                    }
                }
            }
            Err(e) => {
                output = Err(e);
            }
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
        let mut tmp_color;
        let mut new_values = Vec::new();

        let mut output = Ok(());

        match self.get_rect_mut(rect_id) {
            Ok(rect) => {
                for (_x, _y) in positions.iter() {
                    x = *_x;
                    y = *_y;

                    if !rect.child_space.contains_key(&(x, y)) || rect.child_space[&(x, y)].is_empty() {
                        // Make sure at least default character is present
                        tmp_color = rect.color;

                        tmp_chr = rect.character_space.entry((x, y))
                            .or_insert(rect.default_character);

                        rect._cached_display.entry((x,y))
                            .and_modify(|e| {*e = (*tmp_chr, tmp_color)})
                            .or_insert((*tmp_chr, tmp_color));

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
                rect.has_been_drawn = true;

                /*
                  If a full refresh is requested,
                  fill flags_pos_refresh with all potential coords
                */
                if rect.flag_full_refresh {
                    rect.flag_full_refresh = false;

                    for y in 0 .. rect.height {
                        for x in 0 .. rect.width {
                            flags_pos_refresh.insert((x, y));
                        }
                    }
                    rect.flags_pos_refresh.clear();
                } else {
                    /*
                        Iterate through flags_pos_refresh and update
                        any children that cover the requested positions
                    */
                    for pos in rect.flags_pos_refresh.iter() {
                        flags_pos_refresh.insert((pos.0, pos.1));
                    }
                }
                rect.flags_pos_refresh.clear();
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

    fn get_display(&mut self, rect_id: usize) -> Result<HashMap<(isize, isize), ([u8; 4], u16)>, RectError> {
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
                    for ((x, y), (new_c, color)) in rect._cached_display.iter() {
                        outhash.insert((*x, *y), (*new_c, *color));
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

    fn _draw(&mut self, display_map: &mut Vec<((isize, isize), ([u8; 4], u16))>) {
        let mut renderstring = "".to_string();

        let mut current_col = -1;
        let mut current_row = -1;

        let mut val_a: &[u8];
        let mut color_value: u16;
        let mut current_line_color_value: u16 = 0;
        let mut utf_char: &[u8];
        let mut utf_char_split_index: usize;
        display_map.sort();

        for (pos, val) in display_map.iter() {
            if pos.1 != current_row || pos.0 != current_col {
                renderstring += &format!("\x1B[{};{}H", pos.1 + 1, pos.0 + 1);
            }
            current_col = pos.0;
            current_row = pos.1;

            val_a = &val.0;
            color_value = val.1;
            if color_value != current_line_color_value {
                if color_value == 0 {
                    renderstring += &format!("\x1B[0m");
                } else {
                    // ForeGround
                    if (color_value >> 5) & 16 == 16 {
                        if (color_value >> 5) & 8 == 8 {
                            renderstring += &format!("\x1B[9{}m", ((color_value >> 5) & 7));
                        } else {
                            renderstring += &format!("\x1B[3{}m", ((color_value >> 5) & 7));
                        }
                    } else {
                        renderstring += &format!("\x1B[39m");
                    }

                    // BackGround
                    if color_value & 16 == 16 {
                        if color_value & 8 == 8 {
                            renderstring += &format!("\x1B[10{}m", (color_value & 7));
                        } else {
                            renderstring += &format!("\x1B[4{}m", (color_value & 7));
                        }
                    } else {
                        renderstring += &format!("\x1B[49m");
                    }
                }
                current_line_color_value = color_value;
            }


            utf_char_split_index = 0;
            for i in 0..4 {
                if val_a[i] != 0 {
                    utf_char_split_index = i;
                    break;
                }
            }

            utf_char = val_a.split_at(utf_char_split_index).1;

            renderstring += &format!("{}", str::from_utf8(utf_char).unwrap());
            current_col += 1;
        }

        print!("{}\x1B[0m", renderstring);
        println!("\x1B[1;1H");
    }

    fn draw(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut to_draw = Vec::new();

        let mut offset = (0, 0);
        match self.get_absolute_offset(rect_id) {
            Ok(_offset) => {
                offset = _offset;
            }
            Err(e)=> {
                output = Err(e);
            }
        };

        if output.is_ok() {
            match self.get_display(rect_id) {
                Ok(display_map) => {
                    for (pos, val) in display_map.iter() {
                        to_draw.push(((offset.0 + pos.0, offset.1 + pos.1), *val));
                    }
                }
                Err(e) => {
                    output = Err(e);
                }
            }
        }

        if output.is_ok() {
            self._draw(&mut to_draw);
        }

        output
    }

    // Get n where n is the position in sibling array
    fn get_rank(&self, rect_id: usize) -> Result<usize, RectError> {
        let mut output = Ok(0);
        let mut rank = 0;
        match self.get_parent(rect_id) {
            Ok(parent) => {
                match parent.children.binary_search(&rect_id) {
                    Ok(_rank) => {
                        rank = _rank;
                    }
                    Err(e) => {
                        output = Err(RectError::ChildNotFound);
                    }
                };
            }
            Err(error) => {
                if error == RectError::ParentNotFound || error == RectError::NotFound {
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
                    if error == RectError::ParentNotFound || error == RectError::NotFound {
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

    fn draw_queued(&mut self) -> Result<(), RectError> {
        let mut output = Ok(());
        let mut to_draw = Vec::new();
        let mut depth_tracker: HashMap<(isize, isize), usize> = HashMap::new();

        let mut offset = (0, 0);

        let mut draw_queue = Vec::new();

        for rect_id in self.draw_queue.iter() {
            draw_queue.push((0, 0, *rect_id));
        }
        self.draw_queue.clear();

        for (depth, rank, rect_id) in draw_queue.iter_mut() {
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

        if output.is_ok() {

            draw_queue.sort();


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
                    match self.get_display(rect_id) {
                        Ok(display_map) => {
                            for (pos, val) in display_map.iter() {
                                if ! depth_tracker.contains_key(pos) || *depth_tracker.get(pos).unwrap() >= depth {
                                    to_draw.push(((offset.0 + pos.0, offset.1 + pos.1), *val));
                                    depth_tracker.entry(*pos)
                                        .and_modify(|e| { *e = depth })
                                        .or_insert(depth);
                                }
                            }
                        }
                        Err(e) => {
                            output = Err(e);
                            break;
                        }
                    }
                } else {
                    break;
                }
            }

            self._draw(&mut to_draw);
        }

        output
    }

    fn get_rect_size(&self, rect_id: usize) -> Result<(isize, isize), RectError> {
        let mut output;
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
        let mut pos;


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
                    if error == RectError::ParentNotFound || error == RectError::NotFound {
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

    fn resize(&mut self, rect_id: usize, width: isize, height: isize) -> Result<(), RectError> {
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
                    if error == RectError::ParentNotFound || error == RectError::NotFound {
                        output = Err(error);
                    }
                }
            };
        }

        if output.is_ok() {
            output = self.set_position(rect_id, pos.0, pos.1);
            self.flag_refresh(rect_id);
        }

        output
    }

    fn set_position(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
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
                if error == RectError::ParentNotFound || error == RectError::NotFound {
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
                        if error == RectError::ParentNotFound || error == RectError::NotFound {
                            output = Err(error);
                        }
                        break;
                    }
                };


                match self.get_parent_mut(working_id) {
                    Ok(parent) => {
                        for x in 0 .. dimensions.0 {
                            for y in 0 .. dimensions.1 {
                                parent.flags_pos_refresh.insert((offset.0 + x, offset.1 + y));
                            }
                        }
                        working_id = parent.rect_id;
                    }
                    Err(error) => {
                        if error == RectError::ParentNotFound || error == RectError::NotFound {
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
                        if error == RectError::ParentNotFound || error == RectError::NotFound {
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
                        if error == RectError::ParentNotFound || error == RectError::NotFound {
                            output = Err(error);
                        }
                        break;
                    }
                };
            }
        }

        output
    }

    fn flag_refresh(&mut self, rect_id: usize) -> Result<(), RectError> {
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

    fn disable(&mut self, rect_id: usize) -> Result<(), RectError> {
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

        let mut offset = (0, 0);
        let mut parent_id = 0;

        if was_enabled {
            match self.get_parent_mut(rect_id) {
                Ok(parent) => {
                    parent.clear_child_space(rect_id);
                    parent_id = parent.rect_id;
                }
                Err(error) => {
                    if error == RectError::ParentNotFound || error == RectError::NotFound {
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

    fn enable(&mut self, rect_id: usize) -> Result<(), RectError> {
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
                    if error == RectError::ParentNotFound || error == RectError::NotFound {
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

    fn empty(&mut self, rect_id: usize) -> Result<(), RectError> {
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
            }
        }

        output
    }

    fn detach(&mut self, rect_id: usize) -> Result<(), RectError> {
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
                if error == RectError::ParentNotFound || error == RectError::NotFound {
                    output = Err(error);
                }
            }
        };


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

    fn attach(&mut self, rect_id: usize, new_parent_id: usize) -> Result<(), RectError> {
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

    fn set_character(&mut self, rect_id: usize, x: isize, y: isize, character: [u8;4]) -> Result<(), RectError> {
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

    fn unset_character(&mut self, rect_id: usize, x: isize, y: isize) -> Result<(), RectError> {
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

    fn delete_rect(&mut self, rect_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        match self.get_parent_mut(rect_id) {
            Ok(parent) => {
                parent.detach_child(rect_id);
            }
            Err(e) => {
                output = Err(e);
            }
        };

        self.rects.remove(&rect_id);

        output
    }

    fn update_child_space(&mut self, child_id: usize) -> Result<(), RectError> {
        let mut output = Ok(());

        let mut dimensions = (0, 0);
        match self.get_rect_size(child_id) {
            Ok(_dim) => {
                dimensions = _dim;
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
                    if error == RectError::ParentNotFound || error == RectError::NotFound {
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
                if error == RectError::ParentNotFound || error == RectError::NotFound {
                    output = Err(error);
                }
            }
        }

        output
    }

    fn queue_draw(&mut self, rect_id: usize) -> Result<(), RectError> {
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
}


#[no_mangle]
pub extern "C" fn disable_rect(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.disable(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}


#[no_mangle]
pub extern "C" fn enable_rect(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.enable(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}

#[no_mangle]
pub extern "C" fn queue_draw(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.queue_draw(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}

#[no_mangle]
pub extern "C" fn draw_queued(ptr: *mut RectManager) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.draw_queued();

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}

#[no_mangle]
pub extern "C" fn draw(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let mut result;
    let draw_queue_length = rectmanager.draw_queue.len();
    if draw_queue_length > 0 {
        result = rectmanager.queue_draw(rect_id);
        if (result.is_ok()) {
            result = rectmanager.draw_queued()
        }
    } else {
        result = rectmanager.draw(rect_id);
    }

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}

#[no_mangle]
pub extern "C" fn set_fg_color(ptr: *mut RectManager, rect_id: usize, col: u8) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = match rectmanager.get_rect_mut(rect_id) {
        Ok(rect) => {
            rect.set_fg_color(col);
            Ok(())
        },
        Err(e) => {
            Err(e)
        }
    };

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}

#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut RectManager, rect_id: usize, col: u8) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let mut result = match rectmanager.get_rect_mut(rect_id) {
        Ok(rect) => {
            rect.set_bg_color(col);
            Ok(())
        },
        Err(e) => {
            Err(e)
        }
    };

    if (result.is_ok()) {
        result = rectmanager.flag_refresh(rect_id);
    }

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}



#[no_mangle]
pub extern "C" fn resize(ptr: *mut RectManager, rect_id: usize, new_width: isize, new_height: isize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.resize(rect_id, new_width, new_height);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}


#[no_mangle]
pub extern "C" fn unset_bg_color(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let mut result = match rectmanager.get_rect_mut(rect_id) {
        Ok(rect) => {
            rect.unset_bg_color();
            Ok(())
        },
        Err(e) => {
            Err(e)
        }
    };

    if (result.is_ok()) {
        result = rectmanager.flag_refresh(rect_id);
    }

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}



#[no_mangle]
pub extern "C" fn unset_fg_color(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let mut result = match rectmanager.get_rect_mut(rect_id) {
        Ok(rect) => {
            rect.unset_fg_color();
            Ok(())
        },
        Err(e) => {
            Err(e)
        }
    };

    if (result.is_ok()) {
        result = rectmanager.flag_refresh(rect_id);
    }

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}


#[no_mangle]
pub extern "C" fn unset_color(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = match rectmanager.get_rect_mut(rect_id) {
        Ok(rect) => {
            rect.unset_color();
            Ok(())
        },
        Err(e) => {
            Err(e)
        }
    };

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(e) => e as u32
    }
}


#[no_mangle]
pub extern "C" fn set_character(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize, c: *const c_char) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    //assert!(!c.is_null()); TODO: figure out need for this assertion.
    let c_str = unsafe { CStr::from_ptr(c) };
    let string_bytes = c_str.to_str().unwrap().as_bytes();

    let mut new_c: [u8; 4] = [0; 4];
    for i in 0..cmp::min(4, string_bytes.len()) {
        // Put the 0 offset first
        new_c[(4 - cmp::min(4, string_bytes.len())) + i] = string_bytes[i];
    }


    let result = rectmanager.set_character(rect_id, x, y, new_c);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(error) => error as u32
    }
}


#[no_mangle]
pub extern "C" fn unset_character(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.unset_character(rect_id, x, y);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(error) => error as u32
    }
}


#[no_mangle]
pub extern "C" fn delete_rect(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.delete_rect(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(error) => error as u32
    }
}


#[no_mangle]
pub extern "C" fn new_rect(ptr: *mut RectManager, parent_id: usize, width: isize, height: isize) -> usize {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let new_rect_id = rectmanager.new_rect(Some(parent_id));
    rectmanager.resize(new_rect_id, width, height);

    Box::into_raw(rectmanager); // Prevent Release

    new_rect_id
}

#[no_mangle]
pub extern "C" fn set_position(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize) -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.set_position(rect_id, x, y);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(error) => error as u32
    }
}

#[no_mangle]
pub extern "C" fn empty(ptr: *mut RectManager, rect_id: usize)  -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.empty(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(error) => error as u32
    }
}

#[no_mangle]
pub extern "C" fn detach(ptr: *mut RectManager, rect_id: usize)  -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.detach(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    match result {
        Ok(_) => 0,
        Err(error) => error as u32
    }
}


#[no_mangle]
pub extern "C" fn attach(ptr: *mut RectManager, rect_id: usize, parent_id: usize) -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.attach(rect_id, parent_id);

    Box::into_raw(rectmanager); // Prevent Release


    match result {
        Ok(_) => 0,
        Err(error) => error as u32
    }
}


#[no_mangle]
pub extern "C" fn kill(ptr: *mut RectManager) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let mut rect_ids = Vec::new();
    for (rect_id, rect) in rectmanager.rects.iter() {
        rect_ids.push(*rect_id);
    }

    for rect_id in rect_ids.iter() {
        if *rect_id > 0 {
            rectmanager.detach(*rect_id);
        }
    }

    let (w, h) = rectmanager.get_rect_size(0).ok().unwrap();
    for x in 0 .. w {
        for y in 0 .. h {
            rectmanager.set_character(0, x, y, [0, 0, 0, 0]);
        }
    }

    rectmanager.draw(0);
    print!("\x1B[?25h"); // Show Cursor
    println!("\x1B[?1049l"); // Return to previous screen


    // TODO: Figure out why releasing causes segfault
    Box::into_raw(rectmanager); // Prevent Release
    // Releases boxes
}


#[no_mangle]
pub extern "C" fn init(width: isize, height: isize) -> *mut RectManager {
    let mut rectmanager = RectManager::new();

    rectmanager.resize(0, width, height);

    print!("\x1B[?25l"); // Hide Cursor
    println!("\x1B[?1049h"); // New screen

    Box::into_raw(Box::new(rectmanager))
}

