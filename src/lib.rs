use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::str;
use std::cmp;

/*
    TODO
    Maybe change [u8; 4] to a struct like "Character"

    Drawing gets SLOOW with many layers. look for optimizations.
*/


pub enum RectError {
    BadPosition,
    NotFound,
    BadColor,
    InvalidUtf8
}

pub struct RectManager {
    idgen: usize,
    rects: HashMap<usize, Rect>
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
    child_ghosts: HashMap<usize, Vec<(isize, isize)>>,

    character_space: HashMap<(isize,isize), [u8; 4]>,

    flag_full_refresh: bool,
    flags_pos_refresh: Vec<(isize, isize)>,

    enabled: bool,
    has_been_drawn: bool,

    color: u16, // { 7: USEFG, 6-4: FG, 3: USEBG, 2-0: BG }

    _cached_display: HashMap<(isize, isize), ([u8; 4], u16)>
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
            child_ghosts: HashMap::new(),
            character_space: HashMap::new(),
            flag_full_refresh: true,
            flags_pos_refresh: Vec::new(),
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

                    match self.child_ghosts.get_mut(&rect_id) {
                        Some(coord_list) => {
                            coord_list.retain(|&e| e != (x, y));
                        }
                        None => ()
                    }

                }
            }
        }
    }

    fn clear_child_space(&mut self, rect_id: usize) {

        // Works around borrowing
        // TODO: Implement Copy for Vec<(isize, isize)> ?
        let mut new_positions = Vec::new();
        match  self._inverse_child_space.get(&rect_id) {
            Some(positions) => {
                for position in positions.iter() {
                    new_positions.push((position.0, position.1));
                }
            }
            None => ()
        };

        for position in new_positions.iter() {

            match self.child_space.get_mut(&position) {
                Some(child_ids) => {
                    child_ids.retain(|&x| x != rect_id);
                }
                None => ()
            }

            self.child_ghosts.entry(rect_id)
                .or_insert(Vec::new())
                .push(*position);

            self.set_precise_refresh_flag(position.0, position.1);
        }

        self._inverse_child_space.entry(rect_id)
            .or_insert(Vec::new())
            .clear();
    }

    fn set_character(&mut self, x: isize, y: isize, character: [u8;4]) {
        if y < self.height && y >= 0 && x < self.width && x >= 0 {
            self.character_space.entry((x, y))
                .and_modify(|coord| { *coord = character })
                .or_insert(character);
            self.set_precise_refresh_flag(x, y);
        } else {
            panic!("({},{}) is out of bounds on Rect {}", x, y, self.rect_id);
        }
    }

    fn unset_character(&mut self, x: isize, y: isize) {
        self.set_character(x, y, self.default_character);
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

    fn set_precise_refresh_flag(&mut self, x: isize, y: isize) {
        self.flags_pos_refresh.push((x, y));
    }
}

impl RectManager {
    fn new() -> RectManager {
        let mut rectmanager = RectManager {
            idgen: 0,
            rects: HashMap::new()
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
                rect.set_parent(unpacked);
            }
            None => ()
        };

