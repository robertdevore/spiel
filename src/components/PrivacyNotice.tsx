/**
 * Privacy notice displayed at the bottom of the app.
 * Communicates the current privacy posture clearly to users.
 */
export default function PrivacyNotice() {
  return (
    <section className="privacy-notice">
      <h4 className="privacy-title">🔒 Privacy</h4>
      <ul className="privacy-list">
        <li>
          <strong>Phase 1 does not record audio</strong> or send network requests.
        </li>
        <li>
          <strong>No microphone access</strong> is requested or used.
        </li>
        <li>
          <strong>No clipboard access</strong> is requested or used.
        </li>
        <li>
          <strong>No keystroke capture</strong> or global input monitoring.
        </li>
        <li>
          Future transcription and cleanup features will be{" "}
          <strong>explicit and user-controlled</strong>.
        </li>
        <li>
          No data leaves your device unless you explicitly configure a cloud service.
        </li>
      </ul>
      <p className="privacy-dev-note">
        🛠 Development note: This build is a foundation only. Real recording, transcription,
        and text insertion are planned for future phases.
      </p>
    </section>
  );
}
