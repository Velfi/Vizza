<SimulationLayout
    simulationName="Crystal Growth"
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
                    Crystal Growth simulates the formation of beautiful crystal structures using
                    diffusion-limited aggregation (DLA) and related algorithms. Watch as particles
                    diffuse through space and stick to growing crystal surfaces, creating intricate
                    patterns including snowflakes, dendrites, and fractal formations.
                </p>
                <p>
                    The simulation models how crystals grow in nature, with particles moving randomly
                    until they encounter a crystal surface, where they may stick based on temperature,
                    supersaturation, and other physical parameters. Different crystal types exhibit
                    unique growth patterns and branching behaviors.
                </p>
                <p>
                    Click to add seed points where crystal growth begins, or enable auto-seeding
                    for random crystal formation. Adjust parameters like growth rate, diffusion,
                    and temperature to see how they affect the resulting crystal structures.
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

            <!-- Crystal Type -->
            <fieldset>
                <legend>Crystal Type</legend>
                <div class="control-group">
                    <label for="crystal-type">Crystal Structure:</label>
                    <select id="crystal-type" bind:value={settings.crystal_type} on:change={updateSetting}>
                        <option value="Snowflake">Snowflake</option>
                        <option value="Dendrite">Dendrite</option>
                        <option value="Cubic">Cubic</option>
                        <option value="Hexagonal">Hexagonal</option>
                        <option value="Fractal">Fractal</option>
                    </select>
                </div>
            </fieldset>

            <!-- Growth Parameters -->
            <fieldset>
                <legend>Growth Parameters</legend>
                <div class="control-group">
                    <label for="growth-rate">Growth Rate:</label>
                    <NumberDragBox
                        id="growth-rate"
                        bind:value={settings.growth_rate}
                        min={0.01}
                        max={2.0}
                        step={0.01}
                        precision={2}
                        on:change={updateSetting}
                    />
                    <label for="diffusion-rate">Diffusion Rate:</label>
                    <NumberDragBox
                        id="diffusion-rate"
                        bind:value={settings.diffusion_rate}
                        min={0.1}
                        max={5.0}
                        step={0.1}
                        precision={1}
                        on:change={updateSetting}
                    />
                    <label for="sticking-probability">Sticking Probability:</label>
                    <NumberDragBox
                        id="sticking-probability"
                        bind:value={settings.sticking_probability}
                        min={0.01}
                        max={1.0}
                        step={0.01}
                        precision={2}
                        on:change={updateSetting}
                    />
                </div>
            </fieldset>

            <!-- Physical Properties -->
            <fieldset>
                <legend>Physical Properties</legend>
                <div class="control-group">
                    <label for="temperature">Temperature:</label>
                    <NumberDragBox
                        id="temperature"
                        bind:value={settings.temperature}
                        min={0.1}
                        max={2.0}
                        step={0.01}
                        precision={2}
                        on:change={updateSetting}
                    />
                    <label for="supersaturation">Supersaturation:</label>
                    <NumberDragBox
                        id="supersaturation"
                        bind:value={settings.supersaturation}
                        min={0.1}
                        max={2.0}
                        step={0.01}
                        precision={2}
                        on:change={updateSetting}
                    />
                    <label for="anisotropy">Anisotropy:</label>
                    <NumberDragBox
                        id="anisotropy"
                        bind:value={settings.anisotropy}
                        min={0.0}
                        max={1.0}
                        step={0.01}
                        precision={2}
                        on:change={updateSetting}
                    />
                    <label for="noise-strength">Noise Strength:</label>
                    <NumberDragBox
                        id="noise-strength"
                        bind:value={settings.noise_strength}
                        min={0.0}
                        max={1.0}
                        step={0.01}
                        precision={2}
                        on:change={updateSetting}
                    />
                </div>
            </fieldset>

            <!-- Seed Configuration -->
            <fieldset>
                <legend>Seed Configuration</legend>
                <div class="control-group">
                    <label for="seed-size">Seed Size:</label>
                    <NumberDragBox
                        id="seed-size"
                        bind:value={settings.seed_size}
                        min={1.0}
                        max={20.0}
                        step={0.5}
                        precision={1}
                        on:change={updateSetting}
                    />
                    <label for="seed-spacing">Seed Spacing:</label>
                    <NumberDragBox
                        id="seed-spacing"
                        bind:value={settings.seed_spacing}
                        min={10.0}
                        max={200.0}
                        step={5.0}
                        precision={0}
                        on:change={updateSetting}
                    />
                    <div class="checkbox-group">
                        <label>
                            <input
                                type="checkbox"
                                bind:checked={settings.auto_seed}
                                on:change={updateSetting}
                            />
                            Auto-seed (random placement)
                        </label>
                    </div>
                </div>
            </fieldset>

            <!-- Simulation Control -->
            <fieldset>
                <legend>Simulation Control</legend>
                <div class="control-group">
                    <label for="speed">Speed:</label>
                    <NumberDragBox
                        id="speed"
                        bind:value={settings.speed}
                        min={0.1}
                        max={5.0}
                        step={0.1}
                        precision={1}
                        on:change={updateSetting}
                    />
                    <label for="particle-count">Particle Count:</label>
                    <NumberDragBox
                        id="particle-count"
                        bind:value={settings.particle_count}
                        min={1000}
                        max={50000}
                        step={1000}
                        precision={0}
                        on:change={updateSetting}
                    />
                    <div class="checkbox-group">
                        <label>
                            <input
                                type="checkbox"
                                bind:checked={settings.growth_visualization}
                                on:change={updateSetting}
                            />
                            Show particle growth
                        </label>
                    </div>
                </div>
            </fieldset>

            <!-- Color Scheme -->
            <fieldset>
                <legend>Color Scheme</legend>
                <div class="control-group">
                    <label for="color-scheme">Color Scheme:</label>
                    <select id="color-scheme" bind:value={settings.color_scheme_name} on:change={updateSetting}>
                        {#each available_luts as lut}
                            <option value={lut}>{lut}</option>
                        {/each}
                    </select>
                    <div class="checkbox-group">
                        <label>
                            <input
                                type="checkbox"
                                bind:checked={settings.color_scheme_reversed}
                                on:change={updateSetting}
                            />
                            Reverse color scheme
                        </label>
                    </div>
                </div>
            </fieldset>

            <!-- Actions -->
            <fieldset>
                <legend>Actions</legend>
                <div class="control-group">
                    <Button variant="primary" on:click={resetSimulation}>Reset Simulation</Button>
                    <Button variant="default" on:click={randomizeSettings}>Randomize Settings</Button>
                </div>
            </fieldset>
        </form>
    {/if}
</SimulationLayout>

<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import NumberDragBox from './components/inputs/NumberDragBox.svelte';
    import Button from './components/shared/Button.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';

    const dispatch = createEventDispatcher();

    // Props
    export let menuPosition: string = 'middle';
    export let autoHideDelay: number = 3000;

    // Simulation state
    let running = false;
    let loading = false;
    let showUI = true;
    let currentFps = 0;
    let controlsVisible = true;
    let show_about_section = false;

    // Auto-hide management
    let autoHideManager: AutoHideManager | null = null;
    let eventListeners: { add: () => void; remove: () => void } | null = null;

    // Settings interface
    interface Settings {
        crystal_type: string;
        growth_rate: number;
        diffusion_rate: number;
        sticking_probability: number;
        temperature: number;
        supersaturation: number;
        anisotropy: number;
        noise_strength: number;
        seed_size: number;
        seed_spacing: number;
        auto_seed: boolean;
        speed: number;
        particle_count: number;
        growth_visualization: boolean;
        color_scheme_name: string;
        color_scheme_reversed: boolean;
    }

    let settings: Settings | undefined = undefined;

    // Preset and LUT state
    let current_preset = '';
    let available_presets: string[] = [];
    let available_luts: string[] = [];

    // Render loop
    let renderLoopId: number | null = null;

    // Event handlers
    function returnToMenu() {
        dispatch('navigate', 'menu');
    }


    async function toggleBackendGui() {
        try {
            await invoke('toggle_gui');
            showUI = !showUI;

            if (autoHideManager) {
                autoHideManager.updateState({ showUI, running });
                autoHideManager.handleUIToggle(showUI);
            }
        } catch (error) {
            console.error('Failed to toggle GUI:', error);
        }
    }

    async function stopSimulation() {
        try {
            await invoke('pause_simulation');
            running = false;
        } catch (error) {
            console.error('Failed to pause simulation:', error);
        }
    }

    async function resumeSimulation() {
        try {
            await invoke('resume_simulation');
            running = true;
        } catch (error) {
            console.error('Failed to resume simulation:', error);
        }
    }

    async function handleMouseEvent(event: CustomEvent) {
        try {
            const { worldX, worldY, button } = event.detail;
            await invoke('handle_mouse_interaction_screen', {
                screenX: worldX,
                screenY: worldY,
                mouseButton: button,
            });
        } catch (error) {
            console.error('Failed to handle mouse interaction:', error);
        }
    }

    // Simulation control
    async function startSimulation() {
        if (running || loading) return;

        loading = true;
        try {
            await invoke('start_simulation', { simulationType: 'crystal_growth' });
            loading = false;
            running = true;
            currentFps = 0;
        } catch (error) {
            console.error('Failed to start crystal growth simulation:', error);
        } finally {
            loading = false;
        }
    }

    async function resetSimulation() {
        try {
            await invoke('reset_simulation');
        } catch (error) {
            console.error('Failed to reset simulation:', error);
        }
    }

    async function randomizeSettings() {
        try {
            await invoke('randomize_crystal_growth_settings');
            await syncSettingsFromBackend();
        } catch (error) {
            console.error('Failed to randomize settings:', error);
        }
    }

    // Settings management
    async function updateSetting() {
        if (!settings) return;

        try {
            for (const [key, value] of Object.entries(settings)) {
                await invoke('update_simulation_setting', {
                    settingName: key,
                    value: value,
                });
            }
        } catch (error) {
            console.error('Failed to update setting:', error);
        }
    }

    async function syncSettingsFromBackend() {
        try {
            const backendSettings = await invoke('get_current_settings');
            if (backendSettings) {
                settings = backendSettings as Settings;
            }
        } catch (error) {
            console.error('Failed to sync settings from backend:', error);
        }
    }

    // Preset management
    async function updatePreset(presetName: string) {
        try {
            await invoke('apply_preset', { presetName });
            current_preset = presetName;
            await syncSettingsFromBackend();
        } catch (error) {
            console.error('Failed to apply preset:', error);
        }
    }

    async function savePreset(presetName: string) {
        try {
            await invoke('save_preset', { presetName: presetName.trim() });
            await loadAvailablePresets();
            current_preset = presetName.trim();
        } catch (error) {
            console.error('Failed to save preset:', error);
        }
    }

    async function loadAvailablePresets() {
        try {
            available_presets = await invoke('get_presets_for_simulation_type', {
                simulationType: 'crystal_growth',
            });
            if (available_presets.length > 0 && !current_preset) {
                current_preset = available_presets[0];
            }
        } catch (error) {
            console.error('Failed to load available presets:', error);
        }
    }

    async function loadAvailableLuts() {
        try {
            available_luts = await invoke('get_available_color_schemes');
        } catch (error) {
            console.error('Failed to load available LUTs:', error);
        }
    }


    function stopRenderLoop() {
        if (renderLoopId) {
            cancelAnimationFrame(renderLoopId);
            renderLoopId = null;
        }
    }

    // Lifecycle
    onMount(async () => {
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
            autoHideManager?.handleUserInteraction();
        });
        eventListeners.add();

        // Load initial data
        await Promise.all([
            loadAvailablePresets(),
            loadAvailableLuts(),
        ]);

        // Start simulation
        await startSimulation();

        // Sync settings after simulation is running
        await syncSettingsFromBackend();
    });

    onDestroy(() => {
        stopRenderLoop();
        if (eventListeners) {
            eventListeners.remove();
        }
    });
</script>

<style>
    @import './shared-theme.css';

    .control-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }

    .checkbox-group {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .checkbox-group label {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
    }

    .checkbox-group input[type="checkbox"] {
        margin: 0;
    }

    select {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 4px;
        color: inherit;
        padding: 0.5rem;
        font-family: inherit;
    }

    select:focus {
        outline: none;
        border-color: rgba(255, 255, 255, 0.4);
    }

    fieldset {
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 8px;
        padding: 1rem;
        margin-bottom: 1rem;
    }

    legend {
        padding: 0 0.5rem;
        font-weight: 600;
        color: rgba(255, 255, 255, 0.9);
    }
</style>