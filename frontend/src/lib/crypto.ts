export async function generateKeyPair(): Promise<{ publicKey: CryptoKey, privateKey: CryptoKey }> {
    const keyPair = await window.crypto.subtle.generateKey(
        {
            name: "RSA-OAEP",
            modulusLength: 2048,
            publicExponent: new Uint8Array([1, 0, 1]),
            hash: "SHA-256",
        },
        true,
        ["encrypt", "decrypt"]
    );
    return { publicKey: keyPair.publicKey, privateKey: keyPair.privateKey };
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
    let binary = '';
    const bytes = new Uint8Array(buffer);
    for (let i = 0; i < bytes.byteLength; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return window.btoa(binary);
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binaryStr = window.atob(base64);
    const bytes = new Uint8Array(binaryStr.length);
    for (let i = 0; i < binaryStr.length; i++) {
        bytes[i] = binaryStr.charCodeAt(i);
    }
    return bytes.buffer;
}

export async function exportPublicKey(key: CryptoKey): Promise<string> {
    const exported = await window.crypto.subtle.exportKey("spki", key);
    return arrayBufferToBase64(exported);
}

export async function exportPrivateKey(key: CryptoKey): Promise<string> {
    const exported = await window.crypto.subtle.exportKey("pkcs8", key);
    return arrayBufferToBase64(exported);
}

export async function importPublicKey(pem: string): Promise<CryptoKey> {
    const buffer = base64ToArrayBuffer(pem);
    return await window.crypto.subtle.importKey(
        "spki",
        buffer,
        { name: "RSA-OAEP", hash: "SHA-256" },
        true,
        ["encrypt"]
    );
}

export async function importPrivateKey(pem: string): Promise<CryptoKey> {
    const buffer = base64ToArrayBuffer(pem);
    return await window.crypto.subtle.importKey(
        "pkcs8",
        buffer,
        { name: "RSA-OAEP", hash: "SHA-256" },
        true,
        ["decrypt"]
    );
}

export async function generateAesKey(): Promise<CryptoKey> {
    return await window.crypto.subtle.generateKey(
        { name: "AES-GCM", length: 256 },
        true,
        ["encrypt", "decrypt"]
    );
}

export async function exportAesKey(key: CryptoKey): Promise<string> {
    const exported = await window.crypto.subtle.exportKey("raw", key);
    return arrayBufferToBase64(exported);
}

export async function importAesKey(b64: string): Promise<CryptoKey> {
    const buffer = base64ToArrayBuffer(b64);
    return await window.crypto.subtle.importKey(
        "raw",
        buffer,
        { name: "AES-GCM" },
        true,
        ["encrypt", "decrypt"]
    );
}

export async function aesEncrypt(plaintext: string, key: CryptoKey): Promise<{ ciphertextB64: string, nonceB64: string }> {
    const iv = window.crypto.getRandomValues(new Uint8Array(12));
    const enc = new TextEncoder();
    const encoded = enc.encode(plaintext);

    const ciphertext = await window.crypto.subtle.encrypt(
        { name: "AES-GCM", iv: iv },
        key,
        encoded
    );

    return {
        ciphertextB64: arrayBufferToBase64(ciphertext),
        nonceB64: arrayBufferToBase64(iv.buffer)
    };
}

export async function aesDecrypt(ciphertextB64: string, nonceB64: string, key: CryptoKey): Promise<string> {
    const ctBuffer = base64ToArrayBuffer(ciphertextB64);
    const ivBuffer = base64ToArrayBuffer(nonceB64);

    try {
        const decrypted = await window.crypto.subtle.decrypt(
            { name: "AES-GCM", iv: new Uint8Array(ivBuffer) },
            key,
            ctBuffer
        );
        const dec = new TextDecoder();
        return dec.decode(decrypted);
    } catch (e) {
        throw new Error("[Decryption Failed]");
    }
}

export async function rsaWrapKey(aesKeyB64: string, publicKey: CryptoKey): Promise<string> {
    const buffer = base64ToArrayBuffer(aesKeyB64);
    const wrapped = await window.crypto.subtle.encrypt(
        { name: "RSA-OAEP" },
        publicKey,
        buffer
    );
    return arrayBufferToBase64(wrapped);
}

export async function rsaUnwrapKey(wrappedB64: string, privateKey: CryptoKey): Promise<string> {
    const buffer = base64ToArrayBuffer(wrappedB64);
    try {
        const raw = await window.crypto.subtle.decrypt(
            { name: "RSA-OAEP" },
            privateKey,
            buffer
        );
        return arrayBufferToBase64(raw);
    } catch (e) {
        throw new Error("[Key Unwrap Failed]");
    }
}

async function deriveKeyFromPassword(password: string, salt: Uint8Array): Promise<CryptoKey> {
    const enc = new TextEncoder();
    const pwBytes = enc.encode(password);

    const baseKey = await window.crypto.subtle.importKey(
        "raw",
        pwBytes,
        { name: "PBKDF2" },
        false,
        ["deriveKey"]
    );

    return await window.crypto.subtle.deriveKey(
        {
            name: "PBKDF2",
            salt: salt as BufferSource,
            iterations: 600000,
            hash: "SHA-256"
        },
        baseKey,
        { name: "AES-GCM", length: 256 },
        false,
        ["encrypt", "decrypt"]
    );
}

export async function encryptPrivateKey(privkeyPem: string, password: string): Promise<{ encB64: string, saltB64: string, ivB64: string }> {
    const salt = window.crypto.getRandomValues(new Uint8Array(16));
    const iv = window.crypto.getRandomValues(new Uint8Array(12));

    const wrappingKey = await deriveKeyFromPassword(password, salt);

    const pemBuffer = base64ToArrayBuffer(privkeyPem);

    const encrypted = await window.crypto.subtle.encrypt(
        { name: "AES-GCM", iv: iv },
        wrappingKey,
        pemBuffer
    );

    return {
        encB64: arrayBufferToBase64(encrypted),
        saltB64: arrayBufferToBase64(salt.buffer),
        ivB64: arrayBufferToBase64(iv.buffer)
    };
}

export async function decryptPrivateKey(encryptedB64: string, saltB64: string, ivB64: string, password: string): Promise<string> {
    const saltBuffer = base64ToArrayBuffer(saltB64);
    const ivBuffer = base64ToArrayBuffer(ivB64);
    const encBuffer = base64ToArrayBuffer(encryptedB64);

    const wrappingKey = await deriveKeyFromPassword(password, new Uint8Array(saltBuffer));

    try {
        const decrypted = await window.crypto.subtle.decrypt(
            { name: "AES-GCM", iv: new Uint8Array(ivBuffer) },
            wrappingKey,
            encBuffer
        );
        return arrayBufferToBase64(decrypted);
    } catch (e) {
        throw new Error("wrong password or corrupted key");
    }
}
