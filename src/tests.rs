#[cfg (test)]
use super::*;

// Keep in mind: in debug, the terminal is set to (25,25) for consistency
#[test]
fn test_init() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let rect_width = rectmanager.get_rect_width(ROOT);
    let rect_height = rectmanager.get_rect_height(ROOT);
    let (w, h) = get_terminal_size();
    assert_eq!(rect_width, w as usize);
    assert_eq!(rect_height, h as usize);

    rectmanager.kill()
}

#[test]
fn test_resize() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    let (subwidth, subheight) = (20, 20);
    rectmanager.resize(subrect_id, subwidth, subheight)?;

    match rectmanager.get_rect(subrect_id) {
        Some(subrect) => {
            assert_eq!(subrect.width, subwidth);
            assert_eq!(subrect.height, subheight);
        }
        None => {
            assert!(false);
        }
    }

    rectmanager.kill()
}

#[test]
fn test_add_rect() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(rect.children.len(), 1);
            assert!(rect.has_child(subrect_id));
        }
        None => {
            assert!(false);
        }
    }

    rectmanager.delete_rect(subrect_id)?;
    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(rect.children.len(), 0);
        }
        None => {
            assert!(false);
        }
    }


    rectmanager.kill()
}

#[test]
fn test_detach() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    let subsubrect_id = rectmanager.new_rect(subrect_id).ok().unwrap();
    rectmanager.detach(subrect_id)?;

    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(rect.children.len(), 0);
        }
        None => {
            assert!(false);
        }
    }

    match rectmanager.get_rect(subrect_id) {
        Some(_subrect) => {
            assert!(true);
        }
        None => {
            assert!(false);
        }
    }

    match rectmanager.get_rect(subsubrect_id) {
        Some(_subrect) => {
            assert!(true);
        }
        None => {
            assert!(false);
        }
    }

    rectmanager.kill()
}

#[test]
fn test_delete() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    let subsubrect_id = rectmanager.new_rect(subrect_id).ok().unwrap();

    rectmanager.delete_rect(subrect_id)?;

    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(rect.children.len(), 0);
        }
        None => {
            assert!(false);
        }
    }

    match rectmanager.get_rect(subrect_id) {
        Some(_subrect) => {
            assert!(false);
        }
        None => {
            assert!(true);
        }
    }

    match rectmanager.get_rect(subsubrect_id) {
        Some(_subrect) => {
            assert!(false);
        }
        None => {
            assert!(true);
        }
    }


    rectmanager.kill()
}

#[test]
fn test_move() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    let subsubrect_id = rectmanager.new_rect(subrect_id).ok().unwrap();

    rectmanager.resize(subrect_id, 40, 40)?;
    rectmanager.resize(subsubrect_id, 10, 10)?;
    rectmanager.set_position(subrect_id, 10, 10)?;
    rectmanager.set_position(subsubrect_id, 10, 10)?;

    match rectmanager.get_relative_offset(subrect_id) {
        Some((x, y)) => {
            assert_eq!(x, 10);
            assert_eq!(y, 10);
        }
        None => {
            assert!(false);
        }
    }

    match rectmanager.get_relative_offset(subsubrect_id) {
        Some((x, y)) => {
            assert_eq!(x, 10);
            assert_eq!(y, 10);
        }
        None => {
            assert!(false);
        }
    }

    match rectmanager.get_absolute_offset(subsubrect_id) {
        Some((x, y)) => {
            assert_eq!(x, 20);
            assert_eq!(y, 20);
        }
        None => {
            assert!(false);
        }
    }
    rectmanager.kill()
}

#[test]
fn test_get_parent() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    let subsubrect_id = rectmanager.new_rect(subrect_id).ok().unwrap();
    match rectmanager.get_parent(subsubrect_id) {
        Some(rect) => {
            assert_eq!(rect.rect_id, subrect_id);
        }
        None => {
            assert!(false);
        }
    }
    rectmanager.kill()
}

#[test]
fn test_disable_enable() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();

    // non-existant rects should return false
    assert!(! rectmanager.is_rect_enabled(99));

    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();

    rectmanager.disable(subrect_id)?;
    assert!(! rectmanager.is_rect_enabled(subrect_id));

    rectmanager.enable(subrect_id)?;
    assert!(rectmanager.is_rect_enabled(subrect_id));

    rectmanager.kill()
}

