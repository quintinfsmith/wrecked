use std::ffi::CStr;
use std::os::raw::c_char;
use std::collections::HashMap;
use std::cmp;

fn rects_intersect(rect_a: (usize, usize, usize, usize), rect_b: (usize, usize, usize, usize)) -> bool {
    // TODO: implement. this is for testing, and will be slow to render every box
    (! (rect_a.0 + rect_a.2 < rect_b.0 || rect_a.0 > rect_b.0 + rect_b.2) && ! (rect_a.1 + rect_a.3 < rect_b.1 || rect_a.1 > rect_b.1 + rect_b.3))
}

struct globals {
    asciibox_id: u32,
    asciiboxes: Box<HashMap<u32, AsciiBox>>
}

struct AsciiBox {
    id: u32,
    children: Box<HashMap<u32, AsciiBox>>,
    child_positions: Box<HashMap<u32, (usize, usize)>>,
    width: usize,
    height: usize,
    grid: Box<Vec<Vec<char>>>
}

impl AsciiBox {
    fn new(new_width: usize, new_height: usize) -> AsciiBox {

        let children: HashMap<u32, AsciiBox> = HashMap::new();
        let child_positions: HashMap<u32, (usize, usize)> = HashMap::new();

        let mut rows = Vec::new();

        for y in 0..new_height {
            rows.push(Vec::new());
            match rows.last_mut() {
                Some(v) => {
                    for x in 0..new_width {
                        v.push('\x00')
                    }
                },
                None => ()
            }
        }

        let mut new_id: u32;
        unsafe {
            new_id = ASCII_ID;
            ASCII_ID += 1;
        }

        let mut newbox = AsciiBox {
            children: Box::new(children),
            child_positions: Box::new(child_positions),
            id: new_id,
            width: new_width,
            height: new_height,
            grid: Box::new(rows)
        };

        unsafe {
            ASCIIBOXES.insert(new_id, &newbox);
        }
        newbox
    }

    fn new_child(&mut self, x: usize, y: usize, width: usize, height: usize) -> u32 {
        let mut child = AsciiBox::new(width, height);
        let child_id = child.get_id();
        child.set_id(child_id);
        self.children.insert(child_id, child);
        self.child_positions.insert(child_id, (x, y));
        child_id

    }

    fn get_child(&mut self, cid: u32) -> Option<&mut AsciiBox> {
        self.children.get_mut(&cid)
    }

    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_cell(self, x: usize, y: usize) -> char {
        self.grid[y][x]
    }

    fn get_height(&self) -> usize {
        self.height
    }

    fn get_width(&self) -> usize {
        self.width
    }

    fn set_id(&mut self, id: u32) {
        self.id = id;
    }
    fn set_cell(&mut self, x: usize, y: usize, val: char) {
        self.grid[y][x] = val;
    }

    fn get_visible_grid(&self, offset: (usize, usize), bounds: (usize, usize, usize, usize)) -> Vec<(usize, usize, char)> {
        // bound_x & bound_y may be negative values, and effectly be unbound

        let mut output: Vec<(usize, usize, char)> =  Vec::new();
        let real_x = offset.0;
        let real_y = offset.1;

        let width = self.width;
        let height = self.height;

        let mut subgrid: Vec<(usize, usize, char)>;
        let mut sub_x: usize;
        let mut sub_y: usize;


        let mut ny: usize;
        let mut nx: usize;
        for y in 0 .. height {
            ny = real_y + y;
            if ny < bounds.1 || ny > bounds.3 {
                continue;
            }
            for x in 0 .. width {
                nx = x + real_x;
                if nx < bounds.0 || nx > bounds.2 {
                    continue
                }
                if self.grid[y][x] != '\x00' {
                    output.push((nx, ny, self.grid[y][x]));
                }
            }
        }

        for (child_id, child) in self.children.iter() {
            sub_x = real_x + self.child_positions[child_id].0;
            sub_y = real_y + self.child_positions[child_id].1;
            if (rects_intersect((sub_x, sub_y, child.width, child.height), bounds)) {
                subgrid = child.get_visible_grid((sub_x, sub_y), bounds);
                for sub in subgrid.iter() {
                    output.push(*sub);
                }
            }
        }



        output
    }

    fn display(&self) {
        print!("\x1b[0;0H");

        let width = self.width;
        let height = self.height;
        let cells = self.get_visible_grid((0,0), (0, 0, width, height));
        let mut grid: Vec<Vec<char>> = Vec::new();

        for y in 0..height {
            grid.push(Vec::new());
            for x in 0..width {
                grid[y].push(' ');
            }
        }

        for (x, y, c) in cells {
            grid[y][x] = c;
        }

        for y in 0..height {
            for x in 0..width {
                print!("{}", grid[y][x]);
            }
            print!("\n");
        }
    }

}

static mut ASCII_ID: u32 = 0;
static mut ASCIIBOXES: HashMap<u32, &AsciiBox> = HashMap::new();


#[no_mangle]
pub extern fn init() {
    unsafe {
        static mut ASCII_ID: u32 = 0;
        static mut ASCIIBOXES: HashMap<u32, AsciiBox> = HashMap::new();
    }
}

#[no_mangle]
pub extern fn testfunc() {
    let mut ab = AsciiBox::new(20, 20);
    let mut height = ab.get_height() - 1;
    let mut width = ab.get_width();
    for y in 0..20 {
        for x in 0..20 {
            ab.set_cell(x, y, 'X');
        }
    }

    let cid = ab.new_child(10, 10, 5, 5);

    match ab.get_child(cid) {
        Some(cc) => {
            for y in 0..5 {
                for x in 0..5 {
                    if x != y {
                        cc.set_cell(x, y, '-');
                    }
                }
            }
        },
        None => ()
    };

    ab.display();
}

