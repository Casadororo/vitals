//! Drawing pieces: bars made of colored slices, graphs of past readings, and
//! lines that drop their least important parts when the width runs short.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::GraphStyle;
use crate::theme::Palette;

/// Left-aligned blocks from one eighth of a cell to a whole one.
const EIGHTHS: [&str; 8] = ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
/// Bottom-aligned blocks from nothing to a whole cell.
pub const LEVELS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
/// Braille dots of the left and right columns, filled from the bottom.
const LEFT_DOTS: [u32; 5] = [0, 0x40, 0x44, 0x46, 0x47];
const RIGHT_DOTS: [u32; 5] = [0, 0x80, 0xa0, 0xb0, 0xb8];

/// A bar across the first row of `area`, filled with `slices` in order, each
/// a share of the whole with its color, to an eighth of a cell.
pub fn bar(buf: &mut Buffer, area: Rect, slices: &[(f64, Color)], palette: &Palette) {
    if area.is_empty() {
        return;
    }
    let width = usize::from(area.width);
    let eighths = width * 8;
    let mut ends = Vec::with_capacity(slices.len());
    let mut sum = 0.0;
    for &(share, color) in slices {
        sum += share.max(0.0);
        ends.push((((sum.min(1.0)) * eighths as f64).round() as usize, color));
    }
    let filled = ends.last().map_or(0, |&(end, _)| end);
    // The slice covering an eighth: the first that ends after it.
    let color_at = |eighth: usize| {
        ends.iter()
            .find(|&&(end, _)| end > eighth)
            .or(ends.last())
            .map_or(Color::Reset, |&(_, color)| color)
    };
    for cell in 0..width {
        let start = cell * 8;
        let position = Position::new(area.x + cell as u16, area.y);
        let target = &mut buf[position];
        if filled >= start + 8 {
            target.set_symbol("█").set_fg(color_at(start + 4));
        } else if filled > start {
            target
                .set_symbol(EIGHTHS[filled - start - 1])
                .set_fg(color_at(filled - 1));
            if let Some(track) = palette.track {
                target.set_bg(track);
            }
        } else {
            match palette.track {
                Some(track) => {
                    target.set_symbol(" ").set_bg(track);
                }
                None => {
                    target.set_symbol("░").set_style(palette.muted());
                }
            }
        }
    }
}

/// An area chart of `values` (0 to 1, oldest first) filling `area` from
/// the right, colored from low to high by height.
pub fn graph(buf: &mut Buffer, area: Rect, values: &[f64], style: GraphStyle, palette: &Palette) {
    if area.is_empty() {
        return;
    }
    let per_cell = match style {
        GraphStyle::Braille => 2,
        GraphStyle::Blocks => 1,
    };
    let columns = usize::from(area.width);
    let slots = columns * per_cell;
    let values = &values[values.len().saturating_sub(slots)..];
    // Readings start at the right edge and move left.
    let empty = slots - values.len();
    let rows = usize::from(area.height);
    let steps = match style {
        GraphStyle::Braille => 4,
        GraphStyle::Blocks => 8,
    };
    let filled = |slot: usize| -> usize {
        let Some(&value) = slot.checked_sub(empty).and_then(|index| values.get(index)) else {
            return 0;
        };
        let total = rows * steps;
        if value <= 0.0 {
            0
        } else {
            // Any activity shows at least one step.
            ((value * total as f64).round() as usize).clamp(1, total)
        }
    };
    for row in 0..rows {
        let from_bottom = rows - 1 - row;
        let color = palette.gradient((from_bottom as f64 + 0.5) / rows as f64);
        let in_row = |slot: usize| filled(slot).saturating_sub(from_bottom * steps).min(steps);
        for cell in 0..columns {
            let symbol = match style {
                GraphStyle::Braille => {
                    let dots = LEFT_DOTS[in_row(cell * 2)] | RIGHT_DOTS[in_row(cell * 2 + 1)];
                    if dots == 0 {
                        continue;
                    }
                    char::from_u32(0x2800 + dots).map_or_else(String::new, String::from)
                }
                GraphStyle::Blocks => match in_row(cell) {
                    0 => continue,
                    level => LEVELS[level].to_owned(),
                },
            };
            let position = Position::new(area.x + cell as u16, area.y + row as u16);
            buf[position].set_symbol(&symbol).set_fg(color);
        }
    }
}