#[test]
fn test_set_character() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    let test_character = 'Q';
    rectmanager.resize(subrect_id, 10, 10)?;
    assert!(rectmanager.set_character(subrect_id, 4, 4, test_character).is_ok());
    match rectmanager.get_character(subrect_id, 4, 4) {
        Ok(character) => {
            assert_eq!(character, test_character);
        }
        Err(_e) => {
            assert!(false);
        }
    }
    rectmanager.unset_character(subrect_id, 4, 4)?;

    let default_character = rectmanager.get_default_character(subrect_id);
    match rectmanager.get_character(subrect_id, 4, 4) {
        Ok(character) => {
            assert_eq!(character, default_character);
        }
        Err(_e) => {
            assert!(false);
        }
    }

    assert!(rectmanager.get_character(subrect_id, 200, 1000).is_err());
    assert!(rectmanager.set_character(subrect_id, 230, 1000, test_character).is_err());

    rectmanager.kill()
}

#[test]
fn test_shift_contents() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    rectmanager.set_position(subrect_id, 10, 10)?;
    rectmanager.shift_contents(ROOT, 3, 3)?;
    match rectmanager.get_relative_offset(subrect_id) {
        Some((x, y)) => {
            assert_eq!(13, x);
            assert_eq!(13, y);
        }
        None => {
            assert!(false);
        }
    }
    rectmanager.kill()
}

#[test]
fn test_clear_children() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    for _ in 0 .. 4 {
        rectmanager.new_rect(ROOT).ok().unwrap();
    }

    rectmanager.clear_children(ROOT)?;
    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(0, rect.children.len());
        }
        None => {
            assert!(false);
        }
    }
    rectmanager.kill()
}

#[test]
fn test_clear_effects() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();

    rectmanager.set_bg_color(ROOT, Color::RED)?;
    rectmanager.set_fg_color(ROOT, Color::BLACK)?;
    rectmanager.set_bold_flag(ROOT)?;
    rectmanager.set_strike_flag(ROOT)?;
    rectmanager.set_underline_flag(ROOT)?;

    rectmanager.clear_effects(ROOT)?;
    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert!(rect.is_plain())
        }
        None => {
            assert!(false);
        }
    }

    rectmanager.kill()
}

#[test]
fn test_clear_characters() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let test_character = 'A';
    let width = rectmanager.get_rect_width(ROOT);
    let height = rectmanager.get_rect_height(ROOT);

    for y in 0 .. height {
        for x in 0 .. width {
            rectmanager.set_character(ROOT, x as isize, y as isize, test_character)?;
        }
    }

    let default_character = rectmanager.get_default_character(ROOT);
    rectmanager.clear_characters(ROOT)?;

    for y in 0 .. height {
        for x in 0 .. width {
            match rectmanager.get_character(ROOT, x as isize, y as isize) {
                Ok(character) => {
                    assert_eq!(character, default_character);
                }
                Err(_e) => {
                    assert!(false);
                }
            }
        }
    }
    rectmanager.kill()
}

#[test]
fn test_set_string() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let test_string = "Test String".to_string();
    rectmanager.set_string(ROOT, 0, 0, &test_string)?;

    let mut x;
    let mut y;
    let width = rectmanager.get_rect_width(ROOT);
    for (i, c) in test_string.chars().enumerate() {
        x = (i % width) as isize;
        y = (i / width) as isize;
        match rectmanager.get_character(ROOT, x, y) {
            Ok(actual_character) => {
                assert_eq!(actual_character, c);
            }
            Err(_e) => {
                assert!(false);
            }
        }
    }

    rectmanager.kill()
}

