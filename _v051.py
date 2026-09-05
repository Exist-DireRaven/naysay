import io

with io.open("src/tui.rs", encoding="utf-8") as f:
    s = f.read()

fails = []
def rep(old, new, count=1):
    global s
    n = s.count(old)
    if n != count:
        fails.append("count %d != %d: %r" % (n, count, old[:70]))
        return
    s = s.replace(old, new, count)

# ── 1. display_width refactor: split out char_width ────────────────────────
i = s.index("fn display_width(text: &str) -> usize {")
# find end of function: the closing "}" at column 0 after start
j = s.index("\n}\n", i) + 3
old_fn = s[i:j]

new_fns = '''/// Display width of one character. CJK ideographs, kana, Hangul and
/// fullwidth punctuation are 2 columns; everything else is 1. Same
/// accounting real terminals use for cursor placement and wrapping.
fn char_width(c: char) -> usize {
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

fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}'''

s = s[:i] + new_fns + s[j:]

# ── 2. replace line_height with the pre-wrap machinery ─────────────────────
old_lh = '''/// Estimated terminal-row height of `line` when wrapped at `width`
/// columns. Uses `display_width`, so CJK text wraps on the same accounting
/// ratatui and real terminals use. Empty lines still take one row.
fn line_height(line: &Line<'_>, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let w: usize = line
        .spans
        .iter()
        .map(|sp| display_width(sp.content.as_ref()))
        .sum();
    if w == 0 {
        1
    } else {
        (w as u16).div_ceil(width).max(1)
    }
}'''

new_wrap = '''/// Flatten a styled line into a (style, char) stream — the raw material
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
fn wrap_line_to_width(line: &Line<'_>, width: usize) -> Vec<Line<'static>> {
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
            if b > i { b } else { j }
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
fn wrap_entry_lines(lines: &[Line<'_>], width: usize) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    for l in lines {
        out.extend(wrap_line_to_width(l, width));
    }
    if out.is_empty() {
        out.push(Line::from(""));
    }
    out
}

/// Tail of `text` that fits in `max` display columns (walking from the
/// end). Used to keep the input row's cursor and text visible when the
/// user types past the terminal width.
fn tail_by_display_width(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let mut w = 0usize;
    let mut start = 0usize;
    for (idx, ch) in text.char_indices().rev() {
        let cw = char_width(ch);
        if w + cw > max {
            start = idx + ch.len_utf8();
            break;
        }
        w += cw;
        start = idx;
    }
    text[start..].to_string()
}'''

rep(old_lh, new_wrap)

# ── 3. flush_pending body: exact rows ──────────────────────────────────────
old_flush = '''    let width = terminal.size().map(|s| s.width).unwrap_or(80);
    let mut lines: Vec<Line<'_>> = Vec::new();
    for entry in &state.history[state.flushed..limit] {
        lines.extend(entry_to_lines(entry));
    }
    // u16::MAX ≈ 65k rows ≈ 900 screens — no realistic transcript hits the
    // ceiling. Accumulate in u32 and saturate so a pathological log degrades
    // to a clipped insert instead of an overflow panic.
    let height: u16 = lines
        .iter()
        .map(|l| line_height(l, width) as u32)
        .sum::<u32>()
        .min(u16::MAX as u32) as u16;
    if height == 0 {
        state.flushed = limit;
        return Ok(());
    }

    let x = 0u16;
    terminal
        .insert_before(height, |buf| {
            let mut y = 0u16;
            for line in &lines {
                let h = line_height(line, width);
                buf.set_line(x, y, line, width);
                y += h;
            }
        })
        .context("insert transcript into scrollback")?;
    state.flushed = limit;
    Ok(())
}'''

new_flush = '''    let width = terminal.size().map(|s| s.width as usize).unwrap_or(80);
    if width == 0 {
        return Ok(());
    }

    // Pre-wrap every logical line into exact physical rows. Because each
    // row fits `width` columns, the terminal never re-wraps our output:
    // inserted row count == visible row count, so nothing is clipped and
    // no blank gaps appear. Styles ride along on the spans.
    let mut rows: Vec<Line<'static>> = Vec::new();
    for entry in &state.history[state.flushed..limit] {
        let logical = entry_to_lines(entry);
        rows.extend(wrap_entry_lines(&logical, width));
    }
    state.flushed = limit;

    // Chunks keep every insert's row count comfortably inside u16.
    for chunk in rows.chunks(8_192) {
        let height = chunk.len() as u16;
        terminal
            .insert_before(height, |buf| {
                for (y, line) in chunk.iter().enumerate() {
                    buf.set_line(0, y as u16, line, width as u16);
                }
            })
            .context("insert transcript into scrollback")?;
    }
    Ok(())
}'''

rep(old_flush, new_flush)

# ── 4. input row: tail-scroll when overflow ────────────────────────────────
old_input = '''    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(MUTED)),
        Span::raw(input.to_string()),
    ]);
    f.render_widget(Paragraph::new(input_line), rows);'''
new_input = '''    // Keep the tail of the input visible when it overflows the row: show
    // the last `max_text` display columns instead of clipping the end.
    let max_text = (area.width as usize).saturating_sub(3);
    let visible = tail_by_display_width(input, max_text);
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(MUTED)),
        Span::raw(visible.clone()),
    ]);
    f.render_widget(Paragraph::new(input_line), rows);'''
rep(old_input, new_input)

old_cursor = '''    if !state.busy {
        // Display columns, not char count — CJK chars are 2 columns wide.
        let offset = 2 + display_width(input) as u16;
        let x = (area.x + offset).min(area.x + area.width.saturating_sub(1));
        f.set_cursor_position(Position::new(x, area.y));
    }'''
new_cursor = '''    if !state.busy {
        // Display columns, not char count — CJK chars are 2 columns wide —
        // measured against the VISIBLE tail, not the full input.
        let x = (area.x + 2 + display_width(&visible) as u16)
            .min(area.x + area.width.saturating_sub(1));
        f.set_cursor_position(Position::new(x, area.y));
    }'''
rep(old_cursor, new_cursor)

# ── 5. verdict matcher: strip markdown emphasis before matching ────────────
old_v = '''fn is_verdict_line(line: &str) -> bool {
    let t = line.trim_start();
    let lower = t.to_lowercase();'''
new_v = '''fn is_verdict_line(line: &str) -> bool {
    // Models often emit "**5. Verdict**" or "## Verdict" — markdown
    // emphasis must not hide the verdict from the red accent.
    let t = line
        .trim_start()
        .trim_start_matches(['*', '#', '>', ' ']);
    let lower = t.to_lowercase();'''
rep(old_v, new_v)

with io.open("src/tui.rs", "w", encoding="utf-8", newline="\n") as f:
    f.write(s)
print("fails:", fails)
