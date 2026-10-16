import { buildNumber } from './version.ts';

export const buildWorkflowPath = '.github/workflows/build-apps.yml';
export const legacyBuildOffset = 60;

export function ciBuildIdentity(env: NodeJS.ProcessEnv = process.env) {
  const repository = env.GITHUB_REPOSITORY ?? '';
  const workflowRef = env.GITHUB_WORKFLOW_REF ?? '';
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository) ||
      !workflowRef.startsWith(`${repository}/${buildWorkflowPath}@refs/`)) {
    throw new Error(`Build numbers belong only to ${buildWorkflowPath}; migrate its counter explicitly before renaming`);
  }
  const runNumber = buildNumber(env.GITHUB_RUN_NUMBER);
  const number = buildNumber(String(Number(runNumber) + legacyBuildOffset));
  const runId = env.GITHUB_RUN_ID ?? '';
  const attempt = env.GITHUB_RUN_ATTEMPT ?? '';
  if (!/^[1-9][0-9]*$/.test(runId) || !/^[1-9][0-9]*$/.test(attempt))
    throw new Error('Missing GitHub run ID/attempt');
  if (env.GITHUB_SERVER_URL !== 'https://github.com') throw new Error('Unsupported GitHub server');
  return {
    buildNumber: number, runNumber, runId, attempt, repository,
    workflowPath: buildWorkflowPath,
    runURL: `https://github.com/${repository}/actions/runs/${runId}`,
    attemptURL: `https://github.com/${repository}/actions/runs/${runId}/attempts/${attempt}`,
  };
}
