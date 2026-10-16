import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ciBuildIdentity } from './build-record.ts';

const environment = {
  GITHUB_REPOSITORY: 'owner/repo',
  GITHUB_WORKFLOW_REF: 'owner/repo/.github/workflows/build-apps.yml@refs/heads/master',
  GITHUB_RUN_ID: '38863449266', GITHUB_RUN_NUMBER: '41', GITHUB_RUN_ATTEMPT: '1',
  GITHUB_SERVER_URL: 'https://github.com',
};
test('all platforms share the run number plus the retained offset and a traceable CI URL', () => {
  const result = ciBuildIdentity(environment);
  assert.equal(result.buildNumber, '101');
  assert.equal(result.runURL, 'https://github.com/owner/repo/actions/runs/38863449266');
  assert.equal(result.attemptURL, result.runURL + '/attempts/1');
  assert.equal(Number(result.buildNumber) - 60, Number(environment.GITHUB_RUN_NUMBER));
});
test('reruns retain native identity; a fresh run supplies a newer build', () => {
  const first = ciBuildIdentity(environment);
  const retry = ciBuildIdentity({ ...environment, GITHUB_RUN_ATTEMPT: '2' });
  assert.equal(first.buildNumber, retry.buildNumber);
  assert.notEqual(first.attemptURL, retry.attemptURL);
  const next = ciBuildIdentity({ ...environment, GITHUB_RUN_NUMBER: '42', GITHUB_RUN_ID: '38863449267' });
  assert.equal(Number(next.buildNumber), Number(first.buildNumber) + 1);
});
test('display name changes are allowed but another workflow cannot start a second counter', () => {
  assert.equal(ciBuildIdentity({ ...environment, GITHUB_WORKFLOW: 'Renamed display title' }).buildNumber, '101');
  assert.throws(() => ciBuildIdentity({ ...environment,
    GITHUB_WORKFLOW_REF: 'owner/repo/.github/workflows/store.yml@refs/heads/master' }), /migrate/);
});
test('missing IDs, invalid counters and overflow fail instead of allocating a fallback', () => {
  for (const override of [{ GITHUB_RUN_ID: '' }, { GITHUB_RUN_ATTEMPT: '0' },
    { GITHUB_RUN_NUMBER: '01' }, { GITHUB_RUN_NUMBER: '2100000000' },
    { GITHUB_SERVER_URL: 'https://example.invalid' }]) {
    assert.throws(() => ciBuildIdentity({ ...environment, ...override }));
  }
});
