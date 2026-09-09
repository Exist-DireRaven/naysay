//! Pure text layout: display widths, input windows, and wrapping.
//!
//! Extracted from `tui.rs` for the workspace work (D-035 M2). Everything here
//! is pure and terminal-free, so the TUI's cursor placement and the
//! transcript's row wrapping can be tested without a TTY.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Display width of `text` in terminal columns. CJK ideographs, kana, and
/// fullwidth punctuation are 2 columns; everything else is 1.
///
/// ratatui renders each character using its Unicode width property when
/// drawing Paragraph content, so the cursor must follow the same
/// accounting. Counting `chars()` only gives 1 column per glyph and lands
/// the cursor inside a wide character — visually mid-glyph for Chinese
/// users. This is the same heuristic every terminal emulator uses for
/// cursor placement; we don't pull in `unicode-width` because a narrow
/// inline range check covers 99% of real prompts.
#[allow(dead_code)] // canonical width measure; non-test callers use char_width
pub(crate) fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

/// Display width of one character. CJK ideographs, kana, Hangul and
/// fullwidth punctuation are 2 columns; everything else is 1. Same
/// accounting real terminals use for cursor placement and wrapping.
pub(crate) fn char_width(c: char) -> usize {
    let cp = c as u32;
    let wide = (0x1100..=0x115F).contains(&cp)        // Hangul Jamo
        || (0x2E80..=0x303E).contains(&cp)             // CJK Radicals + Symbols
        || (0x3041..=0x33FF).contains(&cp)             // Hiragana, Katakana, CJK symbols
        || (0x3400..=0x4DBF).contains(&cp)             // CJK Extension A
        || (0x4E00..=0x9FFF).contains(&cp)             // CJK Unified Ideographs
        || (0xA000..=0xA4CF).contains(&cp)             // Yi
        || (0xAC00..=0xD7A3).contains(&cp)             // Hangul Syllables
        || (0xF900..=0xFAFF).contains(&cp)             // CJK Compatibility
        || (0xFE30..=0xFE4F).contains(&cp)             // CJK Compatibility Forms
        || (0xFF00..=0xFF60).contains(&cp)             // Fullwidth Forms
        || (0xFFE0..=0xFFE6).contains(&cp)             // Fullwidth signs
        || (0x20000..=0x2FFFD).contains(&cp)          // CJK Ext B-F + supplement
        || (0x30000..=0x3FFFD).contains(&cp);
    if wide {
        2
    } else {
        1
    }
}

/// Byte offset of char index `n` in `s` (cursor positions are char-based
/// because CJK chars are one cursor step each).
pub(crate) fn byte_index_of_char(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(b, _)| b).unwrap_or(s.len())
}

