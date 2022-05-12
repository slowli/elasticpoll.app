const { webcrypto } = require('crypto');
const { TextEncoder, TextDecoder } = require('util');
const { default: JsdomEnv } = require('jest-environment-jsdom');

// Adds `TextEncoder`, `TextDecoder` and `crypto` globals into the test environment.
module.exports = class EnrichedEnv extends JsdomEnv {
  async setup() {
    await super.setup();
    if (this.global.TextEncoder === undefined) {
      this.global.TextEncoder = TextEncoder;
      this.global.TextDecoder = TextDecoder;
    }
    if (this.global.crypto === undefined) {
      this.global.crypto = webcrypto;
    }
  }
};
