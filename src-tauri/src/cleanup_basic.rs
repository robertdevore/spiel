//! Basic deterministic cleanup provider for Spiel Phase 7.
//!
//! Implements the `CleanupProvider` trait with deterministic, local-only
//! text transformations. No AI, no network, no file I/O.
//!
//! Mode behaviors:
//! - RawDictation: trim whitespace, preserve wording
//! - CleanNotes: normalize whitespace, basic punctuation, split paragraphs
//! - AiPrompt: wrap in deterministic prompt template
//! - DeveloperReview: wrap in structured review template
//! - ThoughtPiece: wrap in structured essay template

use crate::cleanup::{
    CleanupError, CleanupProvider, CleanupProviderKind, CleanupRequest, CleanupResult,
};
use crate::modes::TextModeKind;

/// Basic deterministic cleanup provider.
/// Runs entirely locally — no network, no AI, no file access.
pub struct BasicCleanupProvider;

impl CleanupProvider for BasicCleanupProvider {
    fn cleanup(&self, request: &CleanupRequest) -> Result<CleanupResult, CleanupError> {
        let started_at = std::time::Instant::now();

        // Validate input
        let raw_trimmed = request.raw_text.trim().to_string();
        if raw_trimmed.is_empty() {
            return Err(CleanupError {
                code: "empty_raw_text".into(),
                message: "Cannot clean up empty text. Provide a transcript first.".into(),
                recoverable: true,
                details: None,
            });
        }

        // Apply mode-specific transformation
        let (final_text, warnings) = match request.mode {
            TextModeKind::RawDictation => cleanup_raw_dictation(&raw_trimmed),
            TextModeKind::CleanNotes => cleanup_clean_notes(&raw_trimmed),
            TextModeKind::AiPrompt => cleanup_ai_prompt(&raw_trimmed),
            TextModeKind::DeveloperReview => cleanup_developer_review(&raw_trimmed),
            TextModeKind::ThoughtPiece => cleanup_thought_piece(&raw_trimmed),
        };

        let duration_ms = started_at.elapsed().as_millis() as u64;
        let changed = final_text != raw_trimmed;

        let completed_at = crate::chrono_now_iso();

        Ok(CleanupResult {
            raw_text: raw_trimmed,
            final_text,
            mode: request.mode.clone(),
            provider: CleanupProviderKind::Basic,
            is_mock: false,
            changed,
            created_at: request.created_at.clone(),
            completed_at,
            duration_ms,
            warnings,
            error: None,
        })
    }

    fn name(&self) -> &str {
        "Basic Cleanup"
    }

    fn kind(&self) -> CleanupProviderKind {
        CleanupProviderKind::Basic
    }

    fn is_mock(&self) -> bool {
        false
    }
}

// ── Mode-specific cleanup functions ─────────────────────────

/// Raw Dictation: trim whitespace, preserve wording.
fn cleanup_raw_dictation(text: &str) -> (String, Vec<String>) {
    let warnings =
        vec!["Raw Dictation mode preserves your original wording. No AI cleanup applied.".into()];
    // Minimal: just trim surrounding whitespace, collapse multiple blank lines to 2
    let cleaned = text.trim().to_string();
    let cleaned = collapse_blank_lines(&cleaned, 2);
    (cleaned, warnings)
}

/// Clean Notes: normalize whitespace, basic paragraph splitting.
fn cleanup_clean_notes(text: &str) -> (String, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();

    // Split into paragraphs on double newlines
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    let cleaned_paragraphs: Vec<String> = paragraphs
        .iter()
        .map(|p| {
            // Normalize whitespace within each paragraph
            let normalized = normalize_whitespace(p);
            // Capitalize first letter of paragraph
            capitalize_first(&normalized)
        })
        .filter(|p| !p.trim().is_empty())
        .collect();

    if cleaned_paragraphs.is_empty() {
        return (
            text.trim().to_string(),
            vec!["Text was empty after cleanup.".into()],
        );
    }

    let cleaned = cleaned_paragraphs.join("\n\n");

    warnings.push(
        "Clean Notes applies basic text normalization only. No AI summarization or rewriting."
            .into(),
    );
    warnings.push(
        "Review the output for accuracy — filler words and hesitations are preserved.".into(),
    );

    (cleaned, warnings)
}

/// AI Prompt: wrap in a deterministic template.
fn cleanup_ai_prompt(text: &str) -> (String, Vec<String>) {
    let warnings = vec![
        "AI Prompt mode wraps your text in a template. No AI rewriting was performed.".into(),
        "This is a deterministic preparation step — paste the result into an AI chat manually."
            .into(),
    ];

    let final_text = format!(
        "Use the following spoken notes as context and turn them into a clear, actionable response. Preserve constraints, decisions, and open questions.\n\nSpoken notes:\n---\n{}\n---\n\nInstructions:\n- Address all explicit questions.\n- Preserve any technical constraints mentioned.\n- Highlight decisions that were made vs. still open.\n- Keep the response concise and actionable.",
        text.trim()
    );

    (final_text, warnings)
}

/// Developer Review: wrap in a structured review template.
fn cleanup_developer_review(text: &str) -> (String, Vec<String>) {
    let warnings = vec![
        "Developer Review mode adds structural headings. No technical details were invented.".into(),
        "Review each section — placeholders indicate where information was not found in the transcript.".into(),
    ];

    let final_text = format!(
        "## Issue\n{}\n\n## Why It Matters\n(Not specified — add context from the transcript if available)\n\n## Suggested Fix\n(Not specified — add technical suggestion if discussed)\n\n## Acceptance Criteria\n(Not specified — add criteria if discussed)\n\n## Raw Notes\n{}",
        text.trim(),
        text.trim()
    );

    (final_text, warnings)
}

/// Thought Piece: wrap in a structured essay template.
fn cleanup_thought_piece(text: &str) -> (String, Vec<String>) {
    let warnings = vec![
        "Thought Piece mode adds structural headings. No AI rewriting was performed.".into(),
        "This is a drafting aid — you should refine the content before publishing.".into(),
    ];

    let final_text = format!(
        "## Working Title\n(Add a title based on your notes)\n\n## Core Idea\n{}\n\n## Raw Notes\n{}\n\n## Possible Sections\n- (Add section ideas based on the core idea)\n- \n- ",
        text.trim(),
        text.trim()
    );

    (final_text, warnings)
}

// ── Text utility functions ──────────────────────────────────

/// Normalize whitespace: collapse multiple spaces/tabs, trim lines.
fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Capitalize the first character of a string (if ASCII alphabetic).
fn capitalize_first(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut chars: Vec<char> = trimmed.chars().collect();
    if chars[0].is_ascii_lowercase() {
        chars[0] = chars[0].to_ascii_uppercase();
    }
    chars.into_iter().collect()
}

/// Collapse sequences of blank lines to at most `max_blank` consecutive blank lines.
fn collapse_blank_lines(text: &str, max_blank: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut blank_count = 0;

    for line in lines {
        if line.trim().is_empty() {
            if blank_count < max_blank {
                result.push(String::new());
            }
            blank_count += 1;
        } else {
            blank_count = 0;
            result.push(line.to_string());
        }
    }

    result.join("\n")
}
