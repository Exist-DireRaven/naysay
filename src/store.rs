//! v0.3/v0.5 decision store — persistence, extraction, queries.
//!
//! Extracted from main.rs in v0.5 (D-023): Decision is not a CLI
//! concern. Everything here is deterministic — no LLM calls. The
//! store is cwd-local (`.naysay/decisions/`) by design (D-021).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── v0.3 decision store ──────────────────────────────────────────────────────────────────────

/// One saved decision. A flat JSON per file under `.naysay/decisions/
/// <kind>-<id>.json`. No schema-version field: the shape may drift across
/// naysay versions because the user owns the file and grep is the only
/// API surface this store promises.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DecisionRecord {
    /// 12 hex chars. Minted from a hash of wall-clock nanos; collision
    /// retries up to 8 times before giving up.
    pub id: String,
    /// "premortem" | "spec" | "postmortem"
    pub kind: String,
    /// epoch seconds at write time
    pub ts: u64,
    pub idea: String,
    /// Optional predecessor id, wired by the user in a future revision.
    pub parent: Option<String>,
    /// Full body exactly as the model produced it. Never parsed.
    pub body: String,
    /// The structured sections v0.2 taught the prompts to emit. Extracted
    /// by substring scan; empty when the model skipped them. Not validated.
    pub assumptions: Vec<String>,
    pub evidence: Vec<String>,
    pub unknowns: Vec<String>,
    pub failure_conditions: Vec<String>,
    /// 0..=100 when the model emitted a confidence number.
    pub confidence: Option<u8>,
    /// v0.5: "BUILD" | "DON'T BUILD" — the premortem's structured verdict.
    pub verdict: Option<String>,
    /// v0.5: "BUILT" | "KILLED" | "ABANDONED" | "UNKNOWN" — what actually
    /// happened, extracted from the postmortem's CALIBRATION section.
    pub outcome: Option<String>,
}

/// The store lives in the current working directory: `.naysay/decisions/`.
/// Cwd-local by design (D-021): the user chooses which directory is a
/// project, and therefore which decisions belong together.
fn decisions_dir() -> std::io::Result<PathBuf> {
    let dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".naysay")
        .join("decisions");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 12 hex chars from a hash of wall-clock nanos. The caller retries on
/// collision; at hundreds of records a collision is vanishingly rare.
pub(crate) fn make_decision_id(nanos: u128) -> String {
    let mut h: u64 = (nanos as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    let bytes = h.to_be_bytes();
    let mut out = String::with_capacity(12);
    for b in &bytes[..6] {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Extract the bullet list under a section heading. Forgiving by design:
/// accepts `## HEADING`, `# HEADING`, and bare `HEADING:`; grabs `- ` /
/// `* ` / `1. ` bullets until a blank line or the next heading. Returns
/// an empty list when the heading is absent — never an error.
pub(crate) fn extract_section(body: &str, heading: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    let want = heading.trim().trim_end_matches(':').to_ascii_lowercase();
    for raw in body.lines() {
        let line = raw.trim_end();
        if !in_section {
            let head = line.trim_start_matches('#').trim().trim_end_matches(':');
            if head.to_ascii_lowercase() == want {
                in_section = true;
            }
        } else {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                in_section = false;
                continue;
            }
            if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                out.push(rest.trim().to_string());
            } else {
                let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !digits.is_empty() && digits.len() <= 3 {
                    let after = &t[digits.len()..];
                    if let Some(rest) = after.strip_prefix(". ").or_else(|| after.strip_prefix(')'))
                    {
                        out.push(rest.trim().to_string());
                        continue;
                    }
                }
                in_section = false;
            }
        }
    }
    out
}

/// Parse the confidence number out of a line mentioning CONFIDENCE.
/// Accepts "0.62" and "62"; returns 0..=100.
pub(crate) fn extract_confidence(body: &str) -> Option<u8> {
    for raw in body.lines() {
        let t = raw.trim();
        if !t.to_uppercase().contains("CONFIDENCE") {
            continue;
        }
        let mut digits = String::new();
        let mut seen_dot = false;
        for ch in t.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else if ch == '.' && !seen_dot && !digits.is_empty() {
                digits.push(ch);
                seen_dot = true;
            } else if !digits.is_empty() {
                break;
            }
        }
        if let Ok(v) = digits.parse::<f64>() {
            let scaled = if v <= 1.0 { v * 100.0 } else { v };
            return Some(scaled.round().clamp(0.0, 100.0) as u8);
        }
    }
    None
}

