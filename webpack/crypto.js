/**
 * Cryptographic utils.
 */

const SALT_LEN = 32;
const IV_LEN = 12;
const MAC_LEN = 16;
const KDF_ITERATIONS = 100000;

const ENCODER = new TextEncoder();

function toHex(bytes) {
  return Array.prototype.map.call(
    bytes,
    (byte) => {
      const encoded = byte.toString(16);
      return (encoded.length === 1) ? `0${encoded}` : encoded;
    },
  ).join('');
}

function fromHex(hexString) {
  if (typeof hexString !== 'string') {
    throw new TypeError('invalid hex string type; expected a string');
  }
  if (!/^[0-9a-f]+$/.test(hexString)) {
    throw new TypeError('hex string contains invalid chars');
  }

  const buffer = new Uint8Array(hexString.length / 2);
  for (let i = 0; i < hexString.length; i += 2) {
    buffer[i / 2] = parseInt(hexString.substring(i, i + 2), 16);
  }
  return buffer;
}

export async function sealBox(password, plaintext) {
  if (typeof password !== 'string') {
    throw new TypeError('invalid password type; expected a string');
  }
  if (!(plaintext instanceof Uint8Array)) {
    throw new TypeError('invalid secret type; expected bytes');
  }

  const salt = new Uint8Array(SALT_LEN);
  crypto.getRandomValues(salt);
  const iv = new Uint8Array(IV_LEN);
  crypto.getRandomValues(iv);

  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    ENCODER.encode(password),
    'PBKDF2',
    false,
    ['deriveBits', 'deriveKey'],
  );
  const encryptionKey = await crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations: KDF_ITERATIONS,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 128 },
    false,
    ['encrypt', 'decrypt'],
  );

  const ciphertextWithMac = new Uint8Array(await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    encryptionKey,
    plaintext,
  ));
  const ciphertext = ciphertextWithMac.subarray(0, ciphertextWithMac.length - MAC_LEN);
  const mac = ciphertextWithMac.subarray(ciphertextWithMac.length - MAC_LEN);

  return {
    kdf: 'pbkdf2-sha256',
    cipher: 'aes-128-gcm',
    ciphertext: toHex(ciphertext),
    mac: toHex(mac),
    kdfparams: {
      salt: toHex(salt),
      iterations: KDF_ITERATIONS,
    },
    cipherparams: { iv: toHex(iv) },
  };
}

export async function openBox(password, box) {
  const { kdf, cipher, kdfparams: { iterations } } = box;
  let {
    ciphertext,
    mac,
    kdfparams: { salt },
    cipherparams: { iv },
  } = box;

  if (kdf !== 'pbkdf2-sha256') {
    throw new Error(`Unknown KDF ${kdf}; pbkdf2-sha256 was expected`);
  }
  if (cipher !== 'aes-128-gcm') {
    throw new Error(`Unknown cipher ${cipher}; aes-128-gcm was expected`);
  }

  ciphertext = fromHex(ciphertext);
  mac = fromHex(mac);
  salt = fromHex(salt);
  iv = fromHex(iv);

  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    ENCODER.encode(password),
    'PBKDF2',
    false,
    ['deriveBits', 'deriveKey'],
  );
  const encryptionKey = await crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 128 },
    false,
    ['encrypt', 'decrypt'],
  );

  const ciphertextWithMac = new Uint8Array(ciphertext.length + mac.length);
  ciphertextWithMac.set(ciphertext, 0);
  ciphertextWithMac.set(mac, ciphertext.length);

  try {
    const plaintext = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv },
      encryptionKey,
      ciphertextWithMac,
    );
    return new Uint8Array(plaintext);
  } catch (e) {
    throw new Error('failed decryption (perhaps, the password is incorrect)');
  }
}
