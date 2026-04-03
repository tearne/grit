use color_eyre::eyre::{bail, Result};
use ratatui::style::{Color, Modifier, Style};

const NAMES: &[&str] = &["default", "autumn"];

pub(crate) struct Theme {
    name: &'static str,
    pub(crate) background: Color,
    pub(crate) title_bar: Style,
    pub(crate) list_highlight: Style,
    pub(crate) status_added: Style,
    pub(crate) status_deleted: Style,
    pub(crate) status_modified: Style,
    pub(crate) status_moved: Style,
    pub(crate) count_positive: Color,
    pub(crate) count_negative: Color,
    pub(crate) count_positive_bg: Color,
    pub(crate) count_negative_bg: Color,
    pub(crate) divider: Style,
    pub(crate) footer: Style,
    pub(crate) footer_notification: Style,
    pub(crate) diff_added: Color,
    pub(crate) diff_deleted: Color,
}

impl Theme {
    pub(crate) fn from_name(name: &str) -> Result<Self> {
        match name {
            "default" => Ok(Self::default_theme()),
            "autumn"  => Ok(Self::autumn()),
            _ => bail!("unknown theme '{}' — available: {}", name, NAMES.join(", ")),
        }
    }

    pub(crate) fn next(&self) -> Self {
        let idx = NAMES.iter().position(|&n| n == self.name).unwrap_or(0);
        let next_name = NAMES[(idx + 1) % NAMES.len()];
        Self::from_name(next_name).expect("name sourced from NAMES is always valid")
    }

    fn default_theme() -> Self {
        Theme {
            name:                 "default",
            background:           Color::Reset,
            title_bar:            Style::default().bg(Color::DarkGray).fg(Color::White),
            list_highlight:       Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD),
            status_added:         Style::default().fg(Color::Green),
            status_deleted:       Style::default().fg(Color::Red),
            status_modified:      Style::default().fg(Color::Yellow),
            status_moved:         Style::default().fg(Color::Cyan),
            count_positive:       Color::Green,
            count_negative:       Color::Red,
            count_positive_bg:    Color::Rgb(0, 80, 0),
            count_negative_bg:    Color::Rgb(80, 0, 0),
            divider:              Style::default().fg(Color::DarkGray),
            footer:               Style::default().bg(Color::DarkGray).fg(Color::Gray),
            footer_notification:  Style::default().bg(Color::DarkGray).fg(Color::Yellow),
            diff_added:           Color::Green,
            diff_deleted:         Color::Red,
        }
    }

    fn autumn() -> Self {
        Theme {
            name:                 "autumn",
            background:           Color::Rgb(0x21, 0x21, 0x21),
            title_bar:            Style::default().bg(Color::Rgb(0x40, 0x40, 0x40)).fg(Color::Rgb(0xF3, 0xF2, 0xCC)),
            list_highlight:       Style::default().bg(Color::Rgb(0x50, 0x50, 0x50)).fg(Color::Rgb(0xF3, 0xF2, 0xCC)).add_modifier(Modifier::BOLD),
            status_added:         Style::default().fg(Color::Rgb(0x99, 0xBE, 0x70)),
            status_deleted:       Style::default().fg(Color::Rgb(0xF0, 0x5E, 0x48)),
            status_modified:      Style::default().fg(Color::Rgb(0xFA, 0xD5, 0x66)),
            status_moved:         Style::default().fg(Color::Rgb(0x86, 0xC1, 0xB9)),
            count_positive:       Color::Rgb(0x99, 0xBE, 0x70),
            count_negative:       Color::Rgb(0xF0, 0x5E, 0x48),
            count_positive_bg:    Color::Rgb(0x2A, 0x3D, 0x1E),
            count_negative_bg:    Color::Rgb(0x4A, 0x18, 0x10),
            divider:              Style::default().fg(Color::Rgb(0x64, 0x6F, 0x69)),
            footer:               Style::default().bg(Color::Rgb(0x21, 0x21, 0x21)).fg(Color::Rgb(0xA8, 0xA8, 0xA8)),
            footer_notification:  Style::default().bg(Color::Rgb(0x21, 0x21, 0x21)).fg(Color::Rgb(0xFA, 0xD5, 0x66)),
            diff_added:           Color::Rgb(0x99, 0xBE, 0x70),
            diff_deleted:         Color::Rgb(0xF0, 0x5E, 0x48),
        }
    }
}