        new_id
    }

    fn get_rect(&self, rect_id: usize) -> &Rect {
        match self.rects.get(&rect_id) {
            Some(rect) => {
                rect
            }
            None => {
                panic!("Rect {} Not Found", rect_id);
            }
        }
    }

    fn get_rect_mut(&mut self, rect_id: usize) -> &mut Rect {
        match self.rects.get_mut(&rect_id) {
            Some(rect) => {
                rect
            }
            None => {
                panic!("Rect {} Not Found", rect_id);
            }
        }
    }

    fn get_parent(&self, rect_id: usize) -> Option<&Rect> {
        let mut output = None;
        let mut has_parent = false;
        let mut parent_id = 0;

        let rect = self.get_rect(rect_id);
        match rect.parent {
            Some(pid) => {
                has_parent = true;
                parent_id = pid;
            }
            None => ()
        };

        if has_parent {
            output = Some(self.get_rect(parent_id));
        }

        output
    }

    fn get_parent_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        let mut output = None;
        let mut has_parent = false;
        let mut parent_id = 0;

        let rect = self.get_rect(rect_id);
        match rect.parent {
            Some(pid) => {
                has_parent = true;
                parent_id = pid;
            }
            None => ()
        };

        if has_parent {
            output = Some(self.get_rect_mut(parent_id));
        }

        output
    }

    // Top can be the same as the given rect
    fn get_top(&self, rect_id: usize) -> &Rect {
        let mut current_id = rect_id;
        let mut current_rect;

        loop {
            current_rect = self.get_rect(current_id);
            match current_rect.parent {
                Some(parent_id) => {
                    current_id = parent_id
                }
                None => {
                    break;
                }
            }
        }

        self.get_rect(current_id)
    }

    fn get_top_mut(&mut self, rect_id: usize) -> &mut Rect {
        let mut current_id = rect_id;
        let mut current_rect;
        loop {
            current_rect = self.get_rect(current_id);
            match current_rect.parent {
                Some(parent_id) => {
                    current_id = parent_id
                }
                None => {
                    break;
                }
            }
        }

        self.get_rect_mut(current_id)
    }

    fn has_parent(&self, rect_id: usize) -> bool {
        let mut output = false;
        let rect = self.get_rect(rect_id);
        match rect.parent {
            Some(_) => {
                output = true;
            }
            None => ()
        }

        output
    }

    fn _update_cached_by_positions(&mut self, rect_id: usize, positions: &Vec<(isize, isize)>) {
        // TODO: Double Check the logic in this function. I may have biffed it when refactoring
        /*
            child_recache items are:
                child_id,
                Vector of positions,
                has parent?
                offset (if has parent)
        */
        let mut child_recache: HashMap<usize, (Vec<(isize, isize)>, bool, (isize, isize))> = HashMap::new();
        let mut x;
        let mut y;
        let mut tmp_chr;
        let mut tmp_color;
        let mut new_values = Vec::new();

        let mut rect;
        {
            rect = self.get_rect_mut(rect_id);
            for i in 0 .. positions.len() {
                x = positions[i].0;
                y = positions[i].1;


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

        let mut child;
        for (child_id, (coords, child_has_position, child_position)) in child_recache.iter_mut() {

            if *child_has_position {
                {
                    child = self.get_rect_mut(*child_id);
                    for (x, y) in coords.iter() {
                        child.flags_pos_refresh.push((
                            *x - child_position.0,
                            *y - child_position.1
                        ));
                    }
                }

                self._update_cached_display(*child_id);

                {
                    child = self.get_rect_mut(*child_id);
                    for (x, y) in coords.iter() {
                        match child._cached_display.get(&(*x - child_position.0, *y - child_position.1)) {
                            Some(new_value) => {
                                new_values.push((*new_value, *x, *y));
                            }
                            None => ()
                        };
                    }
                }
            }
        }

        {
            rect = self.get_rect_mut(rect_id);
            for (new_value, x, y) in new_values.iter() {
                rect._cached_display.entry((*x, *y))
                    .and_modify(|e| { *e = *new_value })
                    .or_insert(*new_value);
            }
        }
    }

    fn _update_cached_display(&mut self, rect_id: usize) {
        /*
       //TODO
            Since Children indicate to parents that a refresh is requested,
            if no flag is set, there is no need to delve down
        */
        let mut flags_pos_refresh = Vec::new();
        let rect = self.get_rect_mut(rect_id);
        rect.has_been_drawn = true;

        /*
          If a full refresh is requested,
          fill flags_pos_refresh with all potential coords
        */
        if rect.flag_full_refresh {
            rect.flag_full_refresh = false;

            for y in 0 .. rect.height {
                for x in 0 .. rect.width {
                    flags_pos_refresh.push((x,y));
                }
            }
        } else {
            /*
                Iterate through flags_pos_refresh and update
                any children that cover the requested positions
            */
            for pos in rect.flags_pos_refresh.iter() {
                flags_pos_refresh.push((pos.0, pos.1));
            }
        }
        rect.flags_pos_refresh.clear();

        self._update_cached_by_positions(rect_id, &flags_pos_refresh);
    }

    fn get_display(&mut self, rect_id: usize) -> HashMap<(isize, isize), ([u8; 4], u16)> {
        let mut output = HashMap::new();

        // Handle Ghosts
        let filled_ghosts = self._handle_ghosts(rect_id);

        for (ghostpos, value) in filled_ghosts.iter() {
            output.entry(*ghostpos)
                .or_insert(*value);
        }

        let rect = self.get_rect(rect_id);
        for ((x, y), (new_c, color)) in rect._cached_display.iter() {
            output.insert((*x, *y), (*new_c, *color));
        }

        output
    }

    fn _handle_ghosts(&mut self, rect_id: usize) -> HashMap<(isize, isize), ([u8; 4], u16)> {
        let mut output = HashMap::new();
        let mut working_ghosts = Vec::new();


        // Collect ghosts from parent
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                let offset = parent.child_positions[&rect_id];
                match parent.child_ghosts.get_mut(&rect_id) {
                    Some(ghosts) => {
                        for (x, y) in ghosts.iter() {
                            // store the ghosts relative to the rect, not its parent
                            working_ghosts.push( (*x - offset.0, *y - offset.1) );
                        }
                        ghosts.clear();
                    }
                    None => {
                        parent.child_ghosts.insert(rect_id, Vec::new());
                    }
                }

            }
            None => ()
        }


        let rect_offset = self.get_offset(rect_id);
        let top_id;
        let mut top;

        // Setup positional flags
        {
            top = self.get_top_mut(rect_id);
            top_id = top.rect_id;
            for (x, y) in working_ghosts.iter() {
                top.flags_pos_refresh.push((*x + rect_offset.0, *y + rect_offset.1));
            }
        }
        self._update_cached_display(top_id);
        top = self.get_top_mut(rect_id);

        let mut ghostpos;
        for (x, y) in working_ghosts.iter() {
            // working_ghosts are relative to rect, we need to consider the absolute x & y
            ghostpos = (
                rect_offset.0 + *x,
                rect_offset.1 + *y
            );

            if ghostpos.0 >= 0 && ghostpos.1 >= 0 && ghostpos.0 < top.width && ghostpos.1 < top.height {
                match top._cached_display.get(&ghostpos) {
                    Some(topchar) => {
                        output.insert((*x, *y), *topchar);
                    }
                    None => ()
                }

            }
        }

        output
    }

    fn flag_full_refresh(&mut self, rect_id: usize) {
        let rect = self.get_rect_mut(rect_id);
        rect.flag_full_refresh = true;
    }

    fn draw(&mut self, rect_id: usize) {
        let offset = self.get_offset(rect_id);

        let mut renderstring: String;
        let mut val_a: &[u8];
        let mut color_value: u16;
        let mut current_line_color_value: u16 = 0;
        let mut utf_char: &[u8];
        let mut utf_char_split_index: usize;

        let display_map = self.get_display(rect_id);

        let mut sorted = Vec::new();
        for (pos, val) in display_map.iter() {
            sorted.push((pos, val));
        }
        sorted.sort();

        renderstring = "".to_string();
        let mut current_col = -1;
        let mut current_row = -1;

        for (pos, val) in sorted.iter() {
            if pos.1 + offset.1 != current_row || pos.0 + offset.0 != current_col {
                renderstring += &format!("\x1B[{};{}H", offset.1 + pos.1 + 1, offset.0 + pos.0 + 1);
            }
            current_col = pos.0 + offset.0;
            current_row = pos.1 + offset.1;

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

    fn get_rect_size(&self, rect_id: usize) -> (isize, isize) {
        let rect = self.get_rect(rect_id);
        let dimensions = (rect.width, rect.height);

        dimensions
    }

    fn get_offset(&self, rect_id: usize) -> (isize, isize) {
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
                    // TODO: Throw Rect not found error
                }
            }
        }

        (x, y)
    }

    fn resize(&mut self, rect_id: usize, width: isize, height: isize) {
        let rect = self.get_rect_mut(rect_id);
        rect.resize(width, height);

        let mut pos = (0, 0);

        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                pos = parent.get_child_position(rect_id);
            }
            None => ()
        }

        self.set_position(rect_id, pos.0, pos.1);
    }

    fn set_position(&mut self, rect_id: usize, x: isize, y: isize) {
        let mut has_parent = false;
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.set_child_position(rect_id, x, y);
                has_parent = true;
            }
            None => ()
        }

        if has_parent {
            let dim = self.get_rect_size(rect_id);
            self.update_child_space(rect_id, (x, y, x + dim.0, y + dim.1));
        }
    }

    fn set_precise_refresh_flag(&mut self, rect_id: usize, x: isize, y: isize) {
        let rect = self.get_rect_mut(rect_id);
        rect.set_precise_refresh_flag(x, y);

        // loop top, setting requisite refresh flags
        let mut working_child_id = rect_id;
        let mut tmp_offset;
        loop {
            match self.get_parent_mut(working_child_id) {
                Some(parent) => {
                    tmp_offset = parent.get_child_position(working_child_id);
                    parent.set_precise_refresh_flag(tmp_offset.0 + x, tmp_offset.1 + y);
                    working_child_id = parent.rect_id;
                }
                None => {
                    break;
                }
            };
        }
    }

    fn disable(&mut self, rect_id: usize) {
        let dimensions = self.get_rect_size(rect_id);
        let rect = self.get_rect_mut(rect_id);
        let was_enabled = rect.enabled;

        rect.disable();
        let mut offset = (0, 0);

        let mut parent_id = 0;

        if was_enabled {
            match self.get_parent_mut(rect_id) {
                Some(parent) => {
                    parent.clear_child_space(rect_id);
                    offset = parent.child_positions[&rect_id];
                    parent_id = parent.rect_id;
                }
                None => ()
            }

            for x in offset.0 .. offset.0 + dimensions.0 {
                for y in offset.1 .. offset.1 + dimensions.1 {
                    self.set_precise_refresh_flag(parent_id, x, y);
                }
            }
        }
    }

    fn enable(&mut self, rect_id: usize) {
        let rect = self.get_rect_mut(rect_id);
        let was_enabled = rect.enabled;
        rect.enable();


        if ! was_enabled {
            match self.get_parent_mut(rect_id) {
                Some(parent) => {
                    parent.clear_child_space(rect_id);
                }
                None => ()
            }
        }
    }

    fn detach(&mut self, rect_id: usize) {
        let dimensions = self.get_rect_size(rect_id);

        let mut offset = (0, 0);
        let mut parent_id = 0;
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.detach_child(rect_id);
                parent_id = parent.rect_id;

            }
            None => ()
        };

        for x in offset.0 .. offset.0 + dimensions.0 {
            for y in offset.1 .. offset.1 + dimensions.1 {
                self.set_precise_refresh_flag(parent_id, x, y);
            }
        }


        self.get_rect_mut(rect_id).unset_parent();
    }

    fn attach(&mut self, rect_id: usize, new_parent_id: usize) {
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.detach_child(rect_id);
            }
            None => ()
        };

        self.get_rect_mut(rect_id).set_parent(new_parent_id);

        self.get_rect_mut(new_parent_id).add_child(rect_id);
    }

    fn set_character(&mut self, rect_id: usize, x: isize, y: isize, character: [u8;4]) {
        let rect = self.get_rect_mut(rect_id);
        rect.set_character(x, y, character);
        self.set_precise_refresh_flag(rect_id, x, y);
    }

    fn unset_character(&mut self, rect_id: usize, x: isize, y: isize) {
        let rect = self.get_rect_mut(rect_id);
        rect.unset_character(x, y);
        self.set_precise_refresh_flag(rect_id, x, y);
    }

    fn delete_rect(&mut self, rect_id: usize) {
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.detach_child(rect_id);
            }
            None => ()
        };

        self.rects.remove(&rect_id);
    }

    fn update_child_space(&mut self, child_id: usize, corners: (isize, isize, isize, isize)) {
        let mut working_parent_id = 0;
        let mut ghosts = Vec::new();
        match self.get_parent_mut(child_id) {
            Some(rect) => {
                rect.update_child_space(child_id, corners);
                working_parent_id = rect.rect_id;
                if rect.child_ghosts.contains_key(&child_id) {
                    for ghost in rect.child_ghosts[&child_id].iter() {
                        ghosts.push((ghost.0, ghost.1));
                    }
                }
            }
            None => ()
        }

        let mut new_corners = corners;
        let mut working_offset = (0, 0);
        let mut working_ghosts;
        let mut new_x;
        let mut new_y;

        loop {
            new_corners = (
                new_corners.0 + working_offset.0,
                new_corners.1 + working_offset.1,
                new_corners.2 + working_offset.0,
                new_corners.3 + working_offset.1
            );
            working_ghosts = Vec::new();
            for (x, y) in ghosts.iter() {
                new_x = *x + working_offset.0;
                new_y = *y + working_offset.1;
                self.set_precise_refresh_flag(working_parent_id, new_x, new_y);
                working_ghosts.push((new_x, new_y));
            }
            ghosts = working_ghosts;

            for y in new_corners.1 .. new_corners.3 {
                for x in new_corners.0 .. new_corners.2 {
                    self.set_precise_refresh_flag(working_parent_id, x, y);
                }
            }


            match self.get_parent_mut(working_parent_id) {
                Some(parent) => {
                    working_offset = parent.child_positions[&working_parent_id];
                    working_parent_id = parent.rect_id;
                }
                None => {
                    break;
                }
            }
        }
    }
}


