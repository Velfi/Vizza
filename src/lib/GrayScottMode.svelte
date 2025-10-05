<SimulationLayout
    simulationName="Gray-Scott"
    {running}
    loading={loading || !settings}
    {showUI}
    {currentFps}
    {controlsVisible}
    {menuPosition}
    on:back={returnToMenu}
    on:toggleUI={toggleBackendGui}
    on:pause={stopSimulation}
    on:resume={resumeSimulation}
    on:userInteraction={() => autoHideManager?.handleUserInteraction()}
    on:mouseEvent={handleMouseEvent}
>
    {#if settings}
        <form on:submit|preventDefault>
            <!-- About this simulation -->
            <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
                <p>
                    The Gray-Scott simulation demonstrates reaction-diffusion patterns that occur in
                    chemical and biological systems. Two virtual chemicals, U and V, interact and
                    diffuse through space, creating complex, self-organizing patterns.
                </p>
                <p>
                    The simulation is governed by reaction-diffusion equations with feed and kill
                    rates that determine how the chemicals interact. Different parameter
                    combinations produce dramatically different patterns - from spots and stripes to
                    spirals and labyrinthine structures.
                </p>
                <p>
                    Click to seed reactions, adjust the parameters to explore different behaviors,
                    and watch as simple chemical rules generate intricate, ever-changing patterns
                    reminiscent of natural phenomena like coral growth, bacterial colonies, and
                    animal coat patterns.
                </p>
            </CollapsibleFieldset>

            <!-- Preset Controls -->
            <PresetFieldset
                availablePresets={available_presets}
                bind:currentPreset={current_preset}
                placeholder="Select preset..."
                on:presetChange={({ detail }) => updatePreset(detail.value)}
                on:presetSave={({ detail }) => savePreset(detail.name)}
            />

            <!-- Display Settings -->
            <fieldset>
                <legend>Display Settings</legend>
                <div class="control-group">
                    <ColorSchemeSelector
                        bind:available_color_schemes={available_luts}
                        current_color_scheme={state?.current_color_scheme}
                        reversed={state?.color_scheme_reversed || false}
                        on:select={({ detail }) => updateLut(detail.name)}
                        on:reverse={() => updateLutReversed()}
                    />
                </div>
            </fieldset>

            <!-- Post Processing -->
            <PostProcessingMenu simulationType="gray_scott" />

            <!-- Controls -->
            <ControlsPanel
                mouseInteractionText="🖱️ Left click: Seed reaction | Right click: Erase"
                cursorSize={state?.cursor_size}
                cursorStrength={state?.cursor_strength}
                on:cursorSizeChange={async (e) => {
                    const result = await syncManager.updateStateOptimistic(
                        state,
                        'cursor_size',
                        e.detail,
                        true // sync from backend after update
                    );
                    if (result) state = result;
                }}
                on:cursorStrengthChange={async (e) => {
                    const result = await syncManager.updateStateOptimistic(
                        state,
                        'cursor_strength',
                        e.detail,
                        true // sync from backend after update
                    );
                    if (result) state = result;
                }}
                on:navigate={(e) => dispatch('navigate', e.detail)}
            />

            <!-- Settings -->
            <fieldset>
                <legend>Settings</legend>
                <div class="control-group">
                    <Button
                        variant="warning"
                        type="button"
                        on:click={async () => {
                            try {
                                await invoke('reset_simulation');
                                console.log('Simulation reset successfully');
                            } catch (e) {
                                console.error('Failed to reset simulation:', e);
                            }
                        }}>🔄 Reset Simulation</Button
                    >
                    <Button
                        variant="warning"
                        type="button"
                        on:click={async () => {
                            try {
                                await invoke('randomize_settings');
                                await syncSettingsFromBackend();
                                console.log('Settings randomized successfully');
                            } catch (e) {
                                console.error('Failed to randomize settings:', e);
                            }
                        }}>🎲 Randomize Settings</Button
                    >
                    <Button
                        variant="primary"
                        type="button"
                        on:click={async () => {
                            try {
                                await invoke('seed_random_noise');
                                console.log('Random noise seeded successfully');
                            } catch (e) {
                                console.error('Failed to seed random noise:', e);
                            }
                        }}>🌱 Seed Noise</Button
                    >
                </div>
            </fieldset>

            <!-- Reaction-Diffusion -->
            <fieldset>
                <legend>Reaction-Diffusion</legend>
                <GrayScottDiagram
                    feedRate={settings.feed_rate}
                    killRate={settings.kill_rate}
                    diffusionRateU={settings.diffusion_rate_u}
                    diffusionRateV={settings.diffusion_rate_v}
                    timestep={settings.timestep}
                    on:update={async (e) => {
                        console.log('GrayScottDiagram update event:', e.detail);
                        try {
                            const settingName = e.detail.setting as keyof Settings;
                            const value = e.detail.value as number;
                            if (!settings) return;

                            const updated: Settings = { ...settings };

                            switch (settingName) {
                                case 'feed_rate':
                                    updated.feed_rate = value;
                                    break;
                                case 'kill_rate':
                                    updated.kill_rate = value;
                                    break;
                                case 'diffusion_rate_u':
                                    updated.diffusion_rate_u = value;
                                    break;
                                case 'diffusion_rate_v':
                                    updated.diffusion_rate_v = value;
                                    break;
                                case 'timestep':
                                    updated.timestep = value;
                                    break;
                                case 'max_timestep':
                                    updated.max_timestep = Number(value);
                                    break;
                                case 'stability_factor':
                                    updated.stability_factor = Number(value);
                                    break;
                                case 'enable_adaptive_timestep':
                                    updated.enable_adaptive_timestep = Boolean(value);
                                    break;
                            }

                            settings = updated;

                            await invoke('update_simulation_setting', {
                                settingName,
                                value,
                            });
                        } catch (err) {
                            console.error('Failed to update setting:', err);
                        }
                    }}
                />

                <!-- Mask System -->
                <div class="control-group" style="margin-top: 0.5rem;">
                    <Selector
                        label="Mask Pattern"
                        options={mask_pattern_options}
                        value={state?.mask_pattern}
                        placeholder="Select pattern..."
                        on:change={({ detail }) => updateMaskPattern(detail.value)}
                    />
                </div>

                {#if state?.mask_pattern && state.mask_pattern !== 'Disabled'}
                    <div class="control-group">
                        <Selector
                            label="Mask Target"
                            options={mask_target_options}
                            value={state?.mask_target}
                            placeholder="Select target..."
                            on:change={({ detail }) => updateMaskTarget(detail.value)}
                        />
                    </div>
                    <div class="control-group">
                        <label class="checkbox">
                            <input
                                type="checkbox"
                                checked={state?.mask_mirror_horizontal || false}
                                on:change={(e) =>
                                    updateMaskMirrorHorizontal(
                                        (e.target as HTMLInputElement).checked
                                    )}
                            />
                            Mirror horizontal
                        </label>
                    </div>
                    <div class="control-group">
                        <label class="checkbox">
                            <input
                                type="checkbox"
                                checked={state?.mask_mirror_vertical || false}
                                on:change={(e) =>
                                    updateMaskMirrorVertical(
                                        (e.target as HTMLInputElement).checked
                                    )}
                            />
                            Mirror vertical
                        </label>
                    </div>
                    <div class="control-group">
                        <label class="checkbox">
                            <input
                                type="checkbox"
                                checked={state?.mask_invert_tone || false}
                                on:change={(e) =>
                                    updateMaskInvertTone((e.target as HTMLInputElement).checked)}
                            />
                            Invert tone
                        </label>
                    </div>
                    <div class="control-group">
                        <label for="mask-strength"
                            >Mask Strength: {state?.mask_strength?.toFixed(2) || '0.50'}</label
                        >
                        <input
                            id="mask-strength"
                            type="range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={state?.mask_strength || 0.5}
                            on:input={(e) =>
                                updateMaskStrength(
                                    parseFloat((e.target as HTMLInputElement).value)
                                )}
                        />
                    </div>

                    {#if state?.mask_pattern === 'Image'}
                        <ImageSelector
                            fitMode={state?.mask_image_fit_mode || 'Stretch'}
                            loadCommand="load_gray_scott_nutrient_image"
                            onFitModeChange={async (value) => {
                                try {
                                    await invoke('update_simulation_state', {
                                        stateName: 'mask_image_fit_mode',
                                        value: value,
                                    });
                                } catch (err) {
                                    console.error('Failed to update fit mode:', err);
                                }
                            }}
                        />
                        <WebcamControls
                            {webcamDevices}
                            {webcamActive}
                            onStartWebcam={startWebcam}
                            onStopWebcam={stopWebcam}
                        />
                    {/if}
                {/if}
            </fieldset>
        </form>
    {/if}
</SimulationLayout>

<!-- Shared camera controls component -->
<CameraControls enabled={true} on:toggleGui={toggleBackendGui} on:togglePause={togglePause} />

<script lang="ts">
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import ColorSchemeSelector from './components/shared/ColorSchemeSelector.svelte';
    import GrayScottDiagram from './components/gray-scott/GrayScottDiagram.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
    import ControlsPanel from './components/shared/ControlsPanel.svelte';
    import Button from './components/shared/Button.svelte';
    import Selector from './components/inputs/Selector.svelte';
    import ImageSelector from './components/shared/ImageSelector.svelte';
    import WebcamControls from './components/shared/WebcamControls.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';
    import { createSyncManager } from './utils/sync';
    import './shared-theme.css';

    const dispatch = createEventDispatcher();
    // Webcam state
    let webcamDevices: number[] = [];
    let webcamActive = false;

    async function startWebcam() {
        try {
            await invoke('start_gray_scott_webcam_capture');
            webcamActive = true;
        } catch (e) {
            console.error('Failed to start Gray-Scott webcam:', e);
        }
    }

    async function stopWebcam() {
        try {
            await invoke('stop_gray_scott_webcam_capture');
            webcamActive = false;
        } catch (e) {
            console.error('Failed to stop Gray-Scott webcam:', e);
        }
    }

    async function loadGrayScottWebcams() {
        try {
            webcamDevices = await invoke('get_available_gray_scott_webcam_devices');
        } catch (e) {
            console.error('Failed to load Gray-Scott webcam devices:', e);
        }
    }

    export let menuPosition: string = 'middle';
    export let autoHideDelay: number = 3000;

    interface Settings {
        feed_rate: number;
        kill_rate: number;
        diffusion_rate_u: number;
        diffusion_rate_v: number;
        timestep: number;

        // Optimization settings
        max_timestep: number;
        stability_factor: number;
        enable_adaptive_timestep: boolean;
    }

    interface State {
        // Mask system
        mask_pattern: string;
        mask_target: string;
        mask_strength: number;
        mask_reversed: boolean;
        mask_image_fit_mode: string;
        mask_mirror_horizontal: boolean;
        mask_mirror_vertical: boolean;
        mask_invert_tone: boolean;

        // Cursor settings
        cursor_size: number;
        cursor_strength: number;

        // Color scheme state
        current_color_scheme: string;
        color_scheme_reversed: boolean;

        // UI state
        gui_visible: boolean;

        // Camera state
        camera_position: [number, number];
        camera_zoom: number;

        // Mouse interaction state
        mouse_pressed: boolean;
        mouse_position: [number, number];
        mouse_screen_position: [number, number];

        // Simulation runtime state
        simulation_time: number;
        is_running: boolean;
    }

    // Simulation settings (saved in presets)
    let settings: Settings | undefined = undefined;

    // Simulation state (runtime, not saved in presets)
    let state: State | undefined = undefined;

    // Create sync manager for type-safe backend synchronization
    const syncManager = createSyncManager<Settings, State>();

    // Preset and LUT state
    let current_preset = '';
    let available_presets: string[] = [];
    let available_luts: string[] = [];

    // Mask pattern options (display names match backend serialization)
    const mask_pattern_options: string[] = [
        'Disabled',
        'Checkerboard',
        'Diagonal Gradient',
        'Radial Gradient',
        'Vertical Stripes',
        'Horizontal Stripes',
        'Wave Function',
        'Cosine Grid',
        'Image',
    ];

    // Mask target options
    const mask_target_options: string[] = [
        'Feed Rate',
        'Kill Rate',
        'Diffusion U',
        'Diffusion V',
        'UV Concentration',
    ];

    async function updateMaskPattern(value: string) {
        if (!state) return;
        try {
            state = { ...state, mask_pattern: value };
            await invoke('update_simulation_state', {
                stateName: 'mask_pattern',
                value,
            });
        } catch (err) {
            console.error('Failed to set mask pattern:', err);
        }
    }

    async function updateMaskTarget(value: string) {
        if (!state) return;
        try {
            state = { ...state, mask_target: value };
            await invoke('update_simulation_state', {
                stateName: 'mask_target',
                value,
            });
        } catch (err) {
            console.error('Failed to set mask target:', err);
        }
    }

    async function updateMaskStrength(value: number) {
        if (!state) return;
        try {
            state = { ...state, mask_strength: value };
            await invoke('update_simulation_state', {
                stateName: 'mask_strength',
                value,
            });
        } catch (err) {
            console.error('Failed to set mask strength:', err);
        }
    }

    async function updateMaskMirrorHorizontal(checked: boolean) {
        if (!state) return;
        try {
            state = { ...state, mask_mirror_horizontal: checked };
            await invoke('update_simulation_state', {
                stateName: 'mask_mirror_horizontal',
                value: checked,
            });
        } catch (err) {
            console.error('Failed to toggle mirror horizontal:', err);
        }
    }

    async function updateMaskMirrorVertical(checked: boolean) {
        if (!state) return;
        try {
            state = { ...state, mask_mirror_vertical: checked };
            await invoke('update_simulation_state', {
                stateName: 'mask_mirror_vertical',
                value: checked,
            });
        } catch (err) {
            console.error('Failed to toggle mirror vertical:', err);
        }
    }

    async function updateMaskInvertTone(checked: boolean) {
        if (!state) return;
        try {
            state = { ...state, mask_invert_tone: checked };
            await invoke('update_simulation_state', {
                stateName: 'mask_invert_tone',
                value: checked,
            });
        } catch (err) {
            console.error('Failed to toggle invert tone:', err);
        }
    }

    // removed generic updateSetting used previously for resolution scale

    // UI state
    let show_about_section = false;

    // Auto-hide functionality for controls when UI is hidden
    let controlsVisible = true;

    // Auto-hide manager
    let autoHideManager: AutoHideManager;
    let eventListeners: { add: () => void; remove: () => void };

    async function updateLutReversed() {
        try {
            await invoke('toggle_color_scheme_reversed');
            const synced = await syncManager.syncAll();
            if (synced.settings) settings = synced.settings;
            if (synced.state) state = synced.state;
        } catch (e) {
            console.error('Failed to toggle color scheme reversed:', e);
        }
    }

    async function updatePreset(value: string) {
        current_preset = value;
        try {
            await invoke('apply_preset', { presetName: value });
            const synced = await syncManager.syncAll();
            if (synced.settings) settings = synced.settings;
            if (synced.state) state = synced.state;
            console.log(`Applied preset: ${value}`);
        } catch (e) {
            console.error('Failed to apply preset:', e);
        }
    }

    async function savePreset(presetName: string) {
        try {
            await invoke('save_preset', { presetName: presetName.trim() });
            // Refresh the available presets list
            await getPresets();
            // Set the current preset to the newly saved one
            current_preset = presetName.trim();
            console.log(`Saved preset: ${presetName}`);
        } catch (e) {
            console.error('Failed to save preset:', e);
        }
    }

    // Simulation state
    let running = false;
    let loading = false;
    let currentFps = 0;
    let showUI = true;

    async function startSimulation() {
        if (running || loading) return;

        loading = true;

        try {
            await invoke('start_gray_scott_simulation');
            // Don't set running = true here - wait for simulation-initialized event
            // The simulation-initialized event will set running = true when everything is ready
            currentFps = 0;
        } catch (e) {
            console.error('Failed to start simulation:', e);
            loading = false;
            running = false;
        }
    }

    async function resumeSimulation() {
        if (running || loading) return;

        try {
            // Just restart the render loop without recreating the simulation
            await invoke('resume_simulation');
            running = true;
            currentFps = 0;

            // Update auto-hide manager state and handle resume
            if (autoHideManager) {
                autoHideManager.updateState({ running });
                autoHideManager.handleResume();
            }
        } catch (e) {
            console.error('Failed to resume simulation:', e);
        }
    }

    async function stopSimulation() {
        running = false;

        try {
            // Just pause the render loop, don't destroy simulation
            await invoke('pause_simulation');

            // Reset FPS
            currentFps = 0;

            // Immediately render a frame to show the triangle instead of last simulation frame
            await invoke('render_frame');

            // Update auto-hide manager state and handle pause
            if (autoHideManager) {
                autoHideManager.updateState({ running });
                autoHideManager.handlePause();
            }
        } catch (e) {
            console.error('Failed to stop simulation:', e);
        }
    }

    async function togglePause() {
        if (running) {
            await stopSimulation();
        } else {
            await resumeSimulation();
        }
    }

    async function destroySimulation() {
        running = false;

        try {
            // Actually destroy the simulation completely
            await invoke('destroy_simulation');

            // Reset FPS
            currentFps = 0;

            // Render a frame to show the triangle
            await invoke('render_frame');
        } catch (e) {
            console.error('Failed to destroy simulation:', e);
        }
    }

    async function returnToMenu() {
        await destroySimulation();
        dispatch('back');
    }

    // Load available presets from backend
    let initialPresetApplied = false;
    async function getPresets() {
        try {
            available_presets = await invoke('get_presets_for_simulation_type', {
                simulationType: 'gray_scott',
            });
            // Only apply initial preset once on first load, not on subsequent calls
            if (available_presets.length > 0 && !current_preset && !initialPresetApplied) {
                current_preset = available_presets.includes('Undulating')
                    ? 'Undulating'
                    : available_presets[0];
                // Apply the initial preset to the simulation
                await invoke('apply_preset', { presetName: current_preset });
                initialPresetApplied = true;
                console.log(`Applied initial preset: ${current_preset}`);
            }
        } catch (e) {
            console.error('Failed to load available presets:', e);
        }
    }
    async function getColorSchemes() {
        try {
            available_luts = await invoke('get_available_color_schemes');
        } catch (e) {
            console.error('Failed to load available color schemes:', e);
        }
    }

    // Sync settings and state from backend to frontend
    async function syncSettingsFromBackend() {
        const synced = await syncManager.syncAll();
        if (synced.settings) settings = synced.settings;
        if (synced.state) state = synced.state;
    }

    let simulationInitializedUnlisten: (() => void) | null = null;
    let simulationResumedUnlisten: (() => void) | null = null;
    let fpsUpdateUnlisten: (() => void) | null = null;

    // Add a function to fetch the latest camera state from the backend
    async function fetchCameraState() {
        try {
            const cam = (await invoke('get_camera_state')) as {
                position: number[];
                zoom: number;
                viewport_width: number;
                viewport_height: number;
                aspect_ratio: number;
            };
            if (cam) {
                console.log('Camera state fetched:', cam);
            }
        } catch (e) {
            console.error('Failed to fetch camera state:', e);
        }
    }

    async function sendCursorToBackend(screenX: number, screenY: number) {
        try {
            await invoke('update_cursor_position_screen', {
                screenX,
                screenY,
            });
        } catch (err) {
            console.error('Failed to update cursor position:', err);
        }
    }

    // Mouse state tracking for dragging support
    let isMousePressed = false;
    let currentMouseButton = 0;

    // Mouse event handling for camera controls and simulation interaction
    async function handleMouseEvent(e: CustomEvent) {
        const event = e.detail as MouseEvent | WheelEvent;

        if (event.type === 'wheel') {
            const wheelEvent = event as WheelEvent;
            wheelEvent.preventDefault();

            const zoomDelta = -wheelEvent.deltaY * 0.001;

            // Convert screen coordinates to physical coordinates
            const devicePixelRatio = window.devicePixelRatio || 1;
            const physicalCursorX = wheelEvent.clientX * devicePixelRatio;
            const physicalCursorY = wheelEvent.clientY * devicePixelRatio;

            try {
                await invoke('zoom_camera_to_cursor', {
                    delta: zoomDelta,
                    cursorX: physicalCursorX,
                    cursorY: physicalCursorY,
                });
            } catch (e) {
                console.error('Failed to zoom camera to cursor:', e);
            }
        } else if (event.type === 'mousedown') {
            const mouseEvent = event as MouseEvent;
            mouseEvent.preventDefault();

            isMousePressed = true;
            currentMouseButton = mouseEvent.button;

            // Convert screen coordinates to physical coordinates
            const devicePixelRatio = window.devicePixelRatio || 1;
            const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
            const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

            console.log(
                `Gray-Scott mouse interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: ${mouseEvent.button}`
            );

            try {
                await invoke('handle_mouse_interaction_screen', {
                    screenX: physicalCursorX,
                    screenY: physicalCursorY,
                    mouseButton: mouseEvent.button,
                });
            } catch (e) {
                console.error('Failed to handle Gray-Scott mouse interaction:', e);
            }
        } else if (event.type === 'mousemove') {
            const mouseEvent = event as MouseEvent;

            // Convert screen coordinates to physical coordinates
            const devicePixelRatio = window.devicePixelRatio || 1;
            const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
            const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

            if (isMousePressed) {
                // Continue interaction while dragging
                mouseEvent.preventDefault();

                try {
                    await invoke('handle_mouse_interaction_screen', {
                        screenX: physicalCursorX,
                        screenY: physicalCursorY,
                        mouseButton: currentMouseButton,
                    });
                } catch (e) {
                    console.error('Failed to handle Gray-Scott mouse drag:', e);
                }
            } else {
                // Just update cursor position for visual feedback when not dragging
                try {
                    await sendCursorToBackend(physicalCursorX, physicalCursorY);
                } catch (e) {
                    console.error('Failed to update cursor position:', e);
                }
            }
        } else if (event.type === 'mouseup') {
            const mouseEvent = event as MouseEvent;
            mouseEvent.preventDefault();

            // Only handle mouseup if we were actually tracking a mouse press
            if (isMousePressed) {
                isMousePressed = false;

                // Stop cursor interaction when mouse is released
                try {
                    await invoke('handle_mouse_release', { mouseButton: currentMouseButton });
                } catch (e) {
                    console.error('Failed to stop Gray-Scott mouse interaction:', e);
                }
            }
        } else if (event.type === 'contextmenu') {
            // Handle context menu as right-click for simulation interaction
            const mouseEvent = event as MouseEvent;

            // Convert screen coordinates to physical coordinates
            const devicePixelRatio = window.devicePixelRatio || 1;
            const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
            const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

            console.log(
                `Gray-Scott context menu interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: 2`
            );

            // Track as active right-button press to ensure release is generated later
            isMousePressed = true;
            currentMouseButton = 2;

            try {
                await invoke('handle_mouse_interaction_screen', {
                    screenX: physicalCursorX,
                    screenY: physicalCursorY,
                    mouseButton: 2, // Right mouse button
                });
            } catch (e) {
                console.error('Failed to handle Gray-Scott context menu interaction:', e);
            }
        }
    }

    // Initialize camera state with proper type
    // Note: camera_state is now fetched from backend when needed

    async function toggleBackendGui() {
        try {
            await invoke('toggle_gui');
            // Toggle local state directly instead of relying on backend state
            showUI = !showUI;

            // Update auto-hide manager state and handle UI toggle
            if (autoHideManager) {
                autoHideManager.updateState({ showUI, running });
                autoHideManager.handleUIToggle(showUI);
            }
        } catch (err) {
            console.error('Failed to toggle backend GUI:', err);
        }
    }

    async function updateLut(name: string) {
        try {
            await invoke('apply_color_scheme_by_name', { colorSchemeName: name });
            const synced = await syncManager.syncAll();
            if (synced.settings) settings = synced.settings;
            if (synced.state) state = synced.state;
        } catch (e) {
            console.error('Failed to update color scheme:', e);
        }
    }

    onMount(() => {
        // Initialize auto-hide manager
        autoHideManager = new AutoHideManager(
            {
                controlsVisible,
                cursorHidden: false,
                showUI,
                running,
            },
            {
                onControlsShow: () => {
                    controlsVisible = true;
                },
                onControlsHide: () => {
                    controlsVisible = false;
                },
                onCursorShow: () => {
                    document.body.style.cursor = '';
                },
                onCursorHide: () => {
                    document.body.style.cursor = 'none';
                },
            },
            {
                autoHideDelay,
                cursorHideDelay: 2000,
            }
        );

        // Create event listeners
        eventListeners = createAutoHideEventListeners(() => {
            autoHideManager.handleUserInteraction();
        });
        eventListeners.add();

        // Load available webcams once UI mounts
        loadGrayScottWebcams();

        // Listen for simulation initialization event
        listen('simulation-initialized', async () => {
            console.log('Simulation initialized, syncing settings...');
            // Load presets and color schemes after simulation is initialized
            await getPresets();
            await getColorSchemes();
            await syncSettingsFromBackend();

            // Fetch initial camera state to get correct viewport dimensions
            await fetchCameraState();

            // Initialize cursor position to center of screen so golden crosshair is visible
            const centerX = window.innerWidth / 2;
            const centerY = window.innerHeight / 2;
            const devicePixelRatio = window.devicePixelRatio || 1;
            const physicalCenterX = centerX * devicePixelRatio;
            const physicalCenterY = centerY * devicePixelRatio;
            sendCursorToBackend(physicalCenterX, physicalCenterY);

            // Seed random noise to start with interesting patterns
            try {
                await invoke('seed_random_noise');
                console.log('Initial random noise seeded successfully');
            } catch (e) {
                console.error('Failed to seed initial random noise:', e);
            }

            // Now that simulation is fully initialized, set running to true
            loading = false;
            running = true;
            console.log('Simulation is now running and ready for mouse interaction');
        }).then((unlisten) => {
            simulationInitializedUnlisten = unlisten;
        });

        // Listen for simulation resumed event
        listen('simulation-resumed', async () => {
            console.log('Simulation resumed');
            running = true;
            currentFps = 0;
        }).then((unlisten) => {
            simulationResumedUnlisten = unlisten;
        });

        // Listen for FPS updates from backend
        listen('fps-update', (event) => {
            currentFps = event.payload as number;
        }).then((unlisten) => {
            fpsUpdateUnlisten = unlisten;
        });

        // Then start simulation
        startSimulation();

        return () => {
            stopSimulation();
        };
    });

    onDestroy(() => {
        // Remove keyboard event listeners

        // Clean up auto-hide functionality
        if (eventListeners) {
            eventListeners.remove();
        }
        if (autoHideManager) {
            autoHideManager.cleanup();
        }

        // Cancel animation frame

        if (simulationInitializedUnlisten) {
            simulationInitializedUnlisten();
        }
        if (simulationResumedUnlisten) {
            simulationResumedUnlisten();
        }
        if (fpsUpdateUnlisten) {
            fpsUpdateUnlisten();
        }
    });
</script>

<style>
</style>
