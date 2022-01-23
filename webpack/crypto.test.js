/**
 * @jest-environment ./webpack/__test__/EnrichedEnv
 */

/* eslint-env browser,jest */

import { openBox, sealBox } from './crypto';

test('box roundtrip', async () => {
  const password = 'correct horse battery staple';
  const plaintext = new Uint8Array(32);
  crypto.getRandomValues(plaintext);

  const box = await sealBox(password, plaintext);
  const opened = await openBox(password, box);

  expect(opened).toEqual(plaintext);
});

test('opening box with incorrect password', async () => {
  const password = 'correct horse battery staple';
  const plaintext = new Uint8Array(32);
  crypto.getRandomValues(plaintext);

  const box = await sealBox(password, plaintext);
  try {
    await openBox('bogus', box);
  } catch (e) {
    expect(e.message).toMatch('failed decryption');
  }
});
