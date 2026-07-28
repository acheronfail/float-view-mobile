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
  response:
    | { fixture: string; case?: string; chunking: Chunking }
    | { payloadHex: string; provenance: string; chunking: Chunking };
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
  canId?: number;
  hex?: string;
  outcome: "accepted" | "rejected" | "sent" | "fault";
  detail?: string;
}

export interface EngineAction {
  kind: "notify";
  ruleId: string;
  chunks: Uint8Array[];
  interChunkDelayMs: number;
}

export interface VerificationManifest {
  schemaVersion: 1;
  scenarioId: string;
  passed: boolean;
  firstViolation: string | null;
  ruleCounts: Record<string, number>;
  transcriptEntries: number;
  transcriptTruncated: boolean;
}
