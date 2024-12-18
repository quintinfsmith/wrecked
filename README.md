# Wrecked
A library for terminal-based graphics and UI.<br/>
[![Crates.io](https://img.shields.io/crates/d/wrecked?style=flat-square)](https://crates.io/crates/wrecked)
[![Crates.io](https://img.shields.io/crates/v/wrecked?style=flat-square)](https://crates.io/crates/wrecked)
[![Crates.io](https://img.shields.io/crates/l/wrecked?style=flat-square)](https://burnsomni.net/project/wrecked/?branch=master&path=LICENSE)
## About
Wrecked is (hopefully) a straightforward environment for rendering character-based graphics that uses a tree-like structure with rectangles as nodes.
It exists partially because I wanted to give myself a reason to work in rust, but mostly because I didn't want to read through the ncurses documentation.

## Setup
The latest *stable* version can be found at crates.io.
In your project's Cargo.toml...
```toml
[dependencies]
wrecked = "^1.0.0"
```

## Usage
```rust
use wrecked::{RectManager, RectColor};
use std::{time, thread};

// Instantiates the environment. Turns off input echo.
let mut rectmanager = RectManager::new();

// create a rectangle to put text in.
let mut rect_id = rectmanager.new_rect(wrecked::TOP).ok().unwrap();

// set the new rectangle's size
rectmanager.resize(rect_id, 16, 5);

// Add a string to the center of the rectangle
rectmanager.set_string(rect_id, 2, 3, "Hello World!");

// Make that rectangle blue
rectmanager.set_bg_color(rect_id, RectColor::BLUE);

// And finally underline the text of the rectangle
rectmanager.set_underline_flag(rect_id);

// Draw the environment
rectmanager.render();

// Sleep for 2 seconds so you can see the output before it gets torn down
thread::sleep(time::Duration::from_secs(2));

// take down the environment, and turn echo back on.
rectmanager.kill();
```
