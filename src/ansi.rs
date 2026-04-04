use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

/// Parse a byte slice containing ANSI SGR escape sequences into ratatui `Text`.
///
/// Only SGR sequences are handled (colors, bold, reset) — the subset difft
/// outputs. All other escape sequences are stripped silently.
pub(crate) fn parse(bytes: &[u8], diff_added: Color, diff_deleted: Color, diff_unmatched: Color) -> Text<'static> {
    let raw = String::from_utf8_lossy(bytes);
    let lines: Vec<Line<'static>> = raw
        .split('\n')
        .map(|line| parse_line(line, diff_added, diff_deleted, diff_unmatched))
        .collect();
    Text::from(lines)
}

fn parse_line(line: &str, diff_added: Color, diff_deleted: Color, diff_unmatched: Color) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default();
    let mut remaining = line;

    while !remaining.is_empty() {
        if let Some(esc_start) = remaining.find('\x1b') {
            // Emit text before the escape sequence.
            if esc_start > 0 {
                spans.push(Span::styled(remaining[..esc_start].to_owned(), style));
            }
            remaining = &remaining[esc_start..];

            // Try to consume an SGR sequence: ESC [ ... m
            if let Some(seq_end) = parse_sgr(remaining) {
                let params = &remaining[2..seq_end]; // between '[' and 'm'
                style = apply_sgr(style, params, diff_added, diff_deleted, diff_unmatched);
                remaining = &remaining[seq_end + 1..];
            } else {
                // Not a recognised sequence — skip the ESC byte and continue.
                remaining = &remaining[1..];
            }
        } else {
            spans.push(Span::styled(remaining.to_owned(), style));
            break;
        }
    }

    Line::from(spans)
}

/// If `s` starts with an SGR sequence (`ESC [ ... m`), returns the index of
/// the terminating `m`. Returns `None` if the sequence is absent or malformed.
fn parse_sgr(s: &str) -> Option<usize> {
    let mut chars = s.chars();
    if chars.next()? != '\x1b' || chars.next()? != '[' {
        return None;
    }
    // Find the 'm' terminator; bail if a non-SGR terminator appears first.
    let rest = &s[2..];
    let end = rest.find(|c: char| c.is_ascii_alphabetic())?;
    if rest.as_bytes()[end] == b'm' {
        Some(2 + end)
    } else {
        None
    }
}

fn apply_sgr(base: Style, params: &str, diff_added: Color, diff_deleted: Color, diff_unmatched: Color) -> Style {
    let codes: Vec<u8> = params
        .split(';')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut style = base;
    let mut i = 0;

    while i < codes.len() {
        match codes[i] {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            9 => style = style.add_modifier(Modifier::CROSSED_OUT),
            // Standard foreground colours.
            30..=37 => style = style.fg(remap(ansi_color(codes[i] - 30, false), diff_added, diff_deleted, diff_unmatched)),
            38 if codes.get(i + 1) == Some(&5) => {
                if let Some(&n) = codes.get(i + 2) {
                    style = style.fg(Color::Indexed(n));
                    i += 2;
                }
            }
            38 if codes.get(i + 1) == Some(&2) => {
                if let (Some(&r), Some(&g), Some(&b)) =
                    (codes.get(i + 2), codes.get(i + 3), codes.get(i + 4))
                {
                    style = style.fg(Color::Rgb(r, g, b));
                    i += 4;
                }
            }
            39 => style = style.fg(Color::Reset),
            // Standard background colours.
            40..=47 => style = style.bg(remap(ansi_color(codes[i] - 40, false), diff_added, diff_deleted, diff_unmatched)),
            48 if codes.get(i + 1) == Some(&5) => {
                if let Some(&n) = codes.get(i + 2) {
                    style = style.bg(Color::Indexed(n));
                    i += 2;
                }
            }
            48 if codes.get(i + 1) == Some(&2) => {
                if let (Some(&r), Some(&g), Some(&b)) =
                    (codes.get(i + 2), codes.get(i + 3), codes.get(i + 4))
                {
                    style = style.bg(Color::Rgb(r, g, b));
                    i += 4;
                }
            }
            49 => style = style.bg(Color::Reset),
            // Bright foreground colours.
            90..=97 => style = style.fg(remap(ansi_color(codes[i] - 90, true), diff_added, diff_deleted, diff_unmatched)),
            // Bright background colours.
            100..=107 => style = style.bg(remap(ansi_color(codes[i] - 100, true), diff_added, diff_deleted, diff_unmatched)),
            _ => {}
        }
        i += 1;
    }

    style
}

