import { FIXTURE_DEV_PORT } from './lib/contract.ts';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  if (event.url.hostname !== '127.0.0.1' || event.url.port !== String(FIXTURE_DEV_PORT)) {
    return new Response('Unexpected fixture development host', { status: 403 });
  }
  return resolve(event, {
    filterSerializedResponseHeaders: (name) => name === 'content-type',
  });
};
