use std::ffi::CStr;
use std::os::raw::c_char;
use std::str;
use std::io::prelude::*;

use wrecked::{RectManager, Color, WreckedError};

fn cast_result(result: Result<(), WreckedError>) -> u32 {
    match result {
        Ok(_) => 0,
        Err(WreckedError::BadColor) => 1,
        Err(WreckedError::InvalidUtf8) => 2,
        Err(WreckedError::StringTooLong(_, _, _)) => 3,
        Err(WreckedError::NotFound(_)) => 4,
        Err(WreckedError::NoParent(_)) => 5,
        Err(WreckedError::ParentNotFound(_, _)) => 6,
        Err(WreckedError::ChildNotFound(_, _)) => 7,
        Err(WreckedError::BadPosition(_, _)) => 8,
        Err(_) => 255
    }
}

#[no_mangle]
pub extern "C" fn set_transparency(ptr: *mut RectManager, rect_id: usize, transparency: bool) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.set_transparency(rect_id, transparency);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn disable_rect(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.disable(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn enable_rect(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.enable(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn render(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.draw(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn set_fg_color(ptr: *mut RectManager, rect_id: usize, color_n: u8) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let colors = [Color::BLACK, Color::RED, Color::GREEN, Color::YELLOW, Color::BLUE, Color::MAGENTA, Color::CYAN, Color::WHITE, Color::BRIGHTBLACK, Color::BRIGHTRED, Color::BRIGHTGREEN, Color::BRIGHTYELLOW, Color::BRIGHTBLUE, Color::BRIGHTMAGENTA, Color::BRIGHTCYAN, Color::BRIGHTWHITE];

    let result = match colors.get(color_n as usize) {
        Some(color) => {
            rectmanager.set_fg_color(rect_id, *color)
        }
        None => {
            Err(WreckedError::BadColor)
        }
    };

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut RectManager, rect_id: usize, color_n: u8) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };
    let colors = [Color::BLACK, Color::RED, Color::GREEN, Color::YELLOW, Color::BLUE, Color::MAGENTA, Color::CYAN, Color::WHITE, Color::BRIGHTBLACK, Color::BRIGHTRED, Color::BRIGHTGREEN, Color::BRIGHTYELLOW, Color::BRIGHTBLUE, Color::BRIGHTMAGENTA, Color::BRIGHTCYAN, Color::BRIGHTWHITE];
    let result = match colors.get(color_n as usize) {
        Some(color) => {
            rectmanager.set_bg_color(rect_id, *color)
        }
        None => {
            Err(WreckedError::BadColor)
        }
    };

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn set_invert_flag(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.set_invert_flag(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}

#[no_mangle]
pub extern "C" fn set_underline_flag(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.set_underline_flag(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

}


#[no_mangle]
pub extern "C" fn set_bold_flag(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.set_bold_flag(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn unset_invert_flag(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.unset_invert_flag(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}

#[no_mangle]
pub extern "C" fn unset_underline_flag(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.unset_underline_flag(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}


#[no_mangle]
pub extern "C" fn unset_bold_flag(ptr: *mut RectManager, rect_id: usize) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.unset_bold_flag(rect_id);

    Box::into_raw(rectmanager); // Prevent Release
}

#[no_mangle]
pub extern "C" fn resize(ptr: *mut RectManager, rect_id: usize, new_width: usize, new_height: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.resize(rect_id, new_width, new_height);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn unset_bg_color(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.unset_bg_color(rect_id);
    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}



#[no_mangle]
pub extern "C" fn unset_fg_color(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };
    let result = rectmanager.unset_fg_color(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn unset_color(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.unset_color(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn set_string(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize, c: *const c_char) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let c_str = unsafe { CStr::from_ptr(c) };
    let string_ = c_str.to_str().unwrap();

    let result = rectmanager.set_string(rect_id, x, y, string_);


    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn set_character(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize, c: *const c_char) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let c_str = unsafe { CStr::from_ptr(c) };
    let character = c_str.to_str().unwrap().chars().next().unwrap();

    let result = rectmanager.set_character(rect_id, x, y, character);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn unset_character(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.unset_character(rect_id, x, y);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn delete_rect(ptr: *mut RectManager, rect_id: usize) -> u32 {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.delete_rect(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn new_rect(ptr: *mut RectManager, parent_id: usize, width: usize, height: usize) -> usize {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let new_rect_id = rectmanager.new_rect(parent_id).ok().unwrap();
    rectmanager.resize(new_rect_id, width, height);

    Box::into_raw(rectmanager); // Prevent Release

    new_rect_id
}

#[no_mangle]
pub extern "C" fn new_orphan(ptr: *mut RectManager, width: usize, height: usize) -> usize {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let new_rect_id = rectmanager.new_orphan().ok().unwrap();
    rectmanager.resize(new_rect_id, width, height);

    Box::into_raw(rectmanager); // Prevent Release

    new_rect_id
}

#[no_mangle]
pub extern "C" fn set_position(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize) -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.set_position(rect_id, x, y);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn shift_contents(ptr: *mut RectManager, rect_id: usize, x: isize, y: isize) -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.shift_contents(rect_id, x, y);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn clear_characters(ptr: *mut RectManager, rect_id: usize)  -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.clear_characters(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn clear_children(ptr: *mut RectManager, rect_id: usize)  -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.clear_children(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn detach(ptr: *mut RectManager, rect_id: usize)  -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.detach(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn attach(ptr: *mut RectManager, rect_id: usize, parent_id: usize) -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.attach(rect_id, parent_id);

    Box::into_raw(rectmanager); // Prevent Release

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn replace_with(ptr: *mut RectManager, old_rect_id: usize, new_rect_id: usize) -> u32 {

    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let result = rectmanager.replace_with(old_rect_id, new_rect_id);

    Box::into_raw(rectmanager); // Prevent Release


    cast_result(result)
}

#[no_mangle]
pub extern "C" fn kill(ptr: *mut RectManager) {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    rectmanager.kill();

    // TODO: Figure out why releasing causes segfault
    Box::into_raw(rectmanager); // Prevent Release
    // Releases boxes
}

#[no_mangle]
pub extern "C" fn init() -> *mut RectManager {

    let rectmanager = RectManager::new();

    Box::into_raw(Box::new(rectmanager))
}

#[no_mangle]
pub extern "C" fn get_width(ptr: *mut RectManager, rect_id: usize) -> usize {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let width = rectmanager.get_rect_width(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    width
}

#[no_mangle]
pub extern "C" fn get_height(ptr: *mut RectManager, rect_id: usize) -> usize {
    let mut rectmanager = unsafe { Box::from_raw(ptr) };

    let height = rectmanager.get_rect_height(rect_id);

    Box::into_raw(rectmanager); // Prevent Release

    height
}