/// Largest prefix of `s` that is at most `max_bytes` long and ends on a
/// character boundary. `&s[..n]` panics when `n` lands inside a multi-byte
/// character — which mixed ASCII/CJK files hit constantly at round numbers
/// like 24_000.
pub(crate) fn byte_prefix(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// The visible window of the input around the cursor, plus the display
/// width of the text BEFORE the cursor inside that window (for cursor
/// placement). Short input: whole string, cursor mid-window. Overflow:
/// the window pins to the cursor so it is always visible.
pub(crate) fn input_window(input: &str, cursor: usize, max: usize) -> (String, usize) {
    let chars: Vec<char> = input.chars().collect();
    let cursor = cursor.min(chars.len());
    let prefix_w: usize = chars[..cursor].iter().map(|c| char_width(*c)).sum();
    let mut w = 0usize;
    let mut out = String::new();
    if prefix_w <= max {
        for c in &chars {
            let cw = char_width(*c);
            if w + cw > max {
                break;
            }
            w += cw;
            out.push(*c);
        }
        (out, prefix_w)
    } else {
        for c in &chars[cursor..] {
            let cw = char_width(*c);
            if w + cw > max {
                break;
            }
            w += cw;
            out.push(*c);
        }
        (out, 0)
    }
}

/// Flatten a styled line into a (style, char) stream — the raw material
/// for wrapping.
fn flatten_line(line: &Line<'_>) -> Vec<(Style, char)> {
    let mut out = Vec::new();
    for span in &line.spans {
        for ch in span.content.as_ref().chars() {
            out.push((span.style, ch));
        }
    }
    out
}

/// Rebuild one owned row from a (style, char) slice, merging adjacent
/// same-style chars into spans.
fn row_from(chars: &[(Style, char)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut cur_style: Option<Style> = None;
    let mut cur = String::new();
    for (style, ch) in chars {
        match cur_style {
            Some(ref s) if *s == *style => cur.push(*ch),
            _ => {
                if !cur.is_empty() {
                    let st = cur_style.take().unwrap_or_default();
                    spans.push(Span::styled(std::mem::take(&mut cur), st));
                }
                cur_style = Some(*style);
                cur.push(*ch);
            }
        }
    }
    if !cur.is_empty() {
        let st = cur_style.take().unwrap_or_default();
        spans.push(Span::styled(cur, st));
    }
    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
}

/// Word-aware wrap of one logical line into physical rows that each fit
/// `width` display columns (styles preserved). Rows are emitted exactly as
/// they will be seen: the terminal never re-wraps them, so the inserted
/// row count equals the visible row count — no clipped tails, no gaps.
///
/// Long unbreakable runs (URLs, CJK without spaces) hard-split at the
/// column boundary, which is what terminals do too.
pub(crate) fn wrap_line_to_width(line: &Line<'_>, width: usize) -> Vec<Line<'static>> {
    let chars = flatten_line(line);
    if width == 0 {
        return vec![row_from(&chars)];
    }
    let n = chars.len();
    if n == 0 {
        return vec![Line::from("")];
    }
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut i = 0usize;
    while i < n {
        let mut w = 0usize;
        let mut j = i;
        // Most recent wrap candidate: a space fully consumed by this row.
        let mut break_at: Option<usize> = None;
        while j < n {
            let cw = char_width(chars[j].1);
            if w + cw > width {
                break;
            }
            w += cw;
            j += 1;
            if chars[j - 1].1 == ' ' && j < n && w < width {
                break_at = Some(j);
            }
        }
        let end = if j >= n {
            n
        } else if let Some(b) = break_at {
            if b > i {
                b
            } else {
                j
            }
        } else if j > i {
            j
        } else {
            // A single char wider than the whole row: hard-advance one.
            i + 1
        };
        out.push(row_from(&chars[i..end]));
        i = end;
    }
    out
}

/// Wrap every logical line of an entry into exact physical rows.
pub(crate) fn wrap_entry_lines(lines: &[Line<'_>], width: usize) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    for l in lines {
        out.extend(wrap_line_to_width(l, width));
    }
    if out.is_empty() {
        out.push(Line::from(""));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    // ─── display_width ────────────────────────────────────────────────────────

    #[test]
    fn width_ascii_counts_one_per_char() {
        assert_eq!(display_width(""), 0);
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("make a stock monitor"), 20);
    }

    #[test]
    fn width_cjk_counts_two_per_char() {
        // The actual bug — CJK chars must take 2 columns each or the
        // cursor lands mid-glyph when typing Chinese.
        assert_eq!(display_width("做爬虫"), 6); // 3 chars × 2 cols
        assert_eq!(display_width("做个知乎热榜爬虫"), 16); // 8 chars × 2 cols
    }

    #[test]
    fn width_mixed_strings() {
        assert_eq!(display_width("a做b"), 4); // 1 + 2 + 1
        assert_eq!(display_width("hello 世界"), 10); // 5 + 1 + 4
    }

    // ─── wrap_line_to_width (scrollback truncation fix) ────────────────────────

    fn row_text(l: &Line<'static>) -> String {
        l.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn wrap_english_word_boundary_no_row_exceeds_width() {
        // The exact reported bug: a 130+ char line in a ~100-col terminal
        // used to be clipped mid-sentence by the height estimate.
        let line = Line::from("The premise is a voice in the terminal that says no before coding agents say yes. Six months out, it's dead. Here's the autopsy:");
        let rows = wrap_line_to_width(&line, 100);
        assert!(rows.len() >= 2, "must wrap to multiple rows");
        for r in &rows {
            assert!(
                display_width(&row_text(r)) <= 100,
                "row too wide: {:?}",
                row_text(r)
            );
        }
        let joined = rows
            .iter()
            .map(|r| format!("{} ", row_text(r)))
            .collect::<String>();
        assert!(joined.contains("autopsy:"), "tail lost: {joined}");
    }

    #[test]
    fn wrap_cjk_counts_two_columns_per_char() {
        let line = Line::from("做个知乎热榜爬虫并且支持多种数据源");
        let rows = wrap_line_to_width(&line, 11);
        for r in &rows {
            assert!(display_width(&row_text(r)) <= 11, "{:?}", row_text(r));
        }
        let total: usize = rows.iter().map(|r| display_width(&row_text(r))).sum();
        assert_eq!(total, 34);
    }

    #[test]
    fn wrap_long_unbroken_word_hard_splits() {
        let line = Line::from("aaaaaaaaaaaaaaaaaaaa"); // 20 chars, width 8
        let rows = wrap_line_to_width(&line, 8);
        assert_eq!(rows.len(), 3);
        assert_eq!(row_text(&rows[0]), "aaaaaaaa");
        assert_eq!(row_text(&rows[2]), "aaaa");
    }

    #[test]
    fn wrap_preserves_styles_across_rows() {
        let line = Line::from(vec![
            Span::styled("VERDICT: ".to_string(), Style::default().fg(Color::Red)),
            Span::raw("build the smallest version and then keep going with more text to force a wrap here"),
        ]);
        let rows = wrap_line_to_width(&line, 40);
        let red_rows = rows
            .iter()
            .filter(|r| r.spans.iter().any(|s| s.style.fg == Some(Color::Red)))
            .count();
        assert!(red_rows >= 1, "styled span lost in wrap");
        let joined = rows.iter().map(row_text).collect::<String>();
        assert!(joined.contains("force a wrap here"));
    }

    #[test]
    fn wrap_empty_line_is_exactly_one_empty_row() {
        let rows = wrap_line_to_width(&Line::from(""), 80);
        assert_eq!(rows.len(), 1);
        assert_eq!(row_text(&rows[0]), "");
    }

    #[test]
    fn byte_prefix_stops_on_a_char_boundary() {
        assert_eq!(byte_prefix("hello", 10), "hello");
        assert_eq!(byte_prefix("hello", 3), "hel");
        // A cut inside a CJK character backs up instead of panicking.
        let mixed = "abc中文def";
        assert_eq!(byte_prefix(mixed, 4), "abc");
        assert_eq!(byte_prefix(mixed, 6), "abc中");
        assert_eq!(byte_prefix("中文", 1), "");
        assert_eq!(byte_prefix("", 0), "");
        // Every prefix is a valid &str for any byte budget.
        for n in 0..=mixed.len() {
            let _ = byte_prefix(mixed, n);
        }
    }
}
