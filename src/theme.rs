//! Color themes after well-known editor themes, plus the terminal's own
//! colors: the 50 that meridian has too, kept in cerne. Here they color the
//! meters the way htop does: user time green, system time red, nice blue.

use ratatui::style::{Color, Modifier, Style};

pub use cerne::text::fold;
use cerne::themes::{Colors, blend, color};
pub use cerne::themes::{THEMES, Theme, find, truecolor_from_env};

use crate::model::Part;

/// Usage from which a meter turns from the low color to the middle one, and
/// from the middle one to the high one.
pub const MIDDLE: f64 = 0.6;
pub const HIGH: f64 = 0.85;

/// A theme's colors, ready to draw with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// `Color::Reset` for the terminal's own background.
    pub bg: Color,
    pub fg: Color,
    muted: Option<Color>,
    /// Panel titles.
    pub title: Color,
    /// Keys and the selection.
    pub accent: Color,
    /// Light, busy and heavy usage.
    pub low: Color,
    pub middle: Color,
    pub high: Color,
    /// CPU time, as htop colors it.
    pub user: Color,
    pub nice: Color,
    pub system: Color,
    pub steal: Color,
    pub iowait: Color,
    /// Memory slices.
    pub wired: Color,
    pub compressed: Color,
    pub buffers: Color,
    pub cached: Color,
    /// Empty part of the bars; `None` draws it with shading instead.
    pub track: Option<Color>,
    /// Low, middle and high colors as RGB, to blend a smooth gradient.
    gradient: Option<[u32; 3]>,
    truecolor: bool,
}

impl Palette {
    /// Secondary text: the theme's comment color, or dimmed text in the terminal's colors.
    pub fn muted(&self) -> Style {
        match self.muted {
            Some(color) => Style::new().fg(color),
            None => Style::new().add_modifier(Modifier::DIM),
        }
    }

    /// Colors for the whole screen.
    pub fn base(&self) -> Style {
        Style::new().fg(self.fg).bg(self.bg)
    }

    /// Color of a usage from 0 to 1, in three steps.
    pub fn level(&self, usage: f64) -> Color {
        if usage >= HIGH {
            self.high
        } else if usage >= MIDDLE {
            self.middle
        } else {
            self.low
        }
    }

    /// Color of a height from 0 to 1 in a graph: a smooth blend from the low
    /// color to the high one where the terminal shows 24-bit color, the three
    /// steps otherwise.
    pub fn gradient(&self, height: f64) -> Color {
        let Some([low, middle, high]) = self.gradient.filter(|_| self.truecolor) else {
            return self.level(height);
        };
        let height = height.clamp(0.0, 1.0);
        let rgb = if height < MIDDLE {
            blend(low, middle, height / MIDDLE)
        } else {
            blend(middle, high, (height - MIDDLE) / (1.0 - MIDDLE))
        };
        let [_, r, g, b] = rgb.to_be_bytes();
        Color::Rgb(r, g, b)
    }

    /// Color of a slice of the memory bar.
    pub fn part(&self, part: Part) -> Color {
        match part {
            Part::App | Part::Used => self.user,
            Part::Wired => self.wired,
            Part::Compressed | Part::Shared => self.compressed,
            Part::Buffers => self.buffers,
            Part::Cached => self.cached,
        }
    }
}

/// A theme's colors as vitals uses them.
pub trait Paint {
    /// With `truecolor` off, colors come from the 256-color palette.
    fn palette(&self, truecolor: bool) -> Palette;
}

