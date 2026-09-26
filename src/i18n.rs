//! Interface text in English and Brazilian Portuguese, and the numbers in
//! each language's style: "14.4G" or "14,4G".

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Lang {
    En,
    Pt,
}

pub type Keys = &'static [(&'static str, &'static str)];

pub struct Text {
    pub cpu: &'static str,
    pub memory: &'static str,
    pub swap: &'static str,
    pub disks: &'static str,
    /// Short labels for tiny windows, where there are no panel titles.
    pub mem_short: &'static str,
    pub swap_short: &'static str,
    pub user: &'static str,
    pub nice: &'static str,
    pub system: &'static str,
    pub steal: &'static str,
    pub iowait: &'static str,
    pub idle: &'static str,
    pub busy: &'static str,
    pub load: &'static str,
    pub cores: &'static str,
    pub of_cores: &'static str,
    pub process: &'static str,
    pub processes: &'static str,
    pub thread: &'static str,
    pub threads: &'static str,
    pub running: &'static str,
    pub stall: &'static str,
    pub app: &'static str,
    pub wired: &'static str,
    pub compressed: &'static str,
    pub used: &'static str,
    pub shared: &'static str,
    pub buffers: &'static str,
    pub cached: &'static str,
    pub free_plural: &'static str,
    pub of: &'static str,
    pub available: &'static str,
    pub pressure: &'static str,
    pub pressure_levels: [&'static str; 3],
    pub no_swap: &'static str,
    pub swap_in: &'static str,
    pub swap_out: &'static str,
    pub read: &'static str,
    pub write: &'static str,
    pub up: &'static str,
    pub every: &'static str,
    pub reading: &'static str,
    pub no_disks: &'static str,
    pub limits: &'static str,
    pub session: &'static str,
    pub week: &'static str,
    pub month: &'static str,
    pub resets: &'static str,
    pub renewed: &'static str,
    pub tomorrow: &'static str,
    pub in_: &'static str,
    pub live: &'static str,
    /// "{} ago", with the time in place of the braces.
    pub ago: &'static str,
    weekdays: [&'static str; 7],
    months: [&'static str; 12],
    pub themes: &'static str,
    pub no_theme: &'static str,
    pub unsaved: &'static str,
    pub help: &'static str,
    pub main_keys: Keys,
    pub theme_keys: Keys,
    pub help_keys: Keys,
}

const EN: Text = Text {
    cpu: "CPU",
    memory: "Memory",
    swap: "Swap",
    disks: "Disks",
    mem_short: "Mem",
    swap_short: "Swap",
    user: "user",
    nice: "nice",
    system: "system",
    steal: "steal",
    iowait: "iowait",
    idle: "idle",
    busy: "busy",
    load: "load",
    cores: "cores",
    of_cores: "of {} cores",
    process: "process",
    processes: "processes",
    thread: "thread",
    threads: "threads",
    running: "running",
    stall: "stall",
    app: "app",
    wired: "wired",
    compressed: "compressed",
    used: "used",
    shared: "shared",
    buffers: "buffers",
    cached: "cached",
    free_plural: "free",
    of: "of",
    available: "available",
    pressure: "pressure",
    pressure_levels: ["normal", "high", "critical"],
    no_swap: "no swap",
    swap_in: "in",
    swap_out: "out",
    read: "read",
    write: "write",
    up: "up",
    every: "every",
    reading: "Reading…",
    no_disks: "No disks",
    limits: "AI limits",
    session: "session",
    week: "week",
    month: "month",
    resets: "resets",
    renewed: "renewed",
    tomorrow: "tomorrow",
    in_: "in",
    live: "live",
    ago: "{} ago",
    weekdays: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    months: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    themes: "Themes",
    no_theme: "No theme found",
    unsaved: "Could not save the settings",
    help: "Keys",
    main_keys: &[
        ("t", "theme"),
        ("v", "graph style"),
        ("g", "graphs"),
        ("c", "cores"),
        ("l", "AI limits"),
        ("+-", "interval"),
        ("?", "help"),
        ("q", "quit"),
    ],
    theme_keys: &[("↑↓", "try"), ("Enter", "keep"), ("Esc", "back")],
    help_keys: &[
        ("t", "color theme: 50 editor themes"),
        ("v", "graph style: braille or blocks"),
        ("g", "show or hide the graphs"),
        ("c", "show or hide each core"),
        ("l", "show or hide the Claude and Codex limits"),
        ("+  -", "read more or less often"),
        ("q  Esc", "quit"),
        ("", ""),
        ("", "CPU is the share of all cores together:"),
        ("", "8 of 10 cores fully busy is 80%."),
    ],
};

const PT: Text = Text {
    cpu: "CPU",
    memory: "Memória",
    swap: "Swap",
    disks: "Discos",
    mem_short: "Mem",
    swap_short: "Swap",
    user: "usuário",
    nice: "nice",
    system: "sistema",
    steal: "steal",
    iowait: "iowait",
    idle: "ocioso",
    busy: "em uso",
    load: "carga",
    cores: "núcleos",
    of_cores: "de {} núcleos",
    process: "processo",
    processes: "processos",
    thread: "thread",
    threads: "threads",
    running: "rodando",
    stall: "espera",
    app: "apps",
    wired: "fixa",
    compressed: "comprimida",
    used: "usada",
    shared: "compartilhada",
    buffers: "buffers",
    cached: "cache",
    free_plural: "livres",
    of: "de",
    available: "disponível",
    pressure: "pressão",
    pressure_levels: ["normal", "alta", "crítica"],
    no_swap: "sem swap",
    swap_in: "entrada",
    swap_out: "saída",
    read: "leitura",
    write: "escrita",
    up: "ligado há",
    every: "a cada",
    reading: "Lendo…",
    no_disks: "Nenhum disco",
    limits: "Limites de IA",
    session: "sessão",
    week: "semana",
    month: "mês",
    resets: "renova",
    renewed: "renovou",
    tomorrow: "amanhã",
    in_: "em",
    live: "ao vivo",
    ago: "há {}",
    weekdays: ["seg", "ter", "qua", "qui", "sex", "sáb", "dom"],
    months: [
        "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
    ],
    themes: "Temas",
    no_theme: "Nenhum tema encontrado",
    unsaved: "Não foi possível salvar os ajustes",
    help: "Teclas",
    main_keys: &[
        ("t", "tema"),
        ("v", "estilo do gráfico"),
        ("g", "gráficos"),
        ("c", "núcleos"),
        ("l", "limites de IA"),
        ("+-", "intervalo"),
        ("?", "ajuda"),
        ("q", "sair"),
    ],
    theme_keys: &[
        ("↑↓", "experimentar"),
        ("Enter", "manter"),
        ("Esc", "voltar"),
    ],
    help_keys: &[
        ("t", "tema de cores: 50 temas de editores"),
        ("v", "estilo do gráfico: braille ou blocos"),
        ("g", "mostrar ou esconder os gráficos"),
        ("c", "mostrar ou esconder cada núcleo"),
        ("l", "mostrar ou esconder os limites do Claude e do Codex"),
        ("+  -", "ler com mais ou menos frequência"),
        ("q  Esc", "sair"),
        ("", ""),
        ("", "A CPU é a parte de todos os núcleos juntos:"),
        ("", "8 de 10 núcleos ocupados são 80%."),
    ],
};

impl Lang {
    /// Portuguese when the locale variables ask for it, English otherwise.
    pub fn from_env() -> Self {
        let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty())
            .unwrap_or_default();
        Self::from_locale(&locale)
    }

    fn from_locale(locale: &str) -> Self {
        if locale.to_ascii_lowercase().starts_with("pt") {
            Self::Pt
        } else {
            Self::En
        }
    }

    pub fn text(self) -> &'static Text {
        match self {
            Self::En => &EN,
            Self::Pt => &PT,
        }
    }

    /// `value` with `places` decimals and this language's decimal mark.
    pub fn decimal(self, value: f64, places: usize) -> String {
        let text = format!("{value:.places$}");
        match self {
            Self::En => text,
            Self::Pt => text.replace('.', ","),
        }
    }

    /// Sizes in binary units, as htop and `df -h` show them: "512B", "1.5K",
    /// "14.4G", "460G".
    pub fn bytes(self, bytes: u64) -> String {
        const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
        let mut value = bytes as f64;
        let mut unit = 0;
        while value >= 999.5 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }
        let number = match unit {
            0 => bytes.to_string(),
            _ if value < 99.95 => self.decimal(value, 1),
            _ => self.decimal(value, 0),
        };
        format!("{number}{}", UNITS[unit])
    }