fn remap(color: Color, diff_added: Color, diff_deleted: Color, diff_unmatched: Color) -> Color {
    match color {
        Color::Green   | Color::LightGreen   => diff_added,
        Color::Red     | Color::LightRed     => diff_deleted,
        Color::Magenta | Color::LightMagenta => diff_unmatched,
        other => other,
    }
}

fn ansi_color(index: u8, bright: bool) -> Color {
    match (index, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        (7, true) => Color::White,
        _ => Color::Reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    const CUSTOM_ADDED: Color = Color::Rgb(0x99, 0xBE, 0x70);
    const CUSTOM_DELETED: Color = Color::Rgb(0xF0, 0x5E, 0x48);
    const CUSTOM_UNMATCHED: Color = Color::Rgb(0xBD, 0x82, 0xD7);

    fn first_span_fg(text: &ratatui::text::Text) -> Color {
        text.lines[0].spans[0].style.fg.unwrap()
    }

    fn first_span_bg(text: &ratatui::text::Text) -> Color {
        text.lines[0].spans[0].style.bg.unwrap()
    }

    #[test]
    fn green_fg_remaps_to_diff_added() {
        // ESC[32m is ANSI green foreground
        let input = b"\x1b[32mtext\x1b[0m";
        let text = parse(input, CUSTOM_ADDED, CUSTOM_DELETED, CUSTOM_UNMATCHED);
        assert_eq!(first_span_fg(&text), CUSTOM_ADDED);
    }

    #[test]
    fn red_fg_remaps_to_diff_deleted() {
        // ESC[31m is ANSI red foreground
        let input = b"\x1b[31mtext\x1b[0m";
        let text = parse(input, CUSTOM_ADDED, CUSTOM_DELETED, CUSTOM_UNMATCHED);
        assert_eq!(first_span_fg(&text), CUSTOM_DELETED);
    }

    #[test]
    fn green_bg_remaps_to_diff_added() {
        // ESC[42m is ANSI green background
        let input = b"\x1b[42mtext\x1b[0m";
        let text = parse(input, CUSTOM_ADDED, CUSTOM_DELETED, CUSTOM_UNMATCHED);
        assert_eq!(first_span_bg(&text), CUSTOM_ADDED);
    }

    #[test]
    fn red_bg_remaps_to_diff_deleted() {
        // ESC[41m is ANSI red background
        let input = b"\x1b[41mtext\x1b[0m";
        let text = parse(input, CUSTOM_ADDED, CUSTOM_DELETED, CUSTOM_UNMATCHED);
        assert_eq!(first_span_bg(&text), CUSTOM_DELETED);
    }

    #[test]
    fn magenta_fg_remaps_to_diff_unmatched() {
        // ESC[35m is ANSI magenta foreground
        let input = b"\x1b[35mtext\x1b[0m";
        let text = parse(input, CUSTOM_ADDED, CUSTOM_DELETED, CUSTOM_UNMATCHED);
        assert_eq!(first_span_fg(&text), CUSTOM_UNMATCHED);
    }

    #[test]
    fn bright_magenta_fg_remaps_to_diff_unmatched() {
        // ESC[95m is ANSI bright magenta foreground
        let input = b"\x1b[95mtext\x1b[0m";
        let text = parse(input, CUSTOM_ADDED, CUSTOM_DELETED, CUSTOM_UNMATCHED);
        assert_eq!(first_span_fg(&text), CUSTOM_UNMATCHED);
    }
}