#[no_mangle]
pub extern "C" fn disable_rect(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.disable(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn enable_rect(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.enable(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn draw(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.draw(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}

#[no_mangle]
pub extern "C" fn set_fg_color(ptr: *mut RectManager, rect_id: usize, col: u8) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let rect = rectmanager.get_rect_mut(rect_id);
    rect.set_fg_color(col);

    Box::into_raw(rectmanager); // Prevent Release
}

#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut RectManager, rect_id: usize, col: u8) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let rect = rectmanager.get_rect_mut(rect_id);
    rect.set_bg_color(col);

    Box::into_raw(rectmanager); // Prevent Release
}



#[no_mangle]
pub extern "C" fn resize(ptr: *mut RectManager, rect_id: usize, new_width: isize, new_height: isize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.resize(rect_id, new_width, new_height);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn unset_bg_color(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let rect = rectmanager.get_rect_mut(rect_id);
    rect.unset_bg_color();

    Box::into_raw(rectmanager); // Prevent Release
}



#[no_mangle]
pub extern "C" fn unset_fg_color(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let rect = rectmanager.get_rect_mut(rect_id);
    rect.unset_fg_color();

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn unset_color(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let rect = rectmanager.get_rect_mut(rect_id);
    rect.unset_color();

    Box::into_raw(rectmanager); // Prevent Release
}



#[no_mangle]
pub extern "C" fn set_character(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize, c: *const c_char) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    //assert!(!c.is_null()); TODO: figure out need for this assertion.
    let c_str = unsafe { CStr::from_ptr(c) };
    let string_bytes = c_str.to_str().unwrap().as_bytes();

    let mut new_c: [u8; 4] = [0; 4];
    for i in 0..cmp::min(4, string_bytes.len()) {
        // Put the 0 offset first
        new_c[(4 - cmp::min(4, string_bytes.len())) + i] = string_bytes[i];
    }

    rectmanager.set_character(rect_id, x, y, new_c);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn unset_character(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.unset_character(rect_id, x, y);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn delete_rect(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.delete_rect(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
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
pub extern "C" fn set_position(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize) {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.set_position(rect_id, x, y);

    Box::into_raw(rectmanager); // Prevent Release
}

#[no_mangle]
pub extern "C" fn detach(ptr: *mut RectManager, rect_id: usize) {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.detach(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn attach(ptr: *mut RectManager, rect_id: usize, parent_id: usize) {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.attach(rect_id, parent_id);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn kill(ptr: *mut RectManager) {
    let rectmanager = unsafe { Box::from_raw(ptr) };

    println!("\x1B[?25h"); // Show Cursor
    println!("\x1B[?1049l"); // Return to previous screen

    // TODO: Figure out why releasing causes segfault
    Box::into_raw(rectmanager); // Prevent Release
    // Releases boxes
}


#[no_mangle]
pub extern "C" fn init(width: isize, height: isize) -> *mut RectManager {
    let mut rectmanager = RectManager::new();

    rectmanager.resize(0, width, height);

    println!("\x1B[?1049h"); // New screen
    println!("\x1B[?25l"); // Hide Cursor

    Box::into_raw(Box::new(rectmanager))
}

//
//#[no_mangle]
//pub extern "C" fn draw_area(ptr: *mut RectManager, from_box: usize, x: isize, y: isize, width: isize, height: isize) {
//    let mut rectmanager = unsafe { Box::from_raw(ptr) };
//
//    match _draw_area(&mut rectmanager, from_box, (x, y), (width, height)) {
//        Ok(_) => (),
//        Err(e) => panic!(e)
//    };
//
//    Box::into_raw(rectmanager); // Prevent Release
//}
//
//#[no_mangle]
//pub extern "C" fn fillc(ptr: *mut RectManager, rect_id: usize, c: *const c_char) {
//    let mut rectmanager = unsafe { Box::from_raw(ptr) };
//
//    assert!(!c.is_null());
//
//    let c_str = unsafe { CStr::from_ptr(c) };
//    let string_bytes = c_str.to_str().expect("Not a valid UTF-8 string").as_bytes();
//
//    let boxes = &mut rectmanager.boxes;
//    match boxes.get_mut(&(rect_id as usize)) {
//        Some(bleepsbox) => {
//            bleepsbox.fill(string_bytes);
//        }
//        None => ()
//    };
//
//    Box::into_raw(rectmanager); // Prevent Release
//}
//
