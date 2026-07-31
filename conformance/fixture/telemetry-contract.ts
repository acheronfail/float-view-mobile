export const TELEMETRY_LIMITS = {
  layoutFields: 64,
  fieldNameUtf8Bytes: 64,
  aggregateFieldNameUtf8Bytes: 4_096,
  extensionFields: 32,
  jsonUtf8Bytes: 16_384,
  messageBytes: 65_536,
  scenarioDocumentUtf8Bytes: 65_536,
  diagnostics: 50,
  seriesPoints: 2_000,
  transitionEvents: 2_000,
} as const;

export function validateExtensions(value: Readonly<Record<string, number>>): void {
  const entries = Object.entries(value);
  if (entries.length > TELEMETRY_LIMITS.extensionFields) {
    throw new Error('Telemetry extension map exceeds 32 entries');
  }
  for (const [key, number] of entries) {
    const keyBytes = new TextEncoder().encode(key).length;
    if (keyBytes === 0 || keyBytes > TELEMETRY_LIMITS.fieldNameUtf8Bytes) {
      throw new Error('Telemetry extension key exceeds its UTF-8 bound');
    }
    if (!Number.isFinite(number)) throw new Error('Telemetry extension value must be finite');
  }
  validateJson(value, 'Telemetry extension JSON');
}

export function validateJson(value: unknown, label = 'Telemetry JSON'): string {
  const json = JSON.stringify(value);
  if (new TextEncoder().encode(json).length > TELEMETRY_LIMITS.jsonUtf8Bytes) {
    throw new Error(`${label} exceeds its UTF-8 bound`);
  }
  return json;
}

export function validateMessage(value: string | Uint8Array): void {
  const size = typeof value === 'string' ? new TextEncoder().encode(value).length : value.byteLength;
  if (size > TELEMETRY_LIMITS.messageBytes) throw new Error('Message exceeds 65,536 bytes');
}

export function validateScenarioDocument(value: string): void {
  if (new TextEncoder().encode(value).length > TELEMETRY_LIMITS.scenarioDocumentUtf8Bytes) {
    throw new Error('Scenario document exceeds 65,536 UTF-8 bytes');
  }
}

export function recentBounded<T>(values: readonly T[], limit: number): { values: T[]; truncated: boolean } {
  if (!Number.isInteger(limit) || limit < 0) throw new Error('Bound must be a non-negative integer');
  return {
    values: values.slice(-limit),
    truncated: values.length > limit,
  };
}
