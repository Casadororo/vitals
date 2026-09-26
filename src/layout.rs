//! Splits the window among the panels, and each panel among its lines and
//! graphs, so the most important readings stay on screen from a full screen
//! down to a one-line strip.
//!
//! Each panel lists what it can show, most important first: the bars, then
//! the numbers, then the details, then graphs and each core. Every way of
//! setting the panels in columns is tried, each column granting rows in that
//! order, and the arrangement that shows the most wins. Spare rows go to the
//! graphs. When not even the bars fit inside boxes, the boxes go and each
//! reading takes a bare line.

use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Cpu,
    Memory,
    Swap,
    Disks,
    /// Usage limits of the AI coding tools.
    Limits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    CpuBar,
    CpuTimes,
    CpuLoad,
    CpuTasks,
    CpuGraph,
    Cores(CoreView),
    MemBar,
    MemNumbers,
    MemParts,
    MemGraph,
    SwapBar,
    SwapNumbers,
    SwapGraph,
    DiskBar(usize),
    DiskInfo(usize),
    NoDisks,
    /// A tool's name, plan and how fresh its figures are.
    LimitTool(usize),
    /// One of a tool's limits: the tool, then the window.
    Limit(usize, usize),
}

/// Each core as a bar with its percentage, or as one column of a strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreView {
    Strip,
    Grid { columns: u16 },
}

/// What the reading has to show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Content {
    pub cores: usize,
    pub disks: usize,
    pub load: bool,
    pub tasks: bool,
    pub swap: bool,
    pub graphs: bool,
    pub show_cores: bool,
    /// Columns the CPU time and memory slice lines take on one row; narrower
    /// boxes give them a second row when they can.
    pub cpu_times_width: u16,
    pub mem_parts_width: u16,
    /// Limit windows of each AI tool shown, up to four tools.
    pub limits: [u8; 4],
    /// Share of each window used, from 0 to 1000, expired windows as 0: the
    /// bare lines show the worst first when not every window fits.
    pub limit_used: [[u16; 8]; 4],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    pub kind: Kind,
    pub area: Rect,
    /// With a border and a title; small windows get bare lines instead.
    pub boxed: bool,
    pub items: Vec<(Item, Rect)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    pub header: Option<Rect>,
    pub body: Rect,
    pub panels: Vec<Panel>,
    pub footer: Option<Rect>,
}

impl Screen {
    #[cfg(test)]
    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        self.panels
            .iter()
            .flat_map(|panel| panel.items.iter().map(|&(item, _)| item))
    }
}

/// Rows below which a graph says nothing.
pub const GRAPH_MIN: u16 = 3;
/// Narrowest box, when it has the width to itself.
const BOX_MIN: u16 = 24;
/// Narrowest box when the window is split in columns.
const COLUMN_MIN: u16 = 36;
/// Narrowest bare line: "CPU  24% ███".
const BARE_MIN: u16 = 12;
/// Space between the cells of the core grid.
pub const CORE_GAP: u16 = 2;

/// Worth of what an arrangement shows, by importance: every bar beats any
/// number of details.
const WORTH: [i64; 6] = [100_000, 1000, 300, 100, 50, 20];
/// Worth of a panel's box: boxes read better, but not at the cost of the
/// numbers they would leave out.
const BOX_WORTH: i64 = 400;
/// Space left between bare columns.
const BARE_GAP: u16 = 2;
/// Width from which a line of numbers shows all its parts; narrower lines
/// drop some and are worth less.
const COMFORT: u16 = 44;

/// Worth of a line by how much of it shows: `rows` rows `width` wide out of
/// the `natural` columns it takes on one row, or out of `COMFORT` when that
/// is not known. Bars are worth the same at any width.
fn worth(importance: u8, width: u16, rows: u16, natural: u16) -> i64 {
    let base = WORTH[usize::from(importance)];
    if importance == 0 {
        return base;
    }
    let (shown, whole) = if natural > 0 {
        (u32::from(width) * u32::from(rows), u32::from(natural))
    } else {
        (u32::from(width.min(COMFORT)), u32::from(COMFORT))
    };
    base * i64::from(shown.min(whole)) / i64::from(whole)
}

