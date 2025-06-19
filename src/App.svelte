<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import MainMenu from './lib/MainMenu.svelte';
  import SlimeMoldMode from './lib/SlimeMoldMode.svelte';
  import GrayScottMode from './lib/GrayScottMode.svelte';
  import ParticleLifeMode from './lib/ParticleLifeMode.svelte';

  type AppMode = 'menu' | 'slime-mold' | 'gray-scott' | 'particle-life';
  let currentMode: AppMode = 'menu';

  function navigateToMode(mode: AppMode) {
    currentMode = mode;
  }

  function returnToMenu() {
    currentMode = 'menu';
  }

  // Handle window resize events
  let resizeTimeout: number | null = null;
  let lastResizeTime = 0;
  const resizeDebounceDelay = 500; // 100ms debounce

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

  function debouncedResize() {
    const now = Date.now();
    
    // Clear existing timeout
    if (resizeTimeout) {
      clearTimeout(resizeTimeout);
    }
    
    // Debounce rapid resize events
    if (now - lastResizeTime < resizeDebounceDelay) {
      resizeTimeout = setTimeout(() => {
        handleResize();
        lastResizeTime = Date.now();
      }, resizeDebounceDelay);
    } else {
      // If enough time has passed, handle immediately
      handleResize();
      lastResizeTime = now;
    }
  }

  onMount(() => {
    // Listen for resize events
    window.addEventListener('resize', debouncedResize);
    
    // Send initial size
    debouncedResize();

    return () => {
      window.removeEventListener('resize', debouncedResize);
    };
  });
</script>

<main>
  {#if currentMode === 'menu'}
    <MainMenu on:navigate={(e) => navigateToMode(e.detail)} />
  {:else if currentMode === 'slime-mold'}
    <SlimeMoldMode on:back={returnToMenu} />
  {:else if currentMode === 'gray-scott'}
    <GrayScottMode on:back={returnToMenu} />
  {:else if currentMode === 'particle-life'}
    <ParticleLifeMode on:back={returnToMenu} />
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
</style> 