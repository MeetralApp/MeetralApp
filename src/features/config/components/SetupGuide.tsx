export function GuideListWindows({ compact = false }: { compact?: boolean }) {
  return (
    <div className="space-y-3 text-sm leading-relaxed">
      {!compact && (
        <p className="m-0 text-muted-foreground">
          Example using VoiceMeeter Banana — any router works if you map the four
          audio roles correctly.
        </p>
      )}
      {compact && (
        <p className="m-0">
          <strong>Teams:</strong> Speaker → Voicemeeter Input (VAIO); Microphone
          → Voicemeeter Out B1
        </p>
      )}
      <ol className="m-0 list-decimal space-y-2 pl-5">
        <li>Install and run VoiceMeeter Banana.</li>
        <li>
          In Teams: Speaker → virtual input for meeting audio (e.g. VAIO Input).
        </li>
        <li>
          In Teams: Microphone → virtual output for translated audio (e.g. Out
          B1).
        </li>
        <li>
          Map roles above:
          <ul className="mt-2 list-disc space-y-1 pl-5">
            <li>
              <strong>What you hear from the meeting</strong> → Out B2
            </li>
            <li>
              <strong>What the meeting hears</strong> → AUX Input
            </li>
            <li>
              <strong>Where translation plays</strong> → your headphones
            </li>
            <li>
              <strong>What you speak into</strong> → headset mic (or system
              default)
            </li>
          </ul>
        </li>
        <li>
          Save audio. Use <strong>Direct</strong> for normal meeting audio;
          switch to <strong>Translate</strong> when you need interpretation.
        </li>
      </ol>
    </div>
  );
}

export function GuideListMacos({ compact = false }: { compact?: boolean }) {
  return (
    <div className="space-y-3 text-sm leading-relaxed">
      {!compact && (
        <p className="m-0 text-muted-foreground">
          Recommended: install <strong>BlackHole 2ch</strong> and{" "}
          <strong>BlackHole 16ch</strong> from{" "}
          <a
            href="https://existential.audio/blackhole/"
            target="_blank"
            rel="noreferrer"
            className="text-primary underline-offset-4 hover:underline"
          >
            existential.audio/blackhole
          </a>
          . Two drivers let meeting audio and your mic feed use separate virtual
          devices (same idea as VoiceMeeter buses on Windows).
        </p>
      )}
      {compact && (
        <p className="m-0">
          <strong>Teams:</strong> Speaker → BlackHole 16ch; Microphone → BlackHole
          2ch
        </p>
      )}
      <ol className="m-0 list-decimal space-y-2 pl-5">
        <li>
          Install BlackHole 2ch and BlackHole 16ch; reboot if prompted. In{" "}
          <strong>Audio MIDI Setup</strong>, set both to <strong>48 kHz</strong>{" "}
          (2 ch or 16 ch is fine).
        </li>
        <li>In Teams: set <strong>Speaker</strong> to BlackHole 16ch.</li>
        <li>In Teams: set <strong>Microphone</strong> to BlackHole 2ch.</li>
        <li>
          In Settings → Audio devices, map the four roles (or use{" "}
          <strong>Auto-fill BlackHole</strong>):
          <ul className="mt-2 list-disc space-y-1 pl-5">
            <li>
              <strong>What you hear from the meeting</strong> → BlackHole 16ch
              (input)
            </li>
            <li>
              <strong>What the meeting hears</strong> → BlackHole 2ch (output)
            </li>
            <li>
              <strong>Where translation plays</strong> → MacBook speakers or
              headphones
            </li>
            <li>
              <strong>What you speak into</strong> → built-in or headset mic (or
              system default)
            </li>
          </ul>
        </li>
        <li>
          Save audio. Keep <strong>Meeting</strong> on <strong>Direct</strong> to
          hear the call through local playback; switch to{" "}
          <strong>Translate</strong> when you need interpretation.
        </li>
      </ol>
      {!compact && (
        <p className="m-0 text-muted-foreground">
          Only BlackHole 2ch installed? You can use it for all roles, but
          meeting capture and Teams mic feed share one device — prefer installing
          both drivers when possible.
        </p>
      )}
    </div>
  );
}
