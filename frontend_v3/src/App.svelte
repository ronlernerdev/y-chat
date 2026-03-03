<script lang="ts">
  import { onMount } from "svelte";
  import Chat from "./lib/Chat.svelte";
  import {
    generateKeyPair,
    exportPublicKey,
    exportPrivateKey,
    encryptPrivateKey,
    decryptPrivateKey
  } from "./lib/crypto";
  import { BACKEND_URL } from "./lib/config";
  import type { User, LoginResponse } from "./lib/types";

  let user = $state<User | null>(null);
  let privkeyPem = $state<string | null>(null);

  let reg_un = $state("");
  let reg_pw = $state("");

  let log_un = $state("");
  let log_pw = $state("");

  let status = $state("");

  onMount(() => {
    try {
      const savedUserStr = localStorage.getItem("dih-user");
      if (savedUserStr) {
        const savedUser: User = JSON.parse(savedUserStr);
        user = savedUser;
        const savedPrivkey = localStorage.getItem(`private_key_ig_${savedUser.id}`);
        if (savedPrivkey) {
          privkeyPem = savedPrivkey;
        }
      }
    } catch (e) {
      console.error("COULDNT RESTORE USER WAHAAH", e);
    }
  });
  async function makeaccount(){
    status = "gen"

    try {
      const {publicKey, privateKey} = await generateKeyPair();
      const pub = await exportPublicKey(publicKey);
      const privatey = await exportPrivateKey(privateKey);

      const password = reg_pw
      let encPk, salt, iv
      try {
        const encrypted = await encryptPrivateKey(privatey, password)
        encPk = encrypted.encB64
        salt = encrypted.saltB64
        iv = encrypted.ivB64
      } catch (e) {
        status = "Key encryption failed"
        return
      }

      const body = {
        un: reg_un,
        pw: password,
        pk: pub,
        av: null,
        encrypted_privkey: encPk,
        privkey_salt: salt,
        privkey_iv: iv,
      };

      const resp = await fetch(`${BACKEND_URL}/r`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (resp.ok){
        const loginresponse:LoginResponse = await resp.json();
        const usery:User = {id:loginresponse.id,username:loginresponse.un,pk:loginresponse.pk}
        localStorage.setItem(`private_key_ig_${usery.id}`, privatey);
        localStorage.setItem("dih-user", JSON.stringify(usery));

        privkeyPem = privatey
        user = usery
      }else {
        status = "failed"
      }

    }catch (e){
      status = "RIP, ENCRYPTION FAIL"
      return
    }
  }


  async function dologin(){
    status = "logging";
    try {
    const password = log_pw;
    const body = {
      un:log_un,
      pw:password,
    }

    const resp = await fetch(`${BACKEND_URL}/l`,{
      method:"POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    })

    if(resp.ok){
      const loginresponse:LoginResponse = await resp.json();
      const usery:User = {id:loginresponse.id,username:loginresponse.un,pk:loginresponse.pk};

      localStorage.setItem("dih-user",JSON.stringify(usery));
      if (loginresponse.encrypted_privkey && loginresponse.privkey_salt && loginresponse.privkey_iv) {
          try{
            const privatey = await decryptPrivateKey(
              loginresponse.encrypted_privkey,
              loginresponse.privkey_salt,
              loginresponse.privkey_iv,
              password
            );
            localStorage.setItem(`private_key_ig_${usery.id}`, privatey);
            privkeyPem = privatey

          }catch (e) {
            status = "ur a failure, or maybe i am?";
            localStorage.removeItem("y_user");
            return;
          }
        }else {
          const savedPem = localStorage.getItem(`private_key_ig_${usery.id}`);
          if (savedPem) {
            privkeyPem = savedPem;
          }
        }

        user = usery
    } else {
      status = "rare message unlocked, send a screenshot to me for 10 dollah, the screenshot should also include the terminal.";
    }
    }catch (e){
      status = "your dumbass wifi, or maybe mine?"
    }
  }

  function handleKeyRecovered(pem: string) {
    if (user) {
      localStorage.setItem(`private_key_ig_${user.id}`, pem);
      privkeyPem = pem;
    }
  }
</script>

{#if user}
<Chat currentUser={user} storedPrivkeyPem={privkeyPem} onKeyRecovered={handleKeyRecovered} />
{:else}
<div class="min-h-screen flex items-center justify-center p-4">
  <div class="w-full max-w-sm p-8">
    <h1 class="mb-8 text-center">y-chat</h1>

    <div class="space-y-6">
      <section>
        <h2 class="mb-2">Join</h2>
        <input
          class="w-full"
          placeholder="username"
          bind:value={reg_un}
        />
        <input
          type="password"
          class="w-full mt-2"
          placeholder="password"
          bind:value={reg_pw}
        />
        <button
          class="w-full mt-2"
          onclick={makeaccount}
        >
          CREATE ACCOUNT
        </button>
      </section>

      <div class="relative">
        <span class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2">
          or
        </span>
      </div>

      <section>
        <h2 class="mb-2">Log In</h2>
        <input
          class="w-full"
          placeholder="username"
          bind:value={log_un}
        />
        <input
          type="password"
          class="w-full mt-2"
          placeholder="password"
          bind:value={log_pw}
        />
        <button
          class="w-full mt-2"
          onclick={dologin}
        >
          LOG IN
        </button>
      </section>
    </div>

    <p class="mt-6 text-center">{status}</p>
  </div>
</div>
{/if}