#[derive(Clone, Copy, Debug)]
struct Want {
    item: Item,
    importance: u8,
    /// Granted first within its importance when rows run short.
    priority: u16,
    rows: u16,
    /// Share of the spare rows it takes.
    stretch: u16,
    /// A bigger version it grows into later on: the core grid, or a second
    /// row for a long line.
    grows: Option<(u8, Item, u16)>,
    /// Columns a line takes on one row, when known.
    natural: u16,
    /// Whether it is text, worth less when it shows only in part.
    text: bool,
}

fn line(item: Item, importance: u8) -> Want {
    Want {
        item,
        importance,
        priority: 0,
        rows: 1,
        stretch: 0,
        grows: None,
        natural: 0,
        text: true,
    }
}

/// A line that takes a second row, once the more important things have theirs,
/// when it does not fit on one.
fn wrapping(item: Item, importance: u8, natural: u16, width: u16) -> Want {
    Want {
        grows: (natural > width).then_some((4, item, 2)),
        natural,
        ..line(item, importance)
    }
}

fn graph(item: Item, importance: u8, stretch: u16) -> Want {
    Want {
        rows: GRAPH_MIN,
        stretch,
        text: false,
        ..line(item, importance)
    }
}

/// What a panel can show inside a box `width` columns wide, in display order.
fn wants(kind: Kind, width: u16, content: &Content) -> Vec<Want> {
    let mut list = Vec::new();
    match kind {
        Kind::Cpu => {
            list.push(line(Item::CpuBar, 0));
            list.push(wrapping(Item::CpuTimes, 2, content.cpu_times_width, width));
            if content.load {
                list.push(line(Item::CpuLoad, 1));
            }
            if content.tasks {
                list.push(line(Item::CpuTasks, 2));
            }
            if content.graphs {
                list.push(graph(Item::CpuGraph, 3, 4));
            }
            if content.show_cores && content.cores > 1 && width > 0 {
                let cores = content.cores as u16;
                let columns = grid_columns(width, content.cores);
                list.push(Want {
                    rows: cores.div_ceil(width),
                    grows: Some((
                        4,
                        Item::Cores(CoreView::Grid { columns }),
                        cores.div_ceil(columns),
                    )),
                    text: false,
                    ..line(Item::Cores(CoreView::Strip), 3)
                });
            }
        }
        Kind::Memory => {
            list.push(line(Item::MemBar, 0));
            list.push(line(Item::MemNumbers, 1));
            list.push(wrapping(Item::MemParts, 2, content.mem_parts_width, width));
            if content.graphs {
                list.push(graph(Item::MemGraph, 4, 3));
            }
        }
        Kind::Swap => {
            list.push(line(Item::SwapBar, 0));
            if content.swap {
                list.push(line(Item::SwapNumbers, 1));
                if content.graphs {
                    list.push(graph(Item::SwapGraph, 5, 1));
                }
            }
        }
        Kind::Disks => {
            if content.disks == 0 {
                list.push(line(Item::NoDisks, 0));
            }
            for disk in 0..content.disks {
                list.push(line(Item::DiskBar(disk), u8::from(disk > 0)));
                list.push(line(Item::DiskInfo(disk), 2));
            }
        }
        Kind::Limits => {
            for (tool, &windows) in content.limits.iter().enumerate() {
                if windows == 0 {
                    continue;
                }
                list.push(line(Item::LimitTool(tool), 3));
                for window in 0..usize::from(windows) {
                    let used = content
                        .limit_used
                        .get(tool)
                        .and_then(|used| used.get(window))
                        .copied()
                        .unwrap_or(0);
                    // Granted worst first when rows run short; shown in place.
                    list.push(Want {
                        priority: 1000 - used,
                        ..line(Item::Limit(tool, window), 1)
                    });
                }
            }
        }
    }
    list
}

