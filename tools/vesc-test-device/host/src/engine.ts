import { fromHex, toHex } from "./bytes.js";
import { loadFixture } from "./fixtures.js";
import { matchRequest } from "./protocol.js";
import type {
  EngineAction,
  Scenario,
  ScenarioRule,
  TranscriptEntry,
  VerificationManifest,
} from "./types.js";
import { encodeFrame, VescStreamAssembler } from "./vesc.js";

export interface EngineClock {
  monotonicMilliseconds(): number;
}

export class ScenarioEngine {
  private readonly assembler: VescStreamAssembler;
  private readonly counts: Record<string, number>;
  private readonly transcript: TranscriptEntry[] = [];
  private transcriptTruncated = false;
  private firstViolation: string | null = null;
  private nextExpectedIndex = 0;

  constructor(
    readonly scenario: Scenario,
    private readonly clock: EngineClock = { monotonicMilliseconds: () => performance.now() },
  ) {
    this.assembler = new VescStreamAssembler(scenario.limits.maxBufferedBytes);
    this.counts = Object.fromEntries(scenario.rules.map((rule) => [rule.id, 0]));
  }

  async receiveGattWrite(chunk: Uint8Array): Promise<EngineAction[]> {
    const actions: EngineAction[] = [];
    for (const result of this.assembler.append(chunk)) {
      if (result.kind === "fault") {
        this.violate(`stream ${result.code}: ${result.message}`);
        this.record("internal", "streamFault", "fault", undefined, undefined, undefined, result.message);
        continue;
      }
      const request = matchRequest(result.payload);
      if (request.disposition !== "readOnly") {
        const detail = `${request.disposition} command ${request.commandId} rejected`;
        this.violate(detail);
        this.record("clientToFixture", request.disposition, "rejected", undefined, request.canId, toHex(result.frame), detail);
        continue;
      }
      const rule = this.findRule(request.kind, request.canId);
      if (!rule) {
        const detail = `no rule for ${request.kind}${request.canId === undefined ? "" : ` on CAN ${request.canId}`}`;
        this.violate(detail);
        this.record("clientToFixture", request.kind, "rejected", undefined, request.canId, toHex(result.frame), detail);
        continue;
      }
      this.counts[rule.id] = (this.counts[rule.id] ?? 0) + 1;
      const count = this.counts[rule.id]!;
      if (count > rule.occurrences.maximum) this.violate(`rule ${rule.id} exceeded maximum occurrence ${rule.occurrences.maximum}`);
      this.verifyOrder(rule.id);
      this.record("clientToFixture", request.kind, "accepted", rule.id, request.canId, toHex(result.frame));
      actions.push(await this.responseAction(rule));
    }
    return actions;
  }

  recordNotificationSent(ruleId: string, chunk: Uint8Array): void {
    this.record("fixtureToClient", "notification", "sent", ruleId, undefined, toHex(chunk));
  }

  verify(): VerificationManifest {
    for (const rule of this.scenario.rules) {
      const count = this.counts[rule.id] ?? 0;
      if (count < rule.occurrences.minimum) {
        this.violate(`rule ${rule.id} expected at least ${rule.occurrences.minimum} occurrence(s), observed ${count}`);
        break;
      }
    }
    if (this.nextExpectedIndex < this.scenario.expectations.orderedRuleIds.length) {
      this.violate(`expected ordered rule ${this.scenario.expectations.orderedRuleIds[this.nextExpectedIndex]} was not observed`);
    }
    return {
      schemaVersion: 1,
      scenarioId: this.scenario.id,
      passed: this.firstViolation === null,
      firstViolation: this.firstViolation,
      ruleCounts: { ...this.counts },
      transcriptEntries: this.transcript.length,
      transcriptTruncated: this.transcriptTruncated,
    };
  }

  transcriptSnapshot(): readonly TranscriptEntry[] {
    return this.transcript;
  }

  private findRule(kind: string, canId: number | undefined): ScenarioRule | undefined {
    return this.scenario.rules.find(
      (rule) => rule.match.kind === kind && rule.match.canId === canId,
    );
  }

  private verifyOrder(ruleId: string): void {
    const expected = this.scenario.expectations.orderedRuleIds[this.nextExpectedIndex];
    if (expected === ruleId) {
      this.nextExpectedIndex += 1;
      return;
    }
    if (this.scenario.expectations.orderedRuleIds.slice(0, this.nextExpectedIndex).includes(ruleId)) {
      return;
    }
    this.violate(`expected ordered rule ${expected ?? "<none>"} before ${ruleId}`);
  }

  private async responseAction(rule: ScenarioRule): Promise<EngineAction> {
    let frame: Uint8Array;
    let fixtureSizes: number[] | undefined;
    if ("fixture" in rule.response) {
      const loaded = await loadFixture(rule.response.fixture, rule.response.case);
      frame = loaded.frame;
      fixtureSizes = loaded.chunkSizes;
    } else {
      frame = encodeFrame(fromHex(rule.response.payloadHex));
    }
    const chunking = rule.response.chunking;
    let sizes: number[];
    if (chunking.mode === "single") sizes = [frame.length];
    else if (chunking.mode === "fixture") {
      if (!fixtureSizes) throw new Error(`rule ${rule.id} requests fixture chunking but fixture has no chunkSizes`);
      sizes = fixtureSizes;
    } else sizes = chunking.sizes;
    const chunks = splitExact(frame, sizes, rule.id);
    return {
      kind: "notify",
      ruleId: rule.id,
      chunks,
      interChunkDelayMs: chunking.mode === "sizes" ? chunking.interChunkDelayMs ?? 0 : 0,
    };
  }

  private violate(message: string): void {
    this.firstViolation ??= message;
  }

  private record(
    direction: TranscriptEntry["direction"],
    kind: string,
    outcome: TranscriptEntry["outcome"],
    ruleId?: string,
    canId?: number,
    hex?: string,
    detail?: string,
  ): void {
    if (this.transcript.length >= this.scenario.limits.maxTranscriptEntries) {
      this.transcriptTruncated = true;
      return;
    }
    const base = {
      index: this.transcript.length,
      monotonicMs: this.clock.monotonicMilliseconds(),
      direction,
      kind,
      outcome,
    };
    this.transcript.push({
      ...base,
      ...(ruleId === undefined ? {} : { ruleId }),
      ...(canId === undefined ? {} : { canId }),
      ...(hex === undefined ? {} : { hex }),
      ...(detail === undefined ? {} : { detail }),
    });
  }
}

function splitExact(frame: Uint8Array, sizes: number[], ruleId: string): Uint8Array[] {
  if (sizes.some((size) => !Number.isInteger(size) || size <= 0)) {
    throw new Error(`rule ${ruleId} has invalid chunk sizes`);
  }
  if (sizes.reduce((sum, size) => sum + size, 0) !== frame.length) {
    throw new Error(`rule ${ruleId} chunk sizes do not total frame length ${frame.length}`);
  }
  let offset = 0;
  return sizes.map((size) => {
    const result = frame.slice(offset, offset + size);
    offset += size;
    return result;
  });
}
