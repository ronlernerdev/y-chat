<script lang="ts">
    import {onMount} from "svelte"
    import { aesDecrypt, aesEncrypt, exportAesKey, generateAesKey, importAesKey, importPrivateKey, importPublicKey, rsaUnwrapKey, rsaWrapKey } from "./crypto";
    import type {User} from './types'
    import { BACKEND_URL, WS_URL } from "./config";

    const {currentUser,storedprivatekey,onkeyrecoverd} = $props<{currentUser:User;storedprivatekey:string | null;onkeyrecoverd:(pem:string)=> void;}>()

    interface server {
      id:string,
      name:string,
      owner:string
    }

    interface channel {
      id:string;
      server_id:string;
      name:string;
    }

    interface msgres{
      id:string;
      channel_id:string;
      author_id:string;
      encrypted_content:string;
      nonce:string;
    }

    interface displaymsg {
      id: string;
      author_id:string;
      content:string;
    }

    type WsMsg =
      | {
          type: "Chat";
          channel_id: string;
          author_id: string;
          encrypted_content: string;
          nonce: string;
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

    let privatekeyPem = $derived(storedprivatekey)
    let serverKeys = $state<Record<string,string>>({});

    let servers = $state<server[]>([]);
    let channels = $state<channel[]>([]);
    let messages = $state<displaymsg[]>([]);
    let usersInServer = $state<User[]>([]);

    let currentServer = $state<string | null>(null);
    let currentChannel = $state<string | null>(null);

    let newServerName = $state("");
    let newChanName = $state("");
    let joinServerId = $state("");
    let inputText = $state("");

    let ws:WebSocket | null = null;

    async function fetchAndLoadServerKey(sid:string,uid:string,pem:string):Promise<string | null>{
      try {
        const res = await fetch(`${BACKEND_URL}/sk/pending/${sid}/${uid}`);
        if (!res.ok) return null;
        const body = await res.json();
        const privateKey = await importPrivateKey(pem)
        return await rsaUnwrapKey(body.encrypted_key,privateKey)
      } catch {
        return null;
      }
    }

    async function giveoutkeys(sid:string,aesB64:string){
      try {
        const res = await fetch(`${BACKEND_URL}/sk/pending/${sid}`);
        if (!res.ok) return;
        const pending:{user_id:string;pubkey:string}[] = await res.json();

        for (const member of pending){
          try {
            const pubkey = await importPublicKey(member.pubkey);
            const wrapped = await rsaWrapKey(aesB64,pubkey)
            await fetch(`${BACKEND_URL}/sk`,{
              method:"POST",
              headers:{"Content-Type":"application/json"},
              body:JSON.stringify({
                server_id:sid,
                user_id:member.user_id,
                encrypted_key:wrapped,
              }),
            });
          }catch {}
        }
      }catch {}
    }

    async function loadServers(){
      try {
        const res = await fetch(`${BACKEND_URL}/s/${currentUser.id}`)
        if(res.ok){
          servers = await res.json();
        }
      }catch{}
    }

    async function selectServer(id:string){
      currentServer = id;
      currentChannel = null;
      messages = [];
      channels = []
      usersInServer = []

      try {
        const cRes = await fetch(`${BACKEND_URL}/c/${id}`)
        if (cRes.ok) channels = await cRes.json()

        const uRes = await fetch(`${BACKEND_URL}/u/${id}`)
        if (uRes.ok) usersInServer = await uRes.json()

        if (privatekeyPem){
          const aesB64 = await fetchAndLoadServerKey(id,currentUser.id,privatekeyPem)
          if(aesB64){
            serverKeys[id] = aesB64;
            const s = servers.find((x) => x.id === id);
            if(s && s.owner === currentUser.id){
              await giveoutkeys(id,aesB64)
            }
            return
          }
        }

        const s = servers.find((x) => x.id === id);
        if(s&&s.owner!== currentUser.id){
          const msg:WsMsg = {
            type:"KeyRequest",
            server_id:id,
            requester_id:currentUser.id,
            requester_pubkey:currentUser.pk
          }
          ws?.send(JSON.stringify(msg))
        }

      }catch {}
    }

    async function selectChannel(id:string){
      currentChannel = id;
      try {
        const res = await fetch(`${BACKEND_URL}/m/${id}`)
        if(!res.ok) return
        const msgs:msgres[] = await res.json()

        const sid = currentServer
        let aesKey:CryptoKey|null = null;
        if (sid && serverKeys[sid]){
          try{
            aesKey = await importAesKey(serverKeys[sid]);
          }catch{}
        }

        const display:displaymsg[] = []

        for (const m of msgs){
          let content = "<RELOAD MAYBE? WAHHH WHY IT NOT WORK YOU, SEND A SCREENSHOT TO ME, FOR BUG FIXING WAAAH, ITS ENCRYPTED, WHYYYYYY, >"
          if(aesKey){
            try{
              content = await aesDecrypt(m.encrypted_content,m.nonce,aesKey);
            }catch {}
          }
          display.push({id:m.id,author_id:m.author_id,content})
        }

        messages = display
      }catch{}
    }

    onMount(()=>{
      loadServers()

      ws = new WebSocket(WS_URL)
      ws.onmessage = async (ev) =>{
        try{
          const parsed:WsMsg =  JSON.parse(ev.data);
          if(parsed.type === "Chat"){
            const {channel_id, author_id,encrypted_content,nonce} = parsed;
            if(currentChannel == channel_id){
              const sid = currentServer
              let aesKey:CryptoKey |null = null;
              if(sid&&serverKeys[sid]){
                try {
                  aesKey = await importAesKey(serverKeys[sid])
                }catch{}
              }

               let content = "< (its live) RELOAD MAYBE? WAHHH WHY IT NOT WORK YOU, SEND A SCREENSHOT TO ME, FOR BUG FIXING WAAAH, ITS ENCRYPTED, WHYYYYYY, > "
               if (aesKey){
                 try{
                   content = await aesDecrypt(encrypted_content,nonce,aesKey)
                 }catch{}
               }

               messages = [
                 ...messages,
                 {id:crypto.randomUUID(),author_id,content}
               ]
            }

          }else if(parsed.type ==="KeyRequest"){
            const {server_id,requester_id,requester_pubkey} = parsed
            const s = servers.find((x)=> x.id === server_id)
            if(s&&s.owner===currentUser.id && requester_id !== currentUser.id){
              if(privatekeyPem){
                const aesB64Cached = serverKeys[server_id];
                let clone = aesB64Cached
                if(!clone){
                  try {
                    const privatekeye = await importPrivateKey(privatekeyPem)
                    const r = await fetch(`${BACKEND_URL}/sk/${server_id}/${currentUser.id}`)
                    if(r.ok){
                      const body = await r.json()
                      clone = await rsaUnwrapKey(body.encrypted_key,privatekeye)
                    }
                  }catch{}
                }

                if (clone) {
                    try {
                        const theirPub = await importPublicKey(requester_pubkey);
                        const wrapped = await rsaWrapKey(clone, theirPub);
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
                        ws?.send(JSON.stringify(notif));
                    } catch {}
                }
              }
            }
          }else if(parsed.type ==="KeyDelivered"){
            const { target_user_id, server_id } = parsed;
            if (target_user_id === currentUser.id) {
              if (privatekeyPem) {
                const aesB64 = await fetchAndLoadServerKey(server_id, currentUser.id, privatekeyPem);
                if (aesB64) {
                  serverKeys[server_id] = aesB64;
                  if (currentServer === server_id && currentChannel) {
                    selectChannel(currentChannel);
                  }
                }
              }
            }
          }
        }catch{}
      }


      return ()=>{
        ws?.close();
      }
    })

    async function createServer(){
      if (!newServerName) return;
      try {
        const res = await fetch(`${BACKEND_URL}/ms`,{
          method:"POST",
          headers:{"Content-Type":"application/json"},
          body:JSON.stringify({
            name:newServerName,
            owner_id:currentUser.id
          }),
        });
        if (res.ok){
          const sidStr = await res.text();
          const sid = sidStr.replace(/"/g,"")
          if (privatekeyPem){
            try {
              const pubKey = await importPublicKey(currentUser.pk)
              const aesKey = await generateAesKey()
              const aesB64 = await exportAesKey(aesKey)
              const wrapped = await rsaWrapKey(aesB64,pubKey);

             await fetch(`${BACKEND_URL}/sk`,{
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                server_id: sid,
                user_id: currentUser.id,
                encrypted_key: wrapped,
              }),
             });

             serverKeys[sid] = aesB64;
            }catch{}
          }
        }
        newServerName = "";
        loadServers()
      }catch{}
    }

    async function joinServer(){
      if(!joinServerId) return;
      try {
        await fetch(`${BACKEND_URL}/js`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ server_id: joinServerId, user_id: currentUser.id }),
        });
        joinServerId = ""
        loadServers();

      }catch{}
    }

    async function createChannel(){
      if (!newChanName|| !currentServer)return;
      try {
        await fetch(`${BACKEND_URL}/mc`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ name: newChanName, server_id: currentServer }),
        });
        newChanName = ""
        selectServer(currentServer);

      }catch{}
    }

    async function sendMessage(ev:Event){
      ev.preventDefault()
      if(!inputText|| !currentChannel|| !currentServer) return;

      try {
        const aesB64 = serverKeys[currentServer]
        if (aesB64){
          const aesKey = await importAesKey(aesB64);
          const {ciphertextB64,nonceB64} = await aesEncrypt(inputText,aesKey);
          const msg:WsMsg = {
              type: "Chat",
              channel_id: currentChannel,
              author_id: currentUser.id,
              encrypted_content: ciphertextB64,
              nonce: nonceB64,
            }
          ws?.send(JSON.stringify(msg));
          inputText = "";
        }
      } catch {}
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
        <h3 class="mb-3">{""} servers</h3>
        <ul class="space-y-1">
          {#each servers as s}
            <li
              class="cursor-pointer"
              onclick={() => selectServer(s.id)}
            >
              {currentServer === s.id ? "> " : "  "}
              {s.name}
            </li>
          {/each}
          <li class="mt-4">
            <input
              class="w-full"
              placeholder="add server"
              bind:value={newServerName}
              onkeydown={(ev) => ev.key === "Enter" && createServer()}
            />
          </li>
          <li class="mt-2">
            <input
              class="w-full"
              placeholder="join SERVER VIA id"
              bind:value={joinServerId}
              onkeydown={(ev) => ev.key === "Enter" && joinServer()}
            />
          </li>
        </ul>
      </section>

      {#if currentServer}
        <section>
          <h3 class="mb-1">{""} channels</h3>
          <div class="mb-3">
            ID: {currentServer}
          </div>
          <ul class="space-y-1">
            {#each channels as c}
              <li
                class="cursor-pointer"
                onclick={() => selectChannel(c.id)}
              >
                {currentChannel === c.id ? "> #" : "  #"}
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
              [{usersInServer.find((u) => u.id === m.author_id)?.username || "Unknown"}]
            </span>
            <span>
              {m.content}
            </span>
          </div>
        {/each}
      </div>

      <div class="p-2">
        <form onsubmit={sendMessage} class="flex items-center">
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
