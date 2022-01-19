/**
 * Entry point for the main page.
 */

import './main.scss';
import './icons/bootstrap-icons.scss';

import 'bootstrap/js/dist/collapse';
import Modal from 'bootstrap/js/dist/modal';
import 'bootstrap/js/dist/tab';
import copyTextToClipboard from 'copy-text-to-clipboard';

import { openBox, sealBox } from "./crypto";

function onValueExported({ data }) {
  copyTextToClipboard(data);
}

import(/* webpackChunkName: "bundle" */ '../pkg').then((wasm) => {
  wasm.runApp({
    onexport: onValueExported,
    sealBox: (password, secretBytes) => {
      return sealBox(password, secretBytes).then(JSON.stringify);
    },
    openBox: async (password, boxJson) => {
      const result = await openBox(password, JSON.parse(boxJson));
      return result;
    },
    hideModal: (elementId) => {
      const element = document.getElementById(elementId);
      const modal = Modal.getInstance(element);
      if (!modal) {
        throw new Error(`modal with ID ${elementId} does not exist`);
      }
      modal.hide();
    },
  });
});