/// What a panel shows without a box, a row each, in display order: the
/// lines of the box, without graphs and cores.
fn bare_lines(kind: Kind, content: &Content) -> Vec<Want> {
    let content = Content {
        graphs: false,
        show_cores: false,
        ..*content
    };
    wants(kind, 0, &content)
}

/// Width of the index before a core's bar: "9" or "15".
pub fn core_index_width(cores: usize) -> u16 {
    cores.saturating_sub(1).max(1).ilog10() as u16 + 1
}

/// Narrowest cell of the core grid: index, a bar of 6 and "100%".
fn core_cell_min(cores: usize) -> u16 {
    core_index_width(cores) + 1 + 6 + 5
}

/// Columns of the core grid in `width`: as few as the rows allow, so the
/// cells come out as wide as they can.
pub fn grid_columns(width: u16, cores: usize) -> u16 {
    let cores = cores.max(1) as u16;
    let fit = ((width + CORE_GAP) / (core_cell_min(usize::from(cores)) + CORE_GAP)).clamp(1, cores);
    cores.div_ceil(cores.div_ceil(fit))
}

/// Header and footer take a row each when the window has room, like meridian.
pub fn split(area: Rect, content: &Content) -> Screen {
    let mut screen = None;
    for (with_header, with_footer) in [(true, true), (true, false), (false, false)] {
        let header = (with_header && area.height >= 8 && area.width >= 20)
            .then_some(Rect { height: 1, ..area });
        let footer = (with_footer && area.height >= 10 && area.width >= 20).then(|| Rect {
            y: area.bottom() - 1,
            height: 1,
            ..area
        });
        let top = u16::from(header.is_some());
        let body = Rect {
            y: area.y + top,
            height: area.height - top - u16::from(footer.is_some()),
            ..area
        };
        let (panels, _, essentials) = best(body, content);
        let done = essentials == ESSENTIALS;
        screen = Some(Screen {
            header,
            body,
            panels,
            footer,
        });
        // Header and footer give way when the bars need their rows.
        if done {
            break;
        }
    }
    screen.expect("tried at least once")
}

/// Bars every reading has: CPU, memory, swap and the first disk.
const ESSENTIALS: usize = 4;

const ARRANGEMENTS: [&[&[Kind]]; 5] = {
    use Kind::{Cpu, Disks, Memory, Swap};
    [
        &[&[Cpu, Memory, Swap, Disks]],
        &[&[Cpu], &[Memory, Swap, Disks]],
        &[&[Cpu, Disks], &[Memory, Swap]],
        &[&[Cpu], &[Memory, Swap], &[Disks]],
        &[&[Cpu], &[Memory], &[Swap], &[Disks]],
    ]
};

/// Every arrangement to try: with AI limits, their panel goes at the foot of
/// each column in turn, or in a column of its own.
fn arrangements(limits: bool) -> Vec<Vec<Vec<Kind>>> {
    let mut all = Vec::new();
    for columns in ARRANGEMENTS {
        let columns: Vec<Vec<Kind>> = columns.iter().map(|kinds| kinds.to_vec()).collect();
        if !limits {
            all.push(columns);
            continue;
        }
        for index in 0..columns.len() {
            let mut with = columns.clone();
            with[index].push(Kind::Limits);
            all.push(with);
        }
        let mut apart = columns;
        apart.push(vec![Kind::Limits]);
        all.push(apart);
    }
    all
}

