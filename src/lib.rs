use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::str;
use std::cmp;

//#[cfg(test)]
//mod tests {
//    #[test]
//    fn exploration() {
//        assert_eq!(2 + 2, 4);
//    }
//
//    /* TODO
//        box_gets_removed
//        box_id_gets_reused
//        cant_place_character_out_of_bounds
//        displays_get_cached
//
//    */
//}

pub enum BleepsError {
    BadPosition,
    NotFound,
    BadColor,
    InvalidUtf8
}

pub struct BoxHandler {
    keygen: usize,
    open_keys: Vec<usize>,
    boxes: HashMap<usize, BleepsBox>,
    cached_display: HashMap<(isize, isize), ([u8; 4], u16)>
}

pub struct BleepsBox {
    enabled: bool,
    boxes: Vec<usize>,
    box_positions: HashMap<usize, (isize, isize)>,
    width: usize,
    height: usize,
    grid: HashMap<(usize, usize), [u8; 4]>,
    cached: HashMap<(isize, isize), ([u8; 4], u16)>,
    parent: Option<usize>,

    recache_flag: bool,
    color: u16, // { 7: USEFG, 6-4: FG, 3: USEBG, 2-0: BG }
}

impl BleepsBox {
    fn new(width: usize, height: usize) -> BleepsBox {
        BleepsBox {
            enabled: true,
            boxes: Vec::new(),
            box_positions: HashMap::new(),
            width: width,
            height: height,
            grid: HashMap::new(),
            cached: HashMap::new(),
            parent: None,
            recache_flag: true,
            color: 0
        }
    }

    fn flag_recache(&mut self) {
        self.recache_flag = true;
    }

    fn fill(&mut self, c: &[u8]) {
        let mut new_c: [u8; 4] = [0; 4];
        for i in 0..c.len() {
            new_c[(4 - c.len()) + i] = c[i]; // Put the 0 offset first
        }

        for y in 0..self.height {
            for x in 0..self.width {
                self.grid.entry((x, y))
                    .and_modify(|e| { *e = new_c })
                    .or_insert(new_c);
            }
        }
    }

    fn unset(&mut self, x: usize, y: usize) -> Result<(), BleepsError> {
        if x >= self.width || y >= self.height {
            Err(BleepsError::BadPosition)
        } else {
            self.grid.remove(&(x, y));
            Ok(())
        }
    }

    fn resize(&mut self, width: usize, height: usize) -> Result<(), BleepsError> {
        self.width = width;
        self.height = height;
        let mut keys_to_remove: Vec<(usize, usize)> = Vec::new();

        for ((x, y), val) in self.grid.iter() {
            if (*x >= width || *y >= height) {
                keys_to_remove.push((*x, *y));
            }
        }

        for key in keys_to_remove.iter() {
            self.grid.remove(key);
        }

        Ok(())
    }

    fn set(&mut self, x: usize, y: usize, c: &[u8]) -> Result<(), BleepsError> {
        let mut new_c: [u8; 4] = [0; 4];
        for i in 0..cmp::min(4, c.len()) {
            new_c[(4 - cmp::min(4, c.len())) + i] = c[i]; // Put the 0 offset first
        }

        if x >= self.width || y >= self.height {
            Err(BleepsError::BadPosition)
        } else {
            self.grid.entry((x, y))
                .and_modify(|e| { *e = new_c })
                .or_insert(new_c);
            Ok(())
        }
    }

    fn unset_bg_color(&mut self) {
        self.color &= 0b1111111111100000;
    }

    fn unset_fg_color(&mut self) {
        self.color &= 0b1111110000011111;
    }

    fn unset_color(&mut self) {
        self.color &= 0;
    }

    fn set_bg_color(&mut self, n: u8) -> Result<(), BleepsError> {
        if n > 15 {
            Err(BleepsError::BadColor)
        } else {
            let mut modded_n: u16 = n as u16;
            modded_n &= 0b01111;
            modded_n |= 0b10000;
            self.color |= modded_n;
            Ok(())
        }
    }

    fn set_fg_color(&mut self, n: u8) -> Result<(), BleepsError> {
        if n > 15 {
            Err(BleepsError::BadColor)
        } else {
            let mut modded_n: u16 = n as u16;
            modded_n &= 0b01111;
            modded_n |= 0b10000;
            self.color |= modded_n << 5;
            Ok(())
        }
    }

