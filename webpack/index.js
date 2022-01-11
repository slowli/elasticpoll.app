/**
 * Entry point for the main page.
 */

import './main.scss';
import './icons/bootstrap-icons.scss';

import 'bootstrap/js/dist/collapse';
import 'bootstrap/js/dist/tab';
import copyTextToClipboard from 'copy-text-to-clipboard';

function onValueExported({ data }) {
  copyTextToClipboard(data);
}

import(/* webpackChunkName: "bundle" */ '../pkg').then((wasm) => {
  wasm.runApp({
    onexport: onValueExported,
  });
});
