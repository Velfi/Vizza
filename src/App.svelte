<script lang="ts">
  import { message } from '@tauri-apps/plugin-dialog';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let greetInput = '';
  let greetMsg = '';

  async function openExternalLink(url: string) {
    try {
      await openUrl(url);
    } catch (error) {
      console.error('Failed to open link:', error);
      await message('Failed to open link', { title: 'Error', kind: 'error' });
    }
  }

  async function showAlert(msg: string, title = 'Alert') {
    try {
      await message(msg, { title, kind: 'info' });
    } catch (error) {
      console.error('Failed to show alert:', error);
    }
  }

  async function greet(event: Event) {
    event.preventDefault();
    if (greetInput) {
      greetMsg = `Hello, ${greetInput}!`;
      await showAlert(`Hello, ${greetInput}! Welcome to Tauri with Svelte!`);
    }
  }

  async function testAlert() {
    await showAlert('This is a test alert from Tauri with Svelte!', 'Test Alert');
  }

  onMount(() => {
    let running = true;
    async function renderLoop() {
      if (!running) return;
      try {
        await invoke('render_frame');
      } catch (e) {
        // Optionally handle errors
        console.error(e);
      }
      requestAnimationFrame(renderLoop);
    }

    // Start the simulation, then start the render loop
    invoke('start_slime_mold_simulation').then(() => {
      renderLoop();
    });

    return () => {
      running = false;
    };
  });

  // Make functions available globally
  if (typeof window !== 'undefined') {
    (window as any).showTauriAlert = showAlert;
    (window as any).openTauriLink = openExternalLink;
  }
</script>

<main>
  <div class="container">
    <h1>Welcome to Tauri + Svelte!</h1>

    <div class="row">
      <button class="logo-button" on:click={() => openExternalLink('https://vitejs.dev')}>
        <img src="/src/assets/vite.svg" class="logo vite" alt="Vite logo" />
      </button>
      <button class="logo-button" on:click={() => openExternalLink('https://tauri.app')}>
        <img src="/src/assets/tauri.svg" class="logo tauri" alt="Tauri logo" />
      </button>
      <button class="logo-button" on:click={() => openExternalLink('https://www.typescriptlang.org/docs')}>
        <img src="/src/assets/typescript.svg" class="logo typescript" alt="TypeScript logo" />
      </button>
    </div>

    <p>Click on the logos to learn more about each framework</p>

    <form class="row" on:submit={greet}>
      <input bind:value={greetInput} placeholder="Enter a name..." />
      <button type="submit">Greet</button>
    </form>

    {#if greetMsg}
      <p id="greet-msg">{greetMsg}</p>
    {/if}

    <button class="test-button" on:click={testAlert}>Test Alert</button>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: "Zelda Sans", Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 24px;
    font-weight: 400;
    color-scheme: light dark;
    color: rgba(255, 255, 255, 0.87);
    background-color: transparent;
    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
  }

  .container {
    margin: 0;
    padding-top: 10vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
  }

  .logo {
    height: 6em;
    padding: 1.5em;
    will-change: filter;
    transition: 0.75s;
  }

  .logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
  }

  .logo.tauri:hover {
    filter: drop-shadow(0 0 2em #24c8db);
  }

  .logo.typescript:hover {
    filter: drop-shadow(0 0 2em #2d79c7);
  }

  .logo-button {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    margin: 0;
  }

  .row {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    margin: 1rem 0;
  }

  h1 {
    text-align: center;
    margin-bottom: 2rem;
  }

  input {
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 1rem;
  }

  button {
    padding: 0.5rem 1rem;
    border: 1px solid #646cff;
    border-radius: 4px;
    background-color: #646cff;
    color: white;
    cursor: pointer;
    font-size: 1rem;
    transition: background-color 0.3s;
  }

  button:hover {
    background-color: #535bf2;
  }

  .test-button {
    margin-top: 1rem;
    background-color: #ff6b35;
    border-color: #ff6b35;
  }

  .test-button:hover {
    background-color: #e85a2b;
  }

  #greet-msg {
    margin-top: 1rem;
    font-weight: bold;
    color: #646cff;
  }

  p {
    margin: 1rem 0;
  }
</style> 