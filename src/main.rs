mod app;
mod collect;
mod demo;
mod history;
mod i18n;
mod layout;
mod limits;
mod model;
mod snapshot;
mod state;
mod theme;
mod ui;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{self, KeyEventKind};
use ratatui::{DefaultTerminal, Terminal};

use crate::app::{App, GraphStyle, INTERVALS};
use crate::collect::Collector;
use crate::i18n::Lang;
use crate::model::Sample;
use crate::state::Store;
use crate::theme::THEMES;

/// Terminal monitor of CPU, memory, swap and disk space, with htop's load
/// average, tasks and uptime. CPU is the share of all cores together: 8 of 10
/// cores fully busy is 80%. The layout fits anything from a full screen to a
/// one-line strip, and every setting is saved.
#[derive(Parser)]
#[command(version, about, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Color theme, such as "Tokyo Night", "dracula" or "gruvbox" (see --list-themes)
    #[arg(short, long)]
    theme: Option<String>,

    /// List the themes and exit
    #[arg(long)]
    list_themes: bool,

    /// Seconds between readings, from 0.25 to 30 [default: 1]
    #[arg(short, long, value_name = "SECONDS")]
    interval: Option<f64>,

    /// Look of the graphs
    #[arg(long, value_enum)]
    graph: Option<GraphStyle>,

    /// Hide the graphs
    #[arg(long)]
    no_graphs: bool,

    /// Hide each core's bar
    #[arg(long)]
    no_cores: bool,

    /// Hide the Claude and Codex usage limits, and do not read them
    #[arg(long)]
    no_limits: bool,

    /// Interface language [default: from $LANG]
    #[arg(long, value_enum)]
    lang: Option<Lang>,

    /// File that keeps the settings between runs
    /// [default: ~/.config/vitals/state.json]
    #[arg(long, value_name = "FILE")]
    state: Option<PathBuf>,

    /// Prints one screen of the given size, such as 120x40, and exits
    #[arg(long, hide = true, value_name = "WxH", value_parser = snapshot::parse_size)]
    snapshot: Option<(u16, u16)>,

    /// With --snapshot, writes an SVG picture to this file instead
    #[arg(long, hide = true, value_name = "FILE")]
    svg: Option<PathBuf>,

    /// With --snapshot, shows made-up readings instead of this computer's
    #[arg(long, hide = true)]
    demo: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Update vitals to the latest release
    Update {
        /// Only tell whether a newer version is out
        #[arg(long)]
        check: bool,
    },
}

/// What wakes the screen up: a new reading or something the person did.
enum Event {
    Sample(Box<Sample>),
    Limits(Vec<limits::Usage>),
    Terminal(event::Event),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Some(Command::Update { check }) = cli.command {
        let version = env!("CARGO_PKG_VERSION");
        return cerne::update::run("victorlcampos/vitals", "vitals", version, check);
    }
    if cli.list_themes {
        for theme in &THEMES {
            println!("{}", theme.name);
        }
        return ExitCode::SUCCESS;
    }
    let mut app = App::new(cli.lang.unwrap_or_else(Lang::from_env));
    app.truecolor = theme::truecolor_from_env();

    let mut store = cli
        .state
        .clone()
        .or_else(state::default_path)
        .map(Store::new);
    // A snapshot is a look, not a change: it neither reads nor writes the settings.
    if cli.snapshot.is_some() {
        store = None;
    }
    if let Some(store) = &mut store {
        store.load(&mut app);
    }

    // Options act as if set in the app, so they are saved as well.
    if let Some(name) = &cli.theme {
        let Some(index) = theme::find(name) else {
            eprintln!("vitals: unknown theme {name:?}; see --list-themes");
            return ExitCode::from(2);
        };
        app.theme = index;
    }
    if let Some(seconds) = cli.interval {
        let range = INTERVALS[0]..=INTERVALS[INTERVALS.len() - 1];
        let millis = (seconds * 1000.0).round();
        if !millis.is_finite() || !range.contains(&(millis as u64)) || millis < 0.0 {
            eprintln!("vitals: the interval goes from 0.25 to 30 seconds");
            return ExitCode::from(2);
        }
        app.interval = millis as u64;
    }
    if let Some(style) = cli.graph {
        app.graph_style = style;
    }
    if cli.no_graphs {
        app.show_graphs = false;
    }
    if cli.no_cores {
        app.show_cores = false;
    }
    if cli.no_limits {
        app.show_limits = false;
    }

    if let Some((width, height)) = cli.snapshot {
        return snapshot(app, width, height, cli.svg, cli.demo);
    }
    match ratatui::run(|terminal| run(terminal, &mut app, &mut store)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vitals: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, store: &mut Option<Store>) -> io::Result<()> {
    let (sender, events) = mpsc::channel();
    let (intervals, interval_changes) = mpsc::channel();
    collect::spawn(
        sender.clone(),
        |sample| Event::Sample(Box::new(sample)),
        interval_changes,
        Duration::from_millis(app.interval),
    );
    let (limits_wanted, wanted_changes) = mpsc::channel();
    limits::spawn(
        sender.clone(),
        Event::Limits,
        wanted_changes,
        app.show_limits,
    );
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if sender.send(Event::Terminal(event)).is_err() {
                break;
            }
        }
    });
    let mut interval = app.interval;
    let mut show_limits = app.show_limits;
    loop {
        // Saved on every change, not on exit, so a power cut loses nothing.
        if let Some(store) = store.as_mut() {
            app.unsaved = store.sync(app).is_err();
        }
        if app.quit {
            return Ok(());
        }
        if app.interval != interval {
            interval = app.interval;
            let _ = intervals.send(Duration::from_millis(interval));
        }
        if app.show_limits != show_limits {
            show_limits = app.show_limits;
            let _ = limits_wanted.send(show_limits);
        }
        terminal.draw(|frame| ui::render(frame, app))?;
        let first = events.recv().map_err(io::Error::other)?;
        for event in std::iter::once(first).chain(events.try_iter()) {
            match event {
                Event::Sample(sample) => app.push(*sample),
                Event::Limits(limits) => app.limits = limits,
                Event::Terminal(event::Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    app.on_key(key);
                }
                // A resize just draws again.
                Event::Terminal(_) => {}
            }
        }
    }
}

/// Draws one screen off the terminal: as text on the standard output, or as
/// an SVG picture.
fn snapshot(mut app: App, width: u16, height: u16, svg: Option<PathBuf>, demo: bool) -> ExitCode {
    if demo {
        let (theme, interval) = (app.theme, app.interval);
        app = demo::app(app.lang);
        (app.theme, app.interval) = (theme, interval);
    } else {
        let mut collector = Collector::new();
        for _ in 0..2 {
            thread::sleep(Duration::from_millis(500));
            app.push(collector.sample());
        }
        if app.show_limits {
            app.limits = limits::read_once();
        }
    }
    let mut terminal = match Terminal::new(TestBackend::new(width, height)) {
        Ok(terminal) => terminal,
        Err(error) => {
            eprintln!("vitals: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = terminal.draw(|frame| ui::render(frame, &app)) {
        eprintln!("vitals: {error}");
        return ExitCode::FAILURE;
    }
    let buf = terminal.backend().buffer();
    match svg {
        Some(path) => {
            if let Err(error) = std::fs::write(&path, snapshot::svg(buf)) {
                eprintln!("vitals: {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
        None => print!("{}", snapshot::text(buf)),
    }
    ExitCode::SUCCESS
}
