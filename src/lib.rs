use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::cmp;
use std::io::{self, Write};
use std::slice;
use std::str;

fn write(towrite: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    handle.write(towrite)?;
    Ok(())
}

pub struct BoxHandler {
    boxes: Vec<BleepsBox>,
    cached_display: Vec<((usize, usize), ([u8; 4], u16))>
}

pub struct BleepsBox {
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
    fn set(&mut self, x: usize, y: usize, c: &[u8]) {
        let mut new_c: [u8; 4] = [0; 4];
        for i in 0..c.len() {
            new_c[(4 - c.len()) + i] = c[i]; // Put the 0 offset first
        }

        self.grid.entry((x, y))
            .and_modify(|e| { *e = new_c })
            .or_insert(new_c);
    }

    fn set_bg_color(&mut self, n: u8) {
        let mut modded_n: u16 = n as u16;
        modded_n &= 0b01111;
        modded_n |= 0b10000;
        self.color |= modded_n;
    }

    fn set_fg_color(&mut self, n: u8) {
        let mut modded_n: u16 = n as u16;
        modded_n &= 0b01111;
        modded_n |= 0b10000;
        self.color |= (modded_n << 5)
    }

    fn get(&self, x: usize, y: usize) -> Option<&[u8; 4]> {
        self.grid.get(&(x, y))
    }

    fn get_cached(&self) -> &HashMap<(isize, isize), ([u8; 4], u16)> {
        &self.cached
    }

    fn set_cached(&mut self, tocache: &HashMap<(isize, isize), ([u8; 4], u16)>) {
        self.cached = (*tocache).clone();
        self.recache_flag = false;
    }

}

fn rects_intersect(rect_a: (isize, isize, isize, isize), rect_b: (isize, isize, isize, isize)) -> bool {
    // TODO: implement. this is for testing, and will be slow to render every box
    (! (rect_a.0 + rect_a.2 < rect_b.0 || rect_a.0 > rect_b.0 + rect_b.2) && ! (rect_a.1 + rect_a.3 < rect_b.1 || rect_a.1 > rect_b.1 + rect_b.3))
}


