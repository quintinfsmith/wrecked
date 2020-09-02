#[cfg (test)]
use super::*;
use std::{thread, time};

#[test]
fn test_init() {
    let mut rectmanager = RectManager::new();
    let rect_width = rectmanager.get_rect_width(0);
    let rect_height = rectmanager.get_rect_height(0);
    match terminal_size() {
        Some((Width(w), Height(h))) => {
            assert_eq!(rect_width, w as usize);
            assert_eq!(rect_height, h as usize);
        }
        None => { }
    }

    rectmanager.kill()
}

#[test]
fn test_resize() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    let (subwidth, subheight) = (20, 20);
    rectmanager.resize(subrect_id, subwidth, subheight);

    match rectmanager.get_rect(subrect_id) {
        Ok(subrect) => {
            assert_eq!(subrect.width, subwidth);
            assert_eq!(subrect.height, subheight);
        }
        Err(e) => {
            assert!(false);
        }
    }
}

#[test]
fn test_add_rect() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert_eq!(rect.children.len(), 1);
            assert!(rect.has_child(subrect_id));
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.delete_rect(subrect_id);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert_eq!(rect.children.len(), 0);
        }
        Err(e) => {
            assert!(false);
        }
    }


    rectmanager.kill();
}

#[test]
fn test_detach() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    let subsubrect_id = rectmanager.new_rect(Some(subrect_id));
    rectmanager.detach(subrect_id);

    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert_eq!(rect.children.len(), 0);
        }
        Err(e) => {
            assert!(false);
        }
    }

    match rectmanager.get_rect(subrect_id) {
        Ok(subrect) => {
            assert!(true);
        }
        Err(e) => {
            assert!(false);
        }
    }

    match rectmanager.get_rect(subsubrect_id) {
        Ok(subrect) => {
            assert!(true);
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.kill();
}

#[test]
fn test_delete() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    let subsubrect_id = rectmanager.new_rect(Some(subrect_id));

    rectmanager.delete_rect(subrect_id);

    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert_eq!(rect.children.len(), 0);
        }
        Err(e) => {
            assert!(false);
        }
    }

    match rectmanager.get_rect(subrect_id) {
        Ok(subrect) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e as usize, RectError::NotFound as usize);
        }
    }

    match rectmanager.get_rect(subsubrect_id) {
        Ok(subrect) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e as usize, RectError::NotFound as usize);
        }
    }


    rectmanager.kill();
}


