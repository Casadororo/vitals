//! Pictures of the screen for checking the design: a drawn buffer as plain
//! text or as an SVG with its colors, drawn by cerne.

pub use cerne::snapshot::{parse_size, svg, text};

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