/// Core save, parameterized by directory so tests can use a temp dir.
pub(crate) fn save_decision_to(
    dir: &std::path::Path,
    kind: &str,
    idea: &str,
    body: &str,
    parent: Option<&str>,
    nanos: u128,
) -> std::io::Result<String> {
    std::fs::create_dir_all(dir)?;
    for _ in 0..8 {
        let id = make_decision_id(nanos.wrapping_add(1));
        let path = dir.join(format!("{}-{}.json", kind, id));
        if path.exists() {
            continue;
        }
        let rec = DecisionRecord {
            id: id.clone(),
            kind: kind.to_string(),
            ts: (nanos / 1_000_000_000) as u64,
            idea: idea.to_string(),
            parent: parent.map(|s| s.to_string()),
            body: body.to_string(),
            assumptions: extract_section(body, "ASSUMPTIONS"),
            evidence: extract_section(body, "EVIDENCE"),
            unknowns: extract_section(body, "UNKNOWNS"),
            failure_conditions: extract_section(body, "FAILURE CONDITIONS"),
            confidence: extract_confidence(body),
            verdict: extract_verdict(body),
            outcome: extract_outcome(body),
        };
        let json = serde_json::to_string_pretty(&rec).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)?;
        return Ok(id);
    }
    Err(std::io::Error::other(
        "could not mint a fresh decision id after 8 tries",
    ))
}

/// Save into the cwd store. Best-effort: callers print the error and move
/// on — a failed save must never break the command's primary output.
pub(crate) fn save_decision(
    kind: &str,
    idea: &str,
    body: &str,
    parent: Option<&str>,
) -> std::io::Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = decisions_dir()?;
    save_decision_to(&dir, kind, idea, body, parent, now)
}

pub(crate) fn read_record_by_id(dir: &std::path::Path, id: &str) -> Option<DecisionRecord> {
    let short = id.splitn(2, '-').last().unwrap_or(id);
    let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == id || stem.ends_with(&format!("-{}", short)) {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(rec) = serde_json::from_str::<DecisionRecord>(&raw) {
                    return Some(rec);
                }
            }
        }
    }
    None
}

// ─── v0.3 query commands ───────────────────────────────────────────────────────────────────

pub(crate) fn run_d_by_id(id: &str) -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let Some(rec) = read_record_by_id(&dir, id) else {
        anyhow::bail!("no decision found for id: {id}");
    };
    let json =
        serde_json::to_string_pretty(&rec).map_err(|e| anyhow::anyhow!("serialize record: {e}"))?;
    println!("{json}");
    Ok(())
}

pub(crate) fn run_d_unknowns() -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let mut rows: Vec<DecisionRecord> = Vec::new();
    let entries = std::fs::read_dir(&dir).context("read decision store")?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(rec) = serde_json::from_str::<DecisionRecord>(&raw) {
                if !rec.unknowns.is_empty() {
                    rows.push(rec);
                }
            }
        }
    }
    rows.sort_by_key(|r| r.ts);
    let mut total = 0usize;
    for rec in &rows {
        total += rec.unknowns.len();
        println!(
            "# {}-{}  ({} unknown{})",
            rec.kind,
            rec.id,
            rec.unknowns.len(),
            if rec.unknowns.len() == 1 { "" } else { "s" }
        );
        for u in &rec.unknowns {
            println!("  - {u}");
        }
        println!();
    }
    if total == 0 {
        println!("(no unknowns recorded)");
    }
    Ok(())
}

pub(crate) fn run_d_link(child: &str) -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let Some(target) = read_record_by_id(&dir, child) else {
        anyhow::bail!("no decision found for: {child}");
    };
    println!("# {}: {}", target.kind, target.idea);
    println!();
    let mut current = Some(target);
    let mut depth = 0usize;
    while let Some(rec) = current.take() {
        for _ in 0..depth {
            print!("  ");
        }
        println!("└─ {}-{} (ts={})", rec.kind, rec.id, rec.ts);
        if let Some(parent_id) = rec.parent.as_deref() {
            current = read_record_by_id(&dir, parent_id);
        }
        depth += 1;
    }
    Ok(())
}

/// v0.5: the structured verdict line the premortem prompt asks for
/// (`VERDICT: BUILD` or `VERDICT: DON'T BUILD`). Returns the first
/// recognized verdict token; prose containing the word "verdict" without
/// a colon is ignored.
pub(crate) fn extract_verdict(body: &str) -> Option<String> {
    for raw in body.lines() {
        if !raw.to_uppercase().contains("VERDICT") {
            continue;
        }
        let Some((_, after)) = raw.split_once(':') else {
            continue;
        };
        let after = after.to_uppercase();
        let dont = after.contains("DON'T")
            || after.contains("DONT")
            || after.contains("DO NOT")
            || after.contains("NOT BUILD");
        if dont {
            return Some("DON'T BUILD".into());
        }
        if after.contains("BUILD") {
            return Some("BUILD".into());
        }
    }
    None
}

