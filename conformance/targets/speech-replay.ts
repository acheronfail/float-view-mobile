import { createInterface } from 'node:readline';
import type { RefloatVersion } from '../fixture/refloat.ts';
import type { TelemetrySource } from '../fixture/telemetry.ts';

export interface SpeechReplayOptions {
  telemetry: TelemetrySource;
  refloatVersion: RefloatVersion;
  onSpeech(phrase: string): void;
}

/** Only opt-in software hosts emit these closed, bounded speech observations. */
export function observeSpeechReplay(stream: NodeJS.ReadableStream, marker: string, onSpeech: (phrase: string) => void) {
  const lines = createInterface({ input: stream });
  lines.on('line', (line) => {
    const offset = line.indexOf(marker);
    if (offset < 0 || line.length > 1024) return;
    const value = JSON.parse(line.slice(offset + marker.length)) as { phrase?: unknown };
    if (value.phrase === 'Battery low' || value.phrase === 'Over voltage') onSpeech(value.phrase);
  });
  return () => lines.close();
}
