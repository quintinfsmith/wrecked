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

pub struct BleepsBox {
    boxes: Vec<usize>,
    box_positions: HashMap<usize, (isize, isize)>,
    width: usize,
    height: usize,
    grid: Vec<Vec<[u8; 4]>>,
    cached: Vec<((isize, isize), ([u8; 4], u16))>,
    parent: Option<usize>,

    recache_flag: bool,
    color: u16, // 7: USE FG, 6-4: FG, 3: USEBG, 2-0:BG
}

impl BleepsBox {
    fn new(width: usize, height: usize) -> BleepsBox {
        let mut newgrid = Vec::new();
        let mut newrow: Vec<Option<[u8; 4]>>;
        for y in (0 .. height) {
            newrow = Vec::new();
            for x in (0 .. width) {
                newrow.push(None)
            }
            newgrid.push(newrow);
        }
        BleepsBox {
            boxes: Vec::new(),
            box_positions: HashMap::new(),
            width: width,
            height: height,
            grid: Vec::new(),
            cached: Vec::new(),
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
        while (self.grid.len() <= y) {
            self.grid.push(Vec::new());
        }
        match self.grid.get_mut(y) {
            Some(row) => {
                while (row.len() <= x) {
                    row.push([0u8; 4]);
                }
                row[x] = new_c;
            }
            None => ()
        };
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

    fn get(&self, x: usize, y: usize) -> Option<[u8; 4]> {
        if y < self.grid.len() && x < self.grid[y].len() {
            Some(self.grid[y][x])
        } else {
            None
        }
    }

    fn get_cached(&self) -> Vec<((isize, isize), ([u8; 4], u16))> {
        self.cached.clone()
    }

    fn set_cached(&mut self, tocache: &Vec<((isize, isize), ([u8; 4], u16))>) {
        self.cached = (*tocache).clone();
        self.recache_flag = false;
    }

}

fn rects_intersect(rect_a: (isize, isize, isize, isize), rect_b: (isize, isize, isize, isize)) -> bool {
    // TODO: implement. this is for testing, and will be slow to render every box
    (! (rect_a.0 + rect_a.2 < rect_b.0 || rect_a.0 > rect_b.0 + rect_b.2) && ! (rect_a.1 + rect_a.3 < rect_b.1 || rect_a.1 > rect_b.1 + rect_b.3))
}


fn get_display(box_handler: &mut Vec<BleepsBox>) -> Vec<((isize, isize),([u8; 4], u16))> {
    // box id, xoffset, yoffset
    let mut process_stack: Vec<(usize, (isize, isize))> = Vec::new();
    process_stack.push((0, (0, 0)));
    // Amalgamation of BleepsBoxes
    let mut main_disp: Vec<((isize, isize), ([u8; 4], u16))>;
    main_disp = Vec::new();

    // Content displayed within 'current' BleepsBox
    let mut tmp_disp: Vec<((isize, isize), ([u8; 4], u16))>;


    let mut used_coords: HashMap<(isize, isize), ([u8; 4], u16)> = HashMap::new();
    let mut children_stacked: Vec<usize> = Vec::new();


    let mut current_bleepsbox: &mut BleepsBox;
    let mut current_id: usize;
    let mut current_tuple: (usize, (isize, isize));
    let mut current_offset: (isize, isize);

    let mut pos: (isize, isize);
    let mut val: ([u8; 4], u16);
    let mut new_position: (isize, isize);

    while process_stack.len() > 0 {
        current_tuple = process_stack.pop().unwrap();
        current_id = current_tuple.0;
        current_offset = current_tuple.1;
        match box_handler.get_mut(current_id) {
            Some(current_bleepsbox) => {
                // If the display has been cached on this box, don't descend
                if current_bleepsbox.recache_flag && current_bleepsbox.boxes.len() > 0 && ! children_stacked.contains(&current_id) {
                    // Reinsert id to stack
                    process_stack.push(current_tuple);

                    // Add Children to stack
                    for i in 0..current_bleepsbox.boxes.len() {
                        let mut child_id = current_bleepsbox.boxes[i];
                        pos = (
                                current_offset.0 + current_bleepsbox.box_positions[&child_id].0,
                                current_offset.1 + current_bleepsbox.box_positions[&child_id].1
                              );

                        process_stack.push((child_id, pos));
                    }

                    // Mark that children have been added to stack
                    children_stacked.push(current_id);
                } else {
                    if (current_bleepsbox.recache_flag) {
                        tmp_disp = Vec::new();
                        // TODO: Check if coordinate is on screen
                        for y in 0..current_bleepsbox.height {
                            for x in 0..current_bleepsbox.width {
                                let mut real_pos = ((x as isize + current_offset.0) as isize, (y as isize + current_offset.1) as isize);
                                match used_coords.get(&real_pos) {
                                    Some(_found) => {
                                        tmp_disp.push((real_pos, (_found.0, _found.1))); // TODO: Will Fail
                                    },
                                        None => {
                                            match current_bleepsbox.get(x, y) {
                                                Some(value) => {
                                                    tmp_disp.push(((x as isize, y as isize), (value, current_bleepsbox.color as u16)));
                                                }
                                                None => ()
                                            };
                                        }
                                };
                            }
                        }
                        current_bleepsbox.set_cached(&tmp_disp);
                    }

                    for i in 0..current_bleepsbox.cached.len() {
                        pos = current_bleepsbox.cached[i].0;
                        val = current_bleepsbox.cached[i].1;
                        new_position = ((pos.0 + current_offset.0) as isize, (pos.1 + current_offset.1) as isize);
                        if ! used_coords.contains_key(&new_position) {
                            used_coords.insert(new_position, val);
                            main_disp.push((new_position, val));
                        }
                    }
                }
            }
            None => ()
        };
    }
    main_disp
}

fn _draw_boxes(boxes: &mut Vec<BleepsBox>) {
    let top_disp = get_display(boxes);
    let mut pos: (isize, isize);
    let mut val_a: &[u8];
    let mut val_b: u16;
    let mut s;
    let mut utf_char: &[u8];
    let mut utf_char_split_index: usize;

    for i in 0..top_disp.len() {
        pos = top_disp[i].0;
        val_a = &((top_disp[i].1).0);
        val_b = (top_disp[i].1).1;
        s = format!("\x1B[{};{}H", pos.1 + 1, pos.0 + 1);
        print!("{}", s);
        if (val_b >> 5) & 16 == 16 {
            if (val_b >> 5) & 8 == 8 {
                s = format!("\x1B[9{}m", ((val_b >> 5) & 7));
                print!("{}", s);
            } else {
                s = format!("\x1B[3{}m", ((val_b >> 5) & 7));
                print!("{}", s);
            }
        }

        if val_b & 16 == 16 {
            if (val_b & 8 == 8) {
                s = format!("\x1B[10{}m", (val_b & 7));
                print!("{}", s);
            } else {
                s = format!("\x1B[4{}m", (val_b & 7));
                print!("{}", s);
            }
        }

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
    print!("\n");
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
pub extern "C" fn draw(ptr: *mut Vec<BleepsBox>) {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    _draw_boxes(&mut boxes);

    Box::into_raw(boxes); // Prevent Release
}


#[no_mangle]
pub extern "C" fn flag_recache(ptr: *mut Vec<BleepsBox>, box_id: usize) {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    _flag_recache(&mut boxes, box_id);

    Box::into_raw(boxes); // Prevent Release
}


#[no_mangle]
pub extern "C" fn set_fg_color(ptr: *mut Vec<BleepsBox>, box_id: usize, col: u8) {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    match boxes.get_mut(box_id as usize) {
        Some(bleepsbox) => {
            bleepsbox.set_fg_color(col);
        }
        None => ()
    };

    _flag_recache(&mut boxes, box_id);

    Box::into_raw(boxes); // Prevent Release
}


#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut Vec<BleepsBox>, box_id: usize, col: u8) {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    match boxes.get_mut(box_id as usize) {
        Some(bleepsbox) => {
            bleepsbox.set_bg_color(col);
        }
        None => ()
    };

    _flag_recache(&mut boxes, box_id);

    Box::into_raw(boxes); // Prevent Release
}


#[no_mangle]
pub extern "C" fn setc(ptr: *mut Vec<BleepsBox>, box_id: usize, x: usize, y: usize, c: *const c_char) {
    assert!(!c.is_null());

    let c_str = unsafe { CStr::from_ptr(c) };
    let string_bytes = c_str.to_str().expect("Not a valid UTF-8 string").as_bytes();

    let mut boxes = unsafe { Box::from_raw(ptr) };
    match boxes.get_mut(box_id as usize) {
        Some(bleepsbox) => {
            bleepsbox.set(x as usize, y as usize, string_bytes);
        }
        None => ()
    };

    _flag_recache(&mut boxes, box_id);

    Box::into_raw(boxes); // Prevent Release
}

#[no_mangle]
pub extern "C" fn newbox(ptr: *mut Vec<BleepsBox>, parent_id: usize, width: usize, height: usize) -> usize {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    let id: usize = boxes.len();
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

    Box::into_raw(boxes); // Prevent Release

    id
}

#[no_mangle]
pub extern "C" fn movebox(ptr: *mut Vec<BleepsBox>, box_id: usize, x: isize, y: isize) {

    let mut boxes = unsafe { Box::from_raw(ptr) };

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
                parent.flag_recache();
                if let Some(pos) = parent.box_positions.get_mut(&box_id) {
                    *pos = (x, y);
                }
            }
            None => ()
        }
    }

    Box::into_raw(boxes); // Prevent Release
}


#[no_mangle]
pub extern "C" fn init(width: usize, height: usize) -> *mut Vec<BleepsBox> {
    let mut boxes: Vec<BleepsBox> = Vec::new();
    let top: BleepsBox = BleepsBox::new(width, height);
    boxes.push(top);

    println!("\x1B[?1049h"); // New screen
    println!("\x1B[?25l"); // Hide Cursor

    Box::into_raw(Box::new(boxes))
}

#[no_mangle]
pub extern "C" fn kill(ptr: *mut Vec<BleepsBox>) {
    let mut boxes = unsafe { Box::from_raw(ptr) };

    println!("\x1B[?25h"); // Show Cursor
    println!("\x1B[?1049l"); // Return to previous screen

    // Releases boxes
}