fn get_display(boxes: &mut Vec<BleepsBox>, box_id: usize, offset: (isize, isize), frame: (isize, isize, isize, isize)) -> HashMap<(isize, isize), ([u8; 4], u16)> {
    let mut output: HashMap<(isize, isize), ([u8; 4], u16)>;
    let mut subboxes: Vec<((isize, isize), usize)> = Vec::new();

    let mut descend = false;

    output = HashMap::new();

    match boxes.get(box_id) {
        Some(bleepsbox) => {
            if (rects_intersect(frame, (offset.0, offset.1, bleepsbox.width as isize, bleepsbox.height as isize))) {
                if (bleepsbox.recache_flag) {
                    descend = true;

                    for (position, character) in bleepsbox.grid.iter() {
                        output.insert((position.0 as isize, position.1 as isize), (*character, bleepsbox.color));
                    }

                    let mut subbox_offset: (isize, isize);
                    for subbox_id in bleepsbox.boxes.iter() {
                        //TODO: Don't use unwrap
                        subbox_offset = *bleepsbox.box_positions.get(&subbox_id).unwrap();
                        subboxes.push((subbox_offset, *subbox_id));
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

    if (descend) {
        let mut subbox_output: HashMap<(isize, isize), ([u8; 4], u16)>;
        let mut tmp_value: &([u8; 4], u16);
        for (subbox_offset, subbox_id) in subboxes.iter() {
            subbox_output = get_display(boxes, *subbox_id, (offset.0 + subbox_offset.0, subbox_offset.1 + offset.1), frame);
            for (subpos, value) in subbox_output.iter() {
                *output.entry((subpos.0 + subbox_offset.0, subpos.1 + subbox_offset.1)).or_insert(*value) = *value;
            }
        }
        boxes[box_id].set_cached(&output);
    }


    output
}


fn _draw_boxes(boxhandler: &mut BoxHandler) {

    let mut boxes = &mut boxhandler.boxes;
    {
        let mut width = boxes[0].width as isize;
        let mut height = boxes[0].height as isize;
        let top_disp = get_display(boxes, 0, (0, 0), (0, 0, width, height));
        let mut val_a: &[u8];
        let mut val_b: u16;
        let mut s;
        let mut utf_char: &[u8];
        let mut utf_char_split_index: usize;

        // TODO: This is a **very** shit display algorithm
        // Should first sort, then display in as few print calls
        // as possible
        for (pos, val) in top_disp.iter() {

            val_a = &val.0;
            val_b = val.1;
            s = format!("\x1B[{};{}H", pos.1 + 1, pos.0 + 1);
            print!("{}", s);

            if (val_b >> 5) & 16 == 16 {
                if (val_b >> 5) & 8 == 8 {
                    s = format!("\x1B[9{}m", ((val_b >> 5) & 7));
                } else {
                    s = format!("\x1B[3{}m", ((val_b >> 5) & 7));
                }
            }
            print!("{}", s);

            if val_b & 16 == 16 {
                if (val_b & 8 == 8) {
                    s = format!("\x1B[10{}m", (val_b & 7));
                } else {
                    s = format!("\x1B[4{}m", (val_b & 7));
                }
            }
            print!("{}", s);


            utf_char_split_index = 0;
            for i in 0..4 {
                if (val_a[i] != 0) {
                    utf_char_split_index = i;
                    break;
                }
            }

            utf_char = val_a.split_at(utf_char_split_index).1;

            s = format!("{}\x1B[0m", str::from_utf8(utf_char).unwrap());
            print!("{}", s);
        }
        print!("\r");
    }
}

fn _flag_recache(boxes: &mut Vec<BleepsBox>, box_id: usize) {
    let mut next_box_id: usize = box_id as usize;
    let mut prev_box_id: usize = 0;

    let mut do_next = true;
    while do_next {
        prev_box_id = next_box_id;
        match boxes.get_mut(next_box_id) {
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
}


#[no_mangle]
pub extern "C" fn draw(ptr: *mut BoxHandler) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    _draw_boxes(&mut boxhandler);

    Box::into_raw(boxhandler); // Prevent Release
}


#[no_mangle]
pub extern "C" fn flag_recache(ptr: *mut BoxHandler, box_id: usize) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    {
        let mut boxes = &mut boxhandler.boxes;
        _flag_recache(&mut boxes, box_id);
    }
    Box::into_raw(boxhandler); // Prevent Release
}

#[no_mangle]
pub extern "C" fn set_fg_color(ptr: *mut BoxHandler, box_id: usize, col: u8) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    {
        let mut boxes = &mut boxhandler.boxes;
        match boxes.get_mut(box_id as usize) {
            Some(bleepsbox) => {
                bleepsbox.set_fg_color(col);
            }
            None => ()
        };

        _flag_recache(&mut boxes, box_id);
    }

    Box::into_raw(boxhandler); // Prevent Release
}

#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut BoxHandler, box_id: usize, col: u8) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    {
        let mut boxes = &mut boxhandler.boxes;
        match boxes.get_mut(box_id as usize) {
            Some(bleepsbox) => {
                bleepsbox.set_bg_color(col);
            }
            None => ()
        };

        _flag_recache(&mut boxes, box_id);
    }

    Box::into_raw(boxhandler); // Prevent Release
}


#[no_mangle]
pub extern "C" fn setc(ptr: *mut BoxHandler, box_id: usize, x: usize, y: usize, c: *const c_char) {
    assert!(!c.is_null());

    let c_str = unsafe { CStr::from_ptr(c) };
    let string_bytes = c_str.to_str().expect("Not a valid UTF-8 string").as_bytes();

    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    {
        let mut boxes = &mut boxhandler.boxes;
        match boxes.get_mut(box_id as usize) {
            Some(bleepsbox) => {
                bleepsbox.set(x as usize, y as usize, string_bytes);
            }
            None => ()
        };

        _flag_recache(&mut boxes, box_id);
    }

    Box::into_raw(boxhandler); // Prevent Release
}

#[no_mangle]
pub extern "C" fn newbox(ptr: *mut BoxHandler, parent_id: usize, width: usize, height: usize) -> usize {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };
    let id: usize;
    {
        let mut boxes = &mut boxhandler.boxes;
        id = boxes.len();
        let mut bleepsbox = BleepsBox::new(width, height);

        if boxes.len() > parent_id {
            {
                let mut parent = &mut boxes[parent_id as usize];
                parent.box_positions.insert(id, (0, 0));
                parent.boxes.push(id);
                bleepsbox.parent = Some(parent_id);
            }
            boxes.push(bleepsbox);
        }

    }

    Box::into_raw(boxhandler); // Prevent Release

    id
}

#[no_mangle]
pub extern "C" fn movebox(ptr: *mut BoxHandler, box_id: usize, x: isize, y: isize) {

    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    {
        let mut boxes = &mut boxhandler.boxes;

        let parent_id: usize;

        if boxes.len() > box_id  && box_id > 0 {
            match boxes.get(box_id) {
                Some(_found) => {
                    parent_id = _found.parent.unwrap();
                }
                None => {
                    parent_id = 0;
                }
            };
            match boxes.get_mut(parent_id) {
                Some(parent) => {
                    if let Some(pos) = parent.box_positions.get_mut(&box_id) {
                        *pos = (x, y);
                    }
                }
                None => ()
            }
        }
    }
    _flag_recache(&mut boxhandler.boxes, box_id);

    Box::into_raw(boxhandler); // Prevent Release
}


#[no_mangle]
pub extern "C" fn init(width: usize, height: usize) -> *mut BoxHandler {
    let mut boxhandler = BoxHandler {
        boxes: Vec::new(),
        cached_display: Vec::new()
    };

    let top: BleepsBox = BleepsBox::new(width, height);
    boxhandler.boxes.push(top);

    println!("\x1B[?1049h"); // New screen
    println!("\x1B[?25l"); // Hide Cursor

    Box::into_raw(Box::new(boxhandler))
}

#[no_mangle]
pub extern "C" fn kill(ptr: *mut BoxHandler) {
    let mut boxhandler = unsafe { Box::from_raw(ptr) };

    println!("\x1B[?25h"); // Show Cursor
    println!("\x1B[?1049l"); // Return to previous screen

    // Releases boxes
}