#[test]
fn test_draw_map() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let rect_a = rectmanager.new_rect(ROOT).ok().unwrap();

    let x_offset: usize = 1;
    let y_offset: usize = 1;
    let size: (usize, usize) = (5,5);
    rectmanager.resize(rect_a, size.0, size.1)?;
    rectmanager.set_position(rect_a, x_offset as isize, y_offset as isize)?;

    for i in 0 .. 4 {
        let working_id = rectmanager.new_rect(rect_a).ok().unwrap();
        rectmanager.resize(working_id, 1, 1)?;
        rectmanager.set_position(working_id, i, 0)?;
        match i {
            0 => {
                rectmanager.set_bold_flag(working_id)?;
            }
            1 => {
                rectmanager.set_invert_flag(working_id)?;
            }
            2 => {
                rectmanager.set_underline_flag(working_id)?;
            }
            3 => {
                // block parent effects with childs'
                rectmanager.set_underline_flag(working_id)?;
                let subrect = rectmanager.new_rect(working_id).ok().unwrap();
                rectmanager.resize(subrect, 1, 1)?;
                rectmanager.set_character(subrect, 0, 0, 'X')?;
                rectmanager.set_position(subrect, 0, 0)?;
            }
            _ => {}
        }
    }


    let mut working_flags;
    let mut working_char;
    let mut expected_map = Vec::new();
    for y in 0 .. size.1 {
        for x in 0 .. size.0 {
            working_flags = EffectsHandler::new();
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

    let mut actual_map = rectmanager.get_cached_draw_map(rect_a);
    actual_map.sort();

    assert_eq!(expected_map, actual_map);

    rectmanager.kill()
}

#[test]
fn test_replace() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let rect_a = rectmanager.new_rect(ROOT).ok().unwrap();
    let rect_b = rectmanager.new_rect(ROOT).ok().unwrap();
    rectmanager.detach(rect_b)?;

    rectmanager.replace_with(rect_a, rect_b)?;

    match rectmanager.get_rect(ROOT) {
        Some(top_rect) => {
            assert!(top_rect.has_child(rect_b));
            assert!(!top_rect.has_child(rect_a));
        }
        None => {
            assert!(false);
        }
    }


    rectmanager.kill()
}

#[test]
fn test_get_visible_box() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let subrect = rectmanager.new_rect(ROOT).ok().unwrap();

    let width = rectmanager.get_width();
    let height = rectmanager.get_height();
    rectmanager.set_position(subrect, 0, 0)?;
    rectmanager.resize(subrect, width * 2, height * 2)?;

    let expected_box = (0 as isize, 0 as isize, width as isize, height as isize);
    let visible_box = rectmanager.get_visible_box(subrect).ok().unwrap();
    assert_eq!(visible_box, expected_box);
    rectmanager.kill()
}

#[test]
fn test_set_effects() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let effect_count = 6;
    for i in 0 .. effect_count + 1 {
        match i {
            0 => { assert!(rectmanager.set_bold_flag(ROOT).is_ok()); }
            1 => { assert!(rectmanager.set_underline_flag(ROOT).is_ok()); }
            2 => { assert!(rectmanager.set_invert_flag(ROOT).is_ok()); }
            3 => { assert!(rectmanager.set_italics_flag(ROOT).is_ok()); }
            4 => { assert!(rectmanager.set_strike_flag(ROOT).is_ok()); }
            5 => { assert!(rectmanager.set_blink_flag(ROOT).is_ok()); }
            _ => {}
        }

        match rectmanager.get_rect(ROOT) {
            Some(rect) => {
                assert!((i == 0) == rect.is_bold());
                assert!((i == 1) == rect.is_underlined());
                assert!((i == 2) == rect.is_inverted());
                assert!((i == 3) == rect.is_italicized());
                assert!((i == 4) == rect.is_striken());
                assert!((i == 5) == rect.is_blinking());
            }
            None => {
                assert!(false);
            }
        }

        match i {
            0 => { assert!(rectmanager.unset_bold_flag(ROOT).is_ok()); }
            1 => { assert!(rectmanager.unset_underline_flag(ROOT).is_ok()); }
            2 => { assert!(rectmanager.unset_invert_flag(ROOT).is_ok()); }
            3 => { assert!(rectmanager.unset_italics_flag(ROOT).is_ok()); }
            4 => { assert!(rectmanager.unset_strike_flag(ROOT).is_ok()); }
            5 => { assert!(rectmanager.unset_blink_flag(ROOT).is_ok()); }
            _ => {}
        }
    }


    rectmanager.set_bg_color(ROOT, Color::BLUE)?;
    rectmanager.set_fg_color(ROOT, Color::RED)?;
    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(rect.get_bg_color(), Some(Color::BLUE));
            assert_eq!(rect.get_fg_color(), Some(Color::RED));
        }
        None => {
            assert!(false);
        }
    }

    rectmanager.unset_bg_color(ROOT)?;
    rectmanager.unset_fg_color(ROOT)?;
    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(rect.get_bg_color(), None);
            assert_eq!(rect.get_fg_color(), None);
        }
        None => {
            assert!(false);
        }
    }

    rectmanager.set_bg_color(ROOT, Color::BLUE)?;
    rectmanager.set_fg_color(ROOT, Color::RED)?;
    rectmanager.unset_color(ROOT)?;
    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            assert_eq!(rect.get_bg_color(), None);
            assert_eq!(rect.get_fg_color(), None);
        }
        None => {
            assert!(false);
        }
    }

    rectmanager.kill()
}

