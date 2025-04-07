use super::buffer::Buffer;
use super::terminal::{Position, Terminal};
use super::tui;
use crate::ashe::tui::{BoxPart, draw_box_part};
use crossterm::event::Event::Key;
use crossterm::event::KeyCode::Char;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, read};
use crossterm::style::Color;
use std::path::{Path, PathBuf};

enum EditorMode {
    Edit(Option<u8>),
    Command(String),
}

pub struct Editor {
    cursor: u32,
    bytes_per_line: u32,
    offset: u32,
    path: PathBuf,
    buffer: Buffer,
    mode: EditorMode,
    warning: String,
    should_exit: bool,
}

impl Editor {
    pub fn init(path: &Path, bytes_per_line: u32) -> Result<Self, std::io::Error> {
        Ok(Editor {
            cursor: 0,
            bytes_per_line,
            offset: 0,
            path: path.into(),
            buffer: Buffer::new(std::fs::read(path)?),
            mode: EditorMode::Edit(None),
            warning: "".into(),
            should_exit: false,
        })
    }

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        Terminal::initialize()?;
        let result = self.repl();
        Terminal::terminate()?;
        println!("\r");
        result
    }

    pub fn repl(&mut self) -> Result<(), std::io::Error> {
        while !self.should_exit {
            let max_lines = (Terminal::height()? - 5) as u32;
            self.redraw(self.offset, max_lines)?;
            self.warning = "".into();
            if let Key(event) = read()? {
                self.process_event(event, max_lines);
            }
        }
        Ok(())
    }

    fn process_event(&mut self, event: KeyEvent, max_lines: u32) {
        if event.code == KeyCode::Esc {
            self.mode = EditorMode::Edit(None);
        }
        if let Char(c) = event.code {
            if c == 'c' && event.modifiers == KeyModifiers::CONTROL {
                self.should_exit = true;
            } else if c == ':' {
                self.mode = EditorMode::Command("".into());
            }
        }
        let old_mode = std::mem::replace(&mut self.mode, EditorMode::Edit(None));
        let new_mode = match &old_mode {
            EditorMode::Edit(value) => self.process_edit_event(value, event, max_lines),
            EditorMode::Command(value) => self.process_command_event(value, event),
        };
        self.mode = new_mode.unwrap_or(old_mode);
    }

    fn update_cursor(&mut self, cursor_update: i64) {
        if (self.cursor as i64 + cursor_update) < 0 {
            self.cursor = 0;
        } else if (self.cursor as i64 + cursor_update) >= self.buffer.len() as i64 {
            self.cursor = (self.buffer.len() - 1) as u32;
        } else {
            self.cursor = (self.cursor as i64 + cursor_update) as u32;
        }
    }

    fn process_edit_event(
        &mut self,
        input_buffer: &Option<u8>,
        event: KeyEvent,
        max_lines: u32,
    ) -> Option<EditorMode> {
        let cursor_update = self.process_cursor_update(event, max_lines);
        if cursor_update != 0 {
            self.update_cursor(cursor_update);

            while self.cursor >= (self.offset + max_lines * self.bytes_per_line) {
                self.offset += self.bytes_per_line;
            }
            while self.cursor < self.offset {
                self.offset -= self.bytes_per_line;
            }

            return Some(EditorMode::Edit(None));
        }
        if let Char(c) = event.code {
            if ('a'..='f').contains(&c) || c.is_ascii_digit() {
                let value = if ('a'..='f').contains(&c) {
                    c as u8 - b'a' + 10
                } else {
                    c as u8 - b'0'
                };
                return match input_buffer {
                    None => {
                        self.buffer.update(self.cursor as usize, value);
                        Some(EditorMode::Edit(Some(value)))
                    }
                    Some(previous_value) => {
                        self.buffer
                            .update(self.cursor as usize, (previous_value << 4) | value);
                        Some(EditorMode::Edit(None))
                    }
                };
            }
        }

        None
    }

    fn process_command_event(&mut self, command: &String, event: KeyEvent) -> Option<EditorMode> {
        if let Char(c) = event.code {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                let mut new_command = command.to_string();
                if command.len() < 16 {
                    new_command += &c.to_string();
                } else {
                    self.warning = "Cmd too long".into();
                }
                return Some(EditorMode::Command(new_command));
            }
            return None;
        } else if event.code == KeyCode::Backspace {
            if !command.is_empty() {
                return Some(EditorMode::Command(
                    command[..command.len() - 1].to_string(),
                ));
            }
        } else if event.code == KeyCode::Enter {
            self.process_command(command.as_str());
            return Some(EditorMode::Command("".into()));
        }

        None
    }

    fn process_command(&mut self, value: &str) {
        match value {
            "exit" | "quit" | "q" | "x" => {
                if self.buffer.is_dirty() {
                    self.warning = "Modified Buffer".into();
                } else {
                    self.should_exit = true;
                }
            }
            "wq" | "qw" => {
                if self.save() {
                    self.should_exit = true;
                }
            }
            "write" | "w" => {
                self.save();
            }
            _ => {
                self.warning = "Invalid command".into();
            }
        }
    }

    fn process_cursor_update(&mut self, event: KeyEvent, max_lines: u32) -> i64 {
        let mut cursor_update: i64 = 0;
        if event.code == KeyCode::Down {
            cursor_update = self.bytes_per_line as i64;
        } else if event.code == KeyCode::Up {
            cursor_update = -(self.bytes_per_line as i64);
        } else if event.code == KeyCode::Left {
            cursor_update = -1;
        } else if event.code == KeyCode::Right {
            cursor_update = 1;
        }
        if event.modifiers == KeyModifiers::CONTROL {
            cursor_update *= max_lines as i64;
        }
        cursor_update
    }

    fn save(&mut self) -> bool {
        if !self.buffer.is_dirty() {
            return true;
        }
        match self.buffer.save(&self.path) {
            Ok(_) => true,
            Err(_) => {
                self.warning = "Writing failed".into();
                false
            }
        }
    }

    fn redraw(&self, offset: u32, lines: u32) -> Result<(), std::io::Error> {
        Terminal::move_cursor_to(Position { x: 0, y: 0 })?;
        Terminal::set_foreground_color(Color::DarkYellow)?;
        print!("\r     Ashe");
        Terminal::set_foreground_color(Color::Reset)?;
        println!("      {}", self.path.file_name().unwrap().to_str().unwrap());
        draw_box_part(BoxPart::Top, self.bytes_per_line);
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
                        Terminal::set_background_color(Color::DarkYellow)?;
                    }
                    print!("{:0>2x}", self.buffer[position]);
                    if highlight {
                        Terminal::set_background_color(Color::Reset)?;
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
                        Terminal::set_background_color(Color::DarkYellow)?;
                    }
                    if byte.is_ascii() && !byte.is_ascii_control() {
                        print!("{}", byte as char);
                    } else {
                        Terminal::set_foreground_color(Color::Black)?;
                        print!(".");
                        Terminal::set_foreground_color(Color::Reset)?;
                    }
                    if highlight {
                        Terminal::set_background_color(Color::Reset)?;
                    }
                } else {
                    print!(" ");
                }
            }
            println!(" {}", tui::HORIZONTAL);
        }
        draw_box_part(BoxPart::Bottom, self.bytes_per_line);
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
        Terminal::set_foreground_color(Color::Red)?;
        print!("{}", self.warning);
        println!(
            "{}",
            " ".repeat(self.bytes_per_line as usize - self.warning.len())
        );

        Terminal::execute()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn setup_test_editor() -> Editor {
        // Helper function to initialize an Editor for testing.
        Editor {
            cursor: 0,
            bytes_per_line: 16,
            offset: 0,
            path: PathBuf::from("test.txt"),
            buffer: Buffer::new([0xa, 0xb, 0xc].repeat(100)),
            mode: EditorMode::Edit(None),
            warning: String::new(),
            should_exit: false,
        }
    }

    #[test]
    fn test_editor_initialization_with_bad_file() {
        let path = Path::new("invalid_file.txt");
        let bytes_per_line = 16;

        // File at path does not exist
        let editor = Editor::init(path, bytes_per_line);
        assert!(editor.is_err());
    }

    #[test]
    fn test_editor_update_cursor() {
        let mut editor = setup_test_editor();
        editor.update_cursor(5);
        assert_eq!(editor.cursor, 5);
        editor.update_cursor(-2);
        assert_eq!(editor.cursor, 3);
        editor.update_cursor(340);
        assert_eq!(editor.cursor, 299);
        editor.update_cursor(-1000);
        assert_eq!(editor.cursor, 0);
    }
    #[test]
    fn test_process_edit_event() {
        let mut editor = setup_test_editor();
        assert_eq!(editor.buffer[editor.cursor as usize], 0xa);
        let event = KeyEvent::new(Char('2'), KeyModifiers::NONE);
        let input_buffer = None;
        let new_mode = editor.process_edit_event(&input_buffer, event, 16);
        assert!(matches!(new_mode, Some(EditorMode::Edit(Some(2)))));
        assert_eq!(editor.buffer[editor.cursor as usize], 0x02);

        let event = KeyEvent::new(Char('1'), KeyModifiers::NONE);
        let input_buffer = Some(0x2);
        let new_mode = editor.process_edit_event(&input_buffer, event, 16);
        assert!(matches!(new_mode, Some(EditorMode::Edit(None))));
        assert_eq!(editor.buffer[editor.cursor as usize], 0x21);
    }

    #[test]
    fn test_process_command_event() {
        let mut editor = setup_test_editor();
        let command = String::from("abc");
        let event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let new_mode = editor.process_command_event(&command, event);
        assert!(matches!(new_mode, Some(EditorMode::Command(_))));
        assert_eq!(
            match new_mode {
                Some(EditorMode::Command(value)) => value,
                _ => "".into(),
            },
            "ab"
        );

        editor.buffer.update(0, 0x12);
        assert!(editor.buffer.is_dirty());

        let command = String::from("w");
        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        editor.process_command_event(&command, event);
        assert!(!editor.buffer.is_dirty());

        let command = String::from("q");
        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        editor.process_command_event(&command, event);
        assert!(editor.should_exit);

        editor.buffer.update(0, 0x12);
        assert!(editor.buffer.is_dirty());

        let command = String::from("wq");
        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        editor.process_command_event(&command, event);
        assert!(!editor.buffer.is_dirty());
        assert!(editor.should_exit);
    }

    #[test]
    fn test_process_command() {
        let mut editor = setup_test_editor();
        let command = "exit";

        editor.process_command(command);
        assert!(editor.should_exit);

        let mut editor = setup_test_editor();
        editor.buffer.update(0, 0x12);
        assert!(editor.buffer.is_dirty());
        editor.process_command(command);
        assert!(!editor.should_exit);
        assert!(!editor.warning.is_empty());

        let command = "wq";
        assert!(editor.buffer.is_dirty());
        editor.process_command(command);
        assert!(editor.should_exit);
    }

    #[test]
    fn test_process_cursor_update() {
        let mut editor = setup_test_editor();
        let max_lines = 10;

        let event = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(editor.process_cursor_update(event, max_lines), -1);
        let event = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(editor.process_cursor_update(event, max_lines), 1);
        let event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(
            editor.process_cursor_update(event, max_lines),
            editor.bytes_per_line as i64
        );
        let event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(
            editor.process_cursor_update(event, max_lines),
            -(editor.bytes_per_line as i64)
        );

        let event = KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL);
        assert_eq!(
            editor.process_cursor_update(event, max_lines),
            -(max_lines as i64)
        );
        let event = KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL);
        assert_eq!(
            editor.process_cursor_update(event, max_lines),
            max_lines as i64
        );
        let event = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL);
        assert_eq!(
            editor.process_cursor_update(event, max_lines),
            editor.bytes_per_line as i64 * max_lines as i64
        );
        let event = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        assert_eq!(
            editor.process_cursor_update(event, max_lines),
            -(editor.bytes_per_line as i64 * max_lines as i64)
        );
    }
}
