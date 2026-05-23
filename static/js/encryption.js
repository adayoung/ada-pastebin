// Encryption utilities using Web Crypto API
async function generateEncryptionKey() {
  return await window.crypto.subtle.generateKey(
    {
      name: "AES-GCM",
      length: 256
    },
    true,
    ["encrypt", "decrypt"]
  );
}

async function exportKey(key) {
  const exported = await window.crypto.subtle.exportKey("raw", key);
  // Convert ArrayBuffer to base64
  const bytes = new Uint8Array(exported);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

async function importKey(base64Key) {
  // Convert base64 to ArrayBuffer
  const binary = atob(base64Key);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }

  return await window.crypto.subtle.importKey(
    "raw",
    bytes,
    { name: "AES-GCM", length: 256 },
    true,
    ["encrypt", "decrypt"]
  );
}

async function encryptContent(content, key) {
  const encoder = new TextEncoder();
  const data = encoder.encode(content);

  // Generate random IV (12 bytes for AES-GCM)
  const iv = window.crypto.getRandomValues(new Uint8Array(12));

  const encrypted = await window.crypto.subtle.encrypt(
    {
      name: "AES-GCM",
      iv: iv
    },
    key,
    data
  );

  // Combine IV and encrypted content
  const combined = new Uint8Array(iv.length + encrypted.byteLength);
  combined.set(iv, 0);
  combined.set(new Uint8Array(encrypted), iv.length);

  // Convert to base64
  let binary = '';
  for (let i = 0; i < combined.byteLength; i++) {
    binary += String.fromCharCode(combined[i]);
  }
  return btoa(String.fromCharCode.apply(null, combined));
}

async function decryptContent(encryptedBase64, keyBase64) {
  try {
    // Import the key
    const key = await importKey(keyBase64);

    // Decode from base64
    const combined = new Uint8Array(atob(encryptedBase64).split('').map(c => c.charCodeAt(0)));

    // Extract IV and encrypted data
    const iv = combined.slice(0, 12);
    const encrypted = combined.slice(12);

    const decrypted = await window.crypto.subtle.decrypt(
      {
        name: "AES-GCM",
        iv: iv
      },
      key,
      encrypted
    );

    const decoder = new TextDecoder();
    return decoder.decode(decrypted);
  } catch (e) {
    console.error('Decryption failed:', e);
    throw new Error('Failed to decrypt content. Invalid or missing key.');
  }
}