impl Paint for Theme {
    fn palette(&self, truecolor: bool) -> Palette {
        let Some(colors) = self.colors else {
            return Palette {
                bg: Color::Reset,
                fg: Color::Reset,
                muted: None,
                title: Color::Cyan,
                accent: Color::Cyan,
                low: Color::Green,
                middle: Color::Yellow,
                high: Color::Red,
                user: Color::Green,
                nice: Color::Blue,
                system: Color::Red,
                steal: Color::Cyan,
                iowait: Color::DarkGray,
                wired: Color::Red,
                compressed: Color::Magenta,
                buffers: Color::Blue,
                cached: Color::Yellow,
                track: None,
                gradient: None,
                truecolor,
            };
        };
        let Colors {
            bg,
            fg,
            comment,
            red,
            orange,
            yellow,
            green,
            cyan,
            blue,
            purple,
        } = colors;
        let color = |rgb: u32| color(rgb, truecolor);
        Palette {
            bg: color(bg),
            fg: color(fg),
            muted: Some(color(comment)),
            title: color(colors.slot(self.primary)),
            accent: color(colors.slot(self.accent)),
            low: color(green),
            middle: color(yellow),
            high: color(red),
            user: color(green),
            nice: color(blue),
            system: color(red),
            steal: color(cyan),
            iowait: color(comment),
            wired: color(orange),
            compressed: color(purple),
            buffers: color(blue),
            cached: color(yellow),
            // A shade a quarter of the way from the background to the comments.
            track: Some(color(blend(bg, comment, 0.25))),
            gradient: Some([green, yellow, red]),
            truecolor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_cpu_time_like_htop() {
        let tokyo = THEMES[find("Tokyo Night").unwrap()].palette(true);
        assert_eq!(tokyo.bg, Color::Rgb(0x1a, 0x1b, 0x26));
        assert_eq!(tokyo.user, Color::Rgb(0x9e, 0xce, 0x6a), "green");
        assert_eq!(tokyo.system, Color::Rgb(0xf7, 0x76, 0x8e), "red");
        assert_eq!(tokyo.nice, Color::Rgb(0x7a, 0xa2, 0xf7), "blue");
        assert_eq!(tokyo.title, Color::Rgb(0x7a, 0xa2, 0xf7));
        assert_eq!(tokyo.accent, Color::Rgb(0xbb, 0x9a, 0xf7));
        assert_eq!(tokyo.muted(), Style::new().fg(Color::Rgb(0x56, 0x5f, 0x89)));
        // A quarter of the way from #1a1b26 to #565f89.
        assert_eq!(tokyo.track, Some(Color::Rgb(0x29, 0x2c, 0x3f)));
    }

    #[test]
    fn usage_turns_from_green_to_yellow_to_red() {
        let tokyo = THEMES[find("Tokyo Night").unwrap()].palette(true);
        assert_eq!(tokyo.level(0.3), tokyo.low);
        assert_eq!(tokyo.level(0.7), tokyo.middle);
        assert_eq!(tokyo.level(0.9), tokyo.high);
        assert_eq!(tokyo.gradient(0.0), tokyo.low);
        assert_eq!(tokyo.gradient(MIDDLE), tokyo.middle);
        assert_eq!(tokyo.gradient(1.0), tokyo.high);
        let halfway = tokyo.gradient(MIDDLE / 2.0);
        assert!(![tokyo.low, tokyo.middle].contains(&halfway), "blended");
        // Without 24-bit color the gradient keeps to the three steps.
        let indexed = THEMES[find("Tokyo Night").unwrap()].palette(false);
        assert_eq!(indexed.gradient(0.3), indexed.low);
    }

    #[test]
    fn keeps_the_terminal_colors_by_default() {
        let terminal = THEMES[0].palette(true);
        assert_eq!((terminal.bg, terminal.user), (Color::Reset, Color::Green));
        assert_eq!(terminal.muted(), Style::new().add_modifier(Modifier::DIM));
        assert_eq!(terminal.track, None);
        assert_eq!(terminal.gradient(0.99), Color::Red);
    }

    #[test]
    fn falls_back_to_the_256_color_palette() {
        let tokyo = THEMES[find("Tokyo Night").unwrap()].palette(false);
        assert_eq!(tokyo.bg, Color::Indexed(234));
        assert_eq!(tokyo.title, Color::Indexed(111));
    }
}
