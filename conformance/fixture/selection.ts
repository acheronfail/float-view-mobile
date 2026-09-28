// Physical checkpoints deliberately carry stale software credentials. An ordinary
// host must ignore them and use its OS adapter; a leaked test seam fails to connect.
export const unreachableFixture = {
  endpoint: 'http://127.0.0.1:1/',
  token: '0'.repeat(48),
  run: '0'.repeat(48),
  session: '0'.repeat(48),
};
export const unreachableFixtureEnvironment = {
  FLOAT_VIEW_SOFTWARE_BACKEND: '1',
  FLOAT_VIEW_FIXTURE_ENDPOINT: unreachableFixture.endpoint,
  FLOAT_VIEW_FIXTURE_TOKEN: unreachableFixture.token,
  FLOAT_VIEW_FIXTURE_RUN: unreachableFixture.run,
  FLOAT_VIEW_FIXTURE_SESSION: unreachableFixture.session,
};
