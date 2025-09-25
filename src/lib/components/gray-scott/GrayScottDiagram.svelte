<div class="diagram-container">
    <div class="instructions">
        <span>Drag the colorful handles to adjust reaction-diffusion parameters</span>
    </div>

    <div class="plots-container">
        <!-- Feed Rate vs Kill Rate Plot -->
        <div class="plot-section">
            <XYPlot
                xValue={feedRate}
                yValue={killRate}
                xRange={feedRateRange}
                yRange={killRateRange}
                xLabel="Feed Rate (F)"
                yLabel="Kill Rate (K)"
                title="Feed Rate (F) vs Kill Rate (K)"
                handleColor="#ef4444"
                handleStrokeColor="#dc2626"
                valueLabelX="F"
                valueLabelY="K"
                width={400}
                height={300}
                margin={40}
                on:update={handleFeedKillUpdate}
            />
        </div>

        <!-- Diffusion U vs Diffusion V Plot -->
        <div class="plot-section">
            <XYPlot
                xValue={diffusionRateU}
                yValue={diffusionRateV}
                xRange={diffusionRange}
                yRange={diffusionRange}
                xLabel="Diffusion Rate U (Du)"
                yLabel="Diffusion Rate V (Dv)"
                title="Diffusion Rate U (Du) vs Diffusion Rate V (Dv)"
                handleColor="#22c55e"
                handleStrokeColor="#16a34a"
                valueLabelX="Du"
                valueLabelY="Dv"
                width={400}
                height={300}
                margin={40}
                on:update={handleDiffusionUpdate}
            />
        </div>
    </div>

    <div class="parameter-display">
        <div class="parameter-grid">
            <div class="parameter-item">
                <span class="parameter-label">Feed Rate (F):</span>
                <NumberDragBox
                    bind:value={feedRate}
                    min={feedRateRange.min}
                    max={feedRateRange.max}
                    step={0.001}
                    precision={3}
                    on:change={handleFeedRateChange}
                />
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Kill Rate (K):</span>
                <NumberDragBox
                    bind:value={killRate}
                    min={killRateRange.min}
                    max={killRateRange.max}
                    step={0.001}
                    precision={3}
                    on:change={handleKillRateChange}
                />
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Diffusion U (Du):</span>
                <NumberDragBox
                    bind:value={diffusionRateU}
                    min={diffusionRange.min}
                    max={diffusionRange.max}
                    step={0.001}
                    precision={3}
                    on:change={handleDiffusionRateUChange}
                />
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Diffusion V (Dv):</span>
                <NumberDragBox
                    bind:value={diffusionRateV}
                    min={diffusionRange.min}
                    max={diffusionRange.max}
                    step={0.001}
                    precision={3}
                    on:change={handleDiffusionRateVChange}
                />
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Timestep (Δt):</span>
                <NumberDragBox
                    bind:value={internalTimestep}
                    min={timestepRange.min}
                    max={timestepRange.max}
                    step={0.1}
                    precision={1}
                    on:change={handleTimestepChange}
                />
            </div>
        </div>
    </div>
</div>

<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import XYPlot from './XYPlot.svelte';
    import NumberDragBox from '../inputs/NumberDragBox.svelte';

    export let feedRate: number = 0.055;
    export let killRate: number = 0.062;
    export let diffusionRateU: number = 0.1;
    export let diffusionRateV: number = 0.05;
    export let timestep: number = 1.0;

    const dispatch = createEventDispatcher();

    // Parameter ranges
    const feedRateRange = { min: 0.01, max: 1.0 };
    const killRateRange = { min: 0.01, max: 1.0 };
    const diffusionRange = { min: 0.01, max: 2.0 };
    const timestepRange = { min: 0.1, max: 10.0 };

    // Internal state for timestep
    let internalTimestep = timestep;

    // Handle timestep change from NumberDragBox
    function handleTimestepChange(event: CustomEvent<number>) {
        const newTimestep = event.detail;
        internalTimestep = newTimestep;
        dispatch('update', { setting: 'timestep', value: newTimestep });
    }

    // Handle individual parameter changes from NumberDragBox
    function handleFeedRateChange(event: CustomEvent<number>) {
        const newValue = event.detail;
        feedRate = newValue;
        dispatch('update', { setting: 'feed_rate', value: newValue });
    }

    function handleKillRateChange(event: CustomEvent<number>) {
        const newValue = event.detail;
        killRate = newValue;
        dispatch('update', { setting: 'kill_rate', value: newValue });
    }

    function handleDiffusionRateUChange(event: CustomEvent<number>) {
        const newValue = event.detail;
        diffusionRateU = newValue;
        dispatch('update', { setting: 'diffusion_rate_u', value: newValue });
    }

    function handleDiffusionRateVChange(event: CustomEvent<number>) {
        const newValue = event.detail;
        diffusionRateV = newValue;
        dispatch('update', { setting: 'diffusion_rate_v', value: newValue });
    }

    // Handle XY plot updates
    function handleFeedKillUpdate(event: CustomEvent) {
        const { x, y } = event.detail;
        dispatch('update', { setting: 'feed_rate', value: x });
        dispatch('update', { setting: 'kill_rate', value: y });
    }

    function handleDiffusionUpdate(event: CustomEvent) {
        const { x, y } = event.detail;
        dispatch('update', { setting: 'diffusion_rate_u', value: x });
        dispatch('update', { setting: 'diffusion_rate_v', value: y });
    }

    // Sync with external props when they change
    $: if (timestep !== internalTimestep && Math.abs(timestep - internalTimestep) > 0.01) {
        internalTimestep = timestep;
    }
</script>

<style>
    .diagram-container {
        width: 100%;
        max-width: 100%;
        margin: 0;
        padding: 0;
    }

    .instructions {
        margin: 0 0 15px 0;
        color: rgba(255, 255, 255, 0.7);
        font-size: 0.9em;
        font-style: italic;
    }

    .plots-container {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
        margin-bottom: 1rem;
    }

    .plot-section {
        display: flex;
        flex-direction: column;
    }

    .parameter-display {
        margin: 1rem 0;
        padding: 1rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 4px;
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .parameter-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
        gap: 0.75rem;
    }

    .parameter-item {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding: 0.75rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 4px;
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .parameter-label {
        color: rgba(255, 255, 255, 0.8);
        font-size: 0.875rem;
        margin-bottom: 0.25rem;
    }

    /* Mobile responsive styles */
    @media (max-width: 768px) {
        .instructions {
            font-size: 0.8em;
            text-align: center;
        }

        .plots-container {
            grid-template-columns: 1fr;
            gap: 1rem;
        }

        .parameter-grid {
            grid-template-columns: 1fr;
            gap: 0.5rem;
        }

        .parameter-item {
            padding: 0.75rem;
        }
    }
</style>
