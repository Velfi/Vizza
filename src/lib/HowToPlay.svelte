<div class="how-to-play-container">
    <div class="controls">
        <Button variant="default" on:click={goBack}>← Back</Button>

        <div class="title">How To Play</div>
    </div>

    <div class="content">
        <div class="grid-container">
            <!-- Camera Controls -->
            <div class="section camera-controls">
                <h2>📹 Camera Controls</h2>
                <div class="controls-grid">
                    <div class="control-item">
                        <span class="key">W / ↑</span>
                        <span class="description">Pan up</span>
                    </div>
                    <div class="control-item">
                        <span class="key">S / ↓</span>
                        <span class="description">Pan down</span>
                    </div>
                    <div class="control-item">
                        <span class="key">A / ←</span>
                        <span class="description">Pan left</span>
                    </div>
                    <div class="control-item">
                        <span class="key">D / →</span>
                        <span class="description">Pan right</span>
                    </div>
                    <div class="control-item">
                        <span class="key">Q</span>
                        <span class="description">Zoom out</span>
                    </div>
                    <div class="control-item">
                        <span class="key">E</span>
                        <span class="description">Zoom in</span>
                    </div>
                    <div class="control-item">
                        <span class="key">Mouse Wheel</span>
                        <span class="description">Zoom in/out towards cursor</span>
                    </div>
                    <div class="control-item">
                        <span class="key">C</span>
                        <span class="description">Reset camera</span>
                    </div>
                </div>
            </div>

            <!-- Simulation Controls -->
            <div class="section simulation-controls">
                <h2>🎮 Simulation Controls</h2>
                <div class="controls-grid">
                    <div class="control-item">
                        <span class="key">/</span>
                        <span class="description">Toggle UI visibility</span>
                    </div>
                </div>
            </div>
        </div>
    </div>
</div>

<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import Button from './components/shared/Button.svelte';
    import './shared-theme.css';

    const dispatch = createEventDispatcher();

    function goBack() {
        dispatch('back');
    }
</script>

<style>
    .how-to-play-container {
        display: flex;
        flex-direction: column;
        height: 100vh;
        background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
        color: white;
    }

    .controls {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem;
        background: rgba(0, 0, 0, 0.8);
        backdrop-filter: blur(10px);
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .title {
        font-size: 1.5rem;
        font-weight: bold;
        color: rgba(255, 255, 255, 0.9);
    }

    .content {
        max-width: 800px;
        margin: 0 auto;
        padding: 1rem;
    }

    .grid-container {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1rem;
        margin-top: 1rem;
    }

    .section {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 8px;
        padding: 1rem;
        backdrop-filter: blur(10px);
    }

    .section h2 {
        margin: 0 0 0.75rem 0;
        color: #646cff;
        font-size: 1.1rem;
        font-weight: bold;
    }

    .controls-grid {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        flex: 1;
    }

    .control-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 0.5rem;
    }

    .key {
        background: rgba(255, 255, 255, 0.2);
        border: 1px solid rgba(255, 255, 255, 0.4);
        border-radius: 4px;
        padding: 0.2rem 0.4rem;
        font-family: monospace;
        font-size: 0.8rem;
        min-width: 70px;
        text-align: center;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
        flex-shrink: 0;
    }

    .description {
        color: rgba(255, 255, 255, 0.9);
        font-size: 0.85rem;
        flex: 1;
    }

    /* Specific section styling */
    .camera-controls {
        grid-column: span 1;
    }

    .simulation-controls {
        grid-column: span 1;
    }

    @media (max-width: 768px) {
        .content {
            padding: 0.5rem;
        }

        .grid-container {
            grid-template-columns: 1fr;
            gap: 0.5rem;
        }

        .camera-controls,
        .simulation-controls {
            grid-column: span 1;
        }

        .control-item {
            flex-direction: column;
            align-items: flex-start;
            gap: 0.2rem;
        }

        .key {
            min-width: auto;
            width: fit-content;
        }

        .section {
            padding: 0.5rem;
        }
    }
</style>
