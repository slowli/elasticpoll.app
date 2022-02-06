/**
 * Entry point for the main page.
 */

import './main.scss';
import './icons/bootstrap-icons.scss';

import 'bootstrap/js/dist/collapse';
import Modal from 'bootstrap/js/dist/modal';
import 'bootstrap/js/dist/tab';
import Tooltip from 'bootstrap/js/dist/tooltip';
import copyTextToClipboard from 'copy-text-to-clipboard';

import { openBox, sealBox } from './crypto';

const SERVICE_WORKER_URL = '/service-worker.js';
const CACHE_KEY = 'secret_seed';
const PING_INTERVAL = 10000;

function onValueExported({ data }, target) {
  copyTextToClipboard(data);

  target.removeAttribute('title');
  const tooltip = Tooltip.getOrCreateInstance(target, {
    title: 'Copied to clipboard!',
    trigger: 'manual',
  });
  tooltip.show();
  setTimeout(() => tooltip.hide(), 3000);
}

async function postMessageAsync(controller, message) {
  const channel = new MessageChannel();
  const fullMessage = {
    ...message,
    responsePort: channel.port2,
  };
  controller.postMessage(fullMessage, [fullMessage.responsePort]);

  return new Promise((resolve, reject) => {
    channel.port1.onmessage = (event) => {
      if ('err' in event.data) {
        reject(new Error(event.data.err));
      } else {
        resolve(event.data.ok);
      }
    };
  });
}

let getCachedBox = null;
let cacheBox = null;

if ('serviceWorker' in navigator) {
  const { serviceWorker } = navigator;

  getCachedBox = async () => {
    await serviceWorker.ready;
    return postMessageAsync(serviceWorker.controller, { type: 'GET_CACHE', key: CACHE_KEY });
  };
  cacheBox = async (value) => {
    await serviceWorker.ready;
    serviceWorker.controller.postMessage({ type: 'SET_CACHE', key: CACHE_KEY, value });
  };

  window.addEventListener('load', () => {
    serviceWorker.register(SERVICE_WORKER_URL).catch(console.error);
    serviceWorker.ready.then(() => {
      // Periodically ping the worker so it does not get terminated while the page is active.
      // This is necessary because the root secret is stored in the worker's RAM
      // (for security reasons), so it needs to be unlocked again each time after termination.
      setInterval(
        () => {
          serviceWorker.controller.postMessage({ type: 'PING' });
        },
        PING_INTERVAL,
      );
    });
  });
} else {
  // Use a per-page cache (less efficient than with a service worker, since different
  // tabs / windows will have separate caches).
  let cachedValue;

  getCachedBox = async () => cachedValue;
  cacheBox = async (value) => {
    cachedValue = value;
  };
}

import(/* webpackChunkName: "bundle" */ '../pkg').then((wasm) => {
  wasm.runApp({
    onexport: onValueExported,

    sealBox: (password, secretBytes) => sealBox(password, secretBytes).then(JSON.stringify),
    getCachedBox,
    openBox: async (password, boxJson) => {
      const secret = await openBox(password, JSON.parse(boxJson));
      cacheBox(secret).catch(console.error);
      return secret;
    },

    showModal: (elementId) => {
      const element = document.getElementById(elementId);
      const modal = Modal.getOrCreateInstance(element);
      modal.show();
    },
    hideModal: (elementId) => {
      const element = document.getElementById(elementId);
      const modal = Modal.getInstance(element);
      if (modal) {
        modal.hide();
      }
    },
  });

  if ('__PRERENDER__' in window) {
    document.dispatchEvent(new Event('wasm-rendered'));
  }
});