#[test]
fn test_failures() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let bad_id = 55;
    let good_id = rectmanager.new_rect(ROOT).ok().unwrap();
    rectmanager.resize(good_id, 10, 10)?;

    assert!(rectmanager.get_rect(bad_id).is_none());
    assert!(rectmanager.get_rect_mut(bad_id).is_none());

    assert!(rectmanager.get_parent(bad_id).is_none());
    assert!(rectmanager.get_parent(ROOT).is_none());

    assert!(rectmanager.get_parent_mut(bad_id).is_none());
    assert!(rectmanager.get_parent_mut(ROOT).is_none());

    assert_eq!(rectmanager.get_visible_box(bad_id).err().unwrap(), WreckedError::NotFound(bad_id));

    assert_eq!(rectmanager.set_bg_color(bad_id, Color::RED).err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.set_fg_color(bad_id, Color::RED).err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.unset_bg_color(bad_id).err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.unset_fg_color(bad_id).err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.unset_color(bad_id).err().unwrap(), WreckedError::NotFound(bad_id));

    assert_eq!(rectmanager.unset_bold_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.unset_invert_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.unset_underline_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.unset_strike_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.unset_italics_flag(bad_id), Err(WreckedError::NotFound(bad_id)));

    assert_eq!(rectmanager.set_bold_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.set_invert_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.set_underline_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.set_strike_flag(bad_id), Err(WreckedError::NotFound(bad_id)));
    assert_eq!(rectmanager.set_italics_flag(bad_id), Err(WreckedError::NotFound(bad_id)));

    assert_eq!(rectmanager.replace_with(bad_id, good_id).err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.replace_with(ROOT, bad_id).err().unwrap(), WreckedError::NotFound(ROOT));
    //assert_eq!(rectmanager.replace_with(good_id, bad_id).err().unwrap(), WreckedError::NotFound(bad_id));

    assert_eq!(rectmanager.update_child_space(bad_id).err().unwrap(), WreckedError::NotFound(bad_id));
    //assert!(rectmanager.update_child_space(ROOT).is_ok());

    assert_eq!(rectmanager.delete_rect(bad_id).err().unwrap(), WreckedError::NotFound(bad_id));

    assert_eq!(rectmanager.set_character(bad_id, 0, 0, 'x').err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.set_character(good_id, 1, 100, 'x').err().unwrap(), WreckedError::BadPosition(1, 100));
    assert_eq!(rectmanager.unset_character(bad_id, 0, 0).err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.unset_character(good_id, 1, 100).err().unwrap(), WreckedError::BadPosition(1, 100));

    assert_eq!(rectmanager.get_character(bad_id, 0, 0).err().unwrap(), WreckedError::NotFound(bad_id));
    assert_eq!(rectmanager.get_character(good_id, 1, 100).err().unwrap(), WreckedError::BadPosition(1, 100));

    assert_eq!(rectmanager.set_string(bad_id, 0, 0, &"BOOP").err().unwrap(), WreckedError::NotFound(bad_id));
    let teststr = "afnwjeklnawjekflnawjekflnawejfklanwejfklnawejfklawnefjkawlnefjkawlenfjawkelfnajwkelfafawefBOOP";
    assert_eq!(rectmanager.set_string(good_id, 3000, 0, teststr).err().unwrap(), WreckedError::StringTooLong(good_id, (3000, 0),teststr.to_string()));


    rectmanager.kill()
}

