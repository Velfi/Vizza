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

    <!-- HTML-based timestep slider -->
    <div class="timestep-section">
        <div class="timestep-header">
            <h4>Timestep (Δt)</h4>
            <span class="timestep-value">{internalTimestep.toFixed(1)}</span>
        </div>
        <div class="slider-container">
            <input
                type="range"
                min={timestepRange.min}
                max={timestepRange.max}
                step="0.1"
                value={internalTimestep}
                on:input={handleTimestepChange}
                class="timestep-slider"
                aria-label="Timestep slider"
            />
            <div class="slider-labels">
                <span>{timestepRange.min}</span>
                <span>{timestepRange.max}</span>
            </div>
        </div>
    </div>

    <div class="parameter-display">
        <div class="parameter-grid">
            <div class="parameter-item">
                <span class="parameter-label">Feed Rate (F):</span>
                <span class="parameter-value">{feedRate.toFixed(3)}</span>
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Kill Rate (K):</span>
                <span class="parameter-value">{killRate.toFixed(3)}</span>
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Diffusion U (Du):</span>
                <span class="parameter-value">{diffusionRateU.toFixed(3)}</span>
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Diffusion V (Dv):</span>
                <span class="parameter-value">{diffusionRateV.toFixed(3)}</span>
            </div>
            <div class="parameter-item">
                <span class="parameter-label">Timestep (Δt):</span>
                <span class="parameter-value">{internalTimestep.toFixed(1)}</span>
            </div>
        </div>
    </div>
</div>

<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import XYPlot from './XYPlot.svelte';

    export let feedRate: number = 0.055;
    export let killRate: number = 0.062;
    export let diffusionRateU: number = 0.1;
    export let diffusionRateV: number = 0.05;
    export let timestep: number = 1.0;

    const dispatch = createEventDispatcher();

    // Parameter ranges
    const feedRateRange = { min: 0.02, max: 0.08 };
    const killRateRange = { min: 0.04, max: 0.08 };
    const diffusionRange = { min: 0.05, max: 0.3 };
    const timestepRange = { min: 0.1, max: 3.0 };

    // Internal state for timestep
    let internalTimestep = timestep;

    // Handle timestep slider change
    function handleTimestepChange(event: Event) {
        const target = event.target as HTMLInputElement;
        const newTimestep = parseFloat(target.value);
        internalTimestep = newTimestep;
        dispatch('update', { setting: 'timestep', value: newTimestep });
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

    .timestep-section {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 4px;
        padding: 1rem;
        margin-bottom: 1rem;
    }

    .timestep-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }

    .timestep-header h4 {
        margin: 0;
        color: rgba(255, 255, 255, 0.9);
        font-size: 1rem;
    }

    .timestep-value {
        color: rgba(255, 255, 255, 0.9);
        font-weight: bold;
        font-family: monospace;
        font-size: 1rem;
    }

    .slider-container {
        position: relative;
    }

    .timestep-slider {
        width: 100%;
        height: 8px;
        border-radius: 4px;
        background: rgba(255, 255, 255, 0.2);
        outline: none;
        -webkit-appearance: none;
        appearance: none;
        cursor: pointer;
    }

    .timestep-slider::-webkit-slider-thumb {
        -webkit-appearance: none;
        appearance: none;
        width: 20px;
        height: 20px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.8);
        border: 2px solid rgba(255, 255, 255, 0.3);
        cursor: pointer;
    }

    .timestep-slider::-moz-range-thumb {
        width: 20px;
        height: 20px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.8);
        border: 2px solid rgba(255, 255, 255, 0.3);
        cursor: pointer;
        border: none;
    }

    .slider-labels {
        display: flex;
        justify-content: space-between;
        margin-top: 0.5rem;
        color: rgba(255, 255, 255, 0.6);
        font-size: 0.875rem;
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
        justify-content: space-between;
        align-items: center;
        padding: 0.5rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 4px;
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .parameter-label {
        color: rgba(255, 255, 255, 0.8);
        font-size: 0.875rem;
    }

    .parameter-value {
        color: rgba(255, 255, 255, 0.9);
        font-weight: bold;
        font-family: monospace;
        font-size: 0.875rem;
    }

    .controls-info {
        margin: 1rem 0;
        padding: 1rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 4px;
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .controls-header h4 {
        margin: 0 0 0.75rem 0;
        color: rgba(255, 255, 255, 0.9);
        font-size: 1rem;
    }

    .controls-info ul {
        margin: 0 0 1rem 0;
        padding-left: 1.5rem;
        color: rgba(255, 255, 255, 0.8);
    }

    .controls-info li {
        margin-bottom: 0.5rem;
        font-size: 0.875rem;
    }

    .equation-note {
        padding: 0.75rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 4px;
        border-left: 3px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.7);
        font-size: 0.8rem;
        line-height: 1.4;
        word-break: break-word;
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

        .timestep-section {
            padding: 0.75rem;
        }

        .timestep-header h4 {
            font-size: 0.9rem;
        }

        .timestep-value {
            font-size: 0.9rem;
        }

        .parameter-grid {
            grid-template-columns: 1fr;
            gap: 0.5rem;
        }

        .parameter-item {
            padding: 0.75rem;
        }

        .controls-info {
            padding: 0.75rem;
        }

        .controls-info li {
            font-size: 0.8rem;
        }

        .equation-note {
            font-size: 0.75rem;
        }
    }
</style>