/// Part of a line, with how long it stays when space runs short: rank 0 goes last.
pub struct Piece<'a> {
    pub spans: Vec<Span<'a>>,
    pub rank: u8,
}

impl<'a> Piece<'a> {
    pub fn new(rank: u8, spans: Vec<Span<'a>>) -> Self {
        Self { spans, rank }
    }

    fn width(&self) -> usize {
        self.spans.iter().map(Span::width).sum()
    }
}

const SEPARATOR: &str = " · ";

/// Columns all the pieces take on one row.
pub fn natural_width(pieces: &[Piece]) -> usize {
    pieces.iter().map(Piece::width).sum::<usize>()
        + SEPARATOR.width() * pieces.len().saturating_sub(1)
}

/// The pieces over up to `rows` lines: each line takes the next pieces in
/// order while they fit, and the last one fits what is left.
pub fn fit_rows<'a>(
    pieces: Vec<Piece<'a>>,
    width: usize,
    rows: usize,
    separator: Style,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut rest = pieces;
    while lines.len() + 1 < rows {
        let mut taken = 0;
        let mut used = 0;
        for piece in &rest {
            let needed = piece.width() + if taken > 0 { SEPARATOR.width() } else { 0 };
            if used + needed > width {
                break;
            }
            used += needed;
            taken += 1;
        }
        if taken == 0 || taken == rest.len() {
            break;
        }
        let tail = rest.split_off(taken);
        lines.push(fit(rest, width, separator));
        rest = tail;
    }
    lines.push(fit(rest, width, separator));
    lines
}

/// The pieces that fit in `width`, separated by " · ": the least important
/// go first, the last ones first among equals. A single piece still too long
/// is cut short with "…".
pub fn fit<'a>(pieces: Vec<Piece<'a>>, width: usize, separator: Style) -> Line<'a> {
    let widths: Vec<usize> = pieces.iter().map(Piece::width).collect();
    let mut kept = vec![true; pieces.len()];
    let total = |kept: &[bool]| {
        let shown: Vec<usize> = (0..widths.len())
            .filter(|&index| kept[index])
            .map(|index| widths[index])
            .collect();
        shown.iter().sum::<usize>() + SEPARATOR.width() * shown.len().saturating_sub(1)
    };
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by_key(|&index| {
        (
            std::cmp::Reverse(pieces[index].rank),
            std::cmp::Reverse(index),
        )
    });
    for index in order {
        if total(&kept) <= width || kept.iter().filter(|&&keep| keep).count() <= 1 {
            break;
        }
        kept[index] = false;
    }
    let mut spans = Vec::new();
    for (piece, keep) in pieces.into_iter().zip(kept) {
        if !keep {
            continue;
        }
        if !spans.is_empty() {
            spans.push(Span::styled(SEPARATOR, separator));
        }
        spans.extend(piece.spans);
    }
    Line::from(cut(spans, width))
}