/// v0.5: the structured outcome line the postmortem prompt asks for
/// (`OUTCOME: BUILT|KILLED|ABANDONED|UNKNOWN`). First recognized token
/// wins; anything else is None rather than a guess.
pub(crate) fn extract_outcome(body: &str) -> Option<String> {
    for raw in body.lines() {
        if !raw.to_uppercase().contains("OUTCOME") {
            continue;
        }
        let Some((_, after)) = raw.split_once(':') else {
            continue;
        };
        let after = after.to_uppercase();
        for (token, canon) in [
            ("BUILT", "BUILT"),
            ("KILLED", "KILLED"),
            ("ABANDONED", "ABANDONED"),
            ("UNKNOWN", "UNKNOWN"),
        ] {
            if after.contains(token) {
                return Some(canon.into());
            }
        }
    }
    None
}

/// Lowercase alphanumeric word set, words of length 1 dropped. The unit
/// of deterministic retrieval: no embeddings, no dependencies.
pub(crate) fn tokenize(text: &str) -> std::collections::HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .map(|w| w.to_string())
        .collect()
}

/// Jaccard similarity on token sets. 1.0 = identical sets, 0.0 = disjoint.
pub(crate) fn relevance_score(
    a: &std::collections::HashSet<String>,
    b: &std::collections::HashSet<String>,
) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    inter / union
}

/// Classify a premortem verdict against the linked postmortem outcome.
/// Pure so the calibration table is testable without a store.
pub(crate) fn classify_verdict_outcome(
    verdict: Option<&str>,
    outcome: Option<&str>,
) -> &'static str {
    let (Some(v), Some(o)) = (verdict, outcome) else {
        return "unknown";
    };
    if o == "UNKNOWN" {
        return "unknown";
    }
    let build = v == "BUILD";
    let built = o == "BUILT";
    if build == built {
        "held"
    } else if build && !built {
        "wrong"
    } else {
        "overridden"
    }
}

/// v0.5 calibration: walk every premortem that carries a verdict and
/// confidence, find child postmortems (parent == premortem id) that carry
/// an outcome, and report agreement. Prints an explicit honesty caveat
/// when the corpus is too small for a statistic — a table built on two
/// rows would be performance, not measurement.
pub(crate) fn run_calibration() -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let mut records: Vec<DecisionRecord> = Vec::new();
    let entries = std::fs::read_dir(&dir).context("read decision store")?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(rec) = serde_json::from_str::<DecisionRecord>(&raw) {
                records.push(rec);
            }
        }
    }
    records.sort_by_key(|r| r.ts);

    let premortems: Vec<&DecisionRecord> = records
        .iter()
        .filter(|r| r.kind == "premortem" && r.verdict.is_some())
        .collect();
    println!("decisions stored : {}", records.len());
    println!("premortems       : {}", premortems.len());

    let mut held = 0usize;
    let mut wrong = 0usize;
    let mut overridden = 0usize;
    let mut pairs = 0usize;
    println!();
    println!("verdict vs outcome (linked pairs):");
    for p in &premortems {
        let Some(child) = records
            .iter()
            .find(|r| r.kind == "postmortem" && r.parent.as_deref() == Some(p.id.as_str()))
        else {
            continue;
        };
        let cls = classify_verdict_outcome(p.verdict.as_deref(), child.outcome.as_deref());
        if cls == "unknown" {
            continue;
        }
        pairs += 1;
        match cls {
            "held" => held += 1,
            "wrong" => wrong += 1,
            "overridden" => overridden += 1,
            _ => {}
        }
        println!(
            "  {}-{}  conf={:>3}%  verdict={:<11} outcome={:<9} -> {}",
            p.kind,
            p.id,
            p.confidence.unwrap_or(0),
            p.verdict.as_deref().unwrap_or("?"),
            child.outcome.as_deref().unwrap_or("?"),
            cls
        );
    }
    println!();
    if pairs == 0 {
        println!(
            "no linked premortem+postmortem pairs yet. calibration becomes\n\
             meaningful after a few real loops:\n\
             1. naysay premortem \"idea\"          (auto-saved with an id)\n\
             2. naysay postmortem \"idea\" --parent <that-id>"
        );
    } else {
        println!(
            "held {} / {}  ({:.0}%)  ·  wrong {}  ·  overridden {}",
            held,
            pairs,
            held as f64 * 100.0 / pairs as f64,
            wrong,
            overridden
        );
        if pairs < 3 {
            println!("(fewer than 3 linked pairs — this is a log, not a statistic)");
        }
    }
    Ok(())
}

