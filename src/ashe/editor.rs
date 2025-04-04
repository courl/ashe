use super::buffer::Buffer;
use super::terminal::{Position, Terminal};
use super::tui;
use crossterm::event::Event::Key;
use crossterm::event::KeyCode::Char;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, read};
use crossterm::queue;
use crossterm::style::{Color, SetBackgroundColor, SetForegroundColor};
use std::io::stdout;
use std::path::Path;

enum EditorMode {
    Edit(Option<u8>),
    Command(String),
}

enum EventProcessingResult {
    Continue,
    Exit,
}

pub struct Editor {
    cursor: u32,
    bytes_per_line: u32,
    offset: u32,
    path: Box<Path>,
    buffer: Buffer,
    mode: EditorMode,
    warning: String,
}

impl Editor {
    pub fn new(path: &Path, bytes_per_line: u32) -> Result<Self, std::io::Error> {
        Ok(Editor {
            cursor: 0,
            bytes_per_line,
            offset: 0,
            path: path.into(),
            buffer: Buffer::new(path)?,
            mode: EditorMode::Edit(None),
            warning: "".into(),
        })
    }

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        Terminal::initialize()?;
        loop {
            let max_lines = (Terminal::height()? - 5) as u32;
            self.redraw(self.offset, max_lines)?;
            self.warning = "".into();
            if let Key(event) = read()? {
                match self.process_event(event, max_lines) {
                    EventProcessingResult::Continue => {}
                    EventProcessingResult::Exit => {
                        break;
                    }
                }
            }
        }
        Terminal::terminate()?;
        Ok(())
    }

    fn process_event(&mut self, event: KeyEvent, max_lines: u32) -> EventProcessingResult {
        if event.code == KeyCode::Esc {
            self.mode = EditorMode::Edit(None);
            return EventProcessingResult::Continue;
        }
        if let Char(c) = event.code {
            if c == 'c' && event.modifiers == KeyModifiers::CONTROL {
                return EventProcessingResult::Exit;
            } else if c == ':' {
                self.mode = EditorMode::Command("".into());
            }
        }
        match self.mode {
            EditorMode::Edit(_) => self.process_edit_event(event, max_lines),
            EditorMode::Command(_) => self.process_command_event(event),
        }
    }

    fn process_edit_event(&mut self, event: KeyEvent, max_lines: u32) -> EventProcessingResult {
        let cursor_update = self.process_cursor_update(event, max_lines);
        if cursor_update != 0 {
            if (self.cursor as i64 + cursor_update as i64) < 0 {
                self.cursor = 0;
            } else if (self.cursor as i64 + cursor_update as i64) >= self.buffer.len() as i64 {
                self.cursor = (self.buffer.len() - 1) as u32;
            } else {
                self.cursor = (self.cursor as i64 + cursor_update as i64) as u32;
            }

            if self.cursor >= (self.offset + max_lines * self.bytes_per_line) {
                self.offset += self.bytes_per_line;
            }
            if self.cursor < self.offset {
                self.offset -= self.bytes_per_line;
            }

            self.mode = EditorMode::Edit(None);
            return EventProcessingResult::Continue;
        }
        if let EditorMode::Edit(input_buffer) = self.mode {
            if let Char(c) = event.code {
                if ('a'..='f').contains(&c) || c.is_ascii_digit() {
                    let value = if ('a'..='f').contains(&c) {
                        c as u8 - b'a' + 10
                    } else {
                        c as u8 - b'0'
                    };
                    match input_buffer {
                        None => {
                            self.buffer.update(self.cursor as usize, value);
                            self.mode = EditorMode::Edit(Some(value));
                        }
                        Some(previous_value) => {
                            self.buffer
                                .update(self.cursor as usize, (previous_value << 4) | value);
                            self.mode = EditorMode::Edit(None);
                        }
                    }
                }
            }
        }

        EventProcessingResult::Continue
    }

    fn process_command_event(&mut self, event: KeyEvent) -> EventProcessingResult {
        let current_mode = std::mem::replace(&mut self.mode, EditorMode::Command("".into()));
        if let EditorMode::Command(value) = current_mode {
            if let Char(c) = event.code {
                if c.is_ascii_lowercase() || c.is_ascii_digit() {
                    let mut s = value.chars().collect::<String>();
                    if value.len() < 16 {
                        s.push(c);
                    } else {
                        self.warning = "Cmd too long".into();
                    }
                    self.mode = EditorMode::Command(s);
                } else {
                    self.mode = EditorMode::Command(value);
                }
            } else if event.code == KeyCode::Backspace {
                let mut chars = value.chars();
                chars.next_back();
                self.mode = EditorMode::Command(chars.collect::<String>());
            } else if event.code == KeyCode::Enter {
                return self.process_command(value.as_str());
            } else {
                self.mode = EditorMode::Command(value);
            }
        }
        EventProcessingResult::Continue
    }

    fn process_command(&mut self, value: &str) -> EventProcessingResult {
        match value {
            "exit" | "q" | "x" => {
                if self.buffer.is_dirty() {
                    self.warning = "Modified Buffer".into();
                    EventProcessingResult::Continue
                } else {
                    EventProcessingResult::Exit
                }
            }
            "wq" | "qw" => {
                if self.save() {
                    EventProcessingResult::Exit
                } else {
                    EventProcessingResult::Continue
                }
            }
            "write" | "w" => {
                self.save();
                EventProcessingResult::Continue
            }
            _ => {
                self.warning = "Invalid command".into();
                EventProcessingResult::Continue
            }
        }
    }

    fn process_cursor_update(&mut self, event: KeyEvent, max_lines: u32) -> i32 {
        let mut cursor_update: i32 = 0;
        if event.code == KeyCode::Down {
            cursor_update = self.bytes_per_line as i32;
        } else if event.code == KeyCode::Up {
            cursor_update = -(self.bytes_per_line as i32);
        } else if event.code == KeyCode::Left {
            cursor_update = -1;
        } else if event.code == KeyCode::Right {
            cursor_update = 1;
        }
        if event.modifiers == KeyModifiers::SHIFT {
            cursor_update *= max_lines as i32;
        }
        cursor_update
    }

    fn save(&mut self) -> bool {
        if !self.buffer.is_dirty() {
            return true;
        }
        match self.buffer.save(self.path.clone()) {
            Ok(_) => true,
            Err(_) => {
                self.warning = "Writing failed".into();
                false
            }
        }
    }

    fn redraw(&self, offset: u32, lines: u32) -> Result<(), std::io::Error> {
        Terminal::move_cursor_to(Position { x: 0, y: 0 })?;
        queue!(stdout(), SetForegroundColor(Color::DarkYellow))?;
        print!("\r     Ashe");
        queue!(stdout(), SetForegroundColor(Color::Reset))?;
        println!("      {}", self.path.file_name().unwrap().to_str().unwrap());
        println!(
            "\r {}{}{}{}{}{}{}",
            tui::TOP_LEFT_CORNER,
            tui::VERTICAL.repeat(11),
            tui::TOP_T,
            tui::VERTICAL.repeat((3 * self.bytes_per_line + 1) as usize),
            tui::TOP_T,
            tui::VERTICAL.repeat(self.bytes_per_line as usize + 2),
            tui::TOP_RIGHT_CORNER
        );
        for line in 0..lines {
            let current_line = offset + line * self.bytes_per_line;
            print!(
                "\r {} {:0>4x} {:0>4x} {} ",
                tui::HORIZONTAL,
                current_line / (256 * 256),
                current_line % (256 * 256),
                tui::HORIZONTAL
            );
            for i in 0..self.bytes_per_line {
                let highlight = self.cursor == self.offset + line * self.bytes_per_line + i;
                let position = (self.offset + line * self.bytes_per_line + i) as usize;
                if position < self.buffer.len() {
                    if highlight {
                        queue!(stdout(), SetBackgroundColor(Color::DarkYellow))?;
                    }
                    print!("{:0>2x}", self.buffer[position]);
                    if highlight {
                        queue!(stdout(), SetBackgroundColor(Color::Reset))?;
                    }
                    print!(" ");
                } else {
                    print!("   ");
                }
            }
            print!("{} ", tui::HORIZONTAL);
            for i in 0..self.bytes_per_line {
                let highlight = self.cursor == self.offset + line * self.bytes_per_line + i;
                let position = (self.offset + line * self.bytes_per_line + i) as usize;
                if position < self.buffer.len() {
                    let byte = self.buffer[position];
                    if highlight {
                        queue!(stdout(), SetBackgroundColor(Color::DarkYellow))?;
                    }
                    if byte.is_ascii() && !byte.is_ascii_control() {
                        print!("{}", byte as char);
                    } else {
                        queue!(stdout(), SetForegroundColor(Color::Black))?;
                        print!(".");
                        queue!(stdout(), SetForegroundColor(Color::Reset))?;
                    }
                    if highlight {
                        queue!(stdout(), SetBackgroundColor(Color::Reset))?;
                    }
                } else {
                    print!(" ");
                }
            }
            println!(" {}", tui::HORIZONTAL);
        }
        println!(
            "\r {}{}{}{}{}{}{}",
            tui::LOWER_LEFT_CORNER,
            tui::VERTICAL.repeat(11),
            tui::LOWER_T,
            tui::VERTICAL.repeat((3 * self.bytes_per_line + 1) as usize),
            tui::LOWER_T,
            tui::VERTICAL.repeat(self.bytes_per_line as usize + 2),
            tui::LOWER_RIGHT_CORNER
        );
        print!(
            "\r   {:0>4x} {:0>4x}   ",
            self.cursor / (256 * 256),
            self.cursor % (256 * 256)
        );
        if let EditorMode::Command(command) = &self.mode {
            print!(":{}", command);
            print!(
                "{}",
                " ".repeat(self.bytes_per_line as usize * 3 - command.len())
            );
        } else {
            print!("{}", " ".repeat(self.bytes_per_line as usize * 3));
        }
        queue!(stdout(), SetForegroundColor(Color::Red))?;
        print!("{}", self.warning);
        println!(
            "{}",
            " ".repeat(self.bytes_per_line as usize - self.warning.len())
        );

        Terminal::execute()?;
        Ok(())
    }
}