/// Spans cut to `width`, the last one ending in "…" when something was left out.
pub fn cut(spans: Vec<Span<'_>>, width: usize) -> Vec<Span<'_>> {
    let total: usize = spans.iter().map(Span::width).sum();
    if total <= width {
        return spans;
    }
    let mut room = width;
    let mut out = Vec::new();
    for span in spans {
        let span_width = span.width();
        if span_width < room {
            room -= span_width;
            out.push(span);
            continue;
        }
        let text = shorten(&span.content, room);
        out.push(Span::styled(text, span.style));
        break;
    }
    out
}

/// `text` shortened to `width` columns with "…" at the end.
pub fn shorten(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if used + ch_width > width - 1 {
            break;
        }
        used += ch_width;
        out.push(ch);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::THEMES;

    fn row(buf: &Buffer) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, 0)].symbol().to_owned())
            .collect()
    }

    #[test]
    fn bars_fill_to_an_eighth_of_a_cell() {
        let palette = THEMES[0].palette(true);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let area = buf.area;
        bar(
            &mut buf,
            area,
            &[(0.25, Color::Green), (0.1, Color::Red)],
            &palette,
        );
        assert_eq!(row(&buf), "███▌░░░░░░");
        assert_eq!(buf[(0, 0)].fg, Color::Green);
        assert_eq!(buf[(2, 0)].fg, Color::Red, "the cell is mostly red");
        assert_eq!(buf[(3, 0)].fg, Color::Red);
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        let area = buf.area;
        bar(&mut buf, area, &[(1.7, Color::Green)], &palette);
        assert_eq!(row(&buf), "████", "never past the end");
    }

    #[test]
    fn themed_bars_draw_the_track_as_a_background() {
        let palette = THEMES[1].palette(true);
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        let area = buf.area;
        bar(&mut buf, area, &[(0.5, palette.user)], &palette);
        assert_eq!(row(&buf), "██  ");
        assert_eq!(buf[(3, 0)].bg, palette.track.unwrap());
    }

    fn picture(values: &[f64], style: GraphStyle, width: u16, height: u16) -> Vec<String> {
        let palette = THEMES[0].palette(true);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        let area = buf.area;
        graph(&mut buf, area, values, style, &palette);
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol().to_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn graphs_grow_from_the_bottom_right() {
        // The newest reading is at the right edge.
        assert_eq!(
            picture(&[1.0, 0.5], GraphStyle::Blocks, 3, 2),
            [" █ ", " ██"]
        );
        // Two readings per braille cell: a quarter on the left, full on the right.
        assert_eq!(
            picture(&[0.0, 0.25, 1.0], GraphStyle::Braille, 2, 1),
            [" ⣸"]
        );
        // Any activity at all shows one step.
        assert_eq!(picture(&[0.001], GraphStyle::Blocks, 1, 2), [" ", "▁"]);
        assert_eq!(picture(&[], GraphStyle::Braille, 3, 1), ["   "]);
    }

    fn text(line: &Line) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn lines_drop_their_least_important_pieces() {
        let pieces = || {
            vec![
                Piece::new(0, vec![Span::raw("load 1.00")]),
                Piece::new(2, vec![Span::raw("extra")]),
                Piece::new(1, vec![Span::raw("10% of 10 cores")]),
            ]
        };
        let style = Style::new();
        assert_eq!(
            text(&fit(pieces(), 80, style)),
            "load 1.00 · extra · 10% of 10 cores"
        );
        assert_eq!(
            text(&fit(pieces(), 30, style)),
            "load 1.00 · 10% of 10 cores"
        );
        assert_eq!(text(&fit(pieces(), 12, style)), "load 1.00");
        assert_eq!(text(&fit(pieces(), 6, style)), "load …");
    }

    #[test]
    fn long_lines_wrap_onto_a_second_row() {
        let pieces = || {
            vec![
                Piece::new(0, vec![Span::raw("app 7.2G")]),
                Piece::new(1, vec![Span::raw("wired 2.3G")]),
                Piece::new(1, vec![Span::raw("compressed 3.0G")]),
                Piece::new(2, vec![Span::raw("cached 2.6G")]),
            ]
        };
        assert_eq!(natural_width(&pieces()), 8 + 10 + 15 + 11 + 3 * 3);
        let style = Style::new();
        let lines: Vec<String> = fit_rows(pieces(), 30, 2, style).iter().map(text).collect();
        assert_eq!(
            lines,
            ["app 7.2G · wired 2.3G", "compressed 3.0G · cached 2.6G"]
        );
        let lines: Vec<String> = fit_rows(pieces(), 80, 2, style).iter().map(text).collect();
        assert_eq!(
            lines,
            ["app 7.2G · wired 2.3G · compressed 3.0G · cached 2.6G"]
        );
        let lines: Vec<String> = fit_rows(pieces(), 30, 1, style).iter().map(text).collect();
        assert_eq!(lines, ["app 7.2G · wired 2.3G"]);
    }

    #[test]
    fn shortens_with_an_ellipsis() {
        assert_eq!(shorten("Macintosh HD", 20), "Macintosh HD");
        assert_eq!(shorten("Macintosh HD", 6), "Macin…");
        assert_eq!(shorten("abc", 0), "");
    }
}
