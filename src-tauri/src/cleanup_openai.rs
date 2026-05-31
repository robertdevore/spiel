//! OpenAI cleanup provider for Spiel Phase 12.
//!
//! Sends transcript text to OpenAI chat completions for AI-powered cleanup.
//! Requires: local-only mode disabled, cloud providers enabled, API key configured.
//! Uses mode-specific system prompts to transform raw dictation into polished text.
//!
//! Architecture:
//! - Implements `CleanupProvider` trait
//! - Sends JSON to POST https://api.openai.com/v1/chat/completions
//! - Uses "gpt-4o-mini" model (configurable via settings)
//! - Validates prerequisites before any network call
//! - No automatic retry, no fallback to basic cleanup

use crate::cleanup::{
    CleanupError, CleanupProvider, CleanupProviderKind, CleanupRequest, CleanupResult,
};
use crate::modes::TextModeKind;
use std::time::Instant;

/// Default OpenAI cleanup model (fast, cost-effective).
const DEFAULT_OPENAI_CLEANUP_MODEL: &str = "gpt-4o-mini";
/// OpenAI chat completions API endpoint.
const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// System prompt prefix applied to all modes.
const OPENAI_SYSTEM_PREFIX: &str = "\
You are a text cleanup assistant for Spiel, a dictation app. \
Your job is to transform raw spoken dictation into polished text. \
Preserve the user's meaning. Do not invent facts. Do not add unsupported claims. \
Do not remove important constraints. Keep the user's voice where practical. \
Return only the final transformed text. Do not include explanations or metadata.";

/// OpenAI-powered cleanup provider.
/// Requires explicit opt-in: local-only mode off, API key configured.
pub struct OpenAiCleanupProvider {
    api_key: String,
    model: String,
}

impl OpenAiCleanupProvider {
    /// Create a new OpenAI cleanup provider.
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_OPENAI_CLEANUP_MODEL.to_string()),
        }
    }

    /// Build the mode-specific system prompt.
    fn mode_prompt(mode: TextModeKind) -> &'static str {
        match mode {
            TextModeKind::RawDictation => {
                "\
Mode: Raw Dictation. \
Apply light punctuation and paragraph cleanup. \
Preserve wording as much as possible. \
Do not summarize. \
Fix obvious typos and add periods/commas where needed."
            }
            TextModeKind::CleanNotes => {
                "\
Mode: Clean Notes. \
Make spoken notes readable and organized. \
Remove filler words (um, uh, like, you know, etc.). \
Split into paragraphs or bullet points if helpful. \
Preserve all details. Do not over-summarize."
            }
            TextModeKind::AiPrompt => {
                "\
Mode: AI Prompt. \
Turn spoken notes into a clear, well-structured prompt for an AI assistant or coding agent. \
Preserve requirements, constraints, decisions, and open questions. \
Do not invent missing requirements. \
Format for clarity and actionability."
            }
            TextModeKind::DeveloperReview => {
                "\
Mode: Developer Review. \
Turn spoken engineering feedback into structured review notes. \
Use this format for each issue:\n\
## Issue\n\
(describe the problem)\n\
## Why it matters\n\
(explain the impact)\n\
## Suggested fix\n\
(propose a solution)\n\
## Acceptance criteria\n\
(define what done looks like)\n\
Use 'Not specified' when details are missing."
            }
            TextModeKind::ThoughtPiece => {
                "\
Mode: Thought Piece. \
Turn long spoken thoughts into a structured draft or outline. \
Preserve the user's voice and perspective. \
Improve clarity and organization. \
Do not introduce unsupported facts or arguments. \
Structure with clear sections and logical flow."
            }
        }
    }

    /// Build the HTTP client.
    fn build_client() -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))
    }
}

impl CleanupProvider for OpenAiCleanupProvider {
    fn name(&self) -> &str {
        "OpenAI Cleanup"
    }

    fn kind(&self) -> CleanupProviderKind {
        CleanupProviderKind::OpenAi
    }

    fn is_mock(&self) -> bool {
        false
    }

    fn cleanup(&self, request: &CleanupRequest) -> Result<CleanupResult, CleanupError> {
        let started_at = Instant::now();
        let mode = request.mode.clone(); // Clone from &CleanupRequest (TextModeKind: Clone not Copy)

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

        // Build the full system prompt
        let system_content = format!(
            "{}\n\n{}",
            OPENAI_SYSTEM_PREFIX,
            Self::mode_prompt(mode.clone())
        );

        // Build request body
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_content
                },
                {
                    "role": "user",
                    "content": raw_trimmed
                }
            ],
            "temperature": 0.3,
            "max_tokens": 4096
        });

        // Send request
        let client = Self::build_client().map_err(|e| CleanupError {
            code: "http_client_error".into(),
            message: e,
            recoverable: true,
            details: None,
        })?;

        let response = client
            .post(OPENAI_CHAT_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| {
                let msg = if e.is_timeout() {
                    "OpenAI request timed out after 120 seconds. Check your network connection."
                        .into()
                } else if e.is_connect() {
                    "Cannot connect to OpenAI API. Check your internet connection.".into()
                } else {
                    format!("OpenAI request failed: {}", e)
                };
                CleanupError {
                    code: "network_error".into(),
                    message: msg,
                    recoverable: true,
                    details: None,
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().unwrap_or_else(|_| "Unknown error".into());

            let (code, message) = if status.as_u16() == 401 {
                (
                    "invalid_api_key",
                    "OpenAI API key is invalid or expired. Check your API key in Settings.",
                )
            } else if status.as_u16() == 429 {
                (
                    "rate_limited",
                    "OpenAI rate limit reached. Wait a moment and try again.",
                )
            } else {
                ("openai_error", "")
            };

            return Err(CleanupError {
                code: code.into(),
                message: if message.is_empty() {
                    format!("OpenAI returned error {}: {}", status.as_u16(), error_body)
                } else {
                    message.into()
                },
                recoverable: true,
                details: Some(error_body),
            });
        }

        // Parse response
        let response_body: serde_json::Value = response.json().map_err(|e| CleanupError {
            code: "parse_error".into(),
            message: format!("Failed to parse OpenAI response: {}", e),
            recoverable: true,
            details: None,
        })?;

        let final_text = response_body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        if final_text.is_empty() {
            return Err(CleanupError {
                code: "empty_response".into(),
                message: "OpenAI returned an empty response. Try again or use Basic cleanup."
                    .into(),
                recoverable: true,
                details: Some(
                    "The model produced no output text. The input may need adjustment.".into(),
                ),
            });
        }

        let duration_ms = started_at.elapsed().as_millis() as u64;
        let changed = final_text != raw_trimmed;
        let created_at = request.created_at.clone();
        let completed_at = crate::chrono_now_iso();
        let mut warnings: Vec<String> = Vec::new();

        // Add cloud usage warning
        warnings.push("Text was sent to OpenAI (api.openai.com) for cloud AI cleanup.".into());

        if !changed {
            warnings.push(
                "OpenAI returned text identical to the input. No cleanup was applied.".into(),
            );
        }

        Ok(CleanupResult {
            raw_text: raw_trimmed,
            final_text,
            provider: CleanupProviderKind::OpenAi,
            mode,
            duration_ms,
            changed,
            is_mock: false,
            created_at,
            completed_at,
            warnings,
            error: None,
        })
    }
}
