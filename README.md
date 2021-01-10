# wrecked_bindings
Python bindings for the wrecked terminal interface library.<br/>
[![PyPI - Downloads](https://img.shields.io/pypi/dw/wrecked?style=flat-square)](https://pypi.org/project/wrecked/)
[![PyPI](https://img.shields.io/pypi/v/wrecked?style=flat-square)](https://pypi.org/project/wrecked/)
[![GitHub](https://img.shields.io/github/license/quintinfsmith/wrecked_bindings?style=flat-square)](https://github.com/quintinfsmith/wrecked_bindings/blob/master/LICENSE)

## Installation
Can be installed through pip
```
pip install wrecked
```

## Usage
```python
import wrecked

# Instantiates the environment. Turns off input echo.
top_rect = wrecked.init()

# create a rectangle to put text in.
new_rect = top_rect.new_rect(width=16, height=5)

# Add a string to the center of the rectangle
new_rect.set_string(2, 3, "Hello World!")

# Make that rectangle blue
new_rect.set_bg_color(wrecked.BLUE)

# And finally underline the text of the rectangle
new_rect.set_underline_flag()

# Draw the environment
top_rect.draw()

# take down the environment, and turn echo back on.
wrecked.kill()
```
