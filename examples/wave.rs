use wrecked;
use std::f64::consts::PI;
use std::thread;
use std::time::Duration;

fn main() {
    let mut rectmanager = wrecked::RectManager::new();
    let (width, height) = rectmanager.get_rect_size(wrecked::TOP).unwrap();
    let mut points = vec![];
    for x in 0 .. width {
        let rect_id = rectmanager.new_rect(wrecked::TOP).ok().unwrap();
        rectmanager.set_bg_color(rect_id, wrecked::RectColor::YELLOW);
        rectmanager.set_character(rect_id, 0, 0, ' ');
        points.push(rect_id);
    }

    rectmanager.draw();

    for x in 0 .. ((width * width)) {
        let rect_id = points[x % width];
        let y = (height as isize / 2 as isize) +
            ((2f64*PI * (x as f64 / (width - 1) as f64)).sin() * (height / 3) as f64) as isize;
        rectmanager.set_position(rect_id, (x % width) as isize, y);
        rectmanager.draw_rect(rect_id);
        thread::sleep(Duration::new(0, 50000));
    }


    for rect in points.iter() {
        rectmanager.set_bg_color(*rect, wrecked::RectColor::BLUE);
    }

    for x in 0 .. ((width * width)) {
        let rect_id = points[x % width];
        let y = (height as isize / 2 as isize) +
            ((2f64*PI * (x as f64 / (width - 1) as f64)).sin() * (height / 3) as f64) as isize;
        rectmanager.set_position(rect_id, (x % width) as isize, y);
        rectmanager.draw();
        thread::sleep(Duration::new(0, 50000));
    }
    thread::sleep(Duration::new(1, 0));

    rectmanager.kill();
}
