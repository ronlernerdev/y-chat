<script lang="ts">
  import { onMount } from "svelte";
  import { BACKEND_URL, WS_URL } from "./config";
  import type { User } from "./types";
  import {
    importPrivateKey,
    importPublicKey,
    rsaUnwrapKey,
    rsaWrapKey,
    generateAesKey,
    exportAesKey,
    importAesKey,
    aesEncrypt,
    aesDecrypt,
    aesEncryptBytes,
    aesDecryptBytes
  } from "./crypto";

  const {
    currentUser,
    storedPrivkeyPem,
    onKeyRecovered,
  } = $props<{
    currentUser: User;
    storedPrivkeyPem: string | null;
    onKeyRecovered: (pem: string) => void;
  }>();

  interface Server {
    id: string;
    name: string;
    owner: string;
  }

  interface Channel {
    id: string;
    server_id: string;
    name: string;
  }

  interface MsgRes {
    id: string;
    channel_id: string;
    author_id: string;
    encrypted_content: string;
    nonce: string;
    attachment: string | null;
  }

  interface DisplayMsg {
    id: string;
    author_id: string;
    content: string;
    attachment?: string;
  }

  type WsMsg =
    | {
        type: "Chat";
        channel_id: string;
        author_id: string;
        encrypted_content: string;
        nonce: string;
        attachment?: string | null;
      }
    | {
        type: "KeyRequest";
        server_id: string;
        requester_id: string;
        requester_pubkey: string;
      }
    | {
        type: "KeyDelivered";
        target_user_id: string;
        server_id: string;
      };

  let privkeyPem = $derived(storedPrivkeyPem);
  let serverKeys = $state<Record<string, string>>({});

  let servers = $state<Server[]>([]);
  let channels = $state<Channel[]>([]);
  let messages = $state<DisplayMsg[]>([]);
  let usersInServer = $state<User[]>([]);

  let activeServer = $state<string | null>(null);
  let activeChannel = $state<string | null>(null);

  let newServerName = $state("");
  let newChanName = $state("");
  let joinServerId = $state("");
  let inputText = $state("");
  let selectedFile = $state<File | null>(null);
  let decryptedImages = $state<Record<string, string>>({});

  let ws: WebSocket | null = null;

  async function handleFile(ev: Event) {
    const t = ev.target as HTMLInputElement;
    if (t.files && t.files.length > 0) {
      selectedFile = t.files[0];
    }
  }

  async function fetchAndLoadServerKey(sid: string, uid: string, pem: string): Promise<string | null> {
    try {
      const res = await fetch(`${BACKEND_URL}/sk/${sid}/${uid}`);
      if (!res.ok) return null;
      const body = await res.json();
      const privKey = await importPrivateKey(pem);
      return await rsaUnwrapKey(body.encrypted_key, privKey);
    } catch {
      return null;
    }
  }

  async function distributeKeysToPending(sid: string, aesB64: string) {
    try {
      const res = await fetch(`${BACKEND_URL}/sk/pending/${sid}`);
      if (!res.ok) return;
      const pending: { user_id: string; pubkey: string }[] = await res.json();

      for (const member of pending) {
        try {
          const pubKey = await importPublicKey(member.pubkey);
          const wrapped = await rsaWrapKey(aesB64, pubKey);
          await fetch(`${BACKEND_URL}/sk`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              server_id: sid,
              user_id: member.user_id,
              encrypted_key: wrapped,
            }),
          });
        } catch {
        }
      }
    } catch {
    }
  }

  async function loadServers() {
    try {
      const res = await fetch(`${BACKEND_URL}/s/${currentUser.id}`);
      if (res.ok) {
        servers = await res.json();
      }
    } catch {
    }
  }

  async function selectServer(id: string) {
    activeServer = id;
    activeChannel = null;
    messages = [];
    channels = [];
    usersInServer = [];

    try {
      const cRes = await fetch(`${BACKEND_URL}/c/${id}/${currentUser.id}`);
      if (cRes.ok) channels = await cRes.json();

      const uRes = await fetch(`${BACKEND_URL}/u/${id}`);
      if (uRes.ok) usersInServer = await uRes.json();

      if (privkeyPem) {
        const aesB64 = await fetchAndLoadServerKey(id, currentUser.id, privkeyPem);
        if (aesB64) {
          serverKeys[id] = aesB64;
          const s = servers.find((x) => x.id === id);
          if (s && s.owner === currentUser.id) {
            await distributeKeysToPending(id, aesB64);
          }
          return;
        }
      }

      const s = servers.find((x) => x.id === id);
      if (s && s.owner !== currentUser.id) {
        const msg: WsMsg = {
          type: "KeyRequest",
          server_id: id,
          requester_id: currentUser.id,
          requester_pubkey: currentUser.pk,
        };
        ws?.send(JSON.stringify(msg));
      }
    } catch {
    }
  }

  async function selectChannel(id: string) {
    activeChannel = id;
    try {
      const res = await fetch(`${BACKEND_URL}/m/${id}/${currentUser.id}`);
      if (!res.ok) return;
      const msgs: MsgRes[] = await res.json();

      const sid = activeServer;
      let aesKey: CryptoKey | null = null;
      if (sid && serverKeys[sid]) {
        try {
          aesKey = await importAesKey(serverKeys[sid]);
        } catch {}
      }

      const display: DisplayMsg[] = [];
      for (const m of msgs) {
        let content = "<encrypted>";
        if (aesKey) {
          try {
            content = await aesDecrypt(m.encrypted_content, m.nonce, aesKey);
            if (m.attachment) {
               fetchAndDecryptImage(m.attachment, aesKey);
            }
          } catch {}
        }
        display.push({ id: m.id, author_id: m.author_id, content, attachment: m.attachment || undefined });
      }
      messages = display;
    } catch {
    }
  }

  onMount(() => {
    loadServers();

    let reconnectTimer: number | null = null;
    let isUnmounted = false;

    function connectWs() {
      if (isUnmounted) return;
      if (ws) {
        ws.onclose = null;
        ws.close();
      }
      console.log("[WS] Connecting to", WS_URL);
      ws = new WebSocket(WS_URL);

      ws.onopen = () => {
        console.log("[WS] Connected");
        if (activeServer && activeChannel) {

        }
      };

      ws.onmessage = async (ev) => {
        try {
          const parsed: WsMsg = JSON.parse(ev.data);
          if (parsed.type === "Chat") {
            const { channel_id, author_id, encrypted_content, nonce, attachment } = parsed;
            if (activeChannel === channel_id) {
              const sid = activeServer;
              let aesKey: CryptoKey | null = null;
              if (sid && serverKeys[sid]) {
                try {
                  aesKey = await importAesKey(serverKeys[sid]);
                } catch {}
              }

              let content = "<encrypted>";
              if (aesKey) {
                try {
                  content = await aesDecrypt(encrypted_content, nonce, aesKey);
                  if (attachment) {
                     fetchAndDecryptImage(attachment, aesKey);
                  }
                } catch {}
              }

              messages = [
                ...messages,
                { id: crypto.randomUUID(), author_id, content, attachment: attachment || undefined },
              ];
            }
          } else if (parsed.type === "KeyRequest") {
            const { server_id, requester_id, requester_pubkey } = parsed;
            const s = servers.find((x) => x.id === server_id);
            if (s && s.owner === currentUser.id && requester_id !== currentUser.id) {
              if (privkeyPem) {
                const aesB64Cached = serverKeys[server_id];
                let myAesOpt = aesB64Cached;
                if (!myAesOpt) {
                  try {
                    const privKey = await importPrivateKey(privkeyPem);
                    const r = await fetch(`${BACKEND_URL}/sk/${server_id}/${currentUser.id}`);
                    if (r.ok) {
                      const body = await r.json();
                      myAesOpt = await rsaUnwrapKey(body.encrypted_key, privKey);
                    }
                  } catch {}
                }

                if (myAesOpt) {
                  try {
                    const theirPub = await importPublicKey(requester_pubkey);
                    const wrapped = await rsaWrapKey(myAesOpt, theirPub);
                    await fetch(`${BACKEND_URL}/sk`, {
                      method: "POST",
                      headers: { "Content-Type": "application/json" },
                      body: JSON.stringify({
                        server_id,
                        user_id: requester_id,
                        encrypted_key: wrapped,
                      }),
                    });

                    const notif: WsMsg = {
                      type: "KeyDelivered",
                      target_user_id: requester_id,
                      server_id,
                    };
                    if (ws?.readyState === WebSocket.OPEN) {
                        ws.send(JSON.stringify(notif));
                    }
                  } catch {}
                }
              }
            }
          } else if (parsed.type === "KeyDelivered") {
            const { target_user_id, server_id } = parsed;
            if (target_user_id === currentUser.id) {
              if (privkeyPem) {
                const aesB64 = await fetchAndLoadServerKey(server_id, currentUser.id, privkeyPem);
                if (aesB64) {
                  serverKeys[server_id] = aesB64;
                  if (activeServer === server_id && activeChannel) {
                    selectChannel(activeChannel);
                  }
                }
              }
            }
          }
        } catch {
        }
      };

      ws.onclose = () => {
        console.log("[WS] Disconnected, reconnecting in 2s...");
        if (!isUnmounted) {
           reconnectTimer = window.setTimeout(connectWs, 2000);
        }
      };

      ws.onerror = (e) => {
        console.error("[WS] Error", e);
        ws?.close();
      };
    }

    connectWs();

    return () => {
      isUnmounted = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (ws) {
        ws.onclose = null;
        ws.close();
      }
      Object.values(decryptedImages).forEach(url => URL.revokeObjectURL(url));
    };
  });

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

  async function fetchAndDecryptImage(url: string, key: CryptoKey) {
      if (decryptedImages[url]) return;
      try {
          const res = await fetch(url);
          const metaB64 = res.headers.get("X-Aes-Nonce");
          if (!metaB64) return;
          const encBuf = await res.arrayBuffer();
          const nonce = base64ToArrayBuffer(metaB64);
          const dec = await aesDecryptBytes(encBuf, nonce, key);
          const blob = new Blob([dec]);
          decryptedImages[url] = URL.createObjectURL(blob);
      } catch (e) {
          console.error("Failed image decrypt", e);
      }
  }

  async function createServer() {
    if (!newServerName) return;
    try {
      const res = await fetch(`${BACKEND_URL}/ms`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: newServerName, owner_id: currentUser.id }),
      });
      if (res.ok) {
        const sidStr = await res.text();
        const sid = sidStr.replace(/"/g, "");
        if (privkeyPem) {
          try {
            const pubKey = await importPublicKey(currentUser.pk);
            const aesKey = await generateAesKey();
            const aesB64 = await exportAesKey(aesKey);
            const wrapped = await rsaWrapKey(aesB64, pubKey);

            await fetch(`${BACKEND_URL}/sk`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                server_id: sid,
                user_id: currentUser.id,
                encrypted_key: wrapped,
              }),
            });

            serverKeys[sid] = aesB64;
          } catch {}
        }
      }
      newServerName = "";
      loadServers();
    } catch {}
  }

  async function joinServer() {
    if (!joinServerId) return;
    try {
      await fetch(`${BACKEND_URL}/js`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ server_id: joinServerId, user_id: currentUser.id }),
      });
      joinServerId = "";
      loadServers();
    } catch {}
  }

  async function createChannel() {
    if (!newChanName || !activeServer) return;
    try {
      await fetch(`${BACKEND_URL}/mc`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: newChanName, server_id: activeServer, user_id: currentUser.id }),
      });
      newChanName = "";
      selectServer(activeServer);
    } catch {}
  }

  async function sendMessage(ev: Event) {
    ev.preventDefault();
    if ((!inputText && !selectedFile) || !activeChannel || !activeServer) return;

    try {
      console.log("SENDING MESSAGE...");
      if (!serverKeys[activeServer]) {
          console.error("NO SERVER KEY FOR THIS SERVER ID", activeServer, serverKeys);
          return;
      }
      const aesB64 = serverKeys[activeServer];
      if (aesB64) {
        const aesKey = await importAesKey(aesB64);
        let attachmentUrl = null;
        
        if (selectedFile) {
          const rawBuf = await selectedFile.arrayBuffer();
          const { ciphertext, nonce } = await aesEncryptBytes(rawBuf, aesKey);
          const nonceB64 = arrayBufferToBase64(nonce);
          
          const ext = selectedFile.name.split('.').pop() || "png";
          const res = await fetch(`${BACKEND_URL}/up?ext=${ext}&nonce=${encodeURIComponent(nonceB64)}`, {
            method: "POST",
            body: ciphertext,
            headers: { "Content-Type": "application/octet-stream" }
          });
          attachmentUrl = await res.text();
          selectedFile = null;
        }

        const txt = inputText || "[picture]";
        const { ciphertextB64, nonceB64: txtNonce } = await aesEncrypt(txt, aesKey);

        const msg: WsMsg = {
          type: "Chat",
          channel_id: activeChannel,
          author_id: currentUser.id,
          encrypted_content: ciphertextB64,
          nonce: txtNonce,
          attachment: attachmentUrl
        };
        ws?.send(JSON.stringify(msg));
        inputText = "";
      }
    } catch (e) {
      console.error("ERROR SENDING MSG:", e);
    }
  }