    /// Bytes per second: "1.2M/s".
    pub fn rate(self, bytes_per_second: f64) -> String {
        format!("{}/s", self.bytes(bytes_per_second.max(0.0).round() as u64))
    }

    /// A span of time: "3d 4h", "4h 12m", "12m" ("min" in Portuguese).
    pub fn duration(self, seconds: u64) -> String {
        let (days, hours) = (seconds / 86_400, seconds % 86_400 / 3600);
        let minutes = seconds % 3600 / 60;
        let min = match self {
            Self::En => "m",
            Self::Pt => "min",
        };
        if days > 0 {
            format!("{days}d {hours}h")
        } else if hours > 0 {
            format!("{hours}h {minutes:02}{min}")
        } else {
            format!("{minutes}{min}")
        }
    }

    /// When a limit starts over, in local time: "17:09" today, "tomorrow
    /// 09:00", "Thu 15:59" within a week, "Oct 1 15:59" later on.
    pub fn reset_time(self, at: i64, now: i64) -> String {
        use chrono::{Datelike, Local, TimeZone};
        let (Some(at), Some(now)) = (
            Local.timestamp_opt(at, 0).single(),
            Local.timestamp_opt(now, 0).single(),
        ) else {
            return String::new();
        };
        let text = self.text();
        let time = at.format("%H:%M").to_string();
        match (at.date_naive() - now.date_naive()).num_days() {
            0 => time,
            1 => format!("{} {time}", text.tomorrow),
            2..=6 => {
                let weekday = text.weekdays[at.weekday().num_days_from_monday() as usize];
                format!("{weekday} {time}")
            }
            _ => {
                let month = text.months[at.month0() as usize];
                match self {
                    Self::En => format!("{month} {} {time}", at.day()),
                    Self::Pt => format!("{} {month} {time}", at.day()),
                }
            }
        }
    }

