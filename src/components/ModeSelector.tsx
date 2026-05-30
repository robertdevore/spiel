import type { TextMode, TextModeKind, ModeDefinition } from "../lib/types";

interface ModeSelectorProps {
  selectedMode: TextMode;
  onModeChange: (mode: TextMode) => void;
  modes: ModeDefinition[];
}

/**
 * Default mode definitions used when backend data hasn't loaded yet.
 */
function getDefaultModes(): ModeDefinition[] {
  return [
    {
      kind: "raw_dictation" as TextModeKind,
      label: "Raw Dictation",
      description: "Use the transcript with minimal cleanup. Preserves wording — no summarization or rewriting.",
      implemented: true,
    },
    {
      kind: "clean_notes" as TextModeKind,
      label: "Clean Notes",
      description: "Turn spoken thoughts into readable notes. Basic punctuation/spacing cleanup, normalize whitespace, split into paragraphs.",
      implemented: true,
    },
    {
      kind: "ai_prompt" as TextModeKind,
      label: "AI Prompt",
      description: "Wrap transcript as a clear prompt for an AI assistant. Deterministic template — no AI rewrite.",
      implemented: true,
    },
    {
      kind: "developer_review" as TextModeKind,
      label: "Developer Review",
      description: "Prepare spoken engineering feedback as structured review notes with headings.",
      implemented: true,
    },
    {
      kind: "thought_piece" as TextModeKind,
      label: "Thought Piece",
      description: "Prepare longer spoken thoughts for essay, memo, or article drafting with structure.",
      implemented: true,
    },
  ];
}

/**
 * Mode selector showing available text processing modes.
 * In Phase 7, all five modes are implemented with deterministic behavior.
 * AI-powered cleanup is planned for future phases.
 */
export default function ModeSelector({ selectedMode, onModeChange, modes }: ModeSelectorProps) {
  const displayModes = modes.length > 0 ? modes : getDefaultModes();

  return (
    <section className="mode-selector">
      <h3 className="section-title">Text Mode</h3>
      <p className="section-hint">
        Choose how your transcript will be processed. All modes use deterministic rules — AI cleanup is planned for future phases.
      </p>
      <div className="mode-list">
        {displayModes.map((mode) => (
          <label
            key={mode.kind}
            className={`mode-option ${selectedMode === mode.kind ? "selected" : ""} ${!mode.implemented ? "is-planned" : ""}`}
          >
            <input
              type="radio"
              name="text-mode"
              value={mode.kind}
              checked={selectedMode === mode.kind}
              onChange={() => onModeChange(mode.kind as TextMode)}
              disabled={!mode.implemented}
            />
            <div className="mode-info">
              <span className="mode-label">
                {mode.label}
                {!mode.implemented && (
                  <span className="badge-planned">planned</span>
                )}
                {mode.implemented && (
                  <span className="badge-implemented">available</span>
                )}
              </span>
              <span className="mode-desc">{mode.description}</span>
            </div>
          </label>
        ))}
      </div>
    </section>
  );
}
