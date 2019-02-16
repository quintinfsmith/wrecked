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
    boxes: Vec<BleepsBox>,
    box_positions: Vec<(isize, isize)>,
    width: usize,
    height: usize,
    grid: Vec<Vec<char>>,
    parent: Option<u32>,
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
            parent: None,
            recache_flag: true
        }
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

    fn get(&self, x: usize, y: usize) -> char {
        self.grid[y][x]
    }

    fn get_display(&self) -> Vec<((isize, isize),(char, u16))> {
        let mut disp: Vec<((isize, isize), (char, u16))> = Vec::new();

        let mut used_coords: HashMap<(isize, isize), bool> = HashMap::new();

        for i in 0..self.boxes.len() {
            let mut subbox = &self.boxes[i];
            let mut subbox_pos = self.box_positions[i];
            let mut subdisp = subbox.get_display();
            for j in 0..subdisp.len() {
                let mut entry = subdisp[j];
                let pos = entry.0;
                let val = entry.1;
                let new_pos = (pos.0 + subbox_pos.0, pos.1 + subbox_pos.1);
                used_coords.insert(new_pos, true);
                disp.push((new_pos, val));
            }
        }

        // TODO: Check if this box is on the screen


        // TODO: Check if coordinate is on screen
        for y in 0..self.height {
            for x in 0..self.width {
                match used_coords.get(&(x as isize, y as isize)) {
                    Some(found) => (),
                    None => {
                        disp.push(((x as isize, y as isize), (self.grid[y][x], 0)));
                    }
                };
            }
        }

        disp

    }

}


fn rects_intersect(rect_a: (isize, isize, isize, isize), rect_b: (isize, isize, isize, isize)) -> bool {
    // TODO: implement. this is for testing, and will be slow to render every box
    (! (rect_a.0 + rect_a.2 < rect_b.0 || rect_a.0 > rect_b.0 + rect_b.2) && ! (rect_a.1 + rect_a.3 < rect_b.1 || rect_a.1 > rect_b.1 + rect_b.3))
}

// printc for testing only
#[no_mangle]
pub extern "C" fn printc(ptr: *mut Vec<BleepsBox>, box_id: u32, x: u32, y: u32) {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    match boxes.get(box_id as usize) {
        Some(bleepsbox) => {
            println!("{}", bleepsbox.get(x as usize, y as usize));
        }
        None => ()
    };

    Box::into_raw(boxes); // Prevent Release
}


//fn draw_screen(boxes: Vec<BleepsBox>) {
//    let top = boxes[0];
//}


//#[no_mangle]
//pub extern "C" fn flag_recache(ptr: *mut Vec<BleepsBox>, box_id: u32) {
//    let mut boxes = unsafe { Box::from_raw(ptr) };
//    let mut bleepsbox = boxes[box_id as usize];
//    bleepsbox.flag_recache();
//
//    Box::into_raw(boxes); // Prevent Release
//}

#[no_mangle]
pub extern "C" fn setc(ptr: *mut Vec<BleepsBox>, box_id: u32, x: u32, y: u32, c: *const c_char) {
    assert!(!c.is_null());

    let c_str = unsafe { CStr::from_ptr(c) };
    let string = c_str.to_str().expect("Not a valid UTF-8 string");
    let use_c = string.chars().next().unwrap();
    println!("{}", use_c);

    let mut boxes = unsafe { Box::from_raw(ptr) };
    match boxes.get_mut(box_id as usize) {
        Some(bleepsbox) => {
            bleepsbox.set(x as usize, y as usize, use_c);
        }
        None => ()
    };

    Box::into_raw(boxes); // Prevent Release
}

#[no_mangle]
pub extern "C" fn newbox(ptr: *mut Vec<BleepsBox>, parent_id: u32) -> u32 {
    let mut boxes = unsafe { Box::from_raw(ptr) };
    let id: u32 = boxes.len() as u32;
    let mut bleepsbox = BleepsBox::new(10, 10);

    match boxes.get_mut(parent_id as usize) {
        Some(parent) => {
            parent.box_positions.push((0, 0));
            bleepsbox.parent = Some(id);
        }
        None => ()
    }
    boxes.push(bleepsbox);

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

