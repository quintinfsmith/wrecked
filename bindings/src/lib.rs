use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str;
use std::io::prelude::*;
use std::mem;
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
pub extern "C" fn set_transparency(ptr: *mut RectManager, rect_id: u64, transparency: bool) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.set_transparency(rect_id as usize, transparency);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn disable_rect(ptr: *mut RectManager, rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.disable(rect_id as usize);


    cast_result(result)
}


#[no_mangle]
pub extern "C" fn enable_rect(ptr: *mut RectManager, rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.enable(rect_id as usize);


    cast_result(result)
}


#[no_mangle]
pub extern "C" fn render(ptr: *mut RectManager, rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.draw(rect_id as usize);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn set_fg_color(ptr: *mut RectManager, rect_id: u64, color_n: u8) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let colors = [Color::BLACK, Color::RED, Color::GREEN, Color::YELLOW, Color::BLUE, Color::MAGENTA, Color::CYAN, Color::WHITE, Color::BRIGHTBLACK, Color::BRIGHTRED, Color::BRIGHTGREEN, Color::BRIGHTYELLOW, Color::BRIGHTBLUE, Color::BRIGHTMAGENTA, Color::BRIGHTCYAN, Color::BRIGHTWHITE];

    let result = match colors.get(color_n as usize) {
        Some(color) => {
            rectmanager.set_fg_color(rect_id as usize, *color)
        }
        None => {
            Err(WreckedError::BadColor)
        }
    };

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn set_bg_color(ptr: *mut RectManager, rect_id: u64, color_n: u8) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let colors = [Color::BLACK, Color::RED, Color::GREEN, Color::YELLOW, Color::BLUE, Color::MAGENTA, Color::CYAN, Color::WHITE, Color::BRIGHTBLACK, Color::BRIGHTRED, Color::BRIGHTGREEN, Color::BRIGHTYELLOW, Color::BRIGHTBLUE, Color::BRIGHTMAGENTA, Color::BRIGHTCYAN, Color::BRIGHTWHITE];
    let result = match colors.get(color_n as usize) {
        Some(color) => {
            rectmanager.set_bg_color(rect_id as usize, *color)
        }
        None => {
            Err(WreckedError::BadColor)
        }
    };

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn set_invert_flag(ptr: *mut RectManager, rect_id: u64) {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    rectmanager.set_invert_flag(rect_id as usize);
}

#[no_mangle]
pub extern "C" fn set_underline_flag(ptr: *mut RectManager, rect_id: u64) {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    rectmanager.set_underline_flag(rect_id as usize);

}


#[no_mangle]
pub extern "C" fn set_bold_flag(ptr: *mut RectManager, rect_id: u64) {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    rectmanager.set_bold_flag(rect_id as usize);
}


#[no_mangle]
pub extern "C" fn unset_invert_flag(ptr: *mut RectManager, rect_id: u64) {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    rectmanager.unset_invert_flag(rect_id as usize);
}

#[no_mangle]
pub extern "C" fn unset_underline_flag(ptr: *mut RectManager, rect_id: u64) {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    rectmanager.unset_underline_flag(rect_id as usize);
}


#[no_mangle]
pub extern "C" fn unset_bold_flag(ptr: *mut RectManager, rect_id: u64) {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    rectmanager.unset_bold_flag(rect_id as usize);
}

#[no_mangle]
pub extern "C" fn resize(ptr: *mut RectManager, rect_id: u64, new_width: u64, new_height: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.resize(rect_id as usize, new_width as usize, new_height as usize);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn unset_bg_color(ptr: *mut RectManager, rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.unset_bg_color(rect_id as usize);

    cast_result(result)
}



#[no_mangle]
pub extern "C" fn unset_fg_color(ptr: *mut RectManager, rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let result = rectmanager.unset_fg_color(rect_id as usize);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn unset_color(ptr: *mut RectManager, rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.unset_color(rect_id as usize);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn set_string(ptr: *mut RectManager, rect_id: u64, x: i64, y: i64, c: *const c_char) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let c_str = unsafe { CStr::from_ptr(c) };
    let string_ = c_str.to_str().unwrap();

    let result = rectmanager.set_string(rect_id as usize, x as isize, y as isize, string_);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn set_character(ptr: *mut RectManager, rect_id: u64, x: i64, y: i64, c: *const c_char) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let c_str = unsafe { CStr::from_ptr(c) };
    let character = c_str.to_str().unwrap().chars().next().unwrap();

    let result = rectmanager.set_character(rect_id as usize, x as isize, y as isize, character);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn unset_character(ptr: *mut RectManager, rect_id: u64, x: i64, y: i64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let result = rectmanager.unset_character(rect_id as usize, x as isize, y as isize);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn delete_rect(ptr: *mut RectManager, rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let result = rectmanager.delete_rect(rect_id as usize);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn new_rect(ptr: *mut RectManager, parent_id: u64, width: u64, height: u64) -> u64 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let new_rect_id = rectmanager.new_rect(parent_id as usize).ok().unwrap();
    rectmanager.resize(new_rect_id, width as usize, height as usize);

    new_rect_id as u64
}

#[no_mangle]
pub extern "C" fn new_orphan(ptr: *mut RectManager, width: u64, height: u64) -> u64 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let new_rect_id = rectmanager.new_orphan().ok().unwrap();
    rectmanager.resize(new_rect_id, width as usize, height as usize);

    new_rect_id as u64
}

#[no_mangle]
pub extern "C" fn set_position(ptr: *mut RectManager, rect_id: u64, x: i64, y: i64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.set_position(rect_id as usize, x as isize, y as isize);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn shift_contents(ptr: *mut RectManager, rect_id: u64, x: i64, y: i64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.shift_contents(rect_id as usize, x as isize, y as isize);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn shift_contents_in_box(ptr: *mut RectManager, rect_id: u64, x: i64, y: i64, xi: i64, yi: i64, xf: i64, yf: i64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let limit = (xi as isize, yi as isize, xf as isize, yf as isize);
    let result = rectmanager.shift_contents_in_box(rect_id as usize, x as isize, y as isize, limit);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn clear_characters(ptr: *mut RectManager, rect_id: u64)  -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.clear_characters(rect_id as usize);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn clear_children(ptr: *mut RectManager, rect_id: u64)  -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.clear_children(rect_id as usize);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn detach(ptr: *mut RectManager, rect_id: u64)  -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.detach(rect_id as usize);

    cast_result(result)
}


#[no_mangle]
pub extern "C" fn attach(ptr: *mut RectManager, rect_id: u64, parent_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.attach(rect_id as usize, parent_id as usize);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn replace_with(ptr: *mut RectManager, old_rect_id: u64, new_rect_id: u64) -> u32 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.replace_with(old_rect_id as usize, new_rect_id as usize);

    cast_result(result)
}

#[no_mangle]
pub extern "C" fn kill(ptr: *mut RectManager) {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    rectmanager.kill();
}

#[no_mangle]
pub extern "C" fn init() -> *mut RectManager {
    let rectmanager = RectManager::new();

    Box::into_raw(Box::new(rectmanager))
}

#[no_mangle]
pub extern "C" fn get_width(ptr: *mut RectManager, rect_id: u64) -> u64 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let width = rectmanager.get_rect_width(rect_id as usize);

    width as u64
}

#[no_mangle]
pub extern "C" fn get_height(ptr: *mut RectManager, rect_id: u64) -> u64 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let height = rectmanager.get_rect_height(rect_id as usize);

    height as u64
}

#[no_mangle]
pub extern "C" fn get_x(ptr: *mut RectManager, rect_id: u64) -> i64 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let output;
    match rectmanager.get_relative_offset(rect_id as usize) {
        Some((x, _)) => {
            output = x;
        }
        None => {
            output = 0;
        }
    }

    output as i64
}

#[no_mangle]
pub extern "C" fn get_y(ptr: *mut RectManager, rect_id: u64) -> i64 {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let output;
    match rectmanager.get_relative_offset(rect_id as usize) {
        Some((_, y)) => {
            output = y;
        }
        None => {
            output = 0;
        }
    }

    output as i64
}

#[no_mangle]
pub extern "C" fn fit_to_terminal(ptr: *mut RectManager) -> bool {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };

    let result = rectmanager.fit_to_terminal();

    result
}


#[no_mangle]
pub extern "C" fn get_current_ansi_string(ptr: *mut RectManager) -> *mut c_char {
    let mut rectmanager = unsafe { mem::ManuallyDrop::new(Box::from_raw(ptr)) };
    let ansi_string = rectmanager.get_current_ansi_string();

    CString::new(ansi_string).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(ptr: *mut c_char) {
    unsafe { CString::from_raw(ptr); }
}
