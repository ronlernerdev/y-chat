<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { fade } from "svelte/transition";

  const dispatch = createEventDispatcher();

  let mode: "signin" | "signup" = "signin";

  let username = "";
  let password = "";
  let remember = false;
  let confirmPassword = "";

  let showPassword = false;
  let showConfirmPassword = false;

  function submit() {
    if (mode === "signin") {
      dispatch("signin", { username, password, remember });
    } else {
      dispatch("signup", { username, password, confirmPassword });
    }
  }
</script>

<div class="grow w-full grid place-items-center min-h-screen bg-white">
  <div class="w-full max-w-md z-10 px-4 py-14">

    <!-- Tabs -->
    <div class="relative flex w-full max-w-max overflow-hidden border border-b-0 border-zinc-200 rounded-t-md">
      <button
        class="relative px-5 py-2 text-sm transition-colors duration-150"
        class:tab-active={mode === "signin"}
        class:tab-inactive={mode !== "signin"}
        on:click={() => (mode = "signin")}
      >
        Sign In
      </button>
      <button
        class="relative px-5 py-2 text-sm transition-colors duration-150"
        class:tab-active={mode === "signup"}
        class:tab-inactive={mode !== "signup"}
        on:click={() => (mode = "signup")}
      >
        Sign Up
      </button>
    </div>

    <!-- Card -->
    <div class="flex flex-col gap-5 border border-zinc-200 rounded-b-md rounded-tr-md bg-white shadow-sm py-7">

      <!-- Header -->
      <div class="px-6 flex flex-col gap-1">
        <h2 class="font-semibold text-[17px] text-zinc-900 tracking-tight">
          {mode === "signin" ? "Sign In" : "Sign Up"}
        </h2>
        <p class="text-zinc-500 text-[13px]">
          {mode === "signin"
            ? "Enter your username below to login to your account"
            : "Enter your username below to create an account"}
        </p>
      </div>

      <!-- Form -->
      <div class="px-6">
        {#key mode}
          <form
            in:fade={{ duration: 120 }}
            class="grid gap-4"
            on:submit|preventDefault={submit}
          >

            <!-- Username -->
            <div class="grid gap-1.5">
              <label class="text-[13px] font-medium text-zinc-800" for="username">Username</label>
              <input
                id="username"
                type="text"
                placeholder="johndoe"
                class="input"
                bind:value={username}
                required
              />
            </div>

            <!-- Password -->
            <div class="grid gap-1.5">
              <label class="text-[13px] font-medium text-zinc-800" for="password">Password</label>
              <div class="relative">
                <input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  placeholder="Password"
                  autocomplete="current-password"
                  class="input pr-10"
                  bind:value={password}
                  required
                />
                <button
                  type="button"
                  class="eye-btn"
                  on:click={() => (showPassword = !showPassword)}
                  aria-label={showPassword ? "Hide password" : "Show password"}
                >
                  {#if showPassword}
                    <!-- Eye Off -->
                    <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                      <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                      <line x1="1" y1="1" x2="23" y2="23"/>
                    </svg>
                  {:else}
                    <!-- Eye -->
                    <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                      <circle cx="12" cy="12" r="3"/>
                    </svg>
                  {/if}
                </button>
              </div>
            </div>

            <!-- Confirm Password (signup only) -->
            {#if mode === "signup"}
              <div class="grid gap-1.5" in:fade={{ duration: 100 }}>
                <label class="text-[13px] font-medium text-zinc-800" for="confirm-password">
                  Confirm Password
                </label>
                <div class="relative">
                  <input
                    id="confirm-password"
                    type={showConfirmPassword ? "text" : "password"}
                    placeholder="Confirm Password"
                    autocomplete="new-password"
                    class="input pr-10"
                    bind:value={confirmPassword}
                    required
                  />
                  <button
                    type="button"
                    class="eye-btn"
                    on:click={() => (showConfirmPassword = !showConfirmPassword)}
                    aria-label={showConfirmPassword ? "Hide password" : "Show password"}
                  >
                    {#if showConfirmPassword}
                      <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                        <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                        <line x1="1" y1="1" x2="23" y2="23"/>
                      </svg>
                    {:else}
                      <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                        <circle cx="12" cy="12" r="3"/>
                      </svg>
                    {/if}
                  </button>
                </div>
              </div>
            {/if}

            <!-- Remember me (signin only) -->
            {#if mode === "signin"}
              <div class="flex items-center gap-2" in:fade={{ duration: 100 }}>
                <input
                  id="remember"
                  type="checkbox"
                  class="h-4 w-4 rounded border-zinc-300 accent-zinc-900 cursor-pointer"
                  bind:checked={remember}
                />
                <label for="remember" class="text-[13px] text-zinc-700 cursor-pointer select-none">
                  Remember me
                </label>
              </div>
            {/if}

            <!-- Submit -->
            <button
              type="submit"
              class="mt-1 h-9 w-full rounded-md bg-zinc-900 text-white text-[13px] font-medium tracking-wide hover:bg-zinc-800 active:bg-zinc-950 transition-colors duration-150 shadow-sm"
            >
              {mode === "signin" ? "Login" : "Create an account"}
            </button>

          </form>
        {/key}
      </div>

      <!-- Footer -->
      <!-- <div class="px-6 pt-2 border-t border-zinc-100 mt-1">
        <p class="text-center text-zinc-400 text-[12px] leading-relaxed">
          By signing {mode === "signin" ? "in" : "up"}, you agree to the
          <a href="/terms" class="underline underline-offset-2 text-zinc-500 hover:text-zinc-700 transition-colors">Terms of Service</a>
          and
          <a href="/privacy" class="underline underline-offset-2 text-zinc-500 hover:text-zinc-700 transition-colors">Privacy Policy</a>.
        </p>
      </div> -->

    </div>
  </div>
</div>

<style>
  .input {
    display: flex;
    height: 2.125rem;
    width: 100%;
    border-radius: 0.375rem;
    border: 1px solid #e4e4e7;
    background: transparent;
    padding: 0 0.75rem;
    font-size: 13px;
    color: #18181b;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .input::placeholder {
    color: #a1a1aa;
  }

  .input:focus {
    border-color: #a1a1aa;
    box-shadow: 0 0 0 3px rgba(24, 24, 27, 0.06);
  }

  .eye-btn {
    position: absolute;
    right: 0.625rem;
    top: 50%;
    transform: translateY(-50%);
    color: #a1a1aa;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s;
  }

  .eye-btn:hover {
    color: #52525b;
  }

  .tab-active {
    background: white;
    color: #18181b;
    font-weight: 500;
  }

  .tab-inactive {
    background: #fafafa;
    color: #71717a;
    font-weight: 400;
  }

  .tab-inactive:hover {
    color: #3f3f46;
    background: #f4f4f5;
  }
</style>
