<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import MainMenu from './lib/MainMenu.svelte';
  import SlimeMoldMode from './lib/SlimeMoldMode.svelte';

  type AppMode = 'menu' | 'slime-mold';
  let currentMode: AppMode = 'menu';

  function navigateToMode(mode: AppMode) {
    currentMode = mode;
  }

  function returnToMenu() {
    currentMode = 'menu';
  }

  // Handle window resize events
  async function handleResize() {
    try {
      // Convert logical pixels to physical pixels using device pixel ratio
      const devicePixelRatio = window.devicePixelRatio || 1;
      const logicalWidth = window.innerWidth;
      const logicalHeight = window.innerHeight;
      const physicalWidth = Math.round(logicalWidth * devicePixelRatio);
      const physicalHeight = Math.round(logicalHeight * devicePixelRatio);
      
      console.log(`Resize: Logical ${logicalWidth}x${logicalHeight}, Physical ${physicalWidth}x${physicalHeight}, DPR ${devicePixelRatio}`);
      
      await invoke('handle_window_resize', { 
        width: physicalWidth, 
        height: physicalHeight 
      });
    } catch (e) {
      console.error('Failed to handle window resize:', e);
    }
  }

  onMount(() => {
    // Listen for resize events
    window.addEventListener('resize', handleResize);
    
    // Send initial size
    handleResize();

    return () => {
      window.removeEventListener('resize', handleResize);
    };
  });
</script>

<main>
  {#if currentMode === 'menu'}
    <MainMenu on:navigate={(e) => navigateToMode(e.detail)} />
  {:else if currentMode === 'slime-mold'}
    <SlimeMoldMode on:back={returnToMenu} />
  {/if}
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