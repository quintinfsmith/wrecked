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
fn test_draw_map() {
    let mut rectmanager = RectManager::new();
    let mut rect_a = rectmanager.new_rect(Some(0));

    let height = rectmanager.get_height();
    let width = rectmanager.get_width();

    let mut x_offset: usize = 1;
    let mut y_offset: usize = 1;
    let mut size: (usize, usize) = (5,5);
    rectmanager.resize(rect_a, size.0, size.1);
    rectmanager.set_position(rect_a, x_offset as isize, y_offset as isize);

    for i in 0 .. 4 {
        let mut working_id = rectmanager.new_rect(Some(rect_a));
        rectmanager.resize(working_id, 1, 1);
        rectmanager.set_position(working_id, i, 0);
        match i {
            0 => {
                rectmanager.set_bold_flag(working_id);
            }
            1 => {
                rectmanager.set_invert_flag(working_id);
            }
            2 => {
                rectmanager.set_underline_flag(working_id);
            }
            3 => {
                // block parent effects with childs'
                rectmanager.set_underline_flag(working_id);
                let mut subrect = rectmanager.new_rect(Some(working_id));
                rectmanager.resize(subrect, 1, 1);
                rectmanager.set_character(subrect, 0, 0, 'X');
                rectmanager.set_position(subrect, 0, 0);
            }
            _ => {}
        }
    }


    let mut working_flags;
    let mut working_char;
    let mut expected_map = Vec::new();
    for y in 0 .. size.1 {
        for x in 0 .. size.0 {
            working_flags = RectEffectsHandler::new();
            working_char = match (x, y) {
                (3,0) => {
                    'X'
                }
                _ => { ' ' }
            };
            match (x,y) {
                (0, 0) => {
                    working_flags.bold = true;
                }
                (1, 0) => {
                    working_flags.invert = true
                }
                (2, 0) => {
                    working_flags.underline = true
                }
                _ => { }
            };
            expected_map.push((((x + x_offset) as isize, (y + y_offset) as isize), (working_char, working_flags)));
        }
    }
    expected_map.sort();

    let mut actual_map = rectmanager.build_draw_map(rect_a);
    actual_map.sort();

    assert_eq!(expected_map, actual_map);

    rectmanager.kill();
}

#[test]
fn test_lineage() {
    let mut rectmanager = RectManager::new();
    let mut rect_a = rectmanager.new_rect(Some(0));
    let mut rect_b = rectmanager.new_rect(Some(rect_a));
    let mut rect_c = rectmanager.new_rect(Some(rect_b));

    let expected_lineage = vec![rect_b, rect_a, 0];
    assert_eq!(rectmanager.trace_lineage(rect_c), expected_lineage);

    match rectmanager.get_top(rect_c) {
        Ok(top) => {
            assert_eq!(0, top.rect_id);
        }
        Err(e) => {
            assert!(false);
        }
    };

    match rectmanager.get_top_mut(rect_c) {
        Ok(top) => {
            assert_eq!(0, top.rect_id);
        }
        Err(e) => {
            assert!(false);
        }
    };
    rectmanager.kill();
}

#[test]
fn test_replace() {
    let mut rectmanager = RectManager::new();
    let mut rect_a = rectmanager.new_rect(Some(0));
    let mut rect_a_a = rectmanager.new_rect(Some(rect_a));
    let mut rect_b = rectmanager.new_rect(Some(0));
    let mut rect_b_a = rectmanager.new_rect(Some(rect_b));
    rectmanager.detach(rect_b);

    rectmanager.replace_with(rect_a, rect_b);

    match rectmanager.get_rect(0) {
        Ok(top_rect) => {
            assert!(top_rect.has_child(rect_b));
            assert!(!top_rect.has_child(rect_a));
        }
        Err(e) => {
            assert!(false);
        }
    }

    match rectmanager.get_top(rect_a_a) {
        Ok(rect) => {
            assert_eq!(rect.rect_id, rect_a);
        }
        Err(e) => {
            assert!(false);
        }
    }

    match rectmanager.get_top(rect_b_a) {
        Ok(rect) => {
            assert_eq!(rect.rect_id, 0);
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.kill();
}

#[test]
fn test_get_visible_box() {
    let mut rectmanager = RectManager::new();
    let mut subrect = rectmanager.new_rect(Some(0));

    let width = rectmanager.get_width();
    let height = rectmanager.get_height();
    rectmanager.set_position(subrect, 0, 0);
    rectmanager.resize(subrect, width * 2, height * 2);

    let expected_box = (0 as isize, 0 as isize, width as isize, height as isize);
    let visible_box = rectmanager.get_visible_box(subrect).ok().unwrap();
    assert_eq!(visible_box, expected_box);
    rectmanager.kill();
}


#[test]
fn test_set_effects() {
    let mut rectmanager = RectManager::new();
    rectmanager.set_bold_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.unset_bold_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.set_underline_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.unset_underline_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.set_invert_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.unset_invert_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.set_italics_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.unset_italics_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.set_strike_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.unset_strike_flag(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert!(!rect.is_bold());
            assert!(!rect.is_underlined());
            assert!(!rect.is_inverted());
            assert!(!rect.is_italicized());
            assert!(!rect.is_striken());
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.set_bg_color(0, RectColor::BLUE);
    rectmanager.set_fg_color(0, RectColor::RED);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert_eq!(rect.get_bg_color(), RectColor::BLUE);
            assert_eq!(rect.get_fg_color(), RectColor::RED);
        }
        Err(e) => {
            assert!(false);
        }
    }
    rectmanager.unset_color(0);
    match rectmanager.get_rect(0) {
        Ok(rect) => {
            assert_eq!(rect.get_bg_color(), RectColor::NONE);
            assert_eq!(rect.get_fg_color(), RectColor::NONE);
        }
        Err(e) => {
            assert!(false);
        }
    }

    rectmanager.kill();
}

#[test]
fn test_failures() {
    let mut rectmanager = RectManager::new();
    let mut bad_id = 55;

    assert_eq!(rectmanager.get_rect(bad_id).err().unwrap(), RectError::NotFound);
    assert_eq!(rectmanager.get_rect_mut(bad_id).err().unwrap(), RectError::NotFound);

    assert_eq!(rectmanager.get_parent(bad_id).err().unwrap(), RectError::NotFound);
    assert_eq!(rectmanager.get_parent(0).err().unwrap(), RectError::NoParent);

    assert_eq!(rectmanager.get_parent_mut(bad_id).err().unwrap(), RectError::NotFound);
    assert_eq!(rectmanager.get_parent_mut(0).err().unwrap(), RectError::NoParent);

    assert_eq!(rectmanager.get_top(bad_id).err().unwrap(), RectError::NotFound);
    assert_eq!(rectmanager.get_top_mut(bad_id).err().unwrap(), RectError::NotFound);

    assert_eq!(rectmanager.get_visible_box(bad_id).err().unwrap(), RectError::NotFound);
}
