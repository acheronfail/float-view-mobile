import type { BackendError, BackendMethod } from './contract';

const genericMessages = new Set([
  'Request is invalid',
  'Backend state does not permit the operation',
  'Permission is required',
  'Resource was not found',
  'Resource revision changed',
  'Backend service is unavailable',
  'Backend operation failed',
]);

/** Keep the envelope for diagnostics; present generic responses in user terms. */
export function backendErrorMessage(error: BackendError, method?: BackendMethod): string {
  if (!genericMessages.has(error.message)) return error.message;
  if (method === 'refloat.configuration.write') {
    if (error.code === 'conflict')
      return 'The configuration changed. Read current values and review your edits before trying again.';
    if (error.code === 'invalid_request')
      return 'Check the configuration values, then read current values and review your edits before trying again.';
    if (error.code === 'permission_required')
      return 'Allow the required permission in Settings, then reconnect and read current values before trying again.';
    if (error.code === 'not_found')
      return 'The board or configuration is no longer available. Reconnect and read current values before trying again.';
    if (error.code === 'internal')
      return 'The configuration write could not be confirmed. Read current values before retrying. If the problem continues, export diagnostic logs for support.';
    return 'The configuration write could not be confirmed. Check that the board is connected, stopped and ready, then read current values before trying again.';
  }
  const action =
    method === 'refloat.configuration.read'
      ? 'read the configuration'
      : method === 'refloat.configuration.schema' || method === 'refloat.configuration.open'
        ? 'open the configuration'
        : method?.startsWith('refloat.tune.')
          ? 'complete the Tune action'
          : method?.startsWith('refloat.')
            ? 'change the Refloat setting'
            : method === 'bluetooth.scan'
              ? 'scan for boards'
              : method === 'device.connect'
                ? 'connect to the board'
                : method?.startsWith('settings.')
                  ? 'save the settings'
                  : method?.startsWith('session.')
                    ? 'complete the ride action'
                    : 'complete this action';
  const recovery = {
    invalid_request: 'Check the entered values and try again.',
    invalid_state: 'Check the connection and any on-screen requirements, then refresh the screen before trying again.',
    permission_required: 'Allow the required permission in Settings, then try again.',
    not_found: 'This item is no longer available. Refresh the list and select it again.',
    conflict: 'This item changed. Refresh the screen and review your changes before trying again.',
    unavailable: 'Check the connection and try again when it is ready.',
    internal: 'Try again. If the problem continues, export diagnostic logs for support.',
  }[error.code];
  return `Could not ${action}. ${recovery}`;
}
