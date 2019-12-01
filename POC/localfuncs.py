import os
import tty
import termios
import sys

def get_terminal_size():
    '''return dimensions of current terminal session'''
    height, width = os.popen("stty size", "r").read().split()
    return (int(width), int(height))

def read_character():
    '''Read character from stdin'''
    init_fileno = sys.stdin.fileno() # store original pipe n
    init_attr = termios.tcgetattr(init_fileno)  # store original input settings
    try:
        tty.setraw(sys.stdin.fileno()) # remove wait for "return"
        ch = sys.stdin.read(1) # Read single character into memory
    finally:
        termios.tcsetattr(init_fileno, termios.TCSADRAIN, init_attr) # reset input settings
    return ch