#[test]
fn test_default_character() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let bad_id = 55;
    assert_eq!(rectmanager.get_default_character(bad_id), rectmanager.default_character);
    let test_character = 'Q';
    match rectmanager.get_rect_mut(ROOT) {
        Some(rect) => {
            rect.default_character = test_character;
        }
        None => ()
    }

    assert_eq!(rectmanager.get_default_character(ROOT), test_character);

    rectmanager.kill()
}

#[test]
fn test_update_child_space() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let first_child = rectmanager.new_rect(ROOT).ok().unwrap();
    let second_child = rectmanager.new_rect(ROOT).ok().unwrap();

    rectmanager.resize(first_child, 1, 1)?;
    rectmanager.set_position(first_child, 0, 0)?;
    rectmanager.resize(second_child, 1, 1)?;
    rectmanager.set_position(second_child, 0, 0)?;

    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            match rect.child_space.get(&(0, 0)) {
                Some(child_space) => {
                    assert_eq!(*child_space, vec![first_child, second_child]);
                }
                None => {
                    assert!(false);
                }
            }
        }
        None => { assert!(false); }
    }

    //Move the second child first, so it is the first to be added to
    // the child_space
    rectmanager.set_position(second_child, 3, 0)?;
    rectmanager.set_position(first_child, 3, 0)?;

    match rectmanager.get_rect(ROOT) {
        Some(rect) => {
            // previous child_space should be empty
            match rect.child_space.get(&(0, 0)) {
                Some(child_space) => {
                    assert_eq!(child_space.len(), 0);
                }
                None => {
                    assert!(false);
                }
            }
            // current child_space should still have the first child before the second
            match rect.child_space.get(&(3, 0)) {
                Some(child_space) => {
                    assert_eq!(*child_space, vec![first_child, second_child]);
                }
                None => {
                    assert!(false);
                }
            }
        }
        None => { assert!(false); }
    }

    rectmanager.kill()
}


#[test]
fn test_transparency() -> Result<(), WreckedError> {
    let mut rectmanager = RectManager::new();
    let mut rect_id = rectmanager.new_rect(ROOT).ok().unwrap();
    let t_rect_id = rectmanager.new_rect(ROOT).ok().unwrap();

    rectmanager.resize(rect_id, 10, 10)?;
    rectmanager.set_bg_color(rect_id, Color::BLUE);

    rectmanager.resize(t_rect_id, 10, 10)?;
    rectmanager.set_position(t_rect_id, 3, 3)?;
    assert!(rectmanager.set_transparency(t_rect_id, true).is_ok());

    let ansi_string = rectmanager.build_latest_rect_string(ROOT).unwrap();

    rectmanager.kill()?;

    // Get comparison string, (don't build the transparent rect)
    rectmanager = RectManager::new();
    rect_id = rectmanager.new_rect(ROOT).ok().unwrap();

    rectmanager.resize(rect_id, 10, 10)?;
    rectmanager.set_bg_color(rect_id, Color::BLUE)?;

    let control_string = rectmanager.build_latest_rect_string(ROOT).unwrap();

    rectmanager.kill()?;

    assert_eq!(control_string, ansi_string);

    Ok(())
}

#[test]
fn test_id_recycling() {
    let mut rectmanager = RectManager::new();
    for _ in 1 .. 5 {
        rectmanager.new_rect(ROOT);
    }

    rectmanager.delete_rect(4);
    assert_eq!(rectmanager.idgen, 4);
    rectmanager.delete_rect(1);
    assert_eq!(rectmanager.idgen, 4);
    assert_eq!(rectmanager.recycle_ids.as_slice(), [1]);
    rectmanager.delete_rect(2);
    assert_eq!(rectmanager.idgen, 4);
    assert_eq!(rectmanager.recycle_ids.as_slice(), [1, 2]);
    rectmanager.delete_rect(3);
    assert_eq!(rectmanager.idgen, 1);
    assert_eq!(rectmanager.recycle_ids.as_slice(), []);
}

