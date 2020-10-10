# Wrecked
A library for terminal-based graphics and UI.

## Table of Contents
* [About](#about)
* [Setup](#setup)
* [Example Usage](#usage)


## About
Wrecked is (hopefully) a straightforward environment for rendering character-based graphics that uses a tree-like structure with rectangles as nodes.
It exists partially because I wanted to give myself a reason to work in rust, but mostly because I didn't want to read through the ncurses documentation.

## Setup
The latest *stable* version can be found at crates.io
For the latest stable version, in your project's Cargo.toml...
```
[dependencies]
wrecked = { version ="*" }
```

## Example Usage
```
use wrecked::{RectManager, RectColor};

// Instantiates the environment. Turns off input echo.
let mut rectmanager = RectManager::new();

// create a rectangle to put text in.
let mut rect_id = rectmanager.new_rect(wrecked::TOP);

// set the new rectangle's size
rectmanager.resize(rect_id, 16, 5);

// Add a string to the center of the rectangle
rectmanager.set_string(rect_id, 2, 3, "Hello World!");

// Make that rectangle blue
rectmanager.set_bg_color(rect_id, RectColor::BLUE);

// Draw the environment
rectmanager.draw();

// take down the environment, and turn echo back on.
rectmanager.kill();
```

