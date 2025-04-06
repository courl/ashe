pub const VERTICAL: &str = "\u{2500}";
pub const HORIZONTAL: &str = "\u{2502}";
pub const TOP_T: &str = "\u{252c}";
pub const BOTTOM_T: &str = "\u{2534}";
pub const TOP_LEFT_CORNER: &str = "\u{256d}";
pub const TOP_RIGHT_CORNER: &str = "\u{256e}";
pub const BOTTOM_RIGHT_CORNER: &str = "\u{256f}";
pub const BOTTOM_LEFT_CORNER: &str = "\u{2570}";

pub enum BoxPart {
    Top,
    Bottom,
}

pub fn draw_box_part(part: BoxPart, bytes_per_line: u32) {
    println!(
        "\r {}{}{}{}{}{}{}",
        match part {
            BoxPart::Top => TOP_LEFT_CORNER,
            BoxPart::Bottom => BOTTOM_LEFT_CORNER,
        },
        VERTICAL.repeat(11),
        match part {
            BoxPart::Top => TOP_T,
            BoxPart::Bottom => BOTTOM_T,
        },
        VERTICAL.repeat((3 * bytes_per_line + 1) as usize),
        match part {
            BoxPart::Top => TOP_T,
            BoxPart::Bottom => BOTTOM_T,
        },
        VERTICAL.repeat(bytes_per_line as usize + 2),
        match part {
            BoxPart::Top => TOP_RIGHT_CORNER,
            BoxPart::Bottom => BOTTOM_RIGHT_CORNER,
        }
    );
}
