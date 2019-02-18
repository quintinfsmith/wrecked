use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::cmp;
use std::io::{self, Write};


fn write(towrite: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    handle.write(towrite)?;
    Ok(())
}

pub struct BleepsBox {
    boxes: Vec<usize>,
    box_positions: Vec<(isize, isize)>,
    width: usize,
    height: usize,
    grid: Vec<Vec<char>>,
    cached: Vec<((isize, isize), (char, u16))>,
    parent: Option<usize>,
    recache_flag: bool
}

impl BleepsBox {
    fn new(width: usize, height: usize) -> BleepsBox {
        let mut newgrid = Vec::new();
        let mut newrow: Vec<Option<char>>;
        for y in (0 .. height) {
            newrow = Vec::new();
            for x in (0 .. width) {
                newrow.push(None)
            }
            newgrid.push(newrow);
        }
        BleepsBox {
            boxes: Vec::new(),
            box_positions: Vec::new(),
            width: width,
            height: height,
            grid: Vec::new(),
            cached: Vec::new(),
            parent: None,
            recache_flag: true
        }
    }
    fn flag_recache(&mut self) {
        self.recache_flag = true;
    }
    fn set(&mut self, x: usize, y: usize, c: char) {
        while (self.grid.len() <= y) {
            self.grid.push(Vec::new());
        }
        match self.grid.get_mut(y) {
            Some(row) => {
                while (row.len() <= x) {
                    row.push(' ');
                }
                row[x] = c;
            }
            None => ()
        };
    }

    fn get(&self, x: usize, y: usize) -> Option<char> {
        if y < self.grid.len() && x < self.grid.len() {
            Some(self.grid[y][x])
        } else {
            None
        }
    }

    fn get_cached(&self) -> Vec<((isize, isize), (char, u16))> {
        self.cached.clone()
    }

    fn set_cached(&mut self, tocache: &Vec<((isize, isize), (char, u16))>) {
        self.cached = (*tocache).clone();
        self.recache_flag = false;
    }

}

fn rects_intersect(rect_a: (isize, isize, isize, isize), rect_b: (isize, isize, isize, isize)) -> bool {
    // TODO: implement. this is for testing, and will be slow to render every box
    (! (rect_a.0 + rect_a.2 < rect_b.0 || rect_a.0 > rect_b.0 + rect_b.2) && ! (rect_a.1 + rect_a.3 < rect_b.1 || rect_a.1 > rect_b.1 + rect_b.3))
}


fn get_display(box_handler: &mut Vec<BleepsBox>) -> Vec<((isize, isize),(char, u16))> {
    // box id, xoffset, yoffset
    let mut process_stack: Vec<(usize, (isize, isize))> = Vec::new();
    process_stack.push((0, (0, 0)));

    // Amalgamation of BleepsBoxes
    let mut main_disp: Vec<((isize, isize), (char, u16))>;
    main_disp = Vec::new();

    // Content displayed within 'current' BleepsBox
    let mut tmp_disp: Vec<((isize, isize), (char, u16))>;

    let mut used_coords: HashMap<(isize, isize), (char, u16)> = HashMap::new();

    let mut children_stacked: Vec<usize> = Vec::new();

    let mut current_bleepsbox: &mut BleepsBox;
    let mut current_id: usize;
    let mut current_tuple: (usize, (isize, isize));
    let mut current_offset: (isize, isize);

    let mut pos: (isize, isize);
    let mut val: (char, u16);
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
                        pos = (
                            current_offset.0 + current_bleepsbox.box_positions[i].0,
                            current_offset.1 + current_bleepsbox.box_positions[i].1
                        );

                        process_stack.push((current_bleepsbox.boxes[i], pos));
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
                                                tmp_disp.push(((x as isize, y as isize), (value, 0)));
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
    let mut val: (char, u16);
    let mut s;

    for i in 0..top_disp.len() {
        pos = top_disp[i].0;
        val = top_disp[i].1;
        s = format!("\x1B[{};{}H{}", pos.0 + 1, pos.1 + 1, val.0);
        println!("{}", s);
        //write(s.as_bytes());
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


//// printc for testing only
//#[no_mangle]
//pub extern "C" fn printc(ptr: *mut Vec<BleepsBox>, box_id: u32, x: u32, y: u32) {
//    let mut boxes = unsafe { Box::from_raw(ptr) };
//    match boxes.get(box_id as usize) {
//        Some(bleepsbox) => {
//            match bleepsbox.get(x as usize, y as usize) {
//                Some(c) => {
//                    println!("{}", c);
//                }
//                None => ()
//            };
//        }
//        None => ()
//    };
//
//    Box::into_raw(boxes); // Prevent Release
//}

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
pub extern "C" fn setc(ptr: *mut Vec<BleepsBox>, box_id: usize, x: usize, y: usize, c: *const c_char) {
    assert!(!c.is_null());

    let c_str = unsafe { CStr::from_ptr(c) };
    let string = c_str.to_str().expect("Not a valid UTF-8 string");
    let use_c = string.chars().next().unwrap();

    let mut boxes = unsafe { Box::from_raw(ptr) };
    match boxes.get_mut(box_id as usize) {
        Some(bleepsbox) => {
            bleepsbox.set(x as usize, y as usize, use_c);
        }
        None => ()
    };

    _flag_recache(&mut boxes, box_id);

    Box::into_raw(boxes); // Prevent Release
}

#[no_mangle]
pub extern "C" fn newbox(ptr: *mut Vec<BleepsBox>, parent_id: usize) -> usize {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    let id: usize = boxes.len();
    let mut bleepsbox = BleepsBox::new(10, 10);

    if boxes.len() > parent_id as usize {
        {
            let mut parent = &mut boxes[parent_id as usize];
            parent.box_positions.push((0, 0));
            parent.boxes.push(id);
            bleepsbox.parent = Some(parent_id);
        }
        boxes.push(bleepsbox);
    }


    Box::into_raw(boxes); // Prevent Release

    id
}


#[no_mangle]
pub extern "C" fn init() -> *mut Vec<BleepsBox> {
    let mut boxes: Vec<BleepsBox> = Vec::new();
    let top: BleepsBox = BleepsBox::new(10, 10);
    boxes.push(top);

    Box::into_raw(Box::new(boxes))
}

