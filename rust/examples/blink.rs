use wrecked;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), wrecked::WreckedError> {
    let mut rectmanager = wrecked::RectManager::new();
    let (width, height) = rectmanager.get_rect_size(wrecked::ROOT).unwrap();
    let blinker = rectmanager.new_rect(wrecked::ROOT)?;
    rectmanager.set_fg_color(blinker, wrecked::Color::RED)?;
    rectmanager.set_bg_color(blinker, wrecked::Color::WHITE)?;
    rectmanager.resize(blinker, width / 2, height / 2)?;
    rectmanager.set_position(blinker, (width / 4) as isize, (height / 4) as isize)?;
    rectmanager.set_string(blinker, (width / 4) as isize - 3, 2, "BLINK!")?;

    for i in 0 .. 54 {
        if i % 2 == 0 {
            rectmanager.disable(blinker)?;
        } else {
            rectmanager.enable(blinker)?;
        }
        rectmanager.render()?;
        thread::sleep(Duration::new(0, 100_000_000));
    }

    rectmanager.kill()?;
    Ok(())
}