#[test]
fn test_move() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    let subsubrect_id = rectmanager.new_rect(Some(subrect_id));

    rectmanager.resize(subrect_id, 40, 40);
    rectmanager.resize(subsubrect_id, 10, 10);
    rectmanager.set_position(subrect_id, 10, 10);
    rectmanager.set_position(subsubrect_id, 10, 10);

    match rectmanager.get_relative_offset(subrect_id) {
        Ok((x, y)) => {
            assert_eq!(x, 10);
            assert_eq!(y, 10);
        }
        Err(e) => {
            assert!(false);
        }
    }

    match rectmanager.get_relative_offset(subsubrect_id) {
        Ok((x, y)) => {
            assert_eq!(x, 10);
            assert_eq!(y, 10);
        }
        Err(e) => {
            assert!(false);
        }
    }

    match rectmanager.get_absolute_offset(subsubrect_id) {
        Ok((x, y)) => {
            assert_eq!(x, 20);
            assert_eq!(y, 20);
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.kill();
}

#[test]
fn test_get_parent() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    let subsubrect_id = rectmanager.new_rect(Some(subrect_id));
    match rectmanager.get_parent(subsubrect_id) {
        Ok(rect) => {
            assert_eq!(rect.rect_id, subrect_id);
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.kill();
}

#[test]
fn test_disable_enable() {
    let mut rectmanager = RectManager::new();

    // non-existant rects should return false
    assert!(! rectmanager.is_rect_enabled(99));

    let subrect_id = rectmanager.new_rect(Some(0));

    rectmanager.disable(subrect_id);
    assert!(! rectmanager.is_rect_enabled(subrect_id));

    rectmanager.enable(subrect_id);
    assert!(rectmanager.is_rect_enabled(subrect_id));

    rectmanager.kill();
}

#[test]
fn test_set_character() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    let test_character = 'Q';
    rectmanager.resize(subrect_id, 10, 10);
    assert!(rectmanager.set_character(subrect_id, 4, 4, test_character).is_ok());
    match rectmanager.get_character(subrect_id, 4, 4) {
        Ok(character) => {
            assert_eq!(character, test_character);
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.unset_character(subrect_id, 4, 4);

    let default_character = rectmanager.get_default_character(subrect_id);
    match rectmanager.get_character(subrect_id, 4, 4) {
        Ok(character) => {
            assert_eq!(character, default_character);
        }
        Err(e) => {
            assert!(false);
        }
    }

    assert!(rectmanager.get_character(subrect_id, 200, 1000).is_err());
    assert!(rectmanager.set_character(subrect_id, 230, 1000, test_character).is_err());
}

#[test]
fn test_shift_contents() {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(Some(0));
    rectmanager.set_position(subrect_id, 10, 10);
    rectmanager.shift_contents(0, 3, 3);
    match rectmanager.get_relative_offset(subrect_id) {
        Ok((x, y)) => {
            assert_eq!(13, x);
            assert_eq!(13, y);
        }
        Err(e) => {
            assert!(false);
        }
    }
}

#[test]
fn test_clear() {
    let mut rectmanager = RectManager::new();
    let test_character = 'A';
    let mut width = rectmanager.get_rect_width(0);
    let mut height = rectmanager.get_rect_height(0);

    for y in 0 .. height {
        for x in 0 .. width {
            rectmanager.set_character(0, x as isize, y as isize, test_character);
        }
    }

    let default_character = rectmanager.get_default_character(0);
    rectmanager.clear(0);

    for y in 0 .. height {
        for x in 0 .. width {
            match rectmanager.get_character(0, x as isize, y as isize) {
                Ok(character) => {
                    assert_eq!(character, default_character);
                }
                Err(e) => {
                    assert!(false);
                }
            }
        }
    }
}

#[test]
fn test_set_string() {
    let mut rectmanager = RectManager::new();
    let test_string = "Test String".to_string();
    rectmanager.set_string(0, 0, 0, &test_string);
    let mut x;
    let mut y;
    let mut width = rectmanager.get_rect_width(0);
    for (i, c) in test_string.chars().enumerate() {
        x = (i % width) as isize;
        y = (i / width) as isize;
        match rectmanager.get_character(0, x, y) {
            Ok(actual_character) => {
                assert_eq!(actual_character, c);
            }
            Err(e) => {
                assert!(false);
            }
        }
    }

    rectmanager.kill();
}


#[test]
fn test_draw() {
    let mut rectmanager = RectManager::new();
    let mut quarters = Vec::new();
    let mut working_id;
    let height = rectmanager.get_height();
    let width = rectmanager.get_width();
    let colors = vec![ RectColor::RED, RectColor::BLUE, RectColor::GREEN, RectColor::MAGENTA ];

    for y in 0 .. 2 {
        for x in 0 .. 2 {
            working_id = rectmanager.new_rect(Some(0));
            quarters.push(working_id);
            rectmanager.set_position(working_id, (x * (width / 2)) as isize, (y * (height / 2)) as isize);
            rectmanager.resize(working_id, width / 2, height / 2);
            rectmanager.set_bg_color(working_id, colors[(y * 2) + x] as u8);
        }
    }

    rectmanager.draw(0);
    let delay = time::Duration::from_millis(1000);
    let now = time::Instant::now();

    thread::sleep(delay);

    rectmanager.kill();
}
