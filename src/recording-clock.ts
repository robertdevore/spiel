import type { StatusSnapshot } from "./contracts";

export class RecordingClock {
  private anchorMs: number | null = null;
  private baseElapsedMs = 0;

  sync(status: StatusSnapshot | null) {
    this.anchorMs = status?.phase === "recording" ? Date.now() : null;
    this.baseElapsedMs = status?.phase === "recording" ? status.recording_elapsed_ms : 0;
  }

  elapsed(status: StatusSnapshot): number {
    if (status.phase !== "recording") return 0;
    if (this.anchorMs === null) return status.recording_elapsed_ms;
    return this.baseElapsedMs + Math.max(0, Date.now() - this.anchorMs);
  }
}
