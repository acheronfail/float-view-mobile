import type { RequestKind } from "./protocol.js";

export interface Scenario {
  schemaVersion: 1;
  id: string;
  description: string;
  advertisement: {
    name: string;
    manufacturerMarkerHex: string;
    profile: "canonicalVescUart";
  };
  topology: {
    transport: { hardwareName: string; firmware: string };
    canNodes: Array<{ canId: number; role: string; hardwareName: string; firmware: string }>;
    primaryCanId: number;
    refloat: { packageVersion: string };
  };
  rules: ScenarioRule[];
  expectations: {
    orderedRuleIds: string[];
    cadence?: CadenceExpectation[];
  };
  limits: {
    maxTranscriptEntries: number;
    maxBufferedBytes: number;
    commandTimeoutMs: number;
  };
}

export interface ScenarioRule {
  id: string;
  match: { kind: RequestKind; canId?: number };
  occurrences: { minimum: number; maximum: number };
  response?: ResponseSpec;
  responses?: OccurrenceResponse[];
}

export type ResponseSpec =
  | { fixture: string; case?: string; chunking: Chunking }
  | { payloadHex: string; provenance: string; chunking: Chunking };

export interface OccurrenceResponse {
  fromOccurrence: number;
  toOccurrence?: number;
  phase: string;
  outcome: ResponseSpec | { action: "disconnect" };
}

export interface CadenceExpectation {
  ruleId: string;
  fromOccurrence: number;
  toOccurrence: number;
  intervalMs: number;
  toleranceMs: number;
  phase: string;
}

export type Chunking =
  | { mode: "fixture" }
  | { mode: "single" }
  | { mode: "sizes"; sizes: number[]; interChunkDelayMs?: number };

export interface TranscriptEntry {
  index: number;
  monotonicMs: number;
  direction: "clientToFixture" | "fixtureToClient" | "internal";
  kind: string;
  ruleId?: string;
  occurrence?: number;
  phase?: string;
  canId?: number;
  hex?: string;
  outcome: "accepted" | "rejected" | "sent" | "fault";
  detail?: string;
}

export type EngineAction =
  | {
      kind: "notify";
      ruleId: string;
      phase?: string;
      chunks: Uint8Array[];
      interChunkDelayMs: number;
    }
  | {
      kind: "disconnect";
      ruleId: string;
      phase: string;
    };

export interface CadenceResult {
  ruleId: string;
  phase: string;
  fromOccurrence: number;
  toOccurrence: number;
  expectedIntervalMs: number;
  toleranceMs: number;
  minimumObservedMs: number;
  maximumObservedMs: number;
  averageObservedMs: number;
  passed: boolean;
}

export interface VerificationManifest {
  schemaVersion: 1;
  scenarioId: string;
  passed: boolean;
  firstViolation: string | null;
  ruleCounts: Record<string, number>;
  transcriptEntries: number;
  transcriptTruncated: boolean;
  cadence?: CadenceResult[];
}
