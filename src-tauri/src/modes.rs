//! Text mode definitions for Spiel Phase 7.
//!
//! Defines the five text processing modes that transform a raw transcript
//! into useful final text. Each mode has a deterministic behavior in Phase 7.
//! Future phases may add AI-powered cleanup for some modes.
//!
//! All modes are implemented in Phase 7 using deterministic rules and templates.
//! No mode invents content or pretends AI rewrote the text.

use serde::{Deserialize, Serialize};

/// Kinds of text processing modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextModeKind {
    /// Raw dictation — minimal cleanup, preserve wording
    RawDictation,
    /// Clean notes — readable notes with basic cleanup
    CleanNotes,
    /// AI Prompt — deterministic template wrapper
    AiPrompt,
    /// Developer Review — structured review template
    DeveloperReview,
    /// Thought Piece — structured essay template
    ThoughtPiece,
}

impl TextModeKind {
    /// Human-readable label for this mode.
    pub fn label(&self) -> &str {
        match self {
            Self::RawDictation => "Raw Dictation",
            Self::CleanNotes => "Clean Notes",
            Self::AiPrompt => "AI Prompt",
            Self::DeveloperReview => "Developer Review",
            Self::ThoughtPiece => "Thought Piece",
        }
    }

    /// Short description of what this mode does.
    pub fn description(&self) -> &str {
        match self {
            Self::RawDictation => {
                "Use the transcript with minimal cleanup. Preserves wording — no summarization or rewriting."
            }
            Self::CleanNotes => {
                "Turn spoken thoughts into readable notes. Basic punctuation/spacing cleanup, normalize whitespace, split into paragraphs."
            }
            Self::AiPrompt => {
                "Wrap transcript as a clear prompt for an AI assistant. Deterministic template — no AI rewrite."
            }
            Self::DeveloperReview => {
                "Prepare spoken engineering feedback as structured review notes with headings."
            }
            Self::ThoughtPiece => {
                "Prepare longer spoken thoughts for essay, memo, or article drafting with structure."
            }
        }
    }

    /// Whether this mode is implemented in the current phase.
    pub fn is_implemented(&self) -> bool {
        true // All five modes are implemented in Phase 7 (deterministic)
    }

    /// All available modes.
    pub fn all_modes() -> Vec<TextModeKind> {
        vec![
            Self::RawDictation,
            Self::CleanNotes,
            Self::AiPrompt,
            Self::DeveloperReview,
            Self::ThoughtPiece,
        ]
    }
}

/// Display-friendly mode definition for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeDefinition {
    pub kind: TextModeKind,
    pub label: String,
    pub description: String,
    pub implemented: bool,
}

impl ModeDefinition {
    /// Create a definition from a mode kind.
    pub fn from_kind(kind: &TextModeKind) -> Self {
        Self {
            kind: kind.clone(),
            label: kind.label().into(),
            description: kind.description().into(),
            implemented: kind.is_implemented(),
        }
    }

    /// All mode definitions for the frontend.
    pub fn all_definitions() -> Vec<ModeDefinition> {
        TextModeKind::all_modes()
            .iter()
            .map(Self::from_kind)
            .collect()
    }
}
