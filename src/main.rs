mod ashe;

use ashe::editor::Editor;
use ashe::terminal::Terminal;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "A Simple Hex Editor", long_about = None)]
struct Args {
    /// File to read
    file: PathBuf,

    /// Number of bytes to display per line
    #[arg(short, long, default_value_t = 16)]
    bytes_per_line: u32,
}

fn main() {
    let args = Args::parse();
    Editor::new(&args.file, args.bytes_per_line)
        .expect("Failed to initialize editor")
        .run()
        .expect("Failed to run editor");

    Terminal::terminate().unwrap();
    println!("\r");
}
