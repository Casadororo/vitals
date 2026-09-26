//! Pictures of the screen for checking the design: a drawn buffer as plain
//! text or as an SVG with its colors.

use std::fmt::Write as _;

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

const CELL_WIDTH: f64 = 9.0;
const CELL_HEIGHT: f64 = 19.0;
const FONT_SIZE: f64 = 15.0;
const BACKGROUND: &str = "#15181e";
const FOREGROUND: &str = "#d6dae0";

pub fn text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        let line: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

pub fn svg(buf: &Buffer) -> String {
    let width = f64::from(buf.area.width) * CELL_WIDTH + 24.0;
    let height = f64::from(buf.area.height) * CELL_HEIGHT + 24.0;
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><rect width="100%" height="100%" rx="10" fill="{BACKGROUND}"/><g transform="translate(12 12)" font-family="Menlo, 'SF Mono', monospace" font-size="{FONT_SIZE}">"#
    );
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            let mut fg = color(cell.fg, FOREGROUND);
            let mut bg = color(cell.bg, BACKGROUND);
            if cell.modifier.contains(Modifier::REVERSED) {
                std::mem::swap(&mut fg, &mut bg);
            }
            let (left, top) = (f64::from(x) * CELL_WIDTH, f64::from(y) * CELL_HEIGHT);
            if bg != BACKGROUND {
                let _ = write!(
                    out,
                    r#"<rect x="{left}" y="{top}" width="{CELL_WIDTH}" height="{CELL_HEIGHT}" fill="{bg}"/>"#
                );
            }
            let symbol = cell.symbol();
            if symbol.trim().is_empty() {
                continue;
            }
            let mut attributes = String::new();
            if cell.modifier.contains(Modifier::BOLD) {
                attributes.push_str(r#" font-weight="bold""#);
            }
            if cell.modifier.contains(Modifier::ITALIC) {
                attributes.push_str(r#" font-style="italic""#);
            }
            if cell.modifier.contains(Modifier::DIM) {
                attributes.push_str(r#" opacity="0.55""#);
            }
            if cell.modifier.contains(Modifier::CROSSED_OUT) {
                attributes.push_str(r#" text-decoration="line-through""#);
            }
            let symbol = symbol
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            let baseline = top + CELL_HEIGHT * 0.76;
            let _ = write!(
                out,
                r#"<text x="{left}" y="{baseline}" fill="{fg}"{attributes} xml:space="preserve">{symbol}</text>"#
            );
        }
    }
    out.push_str("</g></svg>\n");
    out
}

fn color(color: Color, reset: &str) -> String {
    let hex = |r: u8, g: u8, b: u8| format!("#{r:02x}{g:02x}{b:02x}");
    match color {
        Color::Reset => reset.to_string(),
        Color::Rgb(r, g, b) => hex(r, g, b),
        Color::Indexed(n) => {
            let (r, g, b) = xterm(n);
            hex(r, g, b)
        }
        named => {
            let (r, g, b) = match named {
                Color::Black => (29, 31, 33),
                Color::Red => (204, 102, 102),
                Color::Green => (181, 189, 104),
                Color::Yellow => (240, 198, 116),
                Color::Blue => (129, 162, 190),
                Color::Magenta => (178, 148, 187),
                Color::Cyan => (138, 190, 183),
                Color::Gray => (197, 200, 198),
                Color::DarkGray => (102, 102, 102),
                Color::LightRed => (213, 78, 83),
                Color::LightGreen => (185, 202, 74),
                Color::LightYellow => (231, 197, 71),
                Color::LightBlue => (122, 166, 218),
                Color::LightMagenta => (195, 151, 216),
                Color::LightCyan => (112, 192, 177),
                _ => (234, 234, 234),
            };
            hex(r, g, b)
        }
    }
}

/// The xterm 256-color palette.
fn xterm(n: u8) -> (u8, u8, u8) {
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    match n {
        0..16 => BASE[usize::from(n)],
        16..232 => {
            let n = n - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (level(n / 36), level((n / 6) % 6), level(n % 6))
        }
        _ => {
            let gray = 8 + (n - 232) * 10;
            (gray, gray, gray)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::Mode;
    use crate::demo;
    use crate::i18n::Lang;
    use crate::theme;
    use crate::ui::tests::draw;

    #[test]
    fn the_xterm_palette_has_its_corners() {
        assert_eq!(xterm(16), (0, 0, 0));
        assert_eq!(xterm(231), (255, 255, 255));
        assert_eq!(xterm(196), (255, 0, 0));
        assert_eq!(xterm(232), (8, 8, 8));
    }

    #[test]
    fn text_keeps_the_rows_and_drops_trailing_spaces() {
        let app = demo::app(Lang::En);
        let picture = text(&draw(&app, 40, 6));
        assert_eq!(picture.lines().count(), 6);
        assert!(picture.lines().all(|line| !line.ends_with(' ')));
        assert!(svg(&draw(&app, 40, 6)).starts_with("<svg"));
    }

    /// Prints this computer's screen, to check a system without a terminal:
    /// `cargo test this_computer -- --ignored --nocapture`.
    #[test]
    #[ignore = "prints this computer's readings for a person to look at"]
    fn this_computer() {
        let mut collector = crate::collect::Collector::new();
        let mut app = crate::app::App::new(Lang::En);
        for _ in 0..2 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            app.push(collector.sample());
        }
        println!("{}", text(&draw(&app, 110, 30)));
    }

    /// Writes pictures of the made-up computer at several window sizes into
    /// the folder in `VITALS_PICTURES`:
    /// `VITALS_PICTURES=/tmp/p cargo test pictures -- --ignored`.
    #[test]
    #[ignore = "writes files for a person to look at"]
    fn pictures() {
        let dir = PathBuf::from(std::env::var("VITALS_PICTURES").expect("set VITALS_PICTURES"));
        std::fs::create_dir_all(&dir).unwrap();
        let shots: [(&str, u16, u16, &str); 14] = [
            ("wide", 200, 50, "Tokyo Night"),
            ("laptop", 140, 40, "Catppuccin Mocha"),
            ("half", 100, 50, "Gruvbox Dark"),
            ("classic", 80, 24, "Dracula"),
            ("tall-narrow", 50, 60, "Nord"),
            ("narrow", 38, 34, "One Dark"),
            ("short-wide", 180, 16, "Tokyo Night"),
            ("small", 60, 14, "Rosé Pine"),
            ("tiny", 24, 6, "Tokyo Night"),
            ("strip", 140, 1, "Tokyo Night"),
            ("two-rows", 90, 2, "Tokyo Night"),
            ("light", 120, 36, "GitHub Light"),
            ("terminal", 120, 36, "Terminal"),
            ("themes", 100, 30, "Everforest Dark"),
        ];
        for (name, width, height, theme) in shots {
            let mut app = demo::app(Lang::En);
            app.theme = theme::find(theme).unwrap();
            if name == "themes" {
                app.on_key(ratatui::crossterm::event::KeyEvent::new(
                    ratatui::crossterm::event::KeyCode::Char('t'),
                    ratatui::crossterm::event::KeyModifiers::NONE,
                ));
                assert!(matches!(app.mode, Mode::Themes(_)));
            }
            let buf = draw(&app, width, height);
            std::fs::write(dir.join(format!("{name}.svg")), svg(&buf)).unwrap();
            std::fs::write(dir.join(format!("{name}.txt")), text(&buf)).unwrap();
        }
    }
}
