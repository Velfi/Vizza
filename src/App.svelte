<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import MainMenu from './lib/MainMenu.svelte';
  import SlimeMoldMode from './lib/SlimeMoldMode.svelte';
  import GrayScottMode from './lib/GrayScottMode.svelte';
  import ParticleLifeMode from './lib/ParticleLifeMode.svelte';
  import HowToPlay from './lib/HowToPlay.svelte';
  import Settings from './lib/Settings.svelte';

  type AppMode = 'menu' | 'slime-mold' | 'gray-scott' | 'particle-life' | 'how-to-play' | 'settings';
  let currentMode: AppMode = 'menu';
  let previousMode: AppMode | null = null;

  // App settings for UI scaling
  let appSettings: any = {
    ui_scale: 1.0,
    default_fps_limit: 60,
    default_fps_limit_enabled: false,
    window_width: 1200,
    window_height: 800,
    window_maximized: false,
    auto_hide_ui: true,
    auto_hide_delay: 3000,
    default_camera_sensitivity: 1.0,
  };

  // Load app settings and apply UI scale
  async function loadAppSettings() {
    try {
      const settings = await invoke('get_app_settings');
      if (settings) {
        appSettings = settings;
        await applyUIScale(appSettings.ui_scale);
      }
    } catch (e) {
      console.error('Failed to load app settings:', e);
    }
  }

  // Apply UI scale using webview zoom
  async function applyUIScale(scale: number) {
    try {
      // Use webview zoom instead of CSS scaling
      await invoke('set_webview_zoom', { zoom_factor: scale });
      console.log('Webview zoom applied:', scale);
    } catch (e) {
      console.error('Failed to apply webview zoom:', e);
      // Fallback to CSS scaling if webview zoom fails
      const root = document.documentElement;
      root.style.fontSize = `${16 * scale}px`;
      root.style.setProperty('--ui-scale', scale.toString());
    }
  }

  function navigateToMode(mode: AppMode) {
    // Store the current mode as previous before navigating
    // Always store the current mode as previous, regardless of what it is
    previousMode = currentMode;
    currentMode = mode;
  }

  function returnToMenu() {
    currentMode = 'menu';
    previousMode = null;
  }

  function goBack() {
    if (previousMode) {
      currentMode = previousMode;
      previousMode = null;
    } else {
      // Fallback to menu if no previous mode
      returnToMenu();
    }
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
    // Load app settings first
    loadAppSettings();
    
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
    <SlimeMoldMode on:back={goBack} on:navigate={(e) => navigateToMode(e.detail)} />
  {:else if currentMode === 'gray-scott'}
    <GrayScottMode on:back={goBack} on:navigate={(e) => navigateToMode(e.detail)} />
  {:else if currentMode === 'particle-life'}
    <ParticleLifeMode on:back={goBack} on:navigate={(e) => navigateToMode(e.detail)} />
  {:else if currentMode === 'how-to-play'}
    <HowToPlay on:back={goBack} />
  {:else if currentMode === 'settings'}
    <Settings on:back={goBack} on:settingsChanged={async (e) => {
      appSettings = e.detail;
      await applyUIScale(appSettings.ui_scale);
    }} />
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