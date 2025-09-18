<SimulationLayout
    simulationName="Dye Advection"
    {running}
    loading={loading}
    {showUI}
    {currentFps}
    {controlsVisible}
    {menuPosition}
    on:back={returnToMenu}
    on:toggleUI={toggleBackendGui}
    on:pause={stopSimulation}
    on:resume={resumeSimulation}
>
    <form on:submit|preventDefault>
        <fieldset>
            <legend>Controls</legend>
            <div class="control-group">
                <span>Left click to inject dye and add swirl impulse.</span>
            </div>
        </fieldset>
        <PostProcessingMenu simulationType="fluids" />
    </form>
</SimulationLayout>

<CameraControls enabled={true} on:toggleGui={toggleBackendGui} on:togglePause={togglePause} />

<script lang="ts">
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import SimulationLayout from './components/shared/SimulationLayout.svelte';
    import CameraControls from './components/shared/CameraControls.svelte';
    import PostProcessingMenu from './components/shared/PostProcessingMenu.svelte';
    import './shared-theme.css';

    export let menuPosition: string = 'middle';
    export let autoHideDelay: number = 3000;

    const dispatch = createEventDispatcher();

    let running = false;
    let loading = false;
    let showUI = true;
    let currentFps = 0;
    let controlsVisible = true;

    let unlistenFps: (() => void) | null = null;
    let unlistenSimulationInitialized: (() => void) | null = null;

    async function startSimulation() {
        if (running || loading) return;
        loading = true;
        try {
            await invoke('start_simulation', { simulationType: 'fluids' });
            currentFps = 0;
        } catch (e) {
            console.error('Failed to start Fluids:', e);
            loading = false;
            running = false;
        }
    }

    async function toggleBackendGui() {
        try {
            await invoke('toggle_gui');
            showUI = !showUI;
        } catch (error) {
            console.error('Failed to toggle GUI:', error);
        }
    }

    async function stopSimulation() {
        try {
            await invoke('pause_simulation');
            running = false;
        } catch (error) {
            console.error('Failed to stop simulation:', error);
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

    async function togglePause() {
        if (running) {
            await stopSimulation();
        } else {
            await resumeSimulation();
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

    onMount(async () => {
        unlistenSimulationInitialized = await listen('simulation-initialized', async () => {
            running = true;
            loading = false;
        });
        unlistenFps = await listen('fps-update', (event: { payload: number }) => {
            currentFps = event.payload;
        });
        await startSimulation();
    });

    onDestroy(async () => {
        try {
            await invoke('destroy_simulation');
        } catch (error) {
            console.error('Failed to destroy simulation on component destroy:', error);
        }
        if (unlistenFps) unlistenFps();
        if (unlistenSimulationInitialized) unlistenSimulationInitialized();
    });

    import { listen } from '@tauri-apps/api/event';
</script>

<style>
    .control-group {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
</style>

