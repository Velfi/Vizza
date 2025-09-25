<SimulationLayout
    simulationName="Flow Field"
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
    {#if settings && state}
        <form on:submit|preventDefault>
            <!-- About this simulation -->
            <CollapsibleFieldset title="About this simulation" bind:open={show_about_section}>
                <p>
                    Flow Field creates beautiful patterns by moving particles through a vector field
                    generated from noise functions or images. Particles follow the direction of
                    nearby flow vectors, creating organic, flowing animations.
                </p>
                <p>The simulation supports two vector field generation modes:</p>
                <ul>
                    <li>
                        <strong>Noise Mode:</strong> Uses various noise algorithms including Perlin noise,
                        FBM, Billow, and others. Each noise type produces different flow patterns and
                        behaviors.
                    </li>
                    <li>
                        <strong>Image Mode:</strong> Generates flow vectors from grayscale images, where
                        pixel brightness determines flow direction. Perfect for creating custom flow
                        patterns from photographs or artwork.
                    </li>
                </ul>
                <p>
                    Experiment with different vector field types, adjust particle parameters, and
                    watch as simple vector fields create complex, mesmerizing particle flows
                    reminiscent of natural phenomena like wind, water currents, and magnetic fields.
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
                    <label for="colorSchemeSelector">Color Scheme</label>
                    <ColorSchemeSelector
                        available_color_schemes={available_luts}
                        bind:current_color_scheme={state.currentColorScheme}
                        bind:reversed={state.colorSchemeReversed}
                        on:select={({ detail }) => updateColorScheme(detail.name)}
                        on:reverse={(e) => updateColorSchemeReversed(e.detail.reversed)}
                    />
                </div>

                <div class="control-group">
                    <label for="display-mode-select">Particle Color Mode</label>
                    <Selector
                        options={['Age', 'Random', 'Direction']}
                        value={state?.foregroundColorMode}
                        on:change={({ detail }) => updateForegroundColorMode(detail.value)}
                    />
                </div>

                <div class="control-group">
                    <label for="backgroundColorMode">Background Color Mode</label>
                    <Selector
                        id="backgroundColorMode"
                        options={['Black', 'White', 'Gray18', 'Color Scheme']}
                        value={state?.backgroundColorMode}
                        on:change={({ detail }) => updateBackgroundColorMode(detail.value)}
                    />
                </div>
            </fieldset>

            <!-- Post Processing -->
            <PostProcessingMenu simulationType="flow" />

            <!-- Controls -->
            <ControlsPanel
                mouseInteractionText="🖱️ Left click: Spawn particles | Right click: Destroy particles"
                cursorSize={state.cursorSize}
                on:cursorSizeChange={(e) => updateCursorSize(e.detail)}
                on:navigate={(e) => dispatch('navigate', e.detail)}
            />

            <!-- Combined Settings -->
            <fieldset>
                <legend>Settings</legend>

                <!-- General Settings -->
                <div class="settings-section">
                    <div class="control-group">
                        <Button
                            variant="warning"
                            type="button"
                            on:click={async () => {
                                try {
                                    await invoke('reset_simulation');
                                    await syncSettingsFromBackend();
                                    await syncStateFromBackend();
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
                        <Button variant="danger" type="button" on:click={killAllParticles}
                            >💀 Kill All Particles</Button
                        >
                    </div>
                </div>

                <!-- Flow Field Settings -->
                <div class="settings-section">
                    <h3 class="section-header">Flow Field</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="flow-vector-field-type"
                                >Vector Field Type:</label
                            >
                            <Selector
                                id="flow-vector-field-type"
                                options={['Noise', 'Image']}
                                value={settings.vector_field_type}
                                on:change={(e) => updateVectorFieldType(e.detail.value)}
                            />
                        </div>

                        {#if settings.vector_field_type === 'Noise'}
                            <div class="setting-item">
                                <label class="setting-label" for="flow-noise-type"
                                    >Noise Type:</label
                                >
                                <Selector
                                    id="flow-noise-type"
                                    options={[
                                        'OpenSimplex',
                                        'Worley',
                                        'Value',
                                        'FBM',
                                        'FBMBillow',
                                        'FBMClouds',
                                        'FBMRidged',
                                        'Billow',
                                        'RidgedMulti',
                                        'Cylinders',
                                        'Checkerboard',
                                    ]}
                                    value={settings.noise_type}
                                    on:change={(e) => updateNoiseType(e.detail.value)}
                                />
                            </div>
                        {/if}

                        {#if settings.vector_field_type === 'Image'}
                            <ImageSelector
                                fitMode={settings.image_fit_mode}
                                loadCommand="load_flow_vector_field_image"
                                onFitModeChange={(value) => updateImageFitMode(value)}
                            />
                            <WebcamControls
                                {webcamDevices}
                                {webcamActive}
                                onStartWebcam={startWebcam}
                                onStopWebcam={stopWebcam}
                            />
                            <div class="setting-item">
                                <label class="setting-label" for="flow-image-mirror-horizontal"
                                    >Mirror Horizontal:</label
                                >
                                <input
                                    id="flow-image-mirror-horizontal"
                                    type="checkbox"
                                    checked={settings.image_mirror_horizontal}
                                    on:change={(e) =>
                                        updateImageMirrorHorizontal(
                                            (e.target as HTMLInputElement).checked
                                        )}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="flow-image-mirror-vertical"
                                    >Mirror Vertical:</label
                                >
                                <input
                                    id="flow-image-mirror-vertical"
                                    type="checkbox"
                                    checked={settings.image_mirror_vertical}
                                    on:change={(e) =>
                                        updateImageMirrorVertical(
                                            (e.target as HTMLInputElement).checked
                                        )}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="flow-image-invert-tone"
                                    >Invert Tone:</label
                                >
                                <input
                                    id="flow-image-invert-tone"
                                    type="checkbox"
                                    checked={settings.image_invert_tone}
                                    on:change={(e) =>
                                        updateImageInvertTone(
                                            (e.target as HTMLInputElement).checked
                                        )}
                                />
                            </div>
                        {/if}

                        {#if settings.vector_field_type === 'Noise'}
                            <div class="setting-item">
                                <label class="setting-label" for="flow-noise-seed"
                                    >Noise Seed:</label
                                >
                                <NumberDragBox
                                    id="flow-noise-seed"
                                    value={settings.noise_seed}
                                    on:change={({ detail }) => updateNoiseSeed(detail)}
                                    min={0}
                                    step={1}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="flow-noise-scale"
                                    >Noise Scale:</label
                                >
                                <NumberDragBox
                                    id="flow-noise-scale"
                                    value={settings.noise_scale}
                                    on:change={({ detail }) => updateNoiseScale(detail)}
                                    min={0.001}
                                    max={10.0}
                                    step={0.01}
                                    precision={3}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="flow-noise-x">Noise X:</label>
                                <NumberDragBox
                                    id="flow-noise-x"
                                    value={settings.noise_x}
                                    on:change={({ detail }) => updateNoiseX(detail)}
                                    step={1.0}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="flow-noise-y">Noise Y:</label>
                                <NumberDragBox
                                    id="flow-noise-y"
                                    value={settings.noise_y}
                                    on:change={({ detail }) => updateNoiseY(detail)}
                                    step={1.0}
                                />
                            </div>
                            <div class="setting-item">
                                <label class="setting-label" for="flow-noise-dt"
                                    >Noise DT Multiplier:</label
                                >
                                <NumberDragBox
                                    id="flow-noise-dt"
                                    value={settings.noise_dt_multiplier}
                                    on:change={({ detail }) => updateNoiseDtMultiplier(detail)}
                                    min={0.0}
                                    max={10.0}
                                    step={0.1}
                                    precision={1}
                                />
                            </div>
                        {/if}
                        <div class="setting-item">
                            <label class="setting-label" for="flow-vector-magnitude"
                                >Vector Magnitude:</label
                            >
                            <NumberDragBox
                                id="flow-vector-magnitude"
                                value={settings.vector_magnitude}
                                on:change={({ detail }) => updateVectorMagnitude(detail)}
                                min={0.001}
                                max={5.0}
                                step={0.1}
                            />
                        </div>
                    </div>
                </div>

                <!-- Particle Settings -->
                <div class="settings-section">
                    <h3 class="section-header">Particles</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="flow-particle-lifetime"
                                >Particle Lifetime:</label
                            >
                            <NumberDragBox
                                id="flow-particle-lifetime"
                                value={settings.particle_lifetime}
                                on:change={({ detail }) => updateParticleLifetime(detail)}
                                min={0.1}
                                max={60.0}
                                step={0.1}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-particle-speed"
                                >Particle Speed:</label
                            >
                            <NumberDragBox
                                id="flow-particle-speed"
                                value={settings.particle_speed}
                                on:change={({ detail }) => updateParticleSpeed(detail)}
                                min={0.001}
                                max={100.0}
                                step={1.0}
                                precision={3}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-particle-size"
                                >Particle Size (pixels):</label
                            >
                            <NumberDragBox
                                id="flow-particle-size"
                                value={settings.particle_size}
                                on:change={({ detail }) => updateParticleSize(detail)}
                                min={1}
                                max={50}
                                step={1}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-particle-shape"
                                >Particle Shape:</label
                            >
                            <Selector
                                id="flow-particle-shape"
                                options={['Circle', 'Square', 'Triangle', 'Flower', 'Diamond']}
                                value={settings.particle_shape}
                                on:change={(e) => updateParticleShape(e.detail.value)}
                            />
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">
                                <input
                                    type="checkbox"
                                    checked={settings.particle_autospawn}
                                    on:change={(e) =>
                                        updateParticleAutospawn(
                                            (e.target as HTMLInputElement).checked
                                        )}
                                />
                                Auto-spawn Particles
                            </span>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">
                                <input
                                    type="checkbox"
                                    checked={state?.showParticles}
                                    on:change={(e) =>
                                        updateShowParticles((e.target as HTMLInputElement).checked)}
                                />
                                Show Particles
                            </span>
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-autospawn-rate"
                                >Autospawn Rate (particles/sec):</label
                            >
                            <NumberDragBox
                                id="flow-autospawn-rate"
                                value={settings.autospawn_rate}
                                on:change={({ detail }) => updateAutospawnRate(detail)}
                                min={0}
                                max={10000}
                                step={1}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-brush-spawn-rate"
                                >Brush Spawn Rate (particles/sec):</label
                            >
                            <NumberDragBox
                                id="flow-brush-spawn-rate"
                                value={settings.brush_spawn_rate}
                                on:change={({ detail }) => updateBrushSpawnRate(detail)}
                                min={1}
                                max={10000}
                                step={1}
                            />
                        </div>
                    </div>
                </div>

                <!-- Trail Settings -->
                <div class="settings-section">
                    <h3 class="section-header">Trails</h3>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <label class="setting-label" for="flow-trail-decay"
                                >Trail Decay Rate:</label
                            >
                            <NumberDragBox
                                id="flow-trail-decay"
                                value={settings.trail_decay_rate}
                                on:change={({ detail }) => updateTrailDecayRate(detail)}
                                min={0.0}
                                max={1.0}
                                step={0.001}
                                precision={3}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-trail-deposition"
                                >Trail Deposition Rate:</label
                            >
                            <NumberDragBox
                                id="flow-trail-deposition"
                                value={settings.trail_deposition_rate}
                                on:change={({ detail }) => updateTrailDepositionRate(detail)}
                                min={0.0}
                                max={1.0}
                                step={0.01}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-trail-diffusion"
                                >Trail Diffusion Rate:</label
                            >
                            <NumberDragBox
                                id="flow-trail-diffusion"
                                value={settings.trail_diffusion_rate}
                                on:change={({ detail }) => updateTrailDiffusionRate(detail)}
                                min={0.0}
                                max={1.0}
                                step={0.01}
                            />
                        </div>
                        <div class="setting-item">
                            <label class="setting-label" for="flow-trail-washout"
                                >Trail Wash Out Rate:</label
                            >
                            <NumberDragBox
                                id="flow-trail-washout"
                                value={settings.trail_wash_out_rate}
                                on:change={({ detail }) => updateTrailWashOutRate(detail)}
                                min={0.0}
                                max={1.0}
                                step={0.01}
                            />
                        </div>
                    </div>
                </div>
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
    import Button from './components/shared/Button.svelte';
    import NumberDragBox from './components/inputs/NumberDragBox.svelte';
    import Selector from './components/inputs/Selector.svelte';
    import ImageSelector from './components/shared/ImageSelector.svelte';
    import WebcamControls from './components/shared/WebcamControls.svelte';
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import CollapsibleFieldset from './components/shared/CollapsibleFieldset.svelte';
    import PresetFieldset from './components/shared/PresetFieldset.svelte';
    import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
    import ControlsPanel from './components/shared/ControlsPanel.svelte';
    import { AutoHideManager, createAutoHideEventListeners } from './utils/autoHide';
    import './shared-theme.css';
    import ColorSchemeSelector from './components/shared/ColorSchemeSelector.svelte';
    // Webcam state (mirrors SlimeMold approach)
    let webcamDevices: number[] = [];
    let webcamActive = false;

    async function loadWebcamDevices() {
        try {
            webcamDevices = await invoke('get_available_flow_webcam_devices');
        } catch (e) {
            console.error('Failed to load flow webcam devices:', e);
        }
    }

    async function startWebcam() {
        try {
            await invoke('start_flow_webcam_capture');
            webcamActive = true;
        } catch (e) {
            console.error('Failed to start flow webcam:', e);
        }
    }

    async function stopWebcam() {
        try {
            await invoke('stop_flow_webcam_capture');
            webcamActive = false;
        } catch (e) {
            console.error('Failed to stop flow webcam:', e);
        }
    }

    const dispatch = createEventDispatcher();

    export let menuPosition: string = 'middle';
    export let autoHideDelay: number = 3000;

    // Simulation state
    type Settings = {
        // Flow field parameters
        vector_field_type: string;
        noise_type: string;
        noise_seed: number;
        noise_scale: number;
        noise_x: number;
        noise_y: number;
        noise_dt_multiplier: number;
        vector_magnitude: number;

        // Image-based vector field parameters
        image_fit_mode: string;
        image_mirror_horizontal: boolean;
        image_mirror_vertical: boolean;
        image_invert_tone: boolean;

        // Particle parameters
        total_pool_size: number;
        particle_lifetime: number;
        particle_speed: number;
        particle_size: number;
        particle_shape: string;
        particle_autospawn: boolean;
        autospawn_rate: number;
        brush_spawn_rate: number;

        // Display parameters
        foreground_color_mode: string;

        // Trail parameters
        trail_decay_rate: number;
        trail_deposition_rate: number;
        trail_diffusion_rate: number;
        trail_wash_out_rate: number;
    };

    type State = {
        cursorSize: number;
        currentColorScheme: string;
        colorSchemeReversed: boolean;
        backgroundColorMode: string;
        foregroundColorMode: string;
        showParticles: boolean;
    };

    let settings: Settings | undefined = undefined;
    let state: State | undefined = undefined;

    // Preset and LUT state
    let current_preset = '';
    let available_presets: string[] = [];
    let available_luts: string[] = [];

    // UI state
    let show_about_section = false;

    // Simulation control state
    let running = false;
    let loading = false;
    let showUI = true;
    let currentFps = 0;
    let controlsVisible = true;

    // Auto-hide manager
    let autoHideManager: AutoHideManager;
    let eventListeners: { add: () => void; remove: () => void };

    // Event listeners
    let unlistenFps: (() => void) | null = null;
    let unlistenSimulationInitialized: (() => void) | null = null;

    async function updateBackgroundColorMode(value: string) {
        try {
            // Optimistically update local state like other sims
            if (state) state.backgroundColorMode = value;

            await invoke('update_simulation_state', {
                stateName: 'backgroundColorMode',
                value,
            });
            await syncStateFromBackend();
        } catch (e) {
            console.error('Failed to update background color mode:', e);
        }
    }

    async function updateForegroundColorMode(value: string) {
        try {
            // Optimistically update local state like other sims
            if (state) state.foregroundColorMode = value;

            await invoke('update_simulation_state', {
                stateName: 'foregroundColorMode',
                value,
            });
            await syncStateFromBackend();
        } catch (e) {
            console.error('Failed to update foreground color mode:', e);
        }
    }

    async function updateNoiseType(value: string) {
        settings!.noise_type = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'noise_type',
                value,
            });
        } catch (e) {
            console.error('Failed to update noise type:', e);
        }
    }

    async function updateParticleShape(value: string) {
        settings!.particle_shape = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'particleShape',
                value,
            });
        } catch (e) {
            console.error('Failed to update particle shape:', e);
        }
    }

    async function updateParticleAutospawn(value: boolean) {
        settings!.particle_autospawn = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'particleAutospawn',
                value,
            });
        } catch (e) {
            console.error('Failed to update particle autospawn:', e);
        }
    }

    async function updateShowParticles(value: boolean) {
        state!.showParticles = value;
        try {
            await invoke('update_simulation_state', {
                stateName: 'showParticles',
                value,
            });
        } catch (e) {
            console.error('Failed to update show particles:', e);
        }
    }

    async function returnToMenu() {
        try {
            await invoke('destroy_simulation');
            dispatch('back');
        } catch (error) {
            console.error('Failed to return to menu:', error);
        }
    }

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
        } catch (error) {
            console.error('Failed to toggle GUI:', error);
        }
    }

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
            console.error('Failed to stop simulation:', error);
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

    let isMousePressed = false;
    let currentMouseButton = 0;

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

            // Convert screen coordinates to world coordinates
            const devicePixelRatio = window.devicePixelRatio || 1;
            const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
            const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

            console.log(
                `Flow mouse interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: ${mouseEvent.button}`
            );

            try {
                await invoke('handle_mouse_interaction_screen', {
                    screenX: physicalCursorX,
                    screenY: physicalCursorY,
                    mouseButton: mouseEvent.button,
                });
            } catch (e) {
                console.error('Failed to handle Flow mouse interaction:', e);
            }
        } else if (event.type === 'mousemove') {
            if (isMousePressed) {
                const mouseEvent = event as MouseEvent;
                mouseEvent.preventDefault();

                // Convert screen coordinates to world coordinates
                const devicePixelRatio = window.devicePixelRatio || 1;
                const physicalCursorX = mouseEvent.clientX * devicePixelRatio;
                const physicalCursorY = mouseEvent.clientY * devicePixelRatio;

                // Use the same button state as when mouse was first pressed
                try {
                    await invoke('handle_mouse_interaction_screen', {
                        screenX: physicalCursorX,
                        screenY: physicalCursorY,
                        mouseButton: currentMouseButton,
                    });
                } catch (e) {
                    console.error('Failed to handle Flow mouse interaction:', e);
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
                    console.error('Failed to stop Flow mouse interaction:', e);
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
                `Flow context menu interaction at screen coords: (${physicalCursorX}, ${physicalCursorY}), button: 2`
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
                console.error('Failed to handle Flow context menu interaction:', e);
            }
        }

        // Handle auto-hide functionality
        if (autoHideManager) {
            autoHideManager.handleUserInteraction();
        }
    }

    async function startSimulation() {
        if (running || loading) return;

        loading = true;

        try {
            await invoke('start_simulation', { simulationType: 'flow' });
            currentFps = 0;
        } catch (e) {
            console.error('Failed to start simulation:', e);
            loading = false;
            running = false;
        }
    }

    // Setting update functions
    async function updateNoiseSeed(value: number) {
        // Validate the value before using it
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid noise seed value:', value);
            return;
        }

        settings!.noise_seed = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'noise_seed',
                value,
            });
        } catch (e) {
            console.error('Failed to update noise seed:', e);
        }
    }

    async function updateNoiseScale(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid noise scale value:', value);
            return;
        }

        settings!.noise_scale = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'noise_scale',
                value,
            });
        } catch (e) {
            console.error('Failed to update noise scale:', e);
        }
    }

    async function updateNoiseX(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid noise X value:', value);
            return;
        }

        settings!.noise_x = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'noise_x',
                value,
            });
        } catch (e) {
            console.error('Failed to update noise X:', e);
        }
    }

    async function updateNoiseY(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid noise Y value:', value);
            return;
        }

        settings!.noise_y = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'noise_y',
                value,
            });
        } catch (e) {
            console.error('Failed to update noise Y:', e);
        }
    }

    async function updateNoiseDtMultiplier(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid noise DT multiplier value:', value);
            return;
        }

        settings!.noise_dt_multiplier = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'noise_dt_multiplier',
                value,
            });
        } catch (e) {
            console.error('Failed to update noise DT multiplier:', e);
        }
    }

    async function updateVectorMagnitude(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid vector magnitude value:', value);
            return;
        }

        settings!.vector_magnitude = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'vector_magnitude',
                value,
            });
        } catch (e) {
            console.error('Failed to update vector magnitude:', e);
        }
    }

    async function updateParticleLifetime(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid particle lifetime value:', value);
            return;
        }

        settings!.particle_lifetime = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'particle_lifetime',
                value,
            });
        } catch (e) {
            console.error('Failed to update particle lifetime:', e);
        }
    }

    async function updateParticleSpeed(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid particle speed value:', value);
            return;
        }

        settings!.particle_speed = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'particle_speed',
                value,
            });
        } catch (e) {
            console.error('Failed to update particle speed:', e);
        }
    }

    async function updateParticleSize(value: number) {
        // Validate the value before using it
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid particle size value:', value);
            return;
        }

        // Ensure particle size is an integer
        const intValue = Math.round(value);
        settings!.particle_size = intValue;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'particle_size',
                value: intValue,
            });
        } catch (e) {
            console.error('Failed to update particle size:', e);
        }
    }

    async function updateAutospawnRate(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid autospawn rate value:', value);
            return;
        }

        settings!.autospawn_rate = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'autospawn_rate',
                value,
            });
        } catch (e) {
            console.error('Failed to update autospawn rate:', e);
        }
    }

    async function updateBrushSpawnRate(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid brush spawn rate value:', value);
            return;
        }

        settings!.brush_spawn_rate = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'brush_spawn_rate',
                value,
            });
        } catch (e) {
            console.error('Failed to update brush spawn rate:', e);
        }
    }

    async function killAllParticles() {
        try {
            await invoke('kill_all_particles');
            console.log('All particles killed successfully');
        } catch (e) {
            console.error('Failed to kill all particles:', e);
        }
    }

    async function updateTrailDecayRate(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid trail decay rate value:', value);
            return;
        }

        settings!.trail_decay_rate = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'trail_decay_rate',
                value,
            });
        } catch (e) {
            console.error('Failed to update trail decay rate:', e);
        }
    }

    async function updateTrailDepositionRate(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid trail deposition rate value:', value);
            return;
        }

        settings!.trail_deposition_rate = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'trail_deposition_rate',
                value,
            });
        } catch (e) {
            console.error('Failed to update trail deposition rate:', e);
        }
    }

    async function updateTrailDiffusionRate(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid trail diffusion rate value:', value);
            return;
        }

        settings!.trail_diffusion_rate = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'trail_diffusion_rate',
                value,
            });
        } catch (e) {
            console.error('Failed to update trail diffusion rate:', e);
        }
    }

    async function updateTrailWashOutRate(value: number) {
        if (typeof value !== 'number' || isNaN(value)) {
            console.error('Invalid trail wash out rate value:', value);
            return;
        }

        settings!.trail_wash_out_rate = value;
        try {
            await invoke('update_simulation_setting', {
                settingName: 'trail_wash_out_rate',
                value,
            });
        } catch (e) {
            console.error('Failed to update trail wash out rate:', e);
        }
    }

    async function updateColorScheme(colorSchemeName: string) {
        state!.currentColorScheme = colorSchemeName;
        try {
            await invoke('update_simulation_state', {
                stateName: 'currentColorScheme',
                value: colorSchemeName,
            });
            await syncStateFromBackend();
        } catch (e) {
            console.error('Failed to update color scheme:', e);
        }
    }

    async function updateColorSchemeReversed(reversed: boolean) {
        try {
            state!.colorSchemeReversed = reversed;
            await invoke('update_simulation_state', {
                stateName: 'colorSchemeReversed',
                value: reversed,
            });
            await syncStateFromBackend();
        } catch (e) {
            console.error('Failed to update color scheme reversed:', e);
        }
    }

    async function updateCursorSize(value: number) {
        state!.cursorSize = value;
        try {
            await invoke('update_simulation_state', {
                stateName: 'cursorSize',
                value,
            });
            await syncStateFromBackend();
        } catch (e) {
            console.error('Failed to update cursor size:', e);
        }
    }

    async function updatePreset(value: string) {
        current_preset = value;
        try {
            await invoke('apply_preset', { presetName: value });
            await syncSettingsFromBackend();
            console.log(`Applied preset: ${value}`);
        } catch (e) {
            console.error('Failed to apply preset:', e);
        }
    }

    async function savePreset(presetName: string) {
        try {
            await invoke('save_preset', { presetName: presetName.trim() });
            await loadAvailablePresets();
            current_preset = presetName.trim();
            console.log(`Saved preset: ${presetName}`);
        } catch (e) {
            console.error('Failed to save preset:', e);
        }
    }

    async function loadAvailablePresets() {
        try {
            available_presets = await invoke('get_presets_for_simulation_type', {
                simulationType: 'flow',
            });
            if (available_presets.length > 0 && !current_preset) {
                current_preset = available_presets[0];
            }
        } catch (e) {
            console.error('Failed to load available presets:', e);
        }
    }

    async function loadAvailableLuts() {
        try {
            available_luts = await invoke('get_available_color_schemes');
        } catch (e) {
            console.error('Failed to load available color schemes:', e);
        }
    }

    async function syncSettingsFromBackend() {
        try {
            const backendSettings = await invoke('get_current_settings');

            if (backendSettings) {
                // Use backend settings directly
                settings = backendSettings as Settings;
            }
        } catch (e) {
            console.error('Failed to sync settings from backend:', e);
        }
    }

    async function syncStateFromBackend() {
        try {
            const backendState = await invoke('get_current_state');
            if (backendState) {
                state = backendState as State;
            }
        } catch (e) {
            console.error('Failed to sync state from backend:', e);
        }
    }

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
            autoHideManager.handleUserInteraction();
        });
        eventListeners.add();

        // Listen for simulation-initialized event
        unlistenSimulationInitialized = await listen('simulation-initialized', async () => {
            running = true;
            loading = false;

            // Now that simulation is initialized, sync settings and load data
            await syncSettingsFromBackend();
            await syncStateFromBackend();
            await loadAvailablePresets();
            await loadAvailableLuts();
        });

        // Start simulation and keep loading until we get settings
        await startSimulation();

        unlistenFps = await listen('fps-update', (event: { payload: number }) => {
            currentFps = event.payload;
        });
        await loadWebcamDevices();
    });

    onDestroy(async () => {
        try {
            await invoke('destroy_simulation');
        } catch (error) {
            console.error('Failed to destroy simulation on component destroy:', error);
        }

        if (unlistenFps) {
            unlistenFps();
        }
        if (unlistenSimulationInitialized) {
            unlistenSimulationInitialized();
        }

        // Clean up auto-hide functionality
        if (eventListeners) {
            eventListeners.remove();
        }
        if (autoHideManager) {
            autoHideManager.cleanup();
        }
    });

    // Vector field type functions
    async function updateVectorFieldType(value: string) {
        settings!.vector_field_type = value;
        try {
            await invoke('set_flow_vector_field_type', {
                vectorFieldType: value,
            });
        } catch (e) {
            console.error('Failed to update vector field type:', e);
        }
    }

    // Image-related functions
    async function updateImageFitMode(value: string) {
        settings!.image_fit_mode = value;
        try {
            await invoke('set_flow_image_fit_mode', {
                fitMode: value,
            });
        } catch (e) {
            console.error('Failed to update image fit mode:', e);
        }
    }

    async function updateImageMirrorHorizontal(checked: boolean) {
        if (!settings) return;
        settings.image_mirror_horizontal = checked;
        try {
            await invoke('set_flow_image_mirror_horizontal', { mirror: checked });
        } catch (e) {
            console.error('Failed to update image mirror horizontal:', e);
        }
    }

    async function updateImageMirrorVertical(checked: boolean) {
        if (!settings) return;
        settings.image_mirror_vertical = checked;
        try {
            await invoke('set_flow_image_mirror_vertical', { mirror: checked });
        } catch (e) {
            console.error('Failed to update image mirror vertical:', e);
        }
    }

    async function updateImageInvertTone(checked: boolean) {
        if (!settings) return;
        settings.image_invert_tone = checked;
        try {
            await invoke('set_flow_image_invert_tone', { invert: checked });
        } catch (e) {
            console.error('Failed to update image invert tone:', e);
        }
    }
</script>

<style>
    .control-group {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
</style>
