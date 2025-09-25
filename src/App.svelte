<main>
    {#if currentMode === 'menu'}
        <MainMenu on:navigate={handleNavigation} />
    {:else if currentMode === 'slime-mold'}
        <SlimeMoldMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'gray-scott'}
        <GrayScottMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'particle-life'}
        <ParticleLifeMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'flow'}
        <FlowMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'pellets'}
        <PelletsMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'voronoi-ca'}
        <VoronoiCAMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'moire'}
        <MoireMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'primordial-particles'}
        <PrimordialParticlesMode
            menuPosition={appSettings.menu_position}
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'gradient-editor'}
        <GradientEditorMode
            autoHideDelay={appSettings.auto_hide_delay}
            on:back={goBack}
            on:navigate={handleNavigation}
        />
    {:else if currentMode === 'how-to-play'}
        <HowToPlay on:back={goBack} />
    {:else if currentMode === 'settings'}
        <Settings
            on:back={goBack}
            on:settingsChanged={async (e) => {
                appSettings = e.detail;
                await applyUIScale(appSettings.ui_scale);
                await applyCameraSensitivity(appSettings.default_camera_sensitivity);
            }}
        />
    {/if}
</main>

<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import MainMenu from './lib/MainMenu.svelte';
    import SlimeMoldMode from './lib/SlimeMoldMode.svelte';
    import GrayScottMode from './lib/GrayScottMode.svelte';
    import ParticleLifeMode from './lib/ParticleLifeMode.svelte';
    import FlowMode from './lib/FlowMode.svelte';
    import PelletsMode from './lib/PelletsMode.svelte';
    import GradientEditorMode from './lib/GradientEditorMode.svelte';
    import VoronoiCAMode from './lib/VoronoiCAMode.svelte';
    import MoireMode from './lib/MoireMode.svelte';
    import PrimordialParticlesMode from './lib/PrimordialParticlesMode.svelte';

    import HowToPlay from './lib/HowToPlay.svelte';
    import Settings from './lib/Settings.svelte';
    import type { AppMode } from './types/app';

    type MenuPosition = 'left' | 'right' | 'middle';

    interface AppSettings {
        ui_scale: number;
        default_fps_limit: number;
        default_fps_limit_enabled: boolean;
        window_width: number;
        window_height: number;
        window_maximized: boolean;
        auto_hide_ui: boolean;
        auto_hide_delay: number;
        menu_position: MenuPosition;
        default_camera_sensitivity: number;
    }

    let currentMode: AppMode = 'menu';
    let previousMode: AppMode | null = null;

    // App settings for UI scaling
    let appSettings: AppSettings = {
        ui_scale: 1.0,
        default_fps_limit: 60,
        default_fps_limit_enabled: false,
        window_width: 1200,
        window_height: 800,
        window_maximized: false,
        auto_hide_ui: true,
        auto_hide_delay: 3000,
        menu_position: 'middle',
        default_camera_sensitivity: 1.0,
    };

    // Load app settings and apply UI scale
    async function loadAppSettings() {
        try {
            const settings = (await invoke('get_app_settings')) as AppSettings;
            if (settings) {
                appSettings = settings;
                await applyUIScale(appSettings.ui_scale);
                // Apply camera sensitivity
                await applyCameraSensitivity(appSettings.default_camera_sensitivity);
                // Apply window settings on startup (including maximized state)
                await invoke('apply_window_settings_on_startup');
            }
        } catch (e) {
            console.error('Failed to load app settings:', e);
        }
    }

    // Apply UI scale using webview zoom
    async function applyUIScale(scale: number) {
        try {
            // Use webview zoom instead of CSS scaling
            await invoke('set_webview_zoom', { zoomFactor: scale });
            console.log('Webview zoom applied:', scale);
        } catch (e) {
            console.error('Failed to apply webview zoom:', e);
            // Fallback to CSS scaling if webview zoom fails
            const root = document.documentElement;
            root.style.fontSize = `${16 * scale}px`;
            root.style.setProperty('--ui-scale', scale.toString());
        }
    }

    // Apply camera sensitivity
    async function applyCameraSensitivity(sensitivity: number) {
        try {
            await invoke('set_camera_sensitivity', { sensitivity });
            console.log('Camera sensitivity applied:', sensitivity);
        } catch (e) {
            console.error('Failed to apply camera sensitivity:', e);
        }
    }

    async function handleNavigation(event: CustomEvent<AppMode>) {
        // Store the current mode as previous before navigating
        // Always store the current mode as previous, regardless of what it is
        previousMode = currentMode;

        // If navigating to menu, reset graphics resources
        if (event.detail === 'menu') {
            try {
                await invoke('reset_graphics_resources');
                console.log('Graphics resources reset successfully');
            } catch (error) {
                console.error('Failed to reset graphics resources:', error);
                // Continue with navigation even if reset fails
            }
        }

        currentMode = event.detail;
    }

    async function returnToMenu() {
        try {
            // Reset graphics resources to prevent crashed simulations from poisoning the state
            await invoke('reset_graphics_resources');
            console.log('Graphics resources reset successfully');
        } catch (error) {
            console.error('Failed to reset graphics resources:', error);
            // Continue with navigation even if reset fails
        }

        currentMode = 'menu';
        previousMode = null;
    }

    async function goBack() {
        if (previousMode) {
            currentMode = previousMode;
            previousMode = null;
        } else {
            // Fallback to menu if no previous mode
            await returnToMenu();
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

            console.log(
                `Resize: Logical ${logicalWidth}x${logicalHeight}, Physical ${physicalWidth}x${physicalHeight}, DPR ${devicePixelRatio}`
            );

            await invoke('handle_window_resize', {
                width: physicalWidth,
                height: physicalHeight,
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

    // Global keyboard event handler for fullscreen toggle
    async function handleGlobalKeyDown(event: KeyboardEvent) {
        // Check if user is focused on a form element - if so, don't process global shortcuts
        const activeElement = document.activeElement;
        const isInputFocused =
            activeElement &&
            (activeElement.tagName === 'INPUT' ||
                activeElement.tagName === 'TEXTAREA' ||
                activeElement.tagName === 'SELECT' ||
                (activeElement as HTMLElement).contentEditable === 'true');

        if (isInputFocused) {
            return; // Let the form element handle the keyboard input
        }

        // Check for fullscreen toggle shortcuts
        // Windows/Linux: Alt+Enter, macOS: Cmd+F
        if ((event.key === 'Enter' && event.altKey) || (event.key === 'f' && event.metaKey)) {
            event.preventDefault();
            try {
                await invoke('toggle_fullscreen');
            } catch (error) {
                console.error('Failed to toggle fullscreen:', error);
            }
        }
    }

    onMount(() => {
        // Load app settings first
        loadAppSettings();

        // Listen for resize events
        window.addEventListener('resize', debouncedResize);

        // Listen for global keyboard events
        document.addEventListener('keydown', handleGlobalKeyDown);

        // Send initial size
        debouncedResize();

        return () => {
            window.removeEventListener('resize', debouncedResize);
            document.removeEventListener('keydown', handleGlobalKeyDown);
        };
    });
</script>

<style>
    :global(body) {
        margin: 0;
        padding: 0;
        font-family: 'Zelda Sans', Inter, Avenir, Helvetica, Arial, sans-serif;
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
