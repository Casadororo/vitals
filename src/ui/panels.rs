//! The lines, bars and graphs inside the panels.

use std::path::Path;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use super::widgets::{LEVELS, Piece, bar, fit_rows, graph, natural_width, shorten};
use crate::app::{App, GraphStyle};
use crate::i18n::{Lang, Text, percent};
use crate::layout::{CORE_GAP, CoreView, Item, Kind, core_index_width};
use crate::limits::{LIVE_FRESH_SECS, Period, Source, Usage, Window, unix_now};
use crate::model::{CpuTimes, Disk, Part, PressureLevel, Sample, Split, ratio};
use crate::theme::Palette;

/// What every part of a panel needs to draw itself.
pub struct Ctx<'a> {
    pub app: &'a App,
    pub sample: &'a Sample,
    pub palette: Palette,
    pub text: &'static Text,
    pub lang: Lang,
}

impl Ctx<'_> {
    fn muted<'s>(&self, text: impl Into<std::borrow::Cow<'s, str>>) -> Span<'s> {
        Span::styled(text, self.palette.muted())
    }

    fn swatch(&self, color: Color) -> Span<'static> {
        Span::styled("■ ", Style::new().fg(color))
    }

    /// The pieces over the rows of `area`, dropping the least important
    /// when they do not fit.
    fn line(&self, pieces: Vec<Piece<'_>>, area: Rect, buf: &mut Buffer) {
        let lines = fit_rows(
            pieces,
            usize::from(area.width),
            usize::from(area.height),
            self.palette.muted(),
        );
        for (row, line) in lines.into_iter().enumerate() {
            let row = Rect {
                y: area.y + row as u16,
                height: 1,
                ..area
            };
            line.render(row, buf);
        }
    }
}

/// Columns the CPU time and the memory slice lines take on one row.
pub fn measure(ctx: &Ctx) -> (u16, u16) {
    let width = |pieces: &[Piece]| natural_width(pieces).min(usize::from(u16::MAX)) as u16;
    (width(&cpu_times(ctx)), width(&memory_parts(ctx)))
}

/// The label a reading has in a bare line, where there is no panel title.
pub fn bare_label(ctx: &Ctx, item: Item) -> Option<String> {
    let text = ctx.text;
    match item {
        Item::CpuBar => Some(text.cpu.to_owned()),
        Item::MemBar => Some(text.mem_short.to_owned()),
        Item::SwapBar => Some(text.swap_short.to_owned()),
        Item::DiskBar(index) => ctx.sample.disks.get(index).map(short_name),
        Item::Limit(tool, window) => {
            let usage = ctx.app.limits.get(tool)?;
            let window = usage.windows.get(window)?;
            let span = match (&window.model, window.period) {
                (Some(model), _) => model.clone(),
                (None, Period::Session) => "5h".to_owned(),
                (None, Period::Week) => "7d".to_owned(),
                (None, Period::Month) => "30d".to_owned(),
                (None, Period::Minutes(minutes)) => format!("{}h", minutes / 60),
            };
            Some(format!("{} {span}", usage.name()))
        }
        _ => None,
    }
}