    // Unused, but could e useful in the future
    //fn get(&self, x: usize, y: usize) -> Result<Option<&[u8; 4]>, BleepsError> {
    //    if x >= self.width || y >= self.height {
    //        Err(BleepsError::BadPosition)
    //    } else {
    //        Ok(self.grid.get(&(x, y)))
    //    }
    //}

    // Unused, but may be useful in the future
    //fn get_cached(&self) -> &HashMap<(isize, isize), ([u8; 4], u16)> {
    //    &self.cached
    //}

    fn set_cached(&mut self, tocache: &HashMap<(isize, isize), ([u8; 4], u16)>) {
        self.cached = (*tocache).clone();
        self.recache_flag = false;
    }
}

fn _disable_box(boxhandler: &mut BoxHandler, box_id: usize) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    {
        // Check that box exists before proceeding
        try!(
            match boxes.get(&box_id) {
                Some(_found) => Ok(()),
                None => Err(BleepsError::NotFound)
            }
        );

        match boxes.get_mut(&box_id) {
            Some(found) => {
                found.enabled = false;
            }
            None => ()
        };
        match _flag_recache(&mut boxes, box_id) {
            Ok(_) => (),
            Err(e) => panic!(e)
        };
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn disable_box(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    match  _disable_box(&mut boxhandler, box_id) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };
    Box::into_raw(boxhandler); // Prevent Release
}

