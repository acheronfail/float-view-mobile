import { describe, expect, it } from 'vitest';
import { MockDiagnostics } from '../../../backend-mock/diagnostics';
import { installUiErrorReporting } from './ui-errors';

describe('diagnostics contract', () => {
  it('keeps captures immutable, bounded, client owned and clearable', () => {
    let now = 100_000;
    const mock = new MockDiagnostics(() => now);
    const call = (method: string, params = {}, owner = 1) => mock.handle(1, method, params, owner);
    expect(call('diagnostics.capture')).toMatchObject({ result: { incidentId: '1', capturing: true } });
    expect(call('diagnostics.capture')).toMatchObject({ error: { code: 'unavailable' } });
    const prepared = call('diagnostics.prepare', { incidentId: '1' });
    expect(prepared).toMatchObject({ result: { reportId: '2', complete: false } });
    expect(call('diagnostics.export', { reportId: '2' }, 2)).toMatchObject({ error: { code: 'not_found' } });
    expect(call('diagnostics.export', { reportId: '2' })).toMatchObject({ result: { delivery: 'presented' } });
    now += 11_000;
    expect(call('diagnostics.list')).toMatchObject({ result: { incidents: [{ complete: true }] } });
    expect(prepared).toMatchObject({ result: { complete: false } });
    call('diagnostics.prepare', { incidentId: null });
    expect(call('diagnostics.prepare', { incidentId: null })).toMatchObject({ error: { code: 'unavailable' } });
    call('diagnostics.clear');
    expect(call('diagnostics.export', { reportId: '2' })).toMatchObject({ error: { code: 'not_found' } });
    expect(call('diagnostics.list')).toMatchObject({ result: { incidents: [] } });
  });
  it('rejects private text and expires handles', () => {
    let now = 100_000;
    const mock = new MockDiagnostics(() => now);
    const call = (method: string, params = {}) => mock.handle(1, method, params, 1);
    for (const params of [{ path: '/private' }, { incidentId: [] }, {}])
      expect(call('diagnostics.prepare', params)).toMatchObject({ error: { code: 'invalid_request' } });
    expect(call('diagnostics.uiError', { category: 'exception', feature: 'app', message: 'PRIVATE' })).toMatchObject({
      error: { code: 'invalid_request' },
    });
    const error = { category: 'exception', feature: 'app', line: null, column: null };
    for (let i = 0; i < 10; i++) expect(call('diagnostics.uiError', error)).toHaveProperty('result');
    expect(call('diagnostics.uiError', error)).toMatchObject({ error: { code: 'unavailable' } });
    const preview = call('diagnostics.prepare', { incidentId: null });
    expect(JSON.stringify(preview)).not.toContain('PRIVATE');
    now += 600_001;
    if ('result' in preview)
      expect(call('diagnostics.export', { reportId: (preview.result as { reportId: string }).reportId })).toMatchObject(
        { error: { code: 'not_found' } }
      );
  });
});
it('reports only closed frontend categories and bounds repeated exceptions', async () => {
  const sent: unknown[] = [];
  const target = new EventTarget();
  const dispose = installUiErrorReporting(
    async (e) => {
      sent.push(e);
    },
    Object.assign(target, { setInterval: () => 1, clearInterval: () => {} }) as unknown as Window,
    () => 0
  );
  for (let i = 0; i < 50; i++)
    target.dispatchEvent(
      Object.assign(new Event('error'), {
        message: 'PRIVATE',
        error: new Error('PRIVATE'),
        filename: 'https://private',
      })
    );
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(sent).toHaveLength(10);
  expect(sent[0]).toEqual({ category: 'exception', feature: 'app', line: null, column: null });
  expect(JSON.stringify(sent)).not.toContain('PRIVATE');
  dispose();
});
