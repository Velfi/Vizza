<SimulationLayout
    simulationName="Moiré"
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
    on:userInteraction={autoHideManager?.handleUserInteraction}
    on:mouseEvent={handleMouseEvent}
>
    {#if settings}
        <form on:submit|preventDefault>
            <!-- About this simulation -->
            <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
                <p>
                    The Moiré simulation creates beautiful, evolving patterns through mathematical
                    interference.
                </p>
                <p>
                    The simulation generates interference patterns from multiple overlapping grids
                    at different rotations and scales. These patterns are mapped to colors using
                    color schemes, while fluid advection creates temporal evolution and flow.
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
                        bind:current_color_scheme={color_scheme_name}
                        bind:reversed={color_scheme_reversed}
                        on:select={({ detail }) => updateLut(detail.name)}
                        on:reverse={() => updateLutReversed()}
                    />
                </div>
                <div class="control-group">
                    <label>
                        <input
                            type="checkbox"
                            checked={Boolean(settings.image_mode_enabled)}
                            on:change={async (e) => {
                                const checked: boolean = (e.target as HTMLInputElement).checked;
                                if (!settings) return;
                                settings.image_mode_enabled = checked;
                                try {
                                    await invoke('update_simulation_setting', {
                                        settingName: 'image_mode_enabled',
                                        value: checked,
                                    });
                                } catch (err) {
                                    console.error('Failed to update image mode:', err);
                                    settings.image_mode_enabled = !checked;
                                }
                            }}
                        />
                        Enable Image Mode
                    </label>
                </div>
                {#if settings.image_mode_enabled}
                    <div class="control-group">
                        <label class="setting-label" for="moire-interference-mode"
                            >Interference Mode:</label
                        >
                        <select
                            id="moire-interference-mode"
                            value={settings.image_interference_mode as string}
                            on:change={async (e) => {
                                try {
                                    const value = (e.target as HTMLSelectElement).value;
                                    await invoke('update_simulation_setting', {
                                        settingName: 'image_interference_mode',
                                        value: value,
                                    });
                                } catch (err) {
                                    console.error('Failed to update interference mode:', err);
                                }
                            }}
                        >
                            <option value="Modulate">Modulate</option>
                            <option value="Blend">Blend</option>
                        </select>
                    </div>
                    <div class="control-group">
                        <ImageSelector
                            fitMode={settings.image_fit_mode}
                            loadCommand="load_moire_image"
                            onFitModeChange={async (value) => {
                                try {
                                    await invoke('update_simulation_setting', {
                                        settingName: 'image_fit_mode',
                                        value: value,
                                    });
                                } catch (err) {
                                    console.error('Failed to update fit mode:', err);
                                }
                            }}
                        />
                    </div>
                    <div class="control-group">
                        <label class="checkbox">
                            <input
                                type="checkbox"
                                checked={settings.image_mirror_horizontal}
                                on:change={async (e) => {
                                    try {
                                        const checked = (e.target as HTMLInputElement).checked;
                                        await invoke('update_simulation_setting', {
                                            settingName: 'image_mirror_horizontal',
                                            value: checked,
                                        });
                                    } catch (err) {
                                        console.error(
                                            'Failed to update image mirror horizontal:',
                                            err
                                        );
                                    }
                                }}
                            />
                            Mirror horizontal
                        </label>
                    </div>
                    <div class="control-group">
                        <label class="checkbox">
                            <input
                                type="checkbox"
                                checked={settings.image_mirror_vertical}
                                on:change={async (e) => {
                                    try {
                                        const checked = (e.target as HTMLInputElement).checked;
                                        await invoke('update_simulation_setting', {
                                            settingName: 'image_mirror_vertical',
                                            value: checked,
                                        });
                                    } catch (err) {
                                        console.error(
                                            'Failed to update image mirror vertical:',
                                            err
                                        );
                                    }
                                }}
                            />
                            Mirror vertical
                        </label>
                    </div>
                    <div class="control-group">
                        <label class="checkbox">
                            <input
                                type="checkbox"
                                checked={settings.image_invert_tone}
                                on:change={async (e) => {
                                    try {
                                        const checked = (e.target as HTMLInputElement).checked;
                                        await invoke('update_simulation_setting', {
                                            settingName: 'image_invert_tone',
                                            value: checked,
                                        });
                                    } catch (err) {
                                        console.error('Failed to update image invert tone:', err);
                                    }
                                }}
                            />
                            Invert tone
                        </label>
                    </div>
                    <div class="control-group">
                        <WebcamControls
                            {webcamDevices}
                            {webcamActive}
                            onStartWebcam={startWebcamCapture}
                            onStopWebcam={stopWebcamCapture}
                        />
                    </div>
                {/if}
            </fieldset>

            <!-- Controls -->
            <ControlsPanel
                mouseInteractionText="🖱️ Mouse wheel: Zoom | Drag: Pan camera"
                on:navigate={(e) => dispatch('navigate', { detail: e.detail })}
            />

            <!-- Actions -->
            <fieldset>
                <legend>Actions</legend>
                <div class="control-group">
                    <Button variant="default" on:click={randomizeSettings}
                        >Randomize Moiré Settings</Button
                    >
                </div>
            </fieldset>

            <!-- Animation Settings -->
            <fieldset>
                <legend>Animation</legend>
                <div class="control-group">
                    <label class="setting-label" for="moire-speed">Speed:</label>
                    <NumberDragBox
                        id="moire-speed"
                        bind:value={settings.speed}
                        min={0.0}
                        max={5.0}
                        step={0.01}
                    />
                </div>
            </fieldset>

            <!-- Moiré Pattern Settings -->
            <fieldset>
                <legend>Moiré Patterns</legend>
                <div class="control-group">
                    <label class="setting-label" for="moire-generator-type">Generator Type:</label>
                    <select
                        id="moire-generator-type"
                        value={settings.generator_type as string}
                        on:change={async (e) => {
                            try {
                                const value = (e.target as HTMLSelectElement).value;
                                await invoke('update_simulation_setting', {
                                    settingName: 'generator_type',
                                    value: value,
                                });
                            } catch (err) {
                                console.error('Failed to update generator type:', err);
                            }
                        }}
                    >
                        <option value="linear">Linear</option>
                        <option value="radial">Radial</option>
                    </select>
                </div>
                <div class="settings-grid">
                    <div class="setting-item">
                        <label class="setting-label" for="moire-base-freq">Base Frequency:</label>
                        <NumberDragBox
                            id="moire-base-freq"
                            value={settings.base_freq as number}
                            on:change={({ detail }) => (settings!.base_freq = detail)}
                            min={0.1}
                            max={20.0}
                            step={0.1}
                        />
                    </div>
                    <div class="setting-item">
                        <label class="setting-label" for="moire-amount">Moiré Amount:</label>
                        <NumberDragBox
                            id="moire-amount"
                            value={settings.moire_amount as number}
                            on:change={({ detail }) => (settings!.moire_amount = detail)}
                            min={0.0}
                            max={1.0}
                            step={0.01}
                        />
                    </div>
                    <div class="setting-item">
                        <label class="setting-label" for="moire-rotation">Grid Rotation:</label>
                        <NumberDragBox
                            id="moire-rotation"
                            value={((settings!.moire_rotation as number) * 180) / Math.PI}
                            on:change={({ detail }) =>
                                updateSetting('moire_rotation', (detail * Math.PI) / 180)}
                            min={-180}
                            max={180}
                            step={1}
                        />
                    </div>
                    <div class="setting-item">
                        <label class="setting-label" for="moire-scale">Grid Scale:</label>
                        <NumberDragBox
                            id="moire-scale"
                            value={settings.moire_scale as number}
                            on:change={({ detail }) => (settings!.moire_scale = detail)}
                            min={0.1}
                            max={10.0}
                            step={0.1}
                        />
                    </div>
                    <div class="setting-item">
                        <label class="setting-label" for="moire-interference">Interference:</label>
                        <NumberDragBox
                            id="moire-interference"
                            value={settings.moire_interference as number}
                            on:change={({ detail }) => (settings!.moire_interference = detail)}
                            min={0.0}
                            max={1.0}
                            step={0.01}
                        />
                    </div>
                </div>
            </fieldset>

            <!-- Radial Pattern Settings (shown when Radial is selected) -->
            {#if settings.generator_type === 'radial'}
                <fieldset>
                    <legend>Radial Pattern Settings</legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="moire-swirl-strength"
                                >Swirl Strength:</label
                            >
                            <NumberDragBox
                                id="moire-swirl-strength"
                                value={settings.radial_swirl_strength as number}
                                on:change={({ detail }) =>
                                    (settings!.radial_swirl_strength = detail)}
                                min={0.0}
                                max={5.0}
                                step={0.01}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="moire-starburst-count"
                                >Starburst Count:</label
                            >
                            <NumberDragBox
                                id="moire-starburst-count"
                                value={settings.radial_starburst_count as number}
                                on:change={({ detail }) =>
                                    (settings!.radial_starburst_count = detail)}
                                min={0}
                                max={128}
                                step={1}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="moire-center-brightness"
                                >Center Brightness:</label
                            >
                            <NumberDragBox
                                id="moire-center-brightness"
                                value={settings.radial_center_brightness as number}
                                on:change={({ detail }) =>
                                    (settings!.radial_center_brightness = detail)}
                                min={0.0}
                                max={2.0}
                                step={0.01}
                            />
                        </div>
                    </div>
                </fieldset>
            {/if}

            <!-- Advection Flow Settings -->
            <fieldset>
                <legend>Advection Flow</legend>
                <div class="settings-grid">
                    <div class="setting-item">
                        <label class="setting-label" for="moire-flow-strength">Flow Strength:</label
                        >
                        <NumberDragBox
                            id="moire-flow-strength"
                            value={settings.advect_strength as number}
                            on:change={({ detail }) => (settings!.advect_strength = detail)}
                            min={0.0}
                            max={5.0}
                            step={0.01}
                        />
                    </div>
                    <div class="setting-item">
                        <label class="setting-label" for="moire-flow-speed">Flow Speed:</label>
                        <NumberDragBox
                            id="moire-flow-speed"
                            value={settings.advect_speed as number}
                            on:change={({ detail }) => (settings!.advect_speed = detail)}
                            min={0.0}
                            max={10.0}
                            step={0.01}
                        />
                    </div>
                    <div class="setting-item">
                        <label class="setting-label" for="moire-curl">Curl:</label>
                        <NumberDragBox
                            id="moire-curl"
                            value={settings.curl as number}
                            on:change={({ detail }) => (settings!.curl = detail)}
                            min={0.0}
                            max={1.0}
                            step={0.01}
                        />
                    </div>
                    <div class="setting-item">
                        <label class="setting-label" for="moire-decay">Decay:</label>
                        <NumberDragBox
                            id="moire-decay"
                            value={settings.decay as number}
                            on:change={({ detail }) => (settings!.decay = detail)}
                            min={0.0}
                            max={1.0}
                            step={0.01}
                        />
                    </div>
                </div>
            </fieldset>
        </form>
    {/if}
</SimulationLayout>

<!-- Shared camera controls component -->
<CameraControls enabled={true} on:toggleGui={toggleBackendGui} on:togglePause={togglePause} />

<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import ColorSchemeSelector from './components/shared/ColorSchemeSelector.svelte';
    import ImageSelector from './components/shared/ImageSelector.svelte';
    import WebcamControls from './components/shared/WebcamControls.svelte';
    import NumberDragBox from './components/inputs/NumberDragBox.svelte';
    import Button from './components/shared/Button.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import ControlsPanel from './components/shared/ControlsPanel.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';
    import type { AppMode } from '../types/app';

    // Props
    export let menuPosition: 'left' | 'right' | 'middle' = 'right';
    export let autoHideDelay: number = 1000;

    // Event dispatchers
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher<{
        back: void;
        navigate: { detail: AppMode };
    }>();

    // State
    let running = false;
    let loading = true;
    let showUI = true;
    let currentFps = 0;
    let controlsVisible = true;
    let show_about_section = false;

    // Auto-hide manager
    let autoHideManager: AutoHideManager;
    let eventListeners: { add: () => void; remove: () => void };

    // Settings interface
    interface Settings {
        speed: number;
        generator_type: string;
        base_freq: number;
        moire_amount: number;
        moire_rotation: number;
        image_fit_mode: string;
        image_mirror_horizontal: boolean;
        image_mirror_vertical: boolean;
        image_invert_tone: boolean;
        [key: string]: unknown; // Allow additional properties
    }

    // Settings
    let settings: Settings | null = null;
    let available_presets: string[] = [];
    let current_preset = '';

    // State type
    interface State {
        time: number;
        width: number;
        height: number;
        color_scheme_name: string;
        color_scheme_reversed: boolean;
    }

    // Color scheme state
    let available_luts: string[] = [];
    let color_scheme_name = 'ZELDA_Fordite';
    let color_scheme_reversed = false;

    // Webcam state
    let webcamDevices: number[] = [];
    let webcamActive = false;

    // Initialize simulation
    onMount(async () => {
        try {
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

            // Start the simulation
            await invoke('start_moire_simulation');

            // Load initial settings
            await loadSettings();

            // Load initial state
            await loadState();

            // Load available presets
            await loadPresets();

            // Load available color schemes
            await loadColorSchemes();

            // Load available webcam devices
            await loadWebcamDevices();

            // Start render loop
            startRenderLoop();

            running = true;
            loading = false;
        } catch (error) {
            console.error('Failed to start Moiré simulation:', error);
            loading = false;
        }
    });

    onDestroy(() => {
        if (running) {
            invoke('destroy_simulation').catch(console.error);
        }

        // Clean up auto-hide functionality
        if (eventListeners) {
            eventListeners.remove();
        }
        if (autoHideManager) {
            autoHideManager.cleanup();
        }
    });

    // Load current settings
    async function loadSettings() {
        try {
            settings = await invoke('get_current_settings');
        } catch (error) {
            console.error('Failed to load settings:', error);
        }
    }

    // Load current state
    async function loadState() {
        try {
            const state = (await invoke('get_current_state')) as State;
            if (state) {
                color_scheme_name = state.color_scheme_name;
                color_scheme_reversed = state.color_scheme_reversed || false;
            }
        } catch (error) {
            console.error('Failed to load state:', error);
        }
    }

    // Load available presets
    async function loadPresets() {
        try {
            available_presets = await invoke('get_available_presets');
        } catch (error) {
            console.error('Failed to load presets:', error);
        }
    }

    // Load available color schemes
    async function loadColorSchemes() {
        try {
            available_luts = await invoke('get_available_color_schemes');
        } catch (error) {
            console.error('Failed to load color schemes:', error);
        }
    }

    // Update a specific setting
    async function updateSetting(settingName: string, value: string | number | boolean) {
        try {
            await invoke('update_simulation_setting', {
                settingName: settingName,
                value: value,
            });
        } catch (error) {
            console.error(`Failed to update setting ${settingName}:`, error);
        }
    }

    // Preset management
    async function updatePreset(presetName: string) {
        try {
            await invoke('apply_preset', { presetName });
            await loadSettings();
            current_preset = presetName;
        } catch (error) {
            console.error('Failed to apply preset:', error);
        }
    }

    async function savePreset(presetName: string) {
        try {
            await invoke('save_preset', { presetName });
            await loadPresets();
            current_preset = presetName;
        } catch (error) {
            console.error('Failed to save preset:', error);
        }
    }

    // Actions
    async function randomizeSettings() {
        try {
            await invoke('randomize_settings');
            await loadSettings(); // Reload settings to show the new values
        } catch (error) {
            console.error('Failed to randomize settings:', error);
        }
    }

    // resetFlow removed

    // Color scheme functions
    async function updateLut(colorSchemeName: string) {
        try {
            color_scheme_name = colorSchemeName;
            await invoke('apply_color_scheme_by_name', { colorSchemeName });
            await invoke('update_simulation_state', {
                stateName: 'color_scheme_name',
                value: colorSchemeName,
            });
        } catch (error) {
            console.error('Failed to update color scheme:', error);
        }
    }

    async function updateLutReversed() {
        try {
            await invoke('toggle_color_scheme_reversed');
            color_scheme_reversed = !color_scheme_reversed;
        } catch (error) {
            console.error('Failed to update color scheme reversal:', error);
        }
    }

    // Simulation control
    async function stopSimulation() {
        try {
            await invoke('pause_simulation');
            running = false;

            // Update auto-hide manager state and handle pause
            if (autoHideManager) {
                autoHideManager.updateState({ running });
                autoHideManager.handlePause();
            }
        } catch (error) {
            console.error('Failed to pause simulation:', error);
        }
    }

    async function resumeSimulation() {
        try {
            await invoke('resume_simulation');
            running = true;

            // Update auto-hide manager state and handle resume
            if (autoHideManager) {
                autoHideManager.updateState({ running });
                autoHideManager.handleResume();
            }
        } catch (error) {
            console.error('Failed to resume simulation:', error);
        }
    }

    async function togglePause() {
        if (running) {
            await stopSimulation();
        } else {
            await resumeSimulation();
        }
    }

    // Render loop
    let renderLoopId: number | null = null;

    function startRenderLoop() {
        async function renderLoop() {
            if (renderLoopId === null) return;

            try {
                await invoke('render_frame');
                currentFps = 60; // Approximate FPS
            } catch (e) {
                console.error('Render failed:', e);
            }

            if (renderLoopId !== null) {
                renderLoopId = requestAnimationFrame(renderLoop);
            }
        }

        renderLoopId = requestAnimationFrame(renderLoop);
    }

    function stopRenderLoop() {
        if (renderLoopId !== null) {
            cancelAnimationFrame(renderLoopId);
            renderLoopId = null;
        }
    }

    onDestroy(() => {
        stopRenderLoop();
    });

    // UI control
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

    // Navigation
    function returnToMenu() {
        stopRenderLoop();
        dispatch('back');
    }

    // Mouse interaction
    function handleMouseEvent() {
        // Moiré simulation doesn't use mouse interaction, but handle auto-hide
        if (autoHideManager) {
            autoHideManager.handleUserInteraction();
        }
    }

    // Webcam functions
    async function loadWebcamDevices() {
        try {
            webcamDevices = await invoke('get_available_moire_webcam_devices');
            console.log('Available webcam devices for Moire:', webcamDevices);
        } catch (e) {
            console.error('Failed to load webcam devices:', e);
        }
    }

    async function startWebcamCapture() {
        try {
            await invoke('start_moire_webcam_capture');
            webcamActive = true;
            console.log('Moire webcam capture started');
        } catch (e) {
            console.error('Failed to start webcam capture:', e);
        }
    }

    async function stopWebcamCapture() {
        try {
            await invoke('stop_moire_webcam_capture');
            webcamActive = false;
            console.log('Moire webcam capture stopped');
        } catch (e) {
            console.error('Failed to stop webcam capture:', e);
        }
    }
</script>
