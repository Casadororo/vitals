//! What the screen shows, and how the keys change it.

use clap::ValueEnum;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::history::History;
use crate::i18n::Lang;
use crate::layout::Content;
use crate::limits::Usage;
use crate::model::Sample;
use crate::state::State;
use crate::theme::{self, Palette, THEMES, fold};

/// How the graphs are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum GraphStyle {
    /// Braille dots: two readings per column, four levels per row.
    #[default]
    Braille,
    /// Blocks: one reading per column, eight levels per row.
    Blocks,
}

/// Milliseconds between readings that `+` and `-` step through.
pub const INTERVALS: [u64; 8] = [250, 500, 1000, 2000, 3000, 5000, 10_000, 30_000];
/// Rows that Page Up and Page Down move in the theme list.
const PAGE: usize = 10;

pub enum Mode {
    Main,
    Themes(ThemePicker),
    Help,
}

/// The theme list. Moving through it shows each theme at once; Esc goes back
/// to the one in use before.
pub struct ThemePicker {
    pub query: String,
    /// Indices into `THEMES` whose names contain the query.
    pub matches: Vec<usize>,
    pub selected: usize,
    original: usize,
}

impl ThemePicker {
    fn new(current: usize) -> Self {
        Self {
            query: String::new(),
            matches: (0..THEMES.len()).collect(),
            selected: current,
            original: current,
        }
    }

    fn filter(&mut self) {
        let wanted = fold(&self.query);
        self.matches = (0..THEMES.len())
            .filter(|&index| fold(THEMES[index].name).contains(&wanted))
            .collect();
        self.selected = 0;
    }

    pub fn current(&self) -> Option<usize> {
        self.matches.get(self.selected).copied()
    }
}

pub struct App {
    pub lang: Lang,
    /// Index into `THEMES`.
    pub theme: usize,
    /// Whether the terminal shows 24-bit color; otherwise themes use the 256-color palette.
    pub truecolor: bool,
    pub graph_style: GraphStyle,
    pub show_graphs: bool,
    /// Each core's bar, or its column in a strip when space is short.
    pub show_cores: bool,
    /// Milliseconds between readings.
    pub interval: u64,
    /// The usage limits of the AI tools.
    pub show_limits: bool,
    pub limits: Vec<Usage>,
    pub mode: Mode,
    /// The latest reading; none until the first arrives.
    pub sample: Option<Sample>,
    pub history: History,
    pub quit: bool,
    /// The settings could not be saved.
    pub unsaved: bool,
}

impl App {
    pub fn new(lang: Lang) -> Self {
        Self {
            lang,
            theme: 0,
            truecolor: false,
            graph_style: GraphStyle::default(),
            show_graphs: true,
            show_cores: true,
            interval: 1000,
            show_limits: true,
            limits: Vec::new(),
            mode: Mode::Main,
            sample: None,
            history: History::default(),
            quit: false,
            unsaved: false,
        }
    }

    pub fn palette(&self) -> Palette {
        THEMES[self.theme].palette(self.truecolor)
    }

    pub fn push(&mut self, sample: Sample) {
        self.history.push(&sample);
        self.sample = Some(sample);
    }

    /// What the latest reading has to show, for the layout.
    pub fn content(&self) -> Content {
        let Some(sample) = &self.sample else {
            return Content::default();
        };
        let mut limits = [0; 4];
        let mut limit_used = [[0; 8]; 4];
        if self.show_limits {
            // An expired window already renewed: nothing used of it yet.
            let now = crate::limits::unix_now();
            for ((slot, used_slot), usage) in
                limits.iter_mut().zip(&mut limit_used).zip(&self.limits)
            {
                *slot = usage.windows.len().min(usize::from(u8::MAX)) as u8;
                for (used, window) in used_slot.iter_mut().zip(&usage.windows) {
                    *used = if window.resets_at.is_some_and(|at| at <= now) {
                        0
                    } else {
                        (window.used.clamp(0.0, 1.0) * 1000.0).round() as u16
                    };
                }
            }
        }
        Content {
            cores: sample.cpu.cores.len(),
            disks: sample.disks.len(),
            load: sample.load.is_some(),
            tasks: sample.tasks.is_some(),
            swap: sample.swap.total > 0,
            graphs: self.show_graphs,
            show_cores: self.show_cores,
            limits,
            limit_used,
            ..Content::default()
        }
    }

