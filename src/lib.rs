use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::str;
use std::cmp;

/*
    TODO
    Maybe change [u8; 4] to a struct like "Character"
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
        }

        self._inverse_child_space.entry(rect_id)
            .or_insert(Vec::new())
            .clear();
    }

    fn set_character(&mut self, x: isize, y: isize, character: [u8;4]) {
        self.character_space.entry((x, y))
            .and_modify(|coord| { *coord = character })
            .or_insert(character);
        self.set_precise_refresh_flag(x, y);
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

    fn set_bg_color(&mut self, n: u8) -> Result<(), RectError> {
        if n > 15 {
            Err(RectError::BadColor)
        } else {
            let orig_color = self.color;
            let mut modded_n: u16 = n as u16;
            modded_n &= 0b01111;
            modded_n |= 0b10000;
            self.color &= 0b1111111111100000;
            self.color |= modded_n;

            if self.color != orig_color {
                self.flag_full_refresh = true;
            }

            Ok(())
        }
    }

    fn set_fg_color(&mut self, n: u8) -> Result<(), RectError> {
        if n > 15 {
            Err(RectError::BadColor)
        } else {
            let orig_color = self.color;
            let mut modded_n: u16 = n as u16;
            modded_n &= 0b01111;
            modded_n |= 0b10000;
            self.color &= 0b1111110000011111;
            self.color |= modded_n << 5;

            if self.color != orig_color {
                self.flag_full_refresh = true;
            }

            Ok(())
        }
    }

    fn add_child(&mut self, child_id: usize) {
        let rect_id = self.rect_id;

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

        let mut rect = self.rects.entry(new_id)
            .or_insert(Rect::new(new_id));

        match parent_id {
            Some(unpacked) => {
                rect.set_parent(unpacked);
            }
            None => ()
        };

        new_id
    }

    fn add_rect(&mut self, rect_id: usize, new_rect: Rect) {
        let new_id = self.idgen;
        self.rects.insert(rect_id, new_rect);
    }

    fn get_rect(&self, rect_id: usize) -> Option<&Rect> {
        let output;
        match self.rects.get(&rect_id) {
            Some(rect) => {
                output = Some(rect);
            }
            None => {
                output = None;
            }
        }
        output
    }
    fn get_rect_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        let output;
        match self.rects.get_mut(&rect_id) {
            Some(rect) => {
                output = Some(rect);
            }
            None => {
                output = None;
            }
        }
        output
    }

    fn get_parent(&self, rect_id: usize) -> Option<&Rect> {
        let mut output = None;
        let mut has_parent = false;
        let mut parent_id = 0;
        match self.rects.get(&rect_id) {
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
        }

        if (has_parent) {
            match self.rects.get(&parent_id) {
                Some(parent) => {
                    output = Some(parent);
                }
                None => ()
                // TODO: Throw Error. A rect has been removed but its children haven't been informed
            }
        }

        output
    }

    fn get_parent_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        let mut output = None;
        let mut has_parent = false;
        let mut parent_id = 0;
        match self.rects.get_mut(&rect_id) {
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
        }

        if (has_parent) {
            match self.rects.get_mut(&parent_id) {
                Some(parent) => {
                    output = Some(parent);
                }
                None => ()
                // TODO: Throw Error. A rect has been removed but its children haven't been informed
            }
        }

        output
    }

    fn get_top(&self, rect_id: usize) -> Option<&Rect> {
        let output;
        let mut current_id = rect_id;
        let fail;

        loop {
            match self.rects.get(&current_id) {
                Some(rect) => {
                    match rect.parent {
                        Some(parent_id) => {
                            current_id = parent_id
                        }
                        None => {
                            fail = false;
                            break;
                        }
                    }
                }
                None => {
                    fail = true;
                    break;
                }
            }
        }

        if fail {
            output = None;
        } else {
            output = self.rects.get(&current_id);
        }

        output
    }

    fn get_top_mut(&mut self, rect_id: usize) -> Option<&mut Rect> {
        let output;
        let mut current_id = rect_id;
        let fail;

        loop {
            match self.rects.get(&current_id) {
                Some(rect) => {
                    match rect.parent {
                        Some(parent_id) => {
                            current_id = parent_id
                        }
                        None => {
                            fail = false;
                            break;
                        }
                    }
                }
                None => {
                    fail = true;
                    break;
                }
            }
        }

        if fail {
            output = None;
        } else {
            output = self.rects.get_mut(&current_id);
        }

        output
    }

    fn has_parent(&self, rect_id: usize) -> bool {
        let mut output = false;
        match self.get_rect(rect_id) {
            Some(rect) => {
                match rect.parent {
                    Some(pid) => {
                        output = true;
                    }
                    None => ()
                }
            }
            None => ()
        };

        output
    }

    fn _update_cached_by_positions(&mut self, rect_id: usize, positions: &Vec<(isize, isize)>, boundries: (isize, isize, isize, isize)) {
        // TODO: Double Check the logic in this function. I may have biffed it when refactoring
        /*
            child_recache items are:
                child_id,
                Vector of positions,
                has parent?
                offset (if has parent)
        */
        let mut child_recache: HashMap<usize, (Vec<(isize, isize)>, bool, (isize, isize))> = HashMap::new();
        let mut i = 0;
        let mut new_positions = Vec::new();
        let mut x;
        let mut y;
        let mut tmp_chr;
        let mut tmp_color;
        let mut new_values = Vec::new();
        let mut new_boundries;
        let mut child_dim;
        let (width, height) = self.get_rect_size(rect_id);
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                for i in 0 .. positions.len() {
                    x = positions[i].0;
                    y = positions[i].1;

                    //if ! (x >= boundries.0 && x < boundries.2 && y >= boundries.1 && y < boundries.3) {
                    //    continue;
                    //}

                    new_positions.push((x, y));
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
            None => ()
        };

        for (child_id, (coords, child_has_position, child_position)) in child_recache.iter_mut() {
            child_dim = self.get_rect_size(*child_id);

            if *child_has_position {
                new_boundries = (
                    boundries.0 - child_position.0,
                    boundries.1 - child_position.1,
                    boundries.2 - child_position.0,
                    boundries.3 - child_position.1
                );

                self._update_cached_display(*child_id, new_boundries);

                for (x, y) in coords.iter() {
                    //if child_position.0 > *x && child_position.1 > *y && *x <= child_dim.0 && *y <= child_dim.1 {
                    //    continue;
                    //}
                    if *x >= 0 && *x < width && *y >= 0 && *y < height {
                        match self.get_rect_mut(*child_id) {
                            Some(child) => {
                                match child._cached_display.get(&(x - child_position.0, y - child_position.1)) {
                                    Some(new_value) => {
                                        new_values.push((*new_value, *x, *y));
                                    }
                                    None => ()
                                };
                            }
                            None => ()
                        }
                    }
                }
            }
        }

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                for (new_value, x, y) in new_values.iter() {
                    rect._cached_display.entry((*x, *y))
                        .and_modify(|e| { *e = *new_value })
                        .or_insert(*new_value);
                }

                rect.flags_pos_refresh = new_positions;
            }
            None => ()
        };
    }

    fn _update_cached_display(&mut self, rect_id: usize, boundries: (isize, isize, isize, isize)) {
        /*
       //TODO
            Since Children indicate to parents that a refresh is requested,
            if no flag is set, there is no need to delve down
        */
        let mut flags_pos_refresh = Vec::new();
        let mut do_positional_update = false;

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.has_been_drawn = true;

                /*
                  If a full refresh is requested,
                  fill flags_pos_refresh with all potential coords
                */
                if rect.flag_full_refresh {
                    rect.flag_full_refresh = false;
                    rect.flags_pos_refresh = Vec::new();

                    for y in 0 .. rect.height {
                        for x in 0 .. rect.width {
                            rect.flags_pos_refresh.push((x,y));
                        }
                    }
                }

                /*
                    Iterate through flags_pos_refresh and update
                    any children that cover the requested positions
                */
                for pos in rect.flags_pos_refresh.iter() {
                    flags_pos_refresh.push((pos.0, pos.1));
                }
                do_positional_update = true;
            }
            None => ()
        }


        if do_positional_update {

            // Climb and set positional refresh flags
            let mut working_id = rect_id;
            let mut last_flags = flags_pos_refresh;
            let mut parent_flags;
            let mut child_pos;

            loop {
                self._update_cached_by_positions(rect_id, &last_flags, boundries);
                match self.get_parent(working_id) {
                    Some(parent) => {
                        parent_flags = Vec::new();
                        child_pos = parent.get_child_position(working_id);

                        for pos in last_flags.iter() {
                            parent_flags.push((
                                pos.0 - child_pos.0,
                                pos.1 - child_pos.1
                            ));
                        }

                        working_id = parent.rect_id;
                        last_flags = parent_flags;
                    },
                    None => {
                        break;
                    }
                }
            }
        }

    }

    fn get_display(&mut self, rect_id: usize, boundries: (isize, isize, isize, isize)) -> HashMap<(isize, isize), ([u8; 4], u16)> {
        let mut output = HashMap::new();

        self._update_cached_display(rect_id, boundries);

        match self.get_rect(rect_id) {
            Some(rect) => {
                for ((x, y), (new_c, color)) in rect._cached_display.iter() {
                    //if ! (x >= &boundries.0 && x < &boundries.2 && y >= &boundries.1 && y < &boundries.3) {
                    //    continue;
                    //}
                    output.insert((*x, *y), (*new_c, *color));
                }
            }
            None => ()
        };

        // Handle Ghosts
        let filled_ghosts = self._handle_ghosts(rect_id);
        for (ghostpos, value) in filled_ghosts.iter() {
            output.entry(*ghostpos)
                .and_modify(|e| { *e = *value })
                .or_insert(*value);
        }

        output
    }

    fn _handle_ghosts(&mut self, rect_id: usize) -> HashMap<(isize, isize), ([u8; 4], u16)> {
        // TODO: 2things, i think I need to climb up, _updating each rect, instead
        // of jumping to the top.
        //  Also I don't think my working_ghosts are right
        let mut output = HashMap::new();

        let mut parent_id = 0;
        let mut has_parent = false;
        match self.get_parent(rect_id) {
            Some(parent) => {
                parent_id = parent.rect_id;
                has_parent = true;
            }
            None => ()
        }


        if (has_parent) {
            let (mut offx, mut offy) = self.get_offset(parent_id);
            let mut working_ghosts = Vec::new();
            let mut firstoff = (0, 0);

            match self.get_rect_mut(parent_id) {
                Some(parent) => {
                    firstoff = parent.child_positions[&rect_id];
                    match parent.child_ghosts.get_mut(&rect_id) {
                        Some(ghosts) => {
                            for (x, y) in ghosts.iter() {
                                working_ghosts.push( (x + offx, y + offy) );
                            }
                            ghosts.clear();
                        }
                        None => {
                            parent.child_ghosts.insert(rect_id, Vec::new());
                        }
                    }
                }
                None => ()
            };

            let mut top_id = 0;
            match self.get_top_mut(parent_id) {
                Some(top) => {
                    top_id = top.rect_id;
                }
                None => ()
            }
            let mut top_dim = self.get_rect_size(top_id);
            self._update_cached_by_positions(top_id, &working_ghosts, (0, 0, top_dim.0, top_dim.1));

            match self.get_rect_mut(top_id) {
                Some(top) => {
                    for (x, y) in working_ghosts.iter() {
                        let mut ghostpos = (
                            *x - firstoff.0,
                            *y - firstoff.1
                        );

                        if ghostpos.0 >= 0 && ghostpos.1 >= 0 && ghostpos.0 < top.width && ghostpos.1 < top.height {
                            match top._cached_display.get(&(*x, *y)) {
                                Some(topchar) => {
                                    output.entry(ghostpos)
                                        .and_modify(|e| { *e = *topchar })
                                        .or_insert(*topchar);
                                }
                                None => ()
                            }
                        }
                    }
                }
                None => ()
            };
        }

        output
    }

    fn draw(&mut self, rect_id: usize) {
        let offset = self.get_offset(rect_id);

        let mut renderstring: String;
        let mut val_a: &[u8];
        let mut color_value: u16;
        let mut current_line_color_value: u16 = 0;
        let mut utf_char: &[u8];
        let mut utf_char_split_index: usize;
        let mut change_char: bool;

        let (width, height) = self.get_rect_size(rect_id);

        // TODO: top_disp is now a misnomer
        let top_disp = self.get_display(rect_id, (offset.0, offset.1, width, height));

        renderstring = "".to_string();
        for (pos, val) in top_disp.iter() {
            if (offset.0 + pos.0) < offset.0 || (offset.0 + pos.0) >= offset.0 + width || (offset.1 + pos.1) < offset.1 || (offset.1 + pos.1) >= offset.1 + height {
                continue;
            }

            renderstring += &format!("\x1B[{};{}H", offset.1 + pos.1 + 1, offset.0 + pos.0 + 1);

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
        }

        print!("{}\x1B[0m", renderstring);
        println!("\x1B[1;1H");
    }

    fn get_rect_size(&self, rect_id: usize) -> (isize, isize) {
        // TODO: Throw Error instead of passing (0, 0)
        let mut dimensions = (0, 0);
        match self.get_rect(rect_id) {
            Some(rect) => {
                dimensions = (rect.width, rect.height);
            }
            None => ()
        };

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
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.resize(width, height);
            }
            None => ()
        };
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
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_precise_refresh_flag(x, y);
            }
            None => ()
        }

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

        let mut was_enabled = false;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                was_enabled = rect.enabled;
                rect.disable();
            }
            None => ()
        };


        if was_enabled {
            match self.get_parent_mut(rect_id) {
                Some(parent) => {
                    parent.clear_child_space(rect_id);
                }
                None => ()
            }
        }
    }

    fn enable(&mut self, rect_id: usize) {
        let mut was_enabled = true;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                was_enabled = rect.enabled;
                rect.enable();
            }
            None => ()
        };


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

        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.detach_child(rect_id);
            }
            None => ()
        };

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_parent();
            }
            None => ()
        };


    }

    fn attach(&mut self, rect_id: usize, new_parent_id: usize) {
        match self.get_parent_mut(rect_id) {
            Some(parent) => {
                parent.detach_child(rect_id);
            }
            None => ()
        };

        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_parent(new_parent_id);
            }
            None => ()
        };

        match self.get_rect_mut(new_parent_id) {
            Some(new_parent) => {
                new_parent.add_child(rect_id);
            }
            None => ()
        };
    }

    fn set_character(&mut self, rect_id: usize, x: isize, y: isize, character: [u8;4]) {
        let mut do_set_precise = false;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.set_character(x, y, character);
                do_set_precise = true;
            }
            None => ()
        };

        if do_set_precise {
            self.set_precise_refresh_flag(rect_id, x, y);
        }
    }

    fn unset_character(&mut self, rect_id: usize, x: isize, y: isize) {
        let mut do_set_precise = false;
        match self.get_rect_mut(rect_id) {
            Some(rect) => {
                rect.unset_character(x, y);
                do_set_precise = true;
            }
            None => ()
        };

        if do_set_precise {
            self.set_precise_refresh_flag(rect_id, x, y);
        }
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
        let mut parent_id = 0;
        match self.get_parent_mut(child_id) {
            Some(rect) => {
                rect.update_child_space(child_id, corners);
                parent_id = rect.rect_id;
            }
            None => ()
        }

        for y in corners.1 .. corners.3 {
            for x in corners.0 .. corners.2 {
                self.set_precise_refresh_flag(parent_id, x, y);
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

    match rectmanager.get_rect_mut(rect_id) {
        Some(rect) => {
            rect.set_fg_color(col);
        }
        None => ()
    };

    Box::into_raw(rectmanager); // Prevent Release
}

#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut RectManager, rect_id: usize, col: u8) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    match rectmanager.get_rect_mut(rect_id) {
        Some(rect) => {
            rect.set_bg_color(col);
        }
        None => ()
    };

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

    match rectmanager.get_rect_mut(rect_id) {
        Some(rect) => {
            rect.unset_bg_color();
        }
        None => ()
    };

    Box::into_raw(rectmanager); // Prevent Release
}



#[no_mangle]
pub extern "C" fn unset_fg_color(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    match rectmanager.get_rect_mut(rect_id) {
        Some(rect) => {
            rect.unset_fg_color();
        }
        None => ()
    };

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn unset_color(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    match rectmanager.get_rect_mut(rect_id) {
        Some(rect) => {
            rect.unset_color();
        }
        None => ()
    };

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