/// "/" for the root, the last folder for the rest: "Backup", "home".
fn short_name(disk: &Disk) -> String {
    if disk.mount == "/" {
        return disk.mount.clone();
    }
    Path::new(&disk.mount).file_name().map_or_else(
        || disk.mount.clone(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Title and headline percentage of a boxed panel.
pub fn title(ctx: &Ctx, kind: Kind) -> (&'static str, Option<(String, Color)>) {
    let text = ctx.text;
    let sample = ctx.sample;
    let headline = |share: f64| Some((percent(share), ctx.palette.level(share)));
    match kind {
        Kind::Cpu => (text.cpu, headline(sample.cpu.total.busy())),
        Kind::Memory => (text.memory, headline(sample.memory.used_ratio())),
        Kind::Swap if sample.swap.total > 0 => (text.swap, headline(sample.swap.used_ratio())),
        Kind::Swap => (text.swap, None),
        Kind::Disks => (text.disks, None),
        Kind::Limits => (text.limits, None),
    }
}

/// Draws `item` in `area`, next to the other `items` of its panel. A bare
/// item starts with its label, `label_width` wide, and its percentage:
/// "CPU   24% ███░░".
pub fn draw(
    ctx: &Ctx,
    item: Item,
    area: Rect,
    label_width: Option<u16>,
    items: &[(Item, Rect)],
    buf: &mut Buffer,
) {
    let sample = ctx.sample;
    let palette = &ctx.palette;
    let labeled = |ctx: &Ctx| label_width.zip(bare_label(ctx, item));
    match item {
        Item::CpuBar => {
            let total = sample.cpu.total;
            let slices = cpu_slices(palette, &total, sample.cpu.split);
            meter(ctx, labeled(ctx), total.busy(), &slices, area, buf);
        }
        Item::CpuTimes => ctx.line(cpu_times(ctx), area, buf),
        Item::CpuLoad => cpu_load(ctx, area, buf),
        Item::CpuTasks => cpu_tasks(ctx, area, buf),
        Item::CpuGraph => history(ctx, &ctx.app.history.cpu, area, buf),
        Item::Cores(CoreView::Grid { columns }) => cores_grid(ctx, columns, area, buf),
        Item::Cores(CoreView::Strip) => cores_strip(ctx, area, buf),
        Item::MemBar => {
            let memory = &sample.memory;
            let slices: Vec<(f64, Color)> = memory
                .parts
                .iter()
                .map(|&(part, bytes)| (ratio(bytes, memory.total), palette.part(part)))
                .collect();
            meter(ctx, labeled(ctx), memory.used_ratio(), &slices, area, buf);
        }
        Item::MemNumbers => memory_numbers(ctx, area, buf),
        Item::MemParts => ctx.line(memory_parts(ctx), area, buf),
        Item::MemGraph => history(ctx, &ctx.app.history.memory, area, buf),
        Item::SwapBar if sample.swap.total == 0 => {
            let mut spans = Vec::new();
            if let Some((width, label)) = labeled(ctx) {
                spans.push(Span::styled(pad(&label, width), Style::new().bold()));
                spans.push(Span::raw(" "));
            }
            spans.push(ctx.muted(ctx.text.no_swap));
            Line::from(spans).render(area, buf);
        }
        Item::SwapBar => {
            let share = sample.swap.used_ratio();
            meter(
                ctx,
                labeled(ctx),
                share,
                &[(share, palette.level(share))],
                area,
                buf,
            );
        }
        Item::SwapNumbers => swap_numbers(ctx, area, buf),
        Item::SwapGraph => history(ctx, &ctx.app.history.swap, area, buf),
        Item::DiskBar(index) => {
            if let Some(disk) = sample.disks.get(index) {
                match labeled(ctx) {
                    Some(label) => {
                        let share = disk.used_ratio();
                        meter(
                            ctx,
                            Some(label),
                            share,
                            &[(share, palette.level(share))],
                            area,
                            buf,
                        );
                    }
                    None => disk_bar(ctx, disk, area, buf),
                }
            }
        }
        Item::DiskInfo(index) => {
            if let Some(disk) = sample.disks.get(index) {
                disk_info(ctx, disk, area, buf);
            }
        }
        Item::NoDisks => Line::from(ctx.muted(ctx.text.no_disks)).render(area, buf),
        Item::LimitTool(tool) => {
            if let Some(usage) = ctx.app.limits.get(tool) {
                limit_tool(ctx, usage, area, buf);
            }
        }
        Item::Limit(tool, index) => {
            let Some(usage) = ctx.app.limits.get(tool) else {
                return;
            };
            let Some(window) = usage.windows.get(index) else {
                return;
            };
            let now = unix_now();
            match labeled(ctx) {
                Some(label) => {
                    let used = if expired(window, now) {
                        0.0
                    } else {
                        window.used
                    };
                    meter(
                        ctx,
                        Some(label),
                        used,
                        &[(used, palette.level(used))],
                        area,
                        buf,
                    );
                }
                None => {
                    // Under its tool's line the name is not repeated.
                    let named = !items.iter().any(|&(item, _)| item == Item::LimitTool(tool));
                    let name = named.then_some((tool, usage));
                    limit(ctx, name, index, window, now, area, buf);
                }
            }
        }
    }
}

/// When a window starts over, longest first: "  resets 17:09 · in 1h 32m",
/// "  17:09 · 1h 32m", "  17:09", "".
fn reset_texts(ctx: &Ctx, window: &Window, now: i64) -> Vec<String> {
    let (text, lang) = (ctx.text, ctx.lang);
    match window.resets_at {
        None => vec![String::new()],
        Some(at) if at <= now => {
            vec![
                format!("  {} {}", text.renewed, lang.reset_time(at, now)),
                String::new(),
            ]
        }
        Some(at) => {
            let when = lang.reset_time(at, now);
            let left = lang.duration(at.saturating_sub(now).max(0) as u64);
            vec![
                format!("  {} {when} · {} {left}", text.resets, text.in_),
                format!("  {when} · {left}"),
                format!("  {when}"),
                String::new(),
            ]
        }
    }
}

/// Whether a window already started over, so its figures are from before.
fn expired(window: &Window, now: i64) -> bool {
    window.resets_at.is_some_and(|at| at <= now)
}

/// "Claude · Max 20x · live", or how long ago the figures are when they are
/// not fresh: a login the tool has not renewed yet leaves the last live
/// reading ageing in place instead of newer saved figures.
fn limit_tool(ctx: &Ctx, usage: &Usage, area: Rect, buf: &mut Buffer) {
    let text = ctx.text;
    let mut name = vec![Span::raw(usage.name()).bold()];
    if let Some(plan) = &usage.plan {
        name.push(ctx.muted(format!(" {plan}")));
    }
    let age = unix_now().saturating_sub(usage.as_of).max(0) as u64;
    let freshness = match usage.source {
        Source::Live if age <= LIVE_FRESH_SECS as u64 => {
            Span::styled(text.live, Style::new().fg(ctx.palette.low))
        }
        _ => {
            let ago = text.ago.replace("{}", &ctx.lang.duration(age));
            if age >= 3600 {
                Span::styled(ago, Style::new().fg(ctx.palette.middle))
            } else {
                ctx.muted(ago)
            }
        }
    };
    ctx.line(
        vec![Piece::new(0, name), Piece::new(1, vec![freshness])],
        area,
        buf,
    );
}

/// The words for a window: "session", "week", "week Opus".
fn window_label(ctx: &Ctx, window: &Window) -> String {
    let text = ctx.text;
    let period = match window.period {
        Period::Session => text.session.to_owned(),
        Period::Week => text.week.to_owned(),
        Period::Month => text.month.to_owned(),
        Period::Minutes(minutes) => ctx.lang.duration(minutes * 60),
    };
    match &window.model {
        Some(model) => format!("{period} {model}"),
        None => period,
    }
}

/// "session  ████████░░░  57%  resets 17:09 · in 1h 32m", with the tool's
/// name first when it has no line of its own. The reset time shortens
/// before the bar gets too thin.
fn limit(
    ctx: &Ctx,
    name: Option<(usize, &Usage)>,
    index: usize,
    window: &Window,
    now: i64,
    area: Rect,
    buf: &mut Buffer,
) {
    let palette = &ctx.palette;
    let width = usize::from(area.width);
    // Names and labels line up across every tool's windows.
    let name_width = ctx
        .app
        .limits
        .iter()
        .map(|usage| usage.name().width())
        .max()
        .unwrap_or(0);
    let label_width = ctx
        .app
        .limits
        .iter()
        .flat_map(|usage| &usage.windows)
        .map(|window| window_label(ctx, window).width())
        .max()
        .unwrap_or(0);
    let mut spans = Vec::new();
    match name {
        Some((_, usage)) if index == 0 => {
            spans.push(Span::raw(pad(&usage.name(), name_width as u16)).bold());
            spans.push(Span::raw(" "));
        }
        Some(_) => spans.push(Span::raw(" ".repeat(name_width + 1))),
        None => spans.push(Span::raw("  ")),
    }
    spans.push(ctx.muted(pad(&window_label(ctx, window), label_width as u16)));
    spans.push(Span::raw(" "));
    let head: usize = spans.iter().map(Span::width).sum();

    let expired = expired(window, now);
    let (share, percentage) = if expired {
        (0.0, ctx.muted(format!(" {:>4}", "—")))
    } else {
        (
            window.used,
            Span::styled(
                format!(" {:>4}", percent(window.used)),
                Style::new().fg(palette.level(window.used)).bold(),
            ),
        )
    };
    // The longest reset texts that leave the bars some room, padded to the
    // widest so every limit's bar has the same length.
    let texts: Vec<Vec<String>> = ctx
        .app
        .limits
        .iter()
        .flat_map(|usage| &usage.windows)
        .map(|window| reset_texts(ctx, window, now))
        .collect();
    let pick = |texts: &Vec<String>, variant: usize| texts[variant.min(texts.len() - 1)].clone();
    let widest = |variant: usize| {
        texts
            .iter()
            .map(|texts| pick(texts, variant).width())
            .max()
            .unwrap_or(0)
    };
    let variants = texts.iter().map(Vec::len).max().unwrap_or(1);
    let fixed = head + percentage.width();
    let variant = (0..variants)
        .find(|&variant| width >= fixed + widest(variant) + 8)
        .unwrap_or(variants - 1);
    let reset = pad(
        &pick(&reset_texts(ctx, window, now), variant),
        widest(variant) as u16,
    );
    let bar_width = width.saturating_sub(fixed + reset.width());
    let bar_x = area.x + head as u16;
    spans.push(Span::raw(" ".repeat(bar_width)));
    spans.push(percentage);
    spans.push(ctx.muted(reset));
    Line::from(spans).render(area, buf);
    if bar_width >= 3 {
        let color = palette.level(share);
        bar(
            buf,
            Rect::new(bar_x, area.y, bar_width as u16, 1),
            &[(share, color)],
            palette,
        );
    }
}

/// `text` padded with spaces to `width` columns, or shortened to it.
pub fn pad(text: &str, width: u16) -> String {
    let text = shorten(text, usize::from(width));
    let fill = usize::from(width).saturating_sub(text.width());
    format!("{text}{}", " ".repeat(fill))
}

/// A bar filling `area`; with a label, it starts with the label and the
/// percentage.
fn meter(
    ctx: &Ctx,
    label: Option<(u16, String)>,
    share: f64,
    slices: &[(f64, Color)],
    area: Rect,
    buf: &mut Buffer,
) {
    let mut area = Rect { height: 1, ..area };
    if let Some((width, label)) = label {
        let head = Line::from(vec![
            Span::styled(pad(&label, width), Style::new().bold()),
            Span::styled(
                format!(" {:>4} ", percent(share)),
                Style::new().fg(ctx.palette.level(share)).bold(),
            ),
        ]);
        head.render(area, buf);
        let used = width + 6;
        area.x += used.min(area.width);
        area.width = area.width.saturating_sub(used);
        if area.width < 3 {
            return;
        }
    }
    bar(buf, area, slices, &ctx.palette);
}

/// Slices of a CPU bar: user, nice, system and steal time, as htop colors them.
fn cpu_slices(palette: &Palette, times: &CpuTimes, split: Split) -> Vec<(f64, Color)> {
    match split {
        Split::Busy => vec![(times.busy(), palette.level(times.busy()))],
        Split::Basic | Split::Full => vec![
            (times.user, palette.user),
            (times.nice, palette.nice),
            (times.system, palette.system),
            (times.steal, palette.steal),
        ],
    }
}

fn cpu_times<'a>(ctx: &Ctx) -> Vec<Piece<'a>> {
    let (text, palette, lang) = (ctx.text, &ctx.palette, ctx.lang);
    let cpu = &ctx.sample.cpu;
    let total = cpu.total;
    let part = |rank: u8, color: Color, name: &'static str, share: f64| {
        Piece::new(
            rank,
            vec![
                ctx.swatch(color),
                ctx.muted(format!("{name} ")),
                Span::raw(percent(share)),
            ],
        )
    };
    let mut pieces = match cpu.split {
        Split::Busy => vec![part(
            0,
            palette.level(total.busy()),
            text.busy,
            total.busy(),
        )],
        Split::Basic | Split::Full => {
            let mut list = vec![
                part(0, palette.user, text.user, total.user),
                part(0, palette.system, text.system, total.system),
                part(2, palette.nice, text.nice, total.nice),
            ];
            if cpu.split == Split::Full {
                list.push(part(1, palette.iowait, text.iowait, total.iowait));
                list.push(part(3, palette.steal, text.steal, total.steal));
            }
            list
        }
    };
    let idle = match cpu.split {
        Split::Busy => 1.0 - total.busy(),
        _ => total.idle,
    };
    pieces.push(Piece::new(
        3,
        vec![
            ctx.muted(format!("{} ", text.idle)),
            Span::raw(percent(idle)),
        ],
    ));
    if let Some(celsius) = cpu.temperature {
        let color = if celsius >= 90.0 {
            palette.high
        } else if celsius >= 70.0 {
            palette.middle
        } else {
            palette.low
        };
        pieces.push(Piece::new(
            1,
            vec![Span::styled(
                format!("{celsius:.0}°C"),
                Style::new().fg(color),
            )],
        ));
    }
    if let Some(mhz) = cpu.frequency_mhz {
        pieces.push(Piece::new(
            3,
            vec![Span::raw(format!(
                "{} GHz",
                lang.decimal(mhz as f64 / 1000.0, 1)
            ))],
        ));
    }
    if let Some(stall) = cpu.stall {
        pieces.push(stall_piece(ctx, stall));
    }
    pieces
}

fn stall_piece<'a>(ctx: &Ctx, stall: f64) -> Piece<'a> {
    Piece::new(
        3,
        vec![
            ctx.muted(format!("{} ", ctx.text.stall)),
            Span::raw(format!("{}%", ctx.lang.decimal(stall, 1))),
        ],
    )
}

fn cpu_load(ctx: &Ctx, area: Rect, buf: &mut Buffer) {
    let Some([one, five, fifteen]) = ctx.sample.load else {
        return;
    };
    let (text, lang) = (ctx.text, ctx.lang);
    let mut pieces = vec![Piece::new(
        0,
        vec![
            ctx.muted(format!("{} ", text.load)),
            Span::raw(lang.decimal(one, 2)).bold(),
            Span::raw(format!(" {}", lang.decimal(five, 2))),
            ctx.muted(format!(" {}", lang.decimal(fifteen, 2))),
        ],
    )];
    if let Some(share) = ctx.sample.load_share() {
        // A load above the core count is over 100%: work is waiting for a core.
        let cores = text
            .of_cores
            .replace("{}", &ctx.sample.host.cores.to_string());
        pieces.push(Piece::new(
            1,
            vec![
                Span::styled(
                    format!("{:.0}%", share.max(0.0) * 100.0),
                    Style::new().fg(ctx.palette.level(share)),
                ),
                ctx.muted(format!(" {cores}")),
            ],
        ));
    }
    ctx.line(pieces, area, buf);
}

fn cpu_tasks(ctx: &Ctx, area: Rect, buf: &mut Buffer) {
    let Some(tasks) = ctx.sample.tasks else {
        return;
    };
    let text = ctx.text;
    let count = |rank: u8, count: u64, what: &'static str| {
        Piece::new(
            rank,
            vec![Span::raw(count.to_string()), ctx.muted(format!(" {what}"))],
        )
    };
    let (process, processes) = (text.process, text.processes);
    let mut pieces = vec![count(
        0,
        tasks.processes,
        if tasks.processes == 1 {
            process
        } else {
            processes
        },
    )];
    if let Some(threads) = tasks.threads {
        let word = if threads == 1 {
            text.thread
        } else {
            text.threads
        };
        pieces.push(count(1, threads, word));
    }
    if let Some(running) = tasks.running {
        pieces.push(count(1, running, text.running));
    }
    ctx.line(pieces, area, buf);
}

fn history(ctx: &Ctx, series: &crate::history::Series, area: Rect, buf: &mut Buffer) {
    let per_cell = match ctx.app.graph_style {
        GraphStyle::Braille => 2,
        GraphStyle::Blocks => 1,
    };
    let values = series.last(usize::from(area.width) * per_cell);
    graph(buf, area, &values, ctx.app.graph_style, &ctx.palette);
}

/// Each core as a bar with its percentage, numbered down each column like htop.
fn cores_grid(ctx: &Ctx, columns: u16, area: Rect, buf: &mut Buffer) {
    let cores = &ctx.sample.cpu.cores;
    if cores.is_empty() || columns == 0 {
        return;
    }
    let rows = cores.len().div_ceil(usize::from(columns));
    let index_width = usize::from(core_index_width(cores.len()));
    let cell = ((area.width + CORE_GAP) / columns).saturating_sub(CORE_GAP);
    for (index, times) in cores.iter().enumerate() {
        let (column, row) = ((index / rows) as u16, (index % rows) as u16);
        if row >= area.height {
            continue;
        }
        let x = area.x + column * (cell + CORE_GAP);
        let y = area.y + row;
        let label = format!("{index:>index_width$} ");
        buf.set_string(x, y, &label, ctx.palette.muted());
        let share = times.busy();
        let bar_width = cell.saturating_sub(label.width() as u16 + 5);
        let bar_area = Rect::new(x + label.width() as u16, y, bar_width, 1);
        bar(
            buf,
            bar_area,
            &cpu_slices(&ctx.palette, times, ctx.sample.cpu.split),
            &ctx.palette,
        );
        buf.set_string(
            bar_area.right() + 1,
            y,
            format!("{:>4}", percent(share)),
            Style::new().fg(ctx.palette.level(share)),
        );
    }
}

/// Each core as one column of a strip, taller the busier.
fn cores_strip(ctx: &Ctx, area: Rect, buf: &mut Buffer) {
    let cores = &ctx.sample.cpu.cores;
    let label = format!("{} ", ctx.text.cores);
    let width = usize::from(area.width);
    let mut start = 0;
    if area.height == 1 && width >= cores.len() + label.width() {
        buf.set_string(area.x, area.y, &label, ctx.palette.muted());
        start = label.width();
    }
    for (index, times) in cores.iter().enumerate() {
        let spot = start + index;
        let (x, y) = ((spot % width) as u16, (spot / width) as u16);
        if y >= area.height {
            break;
        }
        let share = times.busy();
        let level = (share * 8.0).round() as usize;
        let (symbol, style) = match level {
            0 => (LEVELS[1], ctx.palette.muted()),
            level => (
                LEVELS[level.min(8)],
                Style::new().fg(ctx.palette.level(share)),
            ),
        };
        buf.set_string(area.x + x, area.y + y, symbol, style);
    }
}

fn memory_numbers(ctx: &Ctx, area: Rect, buf: &mut Buffer) {
    let (text, lang, palette) = (ctx.text, ctx.lang, &ctx.palette);
    let memory = &ctx.sample.memory;
    let mut pieces = vec![Piece::new(
        0,
        vec![
            Span::raw(lang.bytes(memory.used)).bold(),
            ctx.muted(format!(" {} {}", text.of, lang.bytes(memory.total))),
        ],
    )];
    if let Some(level) = memory.pressure {
        let (word, color) = match level {
            PressureLevel::Normal => (text.pressure_levels[0], palette.low),
            PressureLevel::Warning => (text.pressure_levels[1], palette.middle),
            PressureLevel::Critical => (text.pressure_levels[2], palette.high),
        };
        pieces.push(Piece::new(
            1,
            vec![
                ctx.muted(format!("{} ", text.pressure)),
                Span::styled(word, Style::new().fg(color)),
            ],
        ));
    }
    if let Some(available) = memory.available {
        pieces.push(Piece::new(
            1,
            vec![
                Span::raw(lang.bytes(available)),
                ctx.muted(format!(" {}", text.available)),
            ],
        ));
    }
    if let Some(stall) = memory.stall {
        pieces.push(stall_piece(ctx, stall));
    }
    ctx.line(pieces, area, buf);
}

fn memory_parts<'a>(ctx: &Ctx) -> Vec<Piece<'a>> {
    let text = ctx.text;
    ctx.sample
        .memory
        .parts
        .iter()
        .map(|&(part, bytes)| {
            let (name, rank) = match part {
                Part::App => (text.app, 0),
                Part::Used => (text.used, 0),
                Part::Wired => (text.wired, 1),
                Part::Compressed => (text.compressed, 1),
                Part::Cached => (text.cached, 1),
                Part::Shared => (text.shared, 2),
                Part::Buffers => (text.buffers, 2),
            };
            Piece::new(
                rank,
                vec![
                    ctx.swatch(ctx.palette.part(part)),
                    ctx.muted(format!("{name} ")),
                    Span::raw(ctx.lang.bytes(bytes)),
                ],
            )
        })
        .collect()
}

fn swap_numbers(ctx: &Ctx, area: Rect, buf: &mut Buffer) {
    let (text, lang) = (ctx.text, ctx.lang);
    let swap = &ctx.sample.swap;
    let mut pieces = vec![Piece::new(
        0,
        vec![
            Span::raw(lang.bytes(swap.used)).bold(),
            ctx.muted(format!(" {} {}", text.of, lang.bytes(swap.total))),
        ],
    )];
    if let Some((swapped_in, swapped_out)) = swap.activity {
        for (name, rate) in [(text.swap_in, swapped_in), (text.swap_out, swapped_out)] {
            pieces.push(Piece::new(
                1,
                vec![ctx.muted(format!("{name} ")), Span::raw(lang.rate(rate))],
            ));
        }
    }
    ctx.line(pieces, area, buf);
}

/// "/  ████████░░░  75%  117G free": the mount point, the bar, how full it is
/// and the free space, which goes first when the width runs short.
fn disk_bar(ctx: &Ctx, disk: &Disk, area: Rect, buf: &mut Buffer) {
    let width = usize::from(area.width);
    let share = disk.used_ratio();
    // Every disk's bar starts at the same column.
    let widest = ctx
        .sample
        .disks
        .iter()
        .map(|disk| disk.mount.width())
        .max()
        .unwrap_or(0);
    let label = pad(&disk.mount, widest.min((width / 3).max(6)) as u16);
    let percentage = format!(" {:>4}", percent(share));
    let free = format!(
        "  {} {}",
        ctx.lang.bytes(disk.available),
        ctx.text.free_plural
    );
    let fixed = label.width() + 1 + percentage.width();
    let show_free = width >= fixed + free.width() + 8;
    let right = percentage.width() + if show_free { free.width() } else { 0 };
    let bar_width = width.saturating_sub(label.width() + 1 + right);
    let mut spans = vec![Span::raw(label.clone()), Span::raw(" ")];
    let bar_x = area.x + (label.width() + 1) as u16;
    if bar_width >= 3 {
        spans.push(Span::raw(" ".repeat(bar_width)));
    }
    spans.push(Span::styled(
        percentage,
        Style::new().fg(ctx.palette.level(share)).bold(),
    ));
    if show_free {
        spans.push(ctx.muted(free));
    }
    Line::from(spans).render(area, buf);
    if bar_width >= 3 {
        let bar_area = Rect::new(bar_x, area.y, bar_width as u16, 1);
        bar(
            buf,
            bar_area,
            &[(share, ctx.palette.level(share))],
            &ctx.palette,
        );
    }
}

/// "  Macintosh HD · apfs · 346G of 460G · read 1.2M/s · write 300K/s".
fn disk_info(ctx: &Ctx, disk: &Disk, area: Rect, buf: &mut Buffer) {
    let (text, lang) = (ctx.text, ctx.lang);
    let mut pieces = Vec::new();
    if !disk.name.is_empty() && disk.name != disk.mount {
        pieces.push(Piece::new(1, vec![Span::raw(disk.name.clone())]));
    }
    if !disk.file_system.is_empty() {
        pieces.push(Piece::new(3, vec![ctx.muted(disk.file_system.clone())]));
    }
    pieces.push(Piece::new(
        0,
        vec![
            Span::raw(lang.bytes(disk.used())),
            ctx.muted(format!(" {} {}", text.of, lang.bytes(disk.total))),
        ],
    ));
    if let Some((read, write)) = disk.io {
        for (name, rate) in [(text.read, read), (text.write, write)] {
            pieces.push(Piece::new(
                2,
                vec![ctx.muted(format!("{name} ")), Span::raw(lang.rate(rate))],
            ));
        }
    }
    if disk.removable {
        pieces.push(Piece::new(3, vec![ctx.muted("⏏")]));
    }
    let indent = 2.min(area.width);
    let area = Rect {
        x: area.x + indent,
        width: area.width - indent,
        ..area
    };
    ctx.line(pieces, area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_and_shortens_labels() {
        assert_eq!(pad("CPU", 5), "CPU  ");
        assert_eq!(pad("Backup Drive", 6), "Backu…");
    }

    #[test]
    fn names_disks_by_their_last_folder() {
        let disk = |mount: &str| Disk {
            mount: mount.into(),
            ..Disk::default()
        };
        assert_eq!(short_name(&disk("/")), "/");
        assert_eq!(short_name(&disk("/Volumes/Backup")), "Backup");
        assert_eq!(short_name(&disk("/home")), "home");
    }
}
