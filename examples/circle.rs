use wrecked;
use std::f64::consts::PI;
use std::thread;
use std::time::Duration;

// Draws a filled-in circle in the middle of the screen
fn main() {
    let mut rectmanager = wrecked::RectManager::new();
    let mut cx = (rectmanager.get_width() / 2);
    let mut cy = (rectmanager.get_height() / 2);
    let mut L = (std::cmp::min(cx, cy) / 2) as usize;
    let rect = rectmanager.new_rect(wrecked::TOP).ok().unwrap();
    rectmanager.resize(rect, L * 2, L * 2);
    rectmanager.set_position(rect, (cx  - L) as isize, (cy - L) as isize);

    rectmanager.set_fg_color(rect, wrecked::RectColor::BLUE);

    let circ_char = '\\';
    for x in 0 .. L {
        let y_len = (((L * L) - (x * x)) as f64).sqrt() as usize;
        for y in 0 .. y_len {
            rectmanager.set_character(rect, (L + x) as isize, (L - y_len + y) as isize, circ_char);
            rectmanager.set_character(rect, (L - x) as isize, (L - y_len + y) as isize, circ_char);
            rectmanager.set_character(rect, (L + x) as isize, (L + y) as isize, circ_char);
            rectmanager.set_character(rect, (L - x) as isize, (L + y) as isize, circ_char);
        }
    }
    rectmanager.draw();
    thread::sleep(Duration::new(3,0));

    rectmanager.kill();
}