/// The arrangement of columns that shows the most, fewer columns on a tie.
fn best(body: Rect, content: &Content) -> (Vec<Panel>, i64, usize) {
    let mut best: Option<(Vec<Panel>, i64, usize)> = None;
    for columns in arrangements(content.limits.iter().any(|&windows| windows > 0)) {
        let several = columns.len() > 1;
        let weights: Vec<u16> = columns
            .iter()
            .map(|kinds| if kinds.contains(&Kind::Cpu) { 5 } else { 4 })
            .collect();
        let total: u16 = weights.iter().sum();
        let mut x = body.x;
        let mut panels = Vec::new();
        let mut score = -5 * columns.len() as i64;
        let mut essentials = 0;
        for (index, (kinds, weight)) in columns.iter().zip(&weights).enumerate() {
            let width = if index == columns.len() - 1 {
                body.right() - x
            } else {
                (u32::from(body.width) * u32::from(*weight) / u32::from(total)) as u16
            };
            let area = Rect { x, width, ..body };
            let gap = if index + 1 < columns.len() {
                BARE_GAP
            } else {
                0
            };
            let (column, worth, shown) = fill(kinds, area, content, several, gap);
            panels.extend(column);
            score += worth;
            essentials += shown;
            x += width;
        }
        if best.as_ref().is_none_or(|(_, best, _)| score > *best) {
            best = Some((panels, score, essentials));
        }
    }
    best.expect("there are arrangements")
}

/// Panels stacked in one column, in boxes or bare, whichever shows more.
/// Bare lines stop `gap` columns short of the next column.
fn fill(
    kinds: &[Kind],
    area: Rect,
    content: &Content,
    several: bool,
    gap: u16,
) -> (Vec<Panel>, i64, usize) {
    let narrowest = if several { COLUMN_MIN } else { BOX_MIN };
    let bare_area = Rect {
        width: area.width.saturating_sub(gap),
        ..area
    };
    let bare = fill_bare(kinds, bare_area, content);
    match (area.width >= narrowest)
        .then(|| fill_boxed(kinds, area, content))
        .flatten()
    {
        Some(boxed) if boxed.1 >= bare.1 => boxed,
        _ => bare,
    }
}

fn fill_boxed(kinds: &[Kind], area: Rect, content: &Content) -> Option<(Vec<Panel>, i64, usize)> {
    let inner_width = area.width.checked_sub(2)?;
    let mut left = area.height.checked_sub(2 * kinds.len() as u16)?;
    let lists: Vec<Vec<Want>> = kinds
        .iter()
        .map(|&kind| wants(kind, inner_width, content))
        .collect();
    let mut rows: Vec<Vec<u16>> = lists.iter().map(|list| vec![0; list.len()]).collect();
    let mut items: Vec<Vec<Item>> = lists
        .iter()
        .map(|list| list.iter().map(|want| want.item).collect())
        .collect();
    let mut score = BOX_WORTH * kinds.len() as i64;
    let mut essentials = 0;
    // Visit order within a pass: the worst AI quota first, then panel order.
    let mut order: Vec<(u8, u16, usize, usize)> = Vec::new();
    for (panel, list) in lists.iter().enumerate() {
        for (index, want) in list.iter().enumerate() {
            order.push((want.importance, want.priority, panel, index));
        }
    }
    order.sort_by(|a, b| (a.1, a.2, a.3).cmp(&(b.1, b.2, b.3)));
    for importance in 0..WORTH.len() as u8 {
        // Every item is seen on every pass: a line granted on an early pass
        // can still grow into its bigger version on a later one.
        for &(_, _, panel, index) in order.iter() {
            let want = &lists[panel][index];
            if want.importance == importance {
                if want.rows <= left {
                    rows[panel][index] = want.rows;
                    left -= want.rows;
                    score += if want.text {
                        worth(importance, inner_width, 1, want.natural)
                    } else {
                        WORTH[usize::from(importance)]
                    };
                    essentials += usize::from(importance == 0);
                } else if importance == 0 {
                    return None;
                }
            }
            if let Some((grows_at, bigger, bigger_rows)) = want.grows
                && grows_at == importance
                && rows[panel][index] > 0
                && bigger_rows >= rows[panel][index]
                && bigger_rows - rows[panel][index] <= left
            {
                left -= bigger_rows - rows[panel][index];
                score += if bigger == want.item {
                    // A second row is worth what the line could not show on one.
                    let line = want.importance;
                    worth(line, inner_width, bigger_rows, want.natural)
                        - worth(line, inner_width, rows[panel][index], want.natural)
                } else {
                    WORTH[usize::from(importance)]
                };
                rows[panel][index] = bigger_rows;
                items[panel][index] = bigger;
            }
        }
    }

    // Spare rows go to the graphs shown, by their share.
    let stretchy: Vec<(usize, usize, u16)> = lists
        .iter()
        .enumerate()
        .flat_map(|(panel, list)| {
            list.iter()
                .enumerate()
                .map(move |(index, want)| (panel, index, want.stretch))
        })
        .filter(|&(panel, index, stretch)| stretch > 0 && rows[panel][index] > 0)
        .collect();
    let shares: u16 = stretchy.iter().map(|&(.., stretch)| stretch).sum();
    if shares > 0 {
        let spare = left;
        for &(panel, index, stretch) in &stretchy {
            let extra = (u32::from(spare) * u32::from(stretch) / u32::from(shares)) as u16;
            rows[panel][index] += extra;
            left -= extra;
        }
        let (panel, index, _) = stretchy[0];
        rows[panel][index] += left;
        left = 0;
    }
    for &(panel, index, _) in &stretchy {
        let cells = f64::from(rows[panel][index]) * f64::from(inner_width);
        score += 2 * cells.sqrt() as i64;
    }

    let mut panels = Vec::with_capacity(kinds.len());
    let mut y = area.y;
    for (panel, &kind) in kinds.iter().enumerate() {
        let used: u16 = rows[panel].iter().sum();
        // The last box reaches the bottom.
        let height = used + 2 + if panel == kinds.len() - 1 { left } else { 0 };
        let mut item_y = y + 1;
        let placed = items[panel]
            .iter()
            .zip(&rows[panel])
            .filter(|&(_, &rows)| rows > 0)
            .map(|(&item, &rows)| {
                let rect = Rect::new(area.x + 1, item_y, inner_width, rows);
                item_y += rows;
                (item, rect)
            })
            .collect();
        panels.push(Panel {
            kind,
            area: Rect { y, height, ..area },
            boxed: true,
            items: placed,
        });
        y += height;
    }
    Some((panels, score, essentials))
}

