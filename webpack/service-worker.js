/**
 * Service worker.
 */

/* eslint-env serviceworker */
/* eslint-disable no-restricted-globals -- `self` is expected to be used in workers */

import { precacheAndRoute } from 'workbox-precaching/precacheAndRoute';

// eslint-disable-next-line no-underscore-dangle
precacheAndRoute(self.__WB_MANIFEST);

const cache = new Map();

self.addEventListener('message', (event) => {
  switch (event.data.type) {
    case 'SET_CACHE': {
      const { key, value } = event.data;
      cache.set(key, value);
      break;
    }
    case 'GET_CACHE': {
      const { key, responsePort } = event.data;
      const cachedValue = cache.get(key);
      responsePort.postMessage({ ok: cachedValue });
      break;
    }
    case 'PING':
      break;
    default:
      throw new Error(`Invalid event type: ${event.data.type}`);
  }
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});