    /// What is saved between runs. While a theme is being picked, the one in
    /// use before counts.
    pub fn state(&self) -> State {
        let theme = match &self.mode {
            Mode::Themes(picker) => picker.original,
            _ => self.theme,
        };
        State {
            theme: THEMES[theme].name.to_owned(),
            graph: self
                .graph_style
                .to_possible_value()
                .map(|value| value.get_name().to_owned())
                .unwrap_or_default(),
            graphs: self.show_graphs,
            cores: self.show_cores,
            interval: self.interval,
            limits: self.show_limits,
        }
    }

    pub fn restore(&mut self, state: &State) {
        if !matches!(self.mode, Mode::Themes(_))
            && let Some(theme) = theme::find(&state.theme)
        {
            self.theme = theme;
        }
        if let Ok(style) = GraphStyle::from_str(&state.graph, true) {
            self.graph_style = style;
        }
        self.show_graphs = state.graphs;
        self.show_cores = state.cores;
        self.show_limits = state.limits;
        self.interval = state
            .interval
            .clamp(INTERVALS[0], INTERVALS[INTERVALS.len() - 1]);
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        self.mode = match std::mem::replace(&mut self.mode, Mode::Main) {
            Mode::Main => self.main_key(key),
            // Any key closes the help.
            Mode::Help => Mode::Main,
            Mode::Themes(picker) => self.themes_key(picker, key, ctrl),
        };
    }