fn _enable_box(boxhandler: &mut BoxHandler, box_id: usize) -> Result<(), BleepsError> {
    // Check that box exists before proceeding
    let mut boxes = &mut boxhandler.boxes;
    {
        try!(
            match boxes.get(&box_id) {
                Some(_found) => Ok(()),
                None => Err(BleepsError::NotFound)
            }
        );

        match boxes.get_mut(&box_id) {
            Some(found) => {
                found.enabled = true;
            }
            None => ()
        };
        match _flag_recache(&mut boxes, box_id) {
            Ok(_) => (),
            Err(e) => panic!(e)
        };
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn enable_box(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    match _enable_box(&mut boxhandler, box_id) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };
    Box::into_raw(boxhandler); // Prevent Release
}

fn _removebox_from_boxes(boxes: &mut HashMap<usize, BleepsBox>, box_id: usize) -> Result<Vec<usize>, BleepsError> {
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    let mut subboxes: Vec<usize> = Vec::new();
    let mut parent_id: usize = 0;

    try!(
        match boxes.get(&box_id) {
            Some(bleepsbox) => {
                for subbox_id in bleepsbox.boxes.iter() {
                    subboxes.push(*subbox_id);
                }
                match bleepsbox.parent {
                    Some(parent) => {
                        parent_id = parent;
                    }
                    None => ()
                };
                Ok(())
            }
            None => Err(BleepsError::NotFound)
        }
    );

    let mut removed_box_ids: Vec<usize> = Vec::new();
    let mut sub_removed_box_ids: Vec<usize>;
    for subbox_id in subboxes.iter() {
        sub_removed_box_ids = try!(_removebox_from_boxes(boxes, *subbox_id));
        removed_box_ids.append(&mut sub_removed_box_ids);
    }

    // No Need to try!() here, its ok if parent doesn't exist
    let mut to_remove = 0;
    let mut do_remove = false;
    match boxes.get_mut(&parent_id) {
        Some(bleepsbox) => {
            for i in 0..bleepsbox.boxes.len() {
                if bleepsbox.boxes[i] == box_id {
                    to_remove = i;
                    do_remove = true;
                    break;
                }
            }
            if do_remove {
                bleepsbox.boxes.remove(to_remove);
            }

            bleepsbox.box_positions.remove(&box_id);
        }
        None => ()
    };

    boxes.remove(&box_id);
    removed_box_ids.push(box_id);

    Ok(removed_box_ids)
}

fn rects_intersect(rect_a: (isize, isize, isize, isize), rect_b: (isize, isize, isize, isize)) -> bool {
    // TODO: implement. this is for testing, and will be slow to render every box
    (! (rect_a.0 + rect_a.2 < rect_b.0 || rect_a.0 > rect_b.0 + rect_b.2 || rect_a.1 + rect_a.3 < rect_b.1 || rect_a.1 > rect_b.1 + rect_b.3))
}

fn get_offset(boxes: &mut HashMap<usize, BleepsBox>, box_id: usize) -> Result<(isize, isize), BleepsError> {
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    let mut offset: (isize, isize) = (0, 0);
    let mut has_parent: bool = false;
    let mut parent_id: usize = 0;

    match boxes.get(&box_id) {
        Some(found) => {
            match found.parent {
                Some(pid) => {
                    parent_id = pid;
                    has_parent = true;
                }
                None => ()
            };
        }
        None => ()
    };

    if has_parent {
        match boxes.get(&parent_id) {
            Some(parent) => {
                match parent.box_positions.get(&box_id) {
                    Some(position) => {
                        offset = *position;
                    }
                    None => ()
                };
            }
            None => ()
        };
        let parent_offset = try!(get_offset(boxes, parent_id));
        offset = (offset.0 + parent_offset.0, offset.1 + parent_offset.1);
    }


    Ok(offset)
}


fn get_display(boxes: &mut HashMap<usize, BleepsBox>, box_id: usize, offset: (isize, isize), frame: (isize, isize, isize, isize)) -> Result<HashMap<(isize, isize), ([u8; 4], u16)>, BleepsError> {
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    let mut output: HashMap<(isize, isize), ([u8; 4], u16)>;
    let mut subboxes: Vec<((isize, isize), usize)> = Vec::new();

    let mut descend = false;
    output = HashMap::new();


    match boxes.get(&box_id) {
        Some(bleepsbox) => {
            if bleepsbox.enabled && rects_intersect(frame, (offset.0, offset.1, bleepsbox.width as isize, bleepsbox.height as isize)) {
                if bleepsbox.recache_flag {
                    descend = true;

                    for (position, character) in bleepsbox.grid.iter() {
                        output.insert((position.0 as isize, position.1 as isize), (*character, bleepsbox.color));
                    }

                    for subbox_id in bleepsbox.boxes.iter() {
                        match bleepsbox.box_positions.get(&subbox_id) {
                            Some(subbox_offset) => {
                                subboxes.push((*subbox_offset, *subbox_id));
                            }
                            None => ()
                        };
                    }

                } else {
                    for (pos, val) in bleepsbox.cached.iter() {
                        output.insert(*pos, *val);
                    }
                }
            }
        }
        None => ()
    };

    if descend {
        let mut subbox_output: HashMap<(isize, isize), ([u8; 4], u16)>;
        for (subbox_offset, subbox_id) in subboxes.iter() {
            subbox_output = try!(get_display(boxes, *subbox_id, (offset.0 + subbox_offset.0, subbox_offset.1 + offset.1), frame));
            for (subpos, value) in subbox_output.iter() {
                output.entry((subpos.0 + subbox_offset.0, subpos.1 + subbox_offset.1))
                    .and_modify(|e| { *e = *value })
                    .or_insert(*value);
            }
        }

        match boxes.get_mut(&box_id) {
            Some(bleepsbox) => {
                bleepsbox.set_cached(&output);
            }
            None => ()
        };
    }

    Ok(output)
}


fn _draw(boxhandler: &mut BoxHandler, box_id: usize) -> Result<(), BleepsError> {
    let boxes = &mut boxhandler.boxes;
    {
        let width = boxes[&box_id].width as isize;
        let height = boxes[&box_id].height as isize;
        let offset = try!(get_offset(boxes, box_id));
        let top_disp = try!(get_display(boxes, box_id, offset, (offset.0, offset.1, width, height)));
        let mut val_a: &[u8];
        let mut color_value: u16;
        let mut current_line_color_value: u16 = 0;
        let mut s: String;
        let mut utf_char: &[u8];
        let mut utf_char_split_index: usize;
        let mut change_char: bool;

        s = "".to_string();
        for (pos, val) in top_disp.iter() {
            if (offset.0 + pos.0) < offset.0 || (offset.0 + pos.0) >= offset.0 + width || (offset.1 + pos.1) < offset.1 || (offset.1 + pos.1) >= offset.1 + height {
                continue;
            }

            change_char = true;
            match boxhandler.cached_display.get(&(pos.0 + offset.0, pos.1 + offset.1)) {
                Some(found) => {
                    change_char = *found != (val.0, val.1);
                }
                None => ()
            };

            if ! change_char {
                continue;
            }

            boxhandler.cached_display.entry((pos.0 + offset.0, pos.1 + offset.1))
                .and_modify(|e| { *e = (val.0, val.1) })
                .or_insert((val.0, val.1));

            s += &format!("\x1B[{};{}H", offset.1 + pos.1 + 1, offset.0 + pos.0 + 1);

            val_a = &val.0;
            color_value = val.1;
            if (color_value != current_line_color_value) {
                if (color_value == 0) {
                    s += &format!("\x1B[0m");
                } else {
                    // ForeGround
                    if (color_value >> 5) & 16 == 16 {
                        if (color_value >> 5) & 8 == 8 {
                            s += &format!("\x1B[9{}m", ((color_value >> 5) & 7));
                        } else {
                            s += &format!("\x1B[3{}m", ((color_value >> 5) & 7));
                        }
                    } else {
                        s += &format!("\x1B[39m");
                    }

                    // BackGround
                    if color_value & 16 == 16 {
                        if color_value & 8 == 8 {
                            s += &format!("\x1B[10{}m", (color_value & 7));
                        } else {
                            s += &format!("\x1B[4{}m", (color_value & 7));
                        }
                    } else {
                        s += &format!("\x1B[49m");
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

            s += &format!("{}", str::from_utf8(utf_char).unwrap());
        }
        print!("{}\x1B[0m", s);
        println!("\x1B[1;1H");
    }

    Ok(())
}


#[no_mangle]
pub extern "C" fn draw(ptr: *mut BoxHandler, from_box: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _draw(&mut boxhandler, from_box) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}


fn _flag_recache(boxes: &mut HashMap<usize, BleepsBox>, box_id: usize) -> Result<(), BleepsError> {
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    let mut next_box_id: usize = box_id as usize;

    let mut do_next = true;


    while do_next {
        match boxes.get_mut(&next_box_id) {
            Some(bleepsbox) => {
                bleepsbox.flag_recache();
                match bleepsbox.parent {
                    Some(found_id) => {
                        next_box_id = found_id as usize;
                    }
                    None => {
                        do_next = false;
                    }
                }
            }
            None => {
                do_next = false;
            }
        };
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn flag_recache(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    {
        let mut boxes = &mut boxhandler.boxes;
        match _flag_recache(&mut boxes, box_id) {
            Ok(_) => (),
            Err(e) => panic!(e)
        };
    }
    Box::into_raw(boxhandler); // Prevent Release
}

fn _set_fg_color(boxhandler: &mut BoxHandler, box_id: usize, col: u8) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            try!(bleepsbox.set_fg_color(col));
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

#[no_mangle]
pub extern "C" fn set_fg_color(ptr: *mut BoxHandler, box_id: usize, col: u8) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _set_fg_color(&mut boxhandler, box_id, col) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

fn _set_bg_color(boxhandler: &mut BoxHandler, box_id: usize, col: u8) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            try!(bleepsbox.set_bg_color(col));
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut BoxHandler, box_id: usize, col: u8) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _set_bg_color(&mut boxhandler, box_id, col) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

fn _unset_bg_color(boxhandler: &mut BoxHandler, box_id: usize) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            bleepsbox.unset_bg_color();
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

fn _resize(boxhandler: &mut BoxHandler, box_id: usize, new_width: usize, new_height: usize) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            bleepsbox.resize(new_width, new_height);
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));
    Ok(())
}

#[no_mangle]
pub extern "C" fn resize(ptr: *mut BoxHandler, box_id: usize, new_width: usize, new_height: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    {
        match _resize(&mut boxhandler, box_id, new_width, new_height) {
            Ok(_) => (),
            Err(e) => panic!(e)
        };
    }

    Box::into_raw(boxhandler); // Prevent Release
}

#[no_mangle]
pub extern "C" fn unset_bg_color(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    {
        match _unset_bg_color(&mut boxhandler, box_id) {
            Ok(_) => (),
            Err(e) => panic!(e)
        };
    }

    Box::into_raw(boxhandler); // Prevent Release
}

fn _unset_fg_color(boxhandler: &mut BoxHandler, box_id: usize) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            bleepsbox.unset_fg_color();
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

#[no_mangle]
pub extern "C" fn unset_fg_color(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    match _unset_fg_color(&mut boxhandler, box_id) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

fn _unset_color(boxhandler: &mut BoxHandler, box_id: usize) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            bleepsbox.unset_color();
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

#[no_mangle]
pub extern "C" fn unset_color(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    match _unset_color(&mut boxhandler, box_id) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

fn _fillc(boxhandler: &mut BoxHandler, box_id: usize, c: *const c_char) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    //assert!(!c.is_null()); TODO: figure out need for this assertion.
    let c_str = unsafe { CStr::from_ptr(c) };
    let string_bytes = match c_str.to_str() {
        Ok(string) => {
            string.as_bytes()
        }
        Err(_e) => return Err(BleepsError::InvalidUtf8)
    };


    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            bleepsbox.fill(string_bytes);
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

#[no_mangle]
pub extern "C" fn fillc(ptr: *mut BoxHandler, box_id: usize, c: *const c_char) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    assert!(!c.is_null());

    let c_str = unsafe { CStr::from_ptr(c) };
    let string_bytes = c_str.to_str().expect("Not a valid UTF-8 string").as_bytes();

    {
        let mut boxes = &mut boxhandler.boxes;
        match boxes.get_mut(&(box_id as usize)) {
            Some(bleepsbox) => {
                bleepsbox.fill(string_bytes);
            }
            None => ()
        };

        match _flag_recache(&mut boxes, box_id) {
            Ok(_) => (),
            Err(e) => panic!(e)
        };
    }

    Box::into_raw(boxhandler); // Prevent Release
}


fn _setc(boxhandler: &mut BoxHandler, box_id: usize, x: usize, y: usize, c: *const c_char) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    //assert!(!c.is_null()); TODO: figure out need for this assertion.
    let c_str = unsafe { CStr::from_ptr(c) };
    let string_bytes = match c_str.to_str() {
        Ok(string) => {
            string.as_bytes()
        }
        Err(_e) => return Err(BleepsError::InvalidUtf8)
    };


    match boxes.get_mut(&(box_id as usize)) {
        Some(bleepsbox) => {
            try!(bleepsbox.set(x as usize, y as usize, string_bytes));
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

#[no_mangle]
pub extern "C" fn setc(ptr: *mut BoxHandler, box_id: usize, x: usize, y: usize, c: *const c_char) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _setc(&mut boxhandler, box_id, x, y, c) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

fn _unsetc(boxhandler: &mut BoxHandler, box_id: usize, x: usize, y: usize) -> Result<(), BleepsError> {
    let mut boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    match boxes.get_mut(&box_id) {
        Some(bleepsbox) => {
            try!(bleepsbox.unset(x as usize, y as usize));
        }
        None => ()
    };

    try!(_flag_recache(&mut boxes, box_id));

    Ok(())
}

#[no_mangle]
pub extern "C" fn unsetc(ptr: *mut BoxHandler, box_id: usize, x: usize, y: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _unsetc(&mut boxhandler, box_id, x, y) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}


fn _removebox(boxhandler: &mut BoxHandler, box_id: usize) -> Result<(), BleepsError> {
    let boxes = &mut boxhandler.boxes;
    {
        let mut removed_ids = try!(_removebox_from_boxes(boxes, box_id));
        boxhandler.open_keys.append(&mut removed_ids);
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn removebox(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _removebox(&mut boxhandler, box_id) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}


fn _newbox(boxhandler: &mut BoxHandler, parent_id: usize, width: usize, height: usize) -> Result<usize, BleepsError> {
    let boxes = &mut boxhandler.boxes;
    // Check that box exists before proceeding
    try!(
        match boxes.get(&parent_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    let id: usize;
    if boxhandler.open_keys.len() > 0 {
        id = boxhandler.open_keys.pop().unwrap();
    } else {
        id = boxhandler.keygen;
        boxhandler.keygen += 1;
    }
    let mut bleepsbox = BleepsBox::new(width, height);

    match boxes.get_mut(&(parent_id as usize)) {
        Some(parent) => {
            parent.box_positions.insert(id, (0, 0));
            parent.boxes.push(id);
            bleepsbox.parent = Some(parent_id);
        }
        None => ()
    };
    boxes.insert(id, bleepsbox);

    Ok(id)
}

#[no_mangle]
pub extern "C" fn newbox(ptr: *mut BoxHandler, parent_id: usize, width: usize, height: usize) -> usize {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    let id: usize;

    match _newbox(&mut boxhandler, parent_id, width, height) {
        Ok(newid) => {
            id = newid;
        }
        Err(_e) => {
            id = 0;
        }
    };

    Box::into_raw(boxhandler); // Prevent Release
    id
}

fn _movebox(boxes: &mut HashMap<usize, BleepsBox>, box_id: usize, x: isize, y: isize) -> Result<(), BleepsError> {
    let mut parent_id: usize = 0;
    let mut found_parent = false;

    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    if boxes.len() > box_id  && box_id > 0 {
        match boxes.get(&box_id) {
            Some(_found) => {
                match _found.parent {
                    Some(pid) => {
                        parent_id = pid;
                        found_parent = true;
                    }
                    None => ()
                };
            }
            None => {
                parent_id = 0;
            }
        };
        if found_parent {
            match boxes.get_mut(&parent_id) {
                Some(parent) => {
                    if let Some(pos) = parent.box_positions.get_mut(&box_id) {
                        *pos = (x, y);
                    }
                }
                None => ()
            };
        }
        try!(_flag_recache(boxes, box_id));
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn movebox(ptr: *mut BoxHandler, box_id: usize, x: isize, y: isize) {

    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _movebox(&mut boxhandler.boxes, box_id, x, y) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

fn _detachbox(boxes: &mut HashMap<usize, BleepsBox>, box_id: usize) -> Result<(), BleepsError> {
    let mut parent_id: usize = 0;
    let mut found_parent = false;

    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    // Need to flag before removing boxes
    try!(_flag_recache(boxes, box_id));

    match boxes.get_mut(&box_id) {
        Some(_found) => {
            match _found.parent {
                Some(pid) => {
                    parent_id = pid;
                    found_parent = true;
                }
                None => ()
            };
            _found.parent = None;
        }
        None => {
            parent_id = 0;
        }
    };

    if found_parent {
        match boxes.get_mut(&parent_id) {
            Some(parent) => {
                parent.box_positions.remove(&box_id);
                let mut m: usize = 0;
                let mut found = false;
                for i in 0..parent.boxes.len() {
                    if parent.boxes[i] == box_id {
                        m = i;
                        found = true;
                        break;
                    }
                }
                if found {
                    parent.boxes.remove(m);
                }
            }
            None => ()
        };
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn detachbox(ptr: *mut BoxHandler, box_id: usize) {

    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _detachbox(&mut boxhandler.boxes, box_id) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

fn _attachbox(boxes: &mut HashMap<usize, BleepsBox>, box_id: usize, parent_id: usize) -> Result<(), BleepsError> {

    // Check that box exists before proceeding
    try!(
        match boxes.get(&box_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    // Check that box exists before proceeding
    try!(
        match boxes.get(&parent_id) {
            Some(_found) => Ok(()),
            None => Err(BleepsError::NotFound)
        }
    );

    // Need to flag before removing boxes
    try!(_flag_recache(boxes, box_id));

    match boxes.get_mut(&box_id) {
        Some(_found) => {
            _found.parent = Some(parent_id);
        }
        None => ()
    };

    match boxes.get_mut(&parent_id) {
        Some(parent) => {
            parent.box_positions.entry(box_id)
                .and_modify(|e| { *e = (0, 0) })
                .or_insert((0, 0));

            parent.boxes.push(box_id);
        }
        None => ()
    };

    Ok(())
}

#[no_mangle]
pub extern "C" fn attachbox(ptr: *mut BoxHandler, box_id: usize, parent_id: usize) {

    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    match _attachbox(&mut boxhandler.boxes, box_id, parent_id) {
        Ok(_) => (),
        Err(e) => panic!(e)
    };

    Box::into_raw(boxhandler); // Prevent Release
}

#[no_mangle]
pub extern "C" fn init(width: usize, height: usize) -> *mut BoxHandler {
    let mut boxhandler = BoxHandler {
        keygen: 1,
        open_keys: Vec::new(),
        boxes: HashMap::new(),
        cached_display: HashMap::new()
    };

    let mut top: BleepsBox = BleepsBox::new(width, height);
    boxhandler.boxes.insert(0, top);



    println!("\x1B[?1049h"); // New screen
    println!("\x1B[?25l"); // Hide Cursor

    Box::into_raw(Box::new(boxhandler))
}

#[no_mangle]
pub extern "C" fn kill(ptr: *mut BoxHandler) {
    let boxhandler = unsafe { Box::from_raw(ptr) };

    println!("\x1B[?25h"); // Show Cursor
    println!("\x1B[?1049l"); // Return to previous screen

    // TODO: Figure out why releasing causes segfault
    Box::into_raw(boxhandler); // Prevent Release
    // Releases boxes
}