    /// How often the screen reads the computer: "1s", "500ms", "2.5s".
    pub fn interval(self, millis: u64) -> String {
        if millis < 1000 {
            format!("{millis}ms")
        } else if millis.is_multiple_of(1000) {
            format!("{}s", millis / 1000)
        } else {
            format!("{}s", self.decimal(millis as f64 / 1000.0, 1))
        }
    }
}

/// A share from 0 to 1 as a whole percentage: "24%".
pub fn percent(share: f64) -> String {
    format!("{:.0}%", (share * 100.0).clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_locale() {
        assert_eq!(Lang::from_locale("pt_BR.UTF-8"), Lang::Pt);
        assert_eq!(Lang::from_locale("PT_pt"), Lang::Pt);
        assert_eq!(Lang::from_locale("en_US.UTF-8"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
    }

    #[test]
    fn formats_sizes_in_binary_units() {
        let size = |bytes| Lang::En.bytes(bytes);
        assert_eq!(size(0), "0B");
        assert_eq!(size(512), "512B");
        assert_eq!(size(1000), "1.0K");
        assert_eq!(size(1536), "1.5K");
        assert_eq!(size(16 << 30), "16.0G");
        assert_eq!(size(14_447_280_128), "13.5G");
        assert_eq!(size(494_384_795_648), "460G");
        assert_eq!(size(2 << 40), "2.0T");
        assert_eq!(Lang::Pt.bytes(1536), "1,5K");
    }

    #[test]
    fn formats_rates_uptime_and_intervals() {
        assert_eq!(Lang::En.rate(1_258_291.2), "1.2M/s");
        assert_eq!(Lang::En.rate(-3.0), "0B/s");
        assert_eq!(Lang::En.duration(3 * 86_400 + 4 * 3600 + 120), "3d 4h");
        assert_eq!(Lang::En.duration(4 * 3600 + 5 * 60), "4h 05m");
        assert_eq!(Lang::Pt.duration(12 * 60), "12min");
        assert_eq!(Lang::En.interval(500), "500ms");
        assert_eq!(Lang::En.interval(2000), "2s");
        assert_eq!(Lang::Pt.interval(2500), "2,5s");
    }

    #[test]
    fn says_when_limits_start_over() {
        use chrono::{Local, NaiveDate, TimeZone};
        let at = |day: u32, hour: u32, minute: u32| {
            let naive = NaiveDate::from_ymd_opt(2026, 9, day)
                .unwrap()
                .and_hms_opt(hour, minute, 0)
                .unwrap();
            Local
                .from_local_datetime(&naive)
                .single()
                .unwrap()
                .timestamp()
        };
        // Friday, September 25, 2026.
        let now = at(25, 15, 0);
        assert_eq!(Lang::En.reset_time(at(25, 17, 9), now), "17:09");
        assert_eq!(Lang::En.reset_time(at(26, 9, 0), now), "tomorrow 09:00");
        assert_eq!(Lang::Pt.reset_time(at(26, 9, 0), now), "amanhã 09:00");
        assert_eq!(Lang::En.reset_time(at(28, 15, 59), now), "Mon 15:59");
        assert_eq!(Lang::Pt.reset_time(at(28, 15, 59), now), "seg 15:59");
        assert_eq!(
            Lang::En.reset_time(at(30, 8, 0) + 86_400 * 3, now),
            "Oct 3 08:00"
        );
        assert_eq!(
            Lang::Pt.reset_time(at(30, 8, 0) + 86_400 * 3, now),
            "3 out 08:00"
        );
    }

    #[test]
    fn rounds_percentages() {
        assert_eq!(percent(0.244), "24%");
        assert_eq!(percent(0.8), "80%");
        assert_eq!(percent(1.3), "100%");
        assert_eq!(percent(-0.1), "0%");
    }

    #[test]
    fn both_languages_have_the_same_keys() {
        let (en, pt) = (Lang::En.text(), Lang::Pt.text());
        let keys = |list: Keys| list.iter().map(|(key, _)| *key).collect::<Vec<_>>();
        assert_eq!(keys(en.main_keys), keys(pt.main_keys));
        assert_eq!(keys(en.theme_keys), keys(pt.theme_keys));
        assert_eq!(keys(en.help_keys), keys(pt.help_keys));
    }
}
