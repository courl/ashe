use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{queue, terminal};
use std::io::{Write, stdout};

#[derive(Copy, Clone)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

pub struct Terminal;

impl Terminal {
    pub fn initialize() -> Result<(), std::io::Error> {
        terminal::enable_raw_mode()?;
        Self::clear_screen()?;
        Self::move_cursor_to(Position { x: 0, y: 0 })?;
        queue!(stdout(), Hide)?;
        Self::execute()?;
        Ok(())
    }

    pub fn terminate() -> Result<(), std::io::Error> {
        Self::execute()?;
        terminal::disable_raw_mode()?;
        queue!(stdout(), Show)?;
        Self::execute()?;
        Ok(())
    }

    pub fn height() -> Result<u16, std::io::Error> {
        let (_, height) = terminal::size()?;
        Ok(height)
    }

    pub fn move_cursor_to(position: Position) -> Result<(), std::io::Error> {
        queue!(stdout(), MoveTo(position.x, position.y))?;
        Ok(())
    }

    pub fn clear_screen() -> Result<(), std::io::Error> {
        queue!(stdout(), Clear(ClearType::All))?;
        Ok(())
    }

    pub fn execute() -> Result<(), std::io::Error> {
        stdout().flush()?;
        Ok(())
    }
}