/// v0.5: deterministic retrieval. Score = Jaccard overlap between the
/// query tokens and (idea + body) tokens of each stored record. No LLM,
/// no network, no dependencies — interpretation is the caller's job.
pub(crate) fn run_d_relevant(idea: &str) -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let query = tokenize(idea);
    let mut rows: Vec<(f64, DecisionRecord)> = Vec::new();
    let entries = std::fs::read_dir(&dir).context("read decision store")?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(rec) = serde_json::from_str::<DecisionRecord>(&raw) {
                let mut doc = rec.idea.clone();
                doc.push(' ');
                doc.push_str(&rec.body);
                let score = relevance_score(&query, &tokenize(&doc));
                if score > 0.05 {
                    rows.push((score, rec));
                }
            }
        }
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    if rows.is_empty() {
        println!("(no stored decision resembles this idea)");
        return Ok(());
    }
    println!("relevant past decisions:");
    for (score, rec) in rows.iter().take(10) {
        println!("  {:.2}  {}-{}  {}", score, rec.kind, rec.id, rec.idea);
        if let Some(v) = &rec.verdict {
            println!("        verdict: {v}");
        }
    }
    Ok(())
}

// ─── tests ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_verdict_build_and_dont_build() {
        let body = "1. Cause of death — scope.\n\nVERDICT: BUILD";
        assert_eq!(extract_verdict(body).as_deref(), Some("BUILD"));
        let body = "VERDICT: DON'T BUILD at the original scope.";
        assert_eq!(extract_verdict(body).as_deref(), Some("DON'T BUILD"));
        let body = "verdict: do not build this";
        assert_eq!(extract_verdict(body).as_deref(), Some("DON'T BUILD"));
    }

    #[test]
    fn extract_verdict_ignores_prose_without_colon() {
        // The premortem's section-5 header ("5. Verdict — build it ...")
        // contains the word but no colon; it must not resolve.
        let body = "5. Verdict — build it (at what scope) or don't (and what to do instead).";
        assert_eq!(extract_verdict(body), None);
    }

    #[test]
    fn extract_outcome_all_four_values() {
        for (line, want) in [
            ("OUTCOME: BUILT", "BUILT"),
            ("OUTCOME: KILLED", "KILLED"),
            ("OUTCOME: ABANDONED", "ABANDONED"),
            ("OUTCOME: UNKNOWN", "UNKNOWN"),
        ] {
            assert_eq!(extract_outcome(line).as_deref(), Some(want), "{line}");
        }
        assert_eq!(extract_outcome("no outcome here"), None);
    }

    #[test]
    fn tokenize_drops_single_chars_and_lowercases() {
        let t = tokenize("Build a Stock Monitor 做监控");
        assert!(t.contains("build") && t.contains("stock") && t.contains("做监控"));
        assert!(!t.contains("a"));
    }

    #[test]
    fn relevance_jaccard_bounds_and_ordering() {
        let a = tokenize("stock monitoring system alerts");
        let same = tokenize("stock monitoring system alerts");
        let partial = tokenize("stock monitoring dashboard");
        let none = tokenize("recipe for soup");
        assert_eq!(relevance_score(&a, &same), 1.0);
        let s_partial = relevance_score(&a, &partial);
        let s_none = relevance_score(&a, &none);
        assert!(s_partial > s_none);
        assert_eq!(s_none, 0.0);
    }

    #[test]
    fn classify_maps_all_four_cells() {
        assert_eq!(
            classify_verdict_outcome(Some("BUILD"), Some("BUILT")),
            "held"
        );
        assert_eq!(
            classify_verdict_outcome(Some("DON'T BUILD"), Some("KILLED")),
            "held"
        );
        assert_eq!(
            classify_verdict_outcome(Some("BUILD"), Some("KILLED")),
            "wrong"
        );
        assert_eq!(
            classify_verdict_outcome(Some("DON'T BUILD"), Some("BUILT")),
            "overridden"
        );
        assert_eq!(
            classify_verdict_outcome(Some("BUILD"), Some("UNKNOWN")),
            "unknown"
        );
        assert_eq!(classify_verdict_outcome(None, Some("BUILT")), "unknown");
    }
}
