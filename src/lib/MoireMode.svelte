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
                            checked={settings.image_mode_enabled}
                            on:change={async (e) => {
                                const checked = (e.target as HTMLInputElement).checked;
                                settings.image_mode_enabled = checked; // Update local state immediately
                                try {
                                    await invoke('update_simulation_setting', {
                                        settingName: 'image_mode_enabled',
                                        value: checked,
                                    });
                                } catch (err) {
                                    console.error('Failed to update image mode:', err);
                                    // Revert local state on error
                                    settings.image_mode_enabled = !checked;
                                }
                            }}
                        />
                        Enable Image Mode
                    </label>
                </div>
                {#if settings.image_mode_enabled}
                    <div class="control-group">
                        <span class="setting-label">Interference Mode:</span>
                        <select
                            value={settings.image_interference_mode}
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
                            <option value="Replace">Replace</option>
                            <option value="Add">Add</option>
                            <option value="Multiply">Multiply</option>
                            <option value="Overlay">Overlay</option>
                            <option value="Mask">Mask</option>
                            <option value="Modulate">Modulate</option>
                        </select>
                    </div>
                    <div class="control-group">
                        <ImageSelector
                            fitMode={settings.image_fit_mode}
                            mirrorHorizontal={settings.image_mirror_horizontal}
                            invertTone={settings.image_invert_tone}
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
                            onMirrorHorizontalChange={async (value) => {
                                try {
                                    await invoke('update_simulation_setting', {
                                        settingName: 'image_mirror_horizontal',
                                        value: value,
                                    });
                                } catch (err) {
                                    console.error('Failed to update mirror:', err);
                                }
                            }}
                            onInvertToneChange={async (value) => {
                                try {
                                    await invoke('update_simulation_setting', {
                                        settingName: 'image_invert_tone',
                                        value: value,
                                    });
                                } catch (err) {
                                    console.error('Failed to update invert:', err);
                                }
                            }}
                        />
                    </div>
                {/if}
            </fieldset>

            <!-- Controls -->
            <fieldset>
                <legend>Controls</legend>
                <div class="interaction-controls-grid">
                    <div class="interaction-help">
                        <div class="control-group">
                            <span>🖱️ Mouse wheel: Zoom | Drag: Pan camera</span>
                        </div>
                        <div class="control-group">
                            <Button
                                variant="default"
                                on:click={() => dispatch('navigate', { detail: 'how-to-play' })}
                            >
                                📖 Camera Controls
                            </Button>
                        </div>
                        <div class="control-group">
                            <span
                                >Camera controls not working? Click the control bar at the top of
                                the screen.</span
                            >
                        </div>
                    </div>
                </div>
            </fieldset>

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
                    <span class="setting-label">Speed:</span>
                    <NumberDragBox
                        bind:value={settings.speed}
                        min={0}
                        max={10}
                        step={0.1}
                        precision={1}
                        on:change={async (e) => updateSetting('speed', e.detail)}
                    />
                </div>
            </fieldset>

            <!-- Moiré Pattern Settings -->
            <fieldset>
                <legend>Moiré Patterns</legend>
                <div class="control-group">
                    <span class="setting-label">Generator Type:</span>
                    <select
                        value={settings.generator_type}
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
                        <option value="Linear">Linear</option>
                        <option value="Radial">Radial</option>
                    </select>
                </div>
                <div class="settings-grid">
                    <div class="setting-item">
                        <span class="setting-label">Base Frequency:</span>
                        <NumberDragBox
                            bind:value={settings.base_freq}
                            min={1}
                            max={1000}
                            step={1}
                            precision={0}
                            on:change={async (e) => updateSetting('base_freq', e.detail)}
                        />
                    </div>
                    <div class="setting-item">
                        <span class="setting-label">Moiré Amount:</span>
                        <NumberDragBox
                            bind:value={settings.moire_amount}
                            min={0}
                            max={1}
                            step={0.01}
                            precision={2}
                            on:change={async (e) => updateSetting('moire_amount', e.detail)}
                        />
                    </div>
                    <div class="setting-item">
                        <span class="setting-label">Grid Rotation:</span>
                        <NumberDragBox
                            value={(settings.moire_rotation * 180) / Math.PI}
                            min={0}
                            max={180}
                            step={1}
                            precision={0}
                            unit="°"
                            on:change={async (e) => {
                                try {
                                    await updateSetting(
                                        'moire_rotation',
                                        (e.detail * Math.PI) / 180
                                    );
                                } catch (err) {
                                    console.error('Failed to update grid rotation:', err);
                                }
                            }}
                        />
                    </div>
                    <div class="setting-item">
                        <span class="setting-label">Grid Scale:</span>
                        <NumberDragBox
                            bind:value={settings.moire_scale}
                            min={0.8}
                            max={1.2}
                            step={0.01}
                            precision={2}
                            on:change={async (e) => updateSetting('moire_scale', e.detail)}
                        />
                    </div>
                    <div class="setting-item">
                        <span class="setting-label">Interference:</span>
                        <NumberDragBox
                            bind:value={settings.moire_interference}
                            min={0}
                            max={1}
                            step={0.01}
                            precision={2}
                            on:change={async (e) => updateSetting('moire_interference', e.detail)}
                        />
                    </div>
                </div>
            </fieldset>

            <!-- Radial Pattern Settings (shown when Radial is selected) -->
            {#if settings.generator_type === 'Radial'}
                <fieldset>
                    <legend>Radial Pattern Settings</legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Swirl Strength:</span>
                            <NumberDragBox
                                bind:value={settings.radial_swirl_strength}
                                min={0}
                                max={2}
                                step={0.01}
                                precision={2}
                                on:change={async (e) =>
                                    updateSetting('radial_swirl_strength', e.detail)}
                            />
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Starburst Count:</span>
                            <NumberDragBox
                                bind:value={settings.radial_starburst_count}
                                min={4}
                                max={64}
                                step={1}
                                precision={0}
                                on:change={async (e) =>
                                    updateSetting('radial_starburst_count', e.detail)}
                            />
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Center Brightness:</span>
                            <NumberDragBox
                                bind:value={settings.radial_center_brightness}
                                min={0}
                                max={3}
                                step={0.1}
                                precision={1}
                                on:change={async (e) =>
                                    updateSetting('radial_center_brightness', e.detail)}
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
                        <span class="setting-label">Flow Strength:</span>
                        <NumberDragBox
                            bind:value={settings.advect_strength}
                            min={0}
                            max={2}
                            step={0.01}
                            precision={2}
                            on:change={async (e) => updateSetting('advect_strength', e.detail)}
                        />
                    </div>
                    <div class="setting-item">
                        <span class="setting-label">Flow Speed:</span>
                        <NumberDragBox
                            bind:value={settings.advect_speed}
                            min={0}
                            max={10}
                            step={0.1}
                            precision={1}
                            on:change={async (e) => updateSetting('advect_speed', e.detail)}
                        />
                    </div>
                    <div class="setting-item">
                        <span class="setting-label">Curl:</span>
                        <NumberDragBox
                            bind:value={settings.curl}
                            min={0}
                            max={3}
                            step={0.01}
                            precision={2}
                            on:change={async (e) => updateSetting('curl', e.detail)}
                        />
                    </div>
                    <div class="setting-item">
                        <span class="setting-label">Decay:</span>
                        <NumberDragBox
                            bind:value={settings.decay}
                            min={0.85}
                            max={1}
                            step={0.001}
                            precision={3}
                            on:change={async (e) => updateSetting('decay', e.detail)}
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
    import NumberDragBox from './components/inputs/NumberDragBox.svelte';
    import Button from './components/shared/Button.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';

    // Props
    export let menuPosition: 'left' | 'right' = 'left';
    export let autoHideDelay: number = 3000;

    // Event dispatchers
    import { createEventDispatcher } from 'svelte';
    const dispatch = createEventDispatcher<{
        back: void;
        navigate: { detail: string };
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

    // Settings
    let settings: any = null;
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
    async function updateSetting(settingName: string, value: any) {
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
            await invoke('randomize_moire_settings');
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
    function handleMouseEvent(_event: CustomEvent) {
        // Moiré simulation doesn't use mouse interaction, but handle auto-hide
        if (autoHideManager) {
            autoHideManager.handleUserInteraction();
        }
    }
</script>
