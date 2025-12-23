use crossterm::{
    cursor::{position, MoveTo},
    execute,
    style::Print,
    terminal::size,
};
use std::io::{stdout, Write};

pub fn handle_backspace() -> std::io::Result<()> {
    let (x, y) = position()?;
    if x > 0 {
        execute!(stdout(), MoveTo(x - 1, y), Print(" "), MoveTo(x - 1, y))?;
    }
    stdout().flush()
}

pub fn handle_enter() -> std::io::Result<()> {
    print!("\r\n");
    stdout().flush()
} 