    fn main_key(&mut self, key: KeyEvent) -> Mode {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('t') => return Mode::Themes(ThemePicker::new(self.theme)),
            KeyCode::Char('v') => {
                // A hidden graph comes back first.
                if self.show_graphs {
                    self.graph_style = match self.graph_style {
                        GraphStyle::Braille => GraphStyle::Blocks,
                        GraphStyle::Blocks => GraphStyle::Braille,
                    };
                }
                self.show_graphs = true;
            }
            KeyCode::Char('g') => self.show_graphs = !self.show_graphs,
            KeyCode::Char('c') => self.show_cores = !self.show_cores,
            KeyCode::Char('l') => self.show_limits = !self.show_limits,
            KeyCode::Char('+' | '=') => {
                self.interval = INTERVALS
                    .into_iter()
                    .find(|&interval| interval > self.interval)
                    .unwrap_or(self.interval);
            }
            KeyCode::Char('-' | '_') => {
                self.interval = INTERVALS
                    .into_iter()
                    .rev()
                    .find(|&interval| interval < self.interval)
                    .unwrap_or(self.interval);
            }
            KeyCode::Char('?' | 'h') | KeyCode::F(1) => return Mode::Help,
            _ => {}
        }
        Mode::Main
    }

    fn themes_key(&mut self, mut picker: ThemePicker, key: KeyEvent, ctrl: bool) -> Mode {
        let last = picker.matches.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc => {
                self.theme = picker.original;
                return Mode::Main;
            }
            KeyCode::Enter if picker.current().is_some() => return Mode::Main,
            KeyCode::Up => picker.selected = picker.selected.saturating_sub(1),
            KeyCode::Down => picker.selected = (picker.selected + 1).min(last),
            KeyCode::PageUp => picker.selected = picker.selected.saturating_sub(PAGE),
            KeyCode::PageDown => picker.selected = (picker.selected + PAGE).min(last),
            KeyCode::Home => picker.selected = 0,
            KeyCode::End => picker.selected = last,
            KeyCode::Backspace => {
                picker.query.pop();
                picker.filter();
            }
            KeyCode::Char('u') if ctrl => {
                picker.query.clear();
                picker.filter();
            }
            KeyCode::Char(ch) if !ctrl => {
                picker.query.push(ch);
                picker.filter();
            }
            _ => {}
        }
        if let Some(theme) = picker.current() {
            self.theme = theme;
        }
        Mode::Themes(picker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventKind;

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn typing(app: &mut App, text: &str) {
        for ch in text.chars() {
            press(app, KeyCode::Char(ch));
        }
    }

    fn theme_name(app: &App) -> &'static str {
        THEMES[app.theme].name
    }

    #[test]
    fn previews_themes_while_moving_through_the_list() {
        let mut app = App::new(Lang::En);
        press(&mut app, KeyCode::Char('t'));
        assert!(matches!(app.mode, Mode::Themes(_)));
        press(&mut app, KeyCode::Down);
        assert_eq!(theme_name(&app), "Tokyo Night");
        press(&mut app, KeyCode::Down);
        assert_eq!(theme_name(&app), "Tokyo Night Storm");
        assert_eq!(app.state().theme, "Terminal", "not saved while previewing");
        press(&mut app, KeyCode::Esc);
        assert_eq!(theme_name(&app), "Terminal", "Esc goes back");
        assert!(matches!(app.mode, Mode::Main));
    }

    #[test]
    fn picks_a_theme_by_typing_part_of_its_name() {
        let mut app = App::new(Lang::En);
        press(&mut app, KeyCode::Char('t'));
        typing(&mut app, "dracu");
        assert_eq!(theme_name(&app), "Dracula");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Main));
        assert_eq!(app.state().theme, "Dracula");
        press(&mut app, KeyCode::Char('t'));
        typing(&mut app, "zzz");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Themes(_)), "nothing to pick");
        press(&mut app, KeyCode::Esc);
        assert_eq!(theme_name(&app), "Dracula");
    }

    #[test]
    fn switches_graphs_cores_and_their_style() {
        let mut app = App::new(Lang::En);
        press(&mut app, KeyCode::Char('g'));
        assert!(!app.show_graphs);
        press(&mut app, KeyCode::Char('v'));
        assert!(app.show_graphs, "v brings a hidden graph back first");
        assert_eq!(app.graph_style, GraphStyle::Braille);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.graph_style, GraphStyle::Blocks);
        press(&mut app, KeyCode::Char('c'));
        assert!(!app.show_cores);
        assert!(!app.state().cores);
        press(&mut app, KeyCode::Char('l'));
        assert!(!app.show_limits && !app.state().limits);
    }

    #[test]
    fn steps_through_the_intervals() {
        let mut app = App::new(Lang::En);
        press(&mut app, KeyCode::Char('+'));
        assert_eq!(app.interval, 2000);
        for _ in 0..20 {
            press(&mut app, KeyCode::Char('+'));
        }
        assert_eq!(app.interval, 30_000);
        for _ in 0..20 {
            press(&mut app, KeyCode::Char('-'));
        }
        assert_eq!(app.interval, 250);
        app.restore(&State {
            interval: 1,
            ..State::default()
        });
        assert_eq!(app.interval, 250, "a hand-edited file cannot make it spin");
    }

    #[test]
    fn expired_limits_count_as_empty_for_the_responsive_order() {
        use crate::limits::{Period, Source, Tool, Usage, Window, unix_now};
        let now = unix_now();
        let mut app = App::new(Lang::En);
        app.sample = Some(crate::demo::sample());
        app.limits = vec![Usage {
            tool: Tool::Claude,
            plan: None,
            windows: vec![
                Window {
                    period: Period::Session,
                    used: 0.2,
                    resets_at: Some(now + 3600),
                    model: None,
                },
                Window {
                    period: Period::Week,
                    used: 0.99,
                    resets_at: Some(now - 10),
                    model: None,
                },
            ],
            source: Source::Live,
            as_of: now,
        }];
        let content = app.content();
        assert_eq!(content.limits[0], 2);
        assert_eq!(content.limit_used[0][0], 200);
        assert_eq!(
            content.limit_used[0][1], 0,
            "already renewed: nothing used of it yet"
        );
    }

    #[test]
    fn help_closes_with_any_key_and_q_quits() {
        let mut app = App::new(Lang::En);
        press(&mut app, KeyCode::Char('?'));
        assert!(matches!(app.mode, Mode::Help));
        press(&mut app, KeyCode::Char('q'));
        assert!(matches!(app.mode, Mode::Main) && !app.quit);
        press(&mut app, KeyCode::Char('q'));
        assert!(app.quit);
        let mut app = App::new(Lang::En);
        press(&mut app, KeyCode::Char('t'));
        let mut ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        ctrl_c.kind = KeyEventKind::Press;
        app.on_key(ctrl_c);
        assert!(app.quit, "Ctrl+C quits from anywhere");
    }
}
