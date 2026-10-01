//! Drawing: the header, the panels, the key hints and the pop-ups.

mod panels;
pub mod widgets;

use std::collections::HashMap;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Widget};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Mode, ThemePicker};
use crate::i18n::Keys;
use crate::layout::{self, Screen};
use crate::theme::{Paint, THEMES};
use panels::Ctx;
use widgets::{Piece, fit};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let palette = app.palette();
    let ctx = app.sample.as_ref().map(|sample| Ctx {
        app,
        sample,
        palette,
        text: app.lang.text(),
        lang: app.lang,
    });
    let mut content = app.content();
    if let Some(ctx) = &ctx {
        (content.cpu_times_width, content.mem_parts_width) = panels::measure(ctx);
    }
    let screen = layout::split(area, &content);
    let buf = frame.buffer_mut();
    buf.set_style(area, palette.base());
    if let Some(header) = screen.header {
        render_header(app, header, buf);
    }
    match &ctx {
        Some(ctx) => render_panels(ctx, &screen, buf),
        None if !screen.body.is_empty() => {
            let body = screen.body;
            let row = Rect {
                y: body.y + body.height / 2,
                height: 1,
                ..body
            };
            Line::styled(app.lang.text().reading, palette.muted())
                .centered()
                .render(row, buf);
        }
        None => {}
    }
    if let Some(footer) = screen.footer {
        render_keys(app, footer, buf);
    }
    match &app.mode {
        Mode::Main => {}
        Mode::Themes(picker) => render_themes(frame, app, picker),
        Mode::Help => render_help(frame, app),
    }
}

fn render_panels(ctx: &Ctx, screen: &Screen, buf: &mut Buffer) {
    // Bare lines of a column share one label width, so their bars line up.
    let mut label_widths: HashMap<u16, u16> = HashMap::new();
    for panel in screen.panels.iter().filter(|panel| !panel.boxed) {
        for &(item, _) in &panel.items {
            if let Some(label) = panels::bare_label(ctx, item) {
                let width = label.width().min(10) as u16;
                let widest = label_widths.entry(panel.area.x).or_default();
                *widest = (*widest).max(width);
            }
        }
    }
    for panel in &screen.panels {
        if panel.boxed {
            let (title, headline) = panels::title(ctx, panel.kind);
            let mut block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(ctx.palette.muted())
                .title_top(Line::from(Span::styled(
                    format!(" {title} "),
                    Style::new().fg(ctx.palette.title).bold(),
                )));
            if let Some((headline, color)) = headline {
                block = block.title_top(
                    Line::from(Span::styled(
                        format!(" {headline} "),
                        Style::new().fg(color).bold(),
                    ))
                    .right_aligned(),
                );
            }
            block.render(panel.area, buf);
        }
        let label_width =
            (!panel.boxed).then(|| label_widths.get(&panel.area.x).copied().unwrap_or_default());
        for &(item, area) in &panel.items {
            panels::draw(ctx, item, area, label_width, &panel.items, buf);
        }
    }
}

/// The computer's name, system and chip on the left; uptime and interval on the right.
fn render_header(app: &App, area: Rect, buf: &mut Buffer) {
    let palette = app.palette();
    let (text, lang) = (app.lang.text(), app.lang);
    let muted = palette.muted();
    let mut right = vec![Piece::new(
        2,
        vec![Span::styled(
            format!("{} {}", text.every, lang.interval(app.interval)),
            muted,
        )],
    )];
    let mut left = Vec::new();
    if let Some(sample) = &app.sample {
        let host = &sample.host;
        right.insert(
            0,
            Piece::new(
                0,
                vec![
                    Span::styled(format!("{} ", text.up), muted),
                    Span::raw(lang.duration(sample.uptime)),
                ],
            ),
        );
        if !host.name.is_empty() {
            left.push(Piece::new(
                0,
                vec![
                    Span::styled("● ", Style::new().fg(palette.low)),
                    Span::styled(host.name.clone(), Style::new().fg(palette.title).bold()),
                ],
            ));
        }
        if !host.os.is_empty() {
            left.push(Piece::new(1, vec![Span::raw(host.os.clone())]));
        }
        if !host.cpu.is_empty() {
            left.push(Piece::new(2, vec![Span::styled(host.cpu.clone(), muted)]));
        }
        let mut cores = format!("{} {}", host.cores, text.cores);
        if !host.core_kinds.is_empty() {
            let kinds: Vec<String> = host
                .core_kinds
                .iter()
                .map(|(kind, count)| format!("{count}{kind}"))
                .collect();
            cores.push_str(&format!(" ({})", kinds.join("+")));
        }
        left.push(Piece::new(3, vec![Span::styled(cores, muted)]));
    }
    let inner = Rect {
        x: area.x + 1.min(area.width),
        width: area.width.saturating_sub(2),
        ..area
    };
    let right = fit(right, usize::from(inner.width) / 2, muted);
    let right_width = right.width() as u16;
    right.render(
        Rect {
            x: inner.right() - right_width.min(inner.width),
            width: right_width.min(inner.width),
            ..inner
        },
        buf,
    );
    let left_width = inner.width.saturating_sub(right_width + 2);
    fit(left, usize::from(left_width), muted).render(
        Rect {
            width: left_width,
            ..inner
        },
        buf,
    );
}