</script>

<div class="h-screen w-full flex flex-col">
  <div class="p-2 flex justify-between items-center">
    <span>y-chat</span>
    <div>
      <span>USER: {currentUser.un}</span>
    </div>
  </div>

  <div class="flex-1 flex overflow-hidden">
    <div class="w-48 flex flex-col overflow-y-auto p-4 space-y-8">
      <section>
        <h3 class="mb-3">{">"} servers</h3>
        <ul class="space-y-1">
          {#each servers as s}
            <li
              class="cursor-pointer"
              onclick={() => selectServer(s.id)}
            >
              {activeServer === s.id ? "> " : "  "}
              {s.name}
            </li>
          {/each}
          <li class="mt-4">
            <input
              class="w-full"
              placeholder="+ server"
              bind:value={newServerName}
              onkeydown={(ev) => ev.key === "Enter" && createServer()}
            />
          </li>
          <li class="mt-2">
            <input
              class="w-full"
              placeholder="+ join id"
              bind:value={joinServerId}
              onkeydown={(ev) => ev.key === "Enter" && joinServer()}
            />
          </li>
        </ul>
      </section>

      {#if activeServer}
        <section>
          <h3 class="mb-1">{">"} channels</h3>
          <div class="mb-3">
            ID: {activeServer}
          </div>
          <ul class="space-y-1">
            {#each channels as c}
              <li
                class="cursor-pointer"
                onclick={() => selectChannel(c.id)}
              >
                {activeChannel === c.id ? "> #" : "  #"}
                {c.name}
              </li>
            {/each}
            <li class="mt-4">
              <input
                class="w-full"
                placeholder="+ channel"
                bind:value={newChanName}
                onkeydown={(ev) => ev.key === "Enter" && createChannel()}
              />
            </li>
          </ul>
        </section>
      {/if}
    </div>

    <div class="flex-1 flex flex-col overflow-hidden relative">
      <div class="flex-1 overflow-y-auto p-4 space-y-1">
        {#each messages as m (m.id)}
          <div>
            <span>
              [{usersInServer.find((u) => u.id === m.author_id)?.un || "Unknown"}]
            </span>
            <span>
              {m.content}
            </span>
            {#if m.attachment && decryptedImages[m.attachment]}
               <img src={decryptedImages[m.attachment]} class="max-w-xs mt-2" alt="pic" />
            {/if}
          </div>
        {/each}
      </div>

      <div class="p-2">
        <form onsubmit={sendMessage} class="flex items-center">
          <input type="file" id="pic" accept="image/*" class="hidden" onchange={handleFile} />
          <button type="button" onclick={() => document.getElementById("pic")?.click()} class="mr-2 text-xs border px-2 py-1">
             {selectedFile ? "+" : "pic"}
          </button>
          <span class="mr-2">{">"}</span>
          <input
            type="text"
            bind:value={inputText}
            class="flex-1"
            placeholder="type command..."
          />
        </form>
      </div>
    </div>
  </div>
</div>
