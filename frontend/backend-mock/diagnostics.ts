import type {
  DiagnosticIncident,
  DiagnosticPreview,
  DiagnosticUiError,
  Response,
} from '../src/lib/backend/contract.ts';

/** Synthetic OS presentation in the mock; production ZIP bytes are owned by Rust. */
export class MockDiagnostics {
  private next = 0;
  private lastManual: number | null = null;
  private incidents: (DiagnosticIncident & { created: number })[] = [];
  private reports = new Map<string, { owner: number; expires: number; preview: DiagnosticPreview }>();
  private uiLimits = new Map<number, { start: number; count: number }>();
  private total = { start: 0, count: 0 };
  private readonly now: () => number;
  constructor(now = () => Date.now()) {
    this.now = now;
  }
  disconnect(owner: number) {
    for (const [id, report] of this.reports) if (report.owner === owner) this.reports.delete(id);
    this.uiLimits.delete(owner);
  }
  recordRequestFailure() {
    const now = this.now();
    if (!this.incidents.length || now - this.incidents.at(-1)!.created >= 60_000) {
      this.incidents.push({
        incidentId: String(++this.next),
        reason: 'backend_request_failure',
        complete: false,
        clipped: false,
        ageMs: 0,
        created: now,
      });
      this.incidents = this.incidents.slice(-3);
    }
  }
  handle(id: number, method: string, params: Record<string, unknown>, owner: number): Response {
    const ok = (result: unknown): Response => ({ id, result });
    const fail = (code: 'invalid_request' | 'not_found' | 'unavailable'): Response => ({
      id,
      error: { code, message: 'Diagnostic request could not be completed' },
    });
    const only = (...keys: string[]) => Object.keys(params).every((k) => keys.includes(k));
    const now = this.now();
    this.incidents = this.incidents.filter((i) => now - i.created < 7 * 86_400_000);
    for (const [key, r] of this.reports) if (now >= r.expires) this.reports.delete(key);
    if (method === 'diagnostics.list') {
      if (!only()) return fail('invalid_request');
      return ok({
        persistence: 'available',
        droppedRecords: 0,
        incidents: [...this.incidents]
          .reverse()
          .map(({ created, ...i }) => ({ ...i, ageMs: now - created, complete: now - created >= 10_000 })),
      });
    }
    if (method === 'diagnostics.capture') {
      if (!only()) return fail('invalid_request');
      if (this.lastManual !== null && now - this.lastManual < 10_000) return fail('unavailable');
      this.lastManual = now;
      const incidentId = String(++this.next);
      this.incidents.push({ incidentId, reason: 'manual', complete: false, clipped: false, ageMs: 0, created: now });
      this.incidents = this.incidents.slice(-3);
      return ok({ incidentId, capturing: true });
    }
    if (method === 'diagnostics.prepare') {
      if (
        !only('incidentId', 'replaceReportId') ||
        !('incidentId' in params) ||
        (params.incidentId !== null &&
          (typeof params.incidentId !== 'string' || !/^[0-9]{1,20}$/.test(params.incidentId)))
      )
        return fail('invalid_request');
      const incident =
        params.incidentId === null ? null : this.incidents.find((i) => i.incidentId === params.incidentId);
      if (params.incidentId !== null && !incident) return fail('not_found');
      if ('replaceReportId' in params) {
        if (typeof params.replaceReportId !== 'string' || !/^[0-9]{1,20}$/.test(params.replaceReportId))
          return fail('invalid_request');
        if (this.reports.get(params.replaceReportId)?.owner !== owner) return fail('not_found');
        this.reports.delete(params.replaceReportId);
      }
      if (this.reports.size >= 2) return fail('unavailable');
      const reportId = String(++this.next);
      const complete = !incident || now - incident.created >= 10_000;
      const preview = {
        reportId,
        summary: `Float View diagnostic report ${reportId}\nBuild: mock\nReason: ${incident?.reason ?? 'manual'}\nComplete capture: ${complete}\nController/motor/battery: unknown\n\nIncludes structured connection, protocol, lifecycle and timing facts.\nExcludes routes, device identifiers, raw packets, configurations and telemetry values.\n`,
        complete,
        clipped: false,
        droppedRecords: 0,
        expiresInMs: 600_000,
      };
      this.reports.set(reportId, { owner, expires: now + 600_000, preview });
      return ok(structuredClone(preview));
    }
    if (method === 'diagnostics.export') {
      if (!only('reportId') || typeof params.reportId !== 'string' || !/^[0-9]{1,20}$/.test(params.reportId))
        return fail('invalid_request');
      const report = this.reports.get(params.reportId);
      if (!report || report.owner !== owner) return fail('not_found');
      return ok({ fileName: `float-view-diagnostics-${params.reportId}.zip`, delivery: 'presented' });
    }
    if (method === 'diagnostics.clear') {
      if (!only()) return fail('invalid_request');
      this.incidents = [];
      this.reports.clear();
      this.lastManual = null;
      return ok({ accepted: true });
    }
    if (method === 'diagnostics.uiError') {
      const e = params as unknown as DiagnosticUiError;
      if (
        !only('category', 'feature', 'line', 'column') ||
        !['exception', 'rejection', 'protocol', 'request'].includes(e.category) ||
        !['app', 'connection', 'configuration', 'sessions', 'storage'].includes(e.feature) ||
        ['line', 'column'].some(
          (k) =>
            params[k] != null &&
            (!Number.isInteger(params[k]) || Number(params[k]) < 0 || Number(params[k]) > 0xffff_ffff)
        )
      )
        return fail('invalid_request');
      if (now - this.total.start >= 60_000) {
        this.total = { start: now, count: 0 };
        this.uiLimits.clear();
      }
      if (this.uiLimits.size >= 64 && !this.uiLimits.has(owner)) return fail('unavailable');
      const limit = this.uiLimits.get(owner) ?? { start: now, count: 0 };
      if (now - limit.start >= 60_000) {
        limit.start = now;
        limit.count = 0;
      }
      if (limit.count >= 10 || this.total.count >= 30) return fail('unavailable');
      limit.count++;
      this.total.count++;
      this.uiLimits.set(owner, limit);
      if (!this.incidents.length || now - this.incidents.at(-1)!.created >= 60_000) {
        this.incidents.push({
          incidentId: String(++this.next),
          reason: 'ui_failure',
          complete: false,
          clipped: false,
          ageMs: 0,
          created: now,
        });
        this.incidents = this.incidents.slice(-3);
      }
      return ok({ accepted: true });
    }
    return fail('invalid_request');
  }
}