fn render_keys(app: &App, area: Rect, buf: &mut Buffer) {
    let text = app.lang.text();
    let palette = app.palette();
    let keys: Keys = match &app.mode {
        Mode::Main => text.main_keys,
        Mode::Themes(_) => text.theme_keys,
        Mode::Help => &[],
    };
    let warning = app.unsaved.then(|| format!("{} ", text.unsaved));
    let room =
        usize::from(area.width).saturating_sub(warning.as_ref().map_or(0, |w| w.width() + 1));
    let mut spans = Vec::new();
    let mut width = 0;
    for (key, action) in keys {
        let item = key.width() + action.width() + 3;
        if width + item > room {
            break;
        }
        width += item;
        spans.push(Span::styled(
            format!(" {key}"),
            Style::new().fg(palette.accent).bold(),
        ));
        spans.push(Span::styled(format!(" {action} "), palette.muted()));
    }
    Line::from(spans).render(area, buf);
    if let Some(warning) = warning {
        Line::styled(warning, Style::new().fg(palette.high))
            .right_aligned()
            .render(area, buf);
    }
}

/// A bordered box centered in the window, cleared, with the space inside it.
fn popup(frame: &mut Frame, app: &App, title: &str, size: (u16, u16)) -> Rect {
    let area = frame.area();
    let (width, height) = (size.0.min(area.width), size.1.min(area.height));
    let area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let palette = app.palette();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {title} "))
        .border_style(Style::new().fg(palette.accent));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.buffer_mut().set_style(area, palette.base());
    frame.render_widget(block, area);
    inner
}

/// The themes, each with a strip of its colors: background, low, middle and
/// high usage, and the accent.
fn render_themes(frame: &mut Frame, app: &App, picker: &ThemePicker) {
    let text = app.lang.text();
    let area = popup(frame, app, text.themes, (40, 22));
    if area.is_empty() {
        return;
    }
    let prompt = "› ";
    let room = usize::from(area.width).saturating_sub(prompt.width() + 1);
    let mut shown = picker.query.as_str();
    while shown.width() > room {
        let mut chars = shown.chars();
        chars.next();
        shown = chars.as_str();
    }
    frame.render_widget(
        Line::from(vec![Span::raw(prompt).bold(), Span::raw(shown)]),
        Rect { height: 1, ..area },
    );
    let cursor = area.x + (prompt.width() + shown.width()) as u16;
    frame.set_cursor_position((cursor.min(area.right().saturating_sub(1)), area.y));
    let list = Rect {
        y: area.y + 1,
        height: area.height - 1,
        ..area
    };
    if list.is_empty() {
        return;
    }
    if picker.matches.is_empty() {
        let note = Line::styled(text.no_theme, app.palette().muted());
        frame.render_widget(note, Rect { height: 1, ..list });
        return;
    }
    let visible = usize::from(list.height);
    let first = picker.selected.saturating_sub(visible - 1);
    for (row, (position, &index)) in picker
        .matches
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .enumerate()
    {
        let theme = THEMES[index].palette(app.truecolor);
        let mut spans = vec![Span::raw(" ")];
        for color in [theme.bg, theme.low, theme.middle, theme.high, theme.accent] {
            spans.push(Span::styled("█", Style::new().fg(color)));
        }
        spans.push(Span::raw(" "));
        let name = Span::raw(format!(" {} ", THEMES[index].name));
        spans.push(if position == picker.selected {
            name.reversed().bold()
        } else {
            name
        });
        let row_area = Rect {
            y: list.y + row as u16,
            height: 1,
            ..list
        };
        frame.render_widget(Line::from(spans), row_area);
    }
}

