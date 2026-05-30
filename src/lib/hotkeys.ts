import type { HotkeyBehavior } from "./types";

/**
 * Default hotkey configuration for Spiel Phase 2.
 *
 * Cross-platform default: Ctrl+Shift+Space
 * This works on macOS, Windows, and Linux.
 *
 * macOS note: Option+Space is often reserved by Spotlight/Alfred.
 * Users can customize this in a future settings phase.
 */

/** The default shortcut string (cross-platform) */
export const DEFAULT_HOTKEY = "Ctrl+Shift+Space";

/** Human-readable shortcut label for display */
export const DEFAULT_HOTKEY_LABEL = "Ctrl+Shift+Space";

/** Default hotkey behavior (toggle: press once to start, press again to stop) */
export const DEFAULT_HOTKEY_BEHAVIOR: HotkeyBehavior = "toggle";

/**
 * Returns a human-readable label for the current platform's default shortcut.
 * Currently returns the cross-platform default; will be extended for
 * platform-specific customization in a future settings phase.
 */
export function getDefaultHotkeyLabel(): string {
  return DEFAULT_HOTKEY_LABEL;
}

/**
 * Normalizes a shortcut string for display.
 * Converts modifier names to platform-appropriate symbols where possible.
 */
export function normalizeShortcutLabel(shortcut: string): string {
  return shortcut
    .replace(/Control/i, "Ctrl")
    .replace(/Command/i, "⌘")
    .replace(/Option/i, "⌥")
    .replace(/Alt/i, "Alt")
    .replace(/Shift/i, "⇧")
    .replace(/\+/g, " + ");
}

/**
 * Formats the last-triggered ISO 8601 timestamp into a readable local time string.
 * Returns "—" if null.
 */
export function formatLastTriggered(isoTimestamp: string | null): string {
  if (!isoTimestamp) return "—";
  try {
    const date = new Date(isoTimestamp);
    return date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return isoTimestamp;
  }
}

/**
 * Formats trigger count for display.
 */
export function formatTriggerCount(count: number): string {
  if (count === 0) return "No triggers yet";
  if (count === 1) return "Triggered 1 time";
  return `Triggered ${count} times`;
}