fn fill_bare(kinds: &[Kind], area: Rect, content: &Content) -> (Vec<Panel>, i64, usize) {
    if area.width < BARE_MIN || area.height == 0 {
        return (Vec::new(), 0, 0);
    }
    let lists: Vec<Vec<Want>> = kinds
        .iter()
        .map(|&kind| bare_lines(kind, content))
        .collect();
    let mut granted: Vec<Vec<bool>> = lists.iter().map(|list| vec![false; list.len()]).collect();
    let mut left = area.height;
    let mut score = 0;
    let mut essentials = 0;
    // Granted by importance, then the worst AI quota, then panel order; the
    // panel still shows them in its own order.
    let mut order: Vec<(u8, u16, usize, usize)> = Vec::new();
    for (panel, list) in lists.iter().enumerate() {
        for (index, want) in list.iter().enumerate() {
            order.push((want.importance, want.priority, panel, index));
        }
    }
    order.sort_by(|a, b| (a.1, a.2, a.3).cmp(&(b.1, b.2, b.3)));
    for importance in 0..WORTH.len() as u8 {
        for &(_, _, panel, index) in order.iter().filter(|&&(imp, _, _, _)| imp == importance) {
            if left == 0 {
                break;
            }
            granted[panel][index] = true;
            left -= 1;
            let want = &lists[panel][index];
            score += worth(importance, area.width, 1, want.natural);
            essentials += usize::from(importance == 0);
        }
    }
    let mut panels = Vec::new();
    let mut y = area.y;
    for (panel, &kind) in kinds.iter().enumerate() {
        let items: Vec<(Item, Rect)> = lists[panel]
            .iter()
            .zip(&granted[panel])
            .filter(|&(_, &granted)| granted)
            .map(|(want, _)| {
                let rect = Rect::new(area.x, y, area.width, 1);
                y += 1;
                (want.item, rect)
            })
            .collect();
        if let Some(&(_, first)) = items.first() {
            let height = items.len() as u16;
            panels.push(Panel {
                kind,
                area: Rect { height, ..first },
                boxed: false,
                items,
            });
        }
    }
    (panels, score, essentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> Content {
        Content {
            cores: 10,
            disks: 2,
            load: true,
            tasks: true,
            swap: true,
            graphs: true,
            show_cores: true,
            cpu_times_width: 60,
            mem_parts_width: 62,
            limits: [0; 4],
            limit_used: [[0; 8]; 4],
        }
    }

    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.is_empty() || outer.union(inner) == outer
    }

    fn shows(screen: &Screen, wanted: Item) -> bool {
        screen.items().any(|item| item == wanted)
    }

    #[test]
    fn every_window_size_gets_a_consistent_layout() {
        let variants = [
            content(),
            Content {
                graphs: false,
                show_cores: false,
                ..content()
            },
            Content {
                cores: 128,
                disks: 9,
                ..content()
            },
            Content {
                load: false,
                tasks: false,
                swap: false,
                disks: 0,
                cores: 1,
                ..content()
            },
        ];
        for content in variants {
            for width in (0..=320).step_by(7) {
                for height in (0..=100).step_by(3) {
                    let area = Rect::new(2, 1, width, height);
                    let screen = split(area, &content);
                    let mut parts: Vec<Rect> = screen.header.into_iter().collect();
                    parts.extend(screen.footer);
                    parts.extend(screen.panels.iter().map(|panel| panel.area));
                    for (i, part) in parts.iter().enumerate() {
                        assert!(inside(*part, area), "{part:?} outside {area:?}");
                        for other in &parts[i + 1..] {
                            assert!(!part.intersects(*other), "{part:?} overlaps {other:?}");
                        }
                    }
                    for panel in &screen.panels {
                        for (index, (_, item)) in panel.items.iter().enumerate() {
                            assert!(inside(*item, panel.area), "{item:?} outside {panel:?}");
                            for (_, other) in &panel.items[index + 1..] {
                                assert!(!item.intersects(*other));
                            }
                        }
                    }
                    // The bars come first: as many as there are rows for.
                    let bars = [Item::CpuBar, Item::MemBar, Item::SwapBar];
                    let shown = bars.iter().filter(|&&bar| shows(&screen, bar)).count();
                    if width >= BARE_MIN {
                        assert!(shown >= 3.min(usize::from(height)), "{width}x{height}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_classic_terminal_shows_every_reading_in_boxes() {
        let screen = split(Rect::new(0, 0, 80, 24), &content());
        assert!(screen.header.is_some() && screen.footer.is_some());
        assert!(screen.panels.iter().all(|panel| panel.boxed));
        let kinds: Vec<Kind> = screen.panels.iter().map(|panel| panel.kind).collect();
        assert_eq!(kinds, [Kind::Cpu, Kind::Memory, Kind::Swap, Kind::Disks]);
        for item in [
            Item::CpuBar,
            Item::CpuTimes,
            Item::CpuLoad,
            Item::CpuTasks,
            Item::MemBar,
            Item::MemNumbers,
            Item::MemParts,
            Item::SwapBar,
            Item::SwapNumbers,
            Item::DiskBar(0),
            Item::DiskBar(1),
        ] {
            assert!(shows(&screen, item), "{item:?} missing");
        }
    }

    #[test]
    fn wide_windows_put_the_cpu_beside_the_rest() {
        let screen = split(Rect::new(0, 0, 160, 45), &content());
        let cpu = &screen.panels[0];
        assert_eq!(cpu.kind, Kind::Cpu);
        let memory = screen
            .panels
            .iter()
            .find(|panel| panel.kind == Kind::Memory);
        assert!(memory.unwrap().area.x >= cpu.area.right());
        assert!(shows(&screen, Item::Cores(CoreView::Grid { columns: 5 })));
        for graph in [Item::CpuGraph, Item::MemGraph, Item::SwapGraph] {
            assert!(shows(&screen, graph), "{graph:?} missing");
        }
    }

    #[test]
    fn graphs_take_the_spare_rows() {
        let screen = split(Rect::new(0, 0, 60, 60), &content());
        let height = |wanted: Item| {
            screen
                .panels
                .iter()
                .flat_map(|panel| &panel.items)
                .find(|(item, _)| *item == wanted)
                .map_or(0, |(_, rect)| rect.height)
        };
        assert!(height(Item::CpuGraph) > height(Item::MemGraph));
        assert!(height(Item::MemGraph) > height(Item::SwapGraph));
        assert!(height(Item::SwapGraph) >= GRAPH_MIN);
        let last = screen.panels.last().unwrap();
        assert_eq!(last.area.bottom(), screen.footer.unwrap().y, "no gap below");
    }

    fn limit_content() -> Content {
        Content {
            limits: [2, 2, 0, 0],
            limit_used: [
                [200, 990, 0, 0, 0, 0, 0, 0],
                [310, 120, 0, 0, 0, 0, 0, 0],
                [0; 8],
                [0; 8],
            ],
            ..content()
        }
    }

    fn bare_items(content: &Content, height: u16) -> Vec<Item> {
        let area = Rect::new(0, 0, 20, height);
        let (panels, _, _) = fill_bare(&[Kind::Limits], area, content);
        panels
            .into_iter()
            .flat_map(|panel| panel.items.into_iter().map(|(item, _)| item))
            .collect()
    }

    #[test]
    fn a_single_bare_row_shows_the_worst_quota() {
        // Session at 20%, week at 99%: the week shows.
        assert_eq!(bare_items(&limit_content(), 1), [Item::Limit(0, 1)]);
    }

    #[test]
    fn short_bare_columns_keep_the_worst_quotas_in_place() {
        assert_eq!(
            bare_items(&limit_content(), 2),
            [Item::Limit(0, 1), Item::Limit(1, 0)]
        );
        assert_eq!(
            bare_items(&limit_content(), 3),
            [Item::Limit(0, 0), Item::Limit(0, 1), Item::Limit(1, 0)]
        );
    }

    #[test]
    fn a_short_box_keeps_the_worst_quotas_without_orphan_headers() {
        let content = limit_content();
        let area = Rect::new(0, 0, 40, 5);
        let (panels, _, _) = fill_boxed(&[Kind::Limits], area, &content).unwrap();
        let items: Vec<Item> = panels
            .into_iter()
            .flat_map(|panel| panel.items.into_iter().map(|(item, _)| item))
            .collect();
        assert_eq!(
            items,
            [Item::Limit(0, 0), Item::Limit(0, 1), Item::Limit(1, 0)]
        );
    }

    #[test]
    fn tiny_panes_get_bare_lines() {
        let screen = split(Rect::new(0, 0, 20, 5), &content());
        assert!(screen.header.is_none() && screen.footer.is_none());
        assert!(screen.panels.iter().all(|panel| !panel.boxed));
        let items: Vec<Item> = screen.items().collect();
        assert_eq!(
            items,
            [
                Item::CpuBar,
                Item::CpuLoad,
                Item::MemBar,
                Item::SwapBar,
                Item::DiskBar(0)
            ]
        );
    }

    #[test]
    fn a_one_line_strip_shows_every_bar_side_by_side() {
        let screen = split(Rect::new(0, 0, 120, 1), &content());
        let items: Vec<Item> = screen.items().collect();
        assert_eq!(
            items,
            [Item::CpuBar, Item::MemBar, Item::SwapBar, Item::DiskBar(0)]
        );
        let xs: Vec<u16> = screen.panels.iter().map(|panel| panel.area.x).collect();
        assert!(xs.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn the_core_grid_fits_its_cells() {
        assert_eq!(core_index_width(10), 1);
        assert_eq!(core_index_width(11), 2);
        assert_eq!(core_index_width(1), 1);
        assert_eq!(grid_columns(78, 10), 5);
        assert_eq!(grid_columns(10, 10), 1);
        assert_eq!(grid_columns(400, 4), 4, "never more columns than cores");
        // Room for 5 columns, but 12 cores in 3 rows need only 4.
        assert_eq!(grid_columns(86, 12), 4);
    }
}