fn render_help(frame: &mut Frame, app: &App) {
    let text = app.lang.text();
    let keys = text.help_keys;
    let area = popup(frame, app, text.help, (60, keys.len() as u16 + 2));
    let key_width = keys.iter().map(|(key, _)| key.width()).max().unwrap_or(0) + 2;
    for (row, (key, action)) in keys.iter().enumerate() {
        if row >= usize::from(area.height) {
            break;
        }
        let line = Line::from(vec![
            Span::styled(
                format!(" {key:<key_width$}"),
                Style::new().fg(app.palette().accent).bold(),
            ),
            Span::raw(*action),
        ]);
        frame.render_widget(
            line,
            Rect {
                y: area.y + row as u16,
                height: 1,
                ..area
            },
        );
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::demo;
    use crate::i18n::Lang;
    use crate::model::{CpuTimes, Split};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    pub fn draw(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        terminal.backend().buffer().clone()
    }

    pub fn screen(buf: &Buffer) -> String {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn draws_every_window_size() {
        for lang in [Lang::En, Lang::Pt] {
            let mut app = demo::app(lang);
            for (width, height) in (0..=240)
                .step_by(9)
                .flat_map(|width| (0..=80).step_by(5).map(move |height| (width, height)))
            {
                app.show_cores = width % 2 == 0;
                draw(&app, width, height);
            }
        }
    }

    #[test]
    fn says_it_is_reading_before_the_first_reading() {
        let app = App::new(Lang::En);
        assert!(screen(&draw(&app, 60, 20)).contains("Reading…"));
    }

    #[test]
    fn eight_busy_cores_of_ten_show_as_eighty_percent() {
        let mut app = demo::app(Lang::En);
        let sample = app.sample.as_mut().unwrap();
        sample.cpu.cores = vec![CpuTimes::from_busy(1.0); 8];
        sample.cpu.cores.extend([CpuTimes::from_busy(0.0); 2]);
        sample.cpu.total = CpuTimes::from_busy(0.8);
        sample.cpu.split = Split::Busy;
        let text = screen(&draw(&app, 100, 30));
        assert!(text.contains(" 80% "), "{text}");
        assert!(!text.contains("800%"));
    }

    #[test]
    fn shows_htop_readings_in_a_classic_terminal() {
        let app = demo::app(Lang::En);
        let text = screen(&draw(&app, 80, 24));
        for wanted in [
            "CPU",
            "Memory",
            "Swap",
            "Disks",
            "load",
            "processes",
            "threads",
            "user",
            "system",
            "pressure",
            "free",
            "up 3d 5h",
        ] {
            assert!(text.contains(wanted), "{wanted} missing:\n{text}");
        }
    }

    #[test]
    fn speaks_portuguese() {
        let app = demo::app(Lang::Pt);
        let text = screen(&draw(&app, 100, 30));
        for wanted in [
            "Memória",
            "Discos",
            "carga",
            "processos",
            "ligado há",
            "livres",
        ] {
            assert!(text.contains(wanted), "{wanted} missing:\n{text}");
        }
    }

    #[test]
    fn shows_the_ai_limits_and_when_they_reset() {
        let app = demo::app(Lang::En);
        let text = screen(&draw(&app, 120, 40));
        for wanted in [
            "AI limits",
            "Claude",
            "Max 5x",
            "live",
            "Codex",
            "25m 0",
            "session",
            "week",
            "57%",
            "86%",
            "in 1h 32m",
        ] {
            assert!(text.contains(wanted), "{wanted} missing:\n{text}");
        }
        let app = demo::app(Lang::Pt);
        let text = screen(&draw(&app, 120, 40));
        for wanted in [
            "Limites de IA",
            "sessão",
            "semana",
            "renova",
            "em 1h 32min",
            "há 25min 0",
        ] {
            assert!(text.contains(wanted), "{wanted} missing:\n{text}");
        }
    }

    #[test]
    fn a_stale_live_reading_shows_its_age_instead_of_live() {
        use crate::limits::{Period, Source, Tool, Usage, Window, unix_now};
        let now = unix_now();
        let mut app = demo::app(Lang::En);
        app.limits = vec![Usage {
            tool: Tool::Claude,
            plan: None,
            windows: vec![Window {
                period: Period::Session,
                used: 0.5,
                resets_at: Some(now + 3600),
                model: None,
            }],
            source: Source::Live,
            as_of: now - 25 * 3600,
            account: None,
        }];
        let text = screen(&draw(&app, 120, 40));
        assert!(
            !text.contains("live"),
            "a day-old reading is not live:\n{text}"
        );
        assert!(text.contains("ago"), "{text}");
        app.limits[0].as_of = now;
        let text = screen(&draw(&app, 120, 40));
        assert!(text.contains("live"), "{text}");
    }

    #[test]
    fn l_hides_the_ai_limits() {
        let mut app = demo::app(Lang::En);
        app.on_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        let text = screen(&draw(&app, 120, 40));
        assert!(
            !text.contains("Claude") && !text.contains("Codex"),
            "the key hint says AI limits, the panel is gone:\n{text}"
        );
    }

    #[test]
    fn a_one_line_pane_shows_every_bar() {
        let app = demo::app(Lang::En);
        let text = screen(&draw(&app, 120, 1));
        for wanted in ["CPU", "Mem", "Swap", "/"] {
            assert!(text.contains(wanted), "{wanted} missing: {text}");
        }
    }

    #[test]
    fn lists_the_themes_with_their_colors() {
        let mut app = demo::app(Lang::En);
        app.on_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        let text = screen(&draw(&app, 100, 30));
        assert!(
            text.contains("Themes") && text.contains("Tokyo Night"),
            "{text}"
        );
    }

    #[test]
    fn shows_the_keys() {
        let mut app = demo::app(Lang::En);
        app.on_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        let text = screen(&draw(&app, 100, 30));
        assert!(text.contains("8 of 10 cores fully busy is 80%"), "{text}");
    }

    #[test]
    fn paints_the_screen_with_the_theme() {
        let mut app = demo::app(Lang::En);
        app.theme = crate::theme::find("Tokyo Night").unwrap();
        let buf = draw(&app, 80, 24);
        let palette = app.palette();
        assert_eq!(buf[(0, 0)].bg, palette.bg);
        assert!(
            (0..buf.area.width).any(|x| buf[(x, 2)].fg == palette.user),
            "the CPU bar is in the theme's green"
        );
    }
}
