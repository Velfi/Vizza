<div
    class="pattern-editor-backdrop"
    role="button"
    tabindex="0"
    on:click={() => dispatch('close')}
    on:keydown={(e) => e.key === 'Escape' && dispatch('close')}
>
    <div class="pattern-editor" role="document" on:click|stopPropagation>
        <div class="editor-header">
            <h3>Pattern Editor - Section {section}</h3>
            <button type="button" class="close-btn" on:click={() => dispatch('close')}>×</button>
        </div>

        <div class="editor-content">
            <div class="pattern-tabs">
                <button
                    class="tab-btn"
                    class:active={activeTab === 'beat'}
                    on:click={() => (activeTab = 'beat')}
                >
                    Beat
                </button>
                <button
                    class="tab-btn"
                    class:active={activeTab === 'bass'}
                    on:click={() => (activeTab = 'bass')}
                >
                    Bass
                </button>
                <button
                    class="tab-btn"
                    class:active={activeTab === 'melody'}
                    on:click={() => (activeTab = 'melody')}
                >
                    Melody
                </button>
            </div>

            <div class="pattern-grid">
                {#if activeTab === 'beat'}
                    <div class="simple-sequencer">
                        <div class="sequencer-header">
                            <h4>Beat Sequencer</h4>
                            <div class="pattern-length">16 Steps</div>
                        </div>

                        <div class="sequencer-grid">
                            <!-- Step numbers -->
                            <div class="step-numbers">
                                {#each Array(16) as _, i}
                                    <div class="step-number">{i + 1}</div>
                                {/each}
                            </div>

                            <!-- Kick row -->
                            <div class="instrument-row">
                                <div class="instrument-name">Kick</div>
                                <div class="step-buttons">
                                    {#each Array(16) as _, i}
                                        <button
                                            class="step-button kick"
                                            class:active={beatPattern[i]?.includes('kick')}
                                            on:click={() => toggleBeatCell(i, 'kick')}
                                        >
                                            ●
                                        </button>
                                    {/each}
                                </div>
                            </div>

                            <!-- Snare row -->
                            <div class="instrument-row">
                                <div class="instrument-name">Snare</div>
                                <div class="step-buttons">
                                    {#each Array(16) as _, i}
                                        <button
                                            class="step-button snare"
                                            class:active={beatPattern[i]?.includes('snare')}
                                            on:click={() => toggleBeatCell(i, 'snare')}
                                        >
                                            ●
                                        </button>
                                    {/each}
                                </div>
                            </div>

                            <!-- Hihat row -->
                            <div class="instrument-row">
                                <div class="instrument-name">Hihat</div>
                                <div class="step-buttons">
                                    {#each Array(16) as _, i}
                                        <button
                                            class="step-button hihat"
                                            class:active={beatPattern[i]?.includes('hihat')}
                                            on:click={() => toggleBeatCell(i, 'hihat')}
                                        >
                                            ×
                                        </button>
                                    {/each}
                                </div>
                            </div>
                        </div>
                    </div>
                {:else if activeTab === 'bass'}
                    <div class="simple-sequencer">
                        <div class="sequencer-header">
                            <h4>Bass Sequencer</h4>
                            <div class="pattern-length">8 Steps</div>
                        </div>

                        <div class="sequencer-grid">
                            <!-- Step numbers -->
                            <div class="step-numbers">
                                {#each Array(8) as _, i}
                                    <div class="step-number">{i + 1}</div>
                                {/each}
                            </div>

                            <!-- Bass row -->
                            <div class="instrument-row">
                                <div class="instrument-name">Bass</div>
                                <div class="step-buttons">
                                    {#each Array(8) as _, i}
                                        <button
                                            class="step-button bass"
                                            class:active={bassPattern[i] !== ''}
                                            on:click={() => cycleBassNote(i)}
                                            title={bassPattern[i] || 'Empty'}
                                        >
                                            {bassPattern[i] || '-'}
                                        </button>
                                    {/each}
                                </div>
                            </div>
                        </div>

                        <div class="note-legend">
                            <span
                                >Click to cycle through notes: C2 → D2 → E2 → F2 → G2 → A2 → B2 → C3
                                → (empty)</span
                            >
                        </div>
                    </div>
                {:else if activeTab === 'melody'}
                    <div class="simple-sequencer">
                        <div class="sequencer-header">
                            <h4>Melody Sequencer</h4>
                            <div class="pattern-length">8 Steps</div>
                        </div>

                        <div class="sequencer-grid">
                            <!-- Step numbers -->
                            <div class="step-numbers">
                                {#each Array(8) as _, i}
                                    <div class="step-number">{i + 1}</div>
                                {/each}
                            </div>

                            <!-- Melody row -->
                            <div class="instrument-row">
                                <div class="instrument-name">Melody</div>
                                <div class="step-buttons">
                                    {#each Array(8) as _, i}
                                        <button
                                            class="step-button melody"
                                            class:active={melodyPattern[i] !== ''}
                                            on:click={() => cycleMelodyNote(i)}
                                            title={melodyPattern[i] || 'Empty'}
                                        >
                                            {melodyPattern[i] || '-'}
                                        </button>
                                    {/each}
                                </div>
                            </div>
                        </div>

                        <div class="note-legend">
                            <span
                                >Click to cycle through notes: C4 → D4 → E4 → F4 → G4 → A4 → B4 → C5
                                → (empty)</span
                            >
                        </div>
                    </div>
                {/if}
            </div>
        </div>

        <div class="editor-actions">
            <button type="button" class="btn-secondary" on:click={() => dispatch('close')}
                >Cancel</button
            >
            <button type="button" class="btn-primary" on:click={savePattern}>Save Pattern</button>
        </div>
    </div>
</div>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import type { Pattern, BeatEvent, BassEvent, MelodyEvent } from '../../audio/musicEngine';

    const dispatch = createEventDispatcher();

    export let section: string;
    export let pattern: Pattern;

    let activeTab: 'beat' | 'bass' | 'melody' = 'beat';
    let beatPattern: string[][] = Array(16)
        .fill(null)
        .map(() => []);
    let bassPattern: string[] = Array(8).fill('');
    let melodyPattern: string[] = Array(8).fill('');

    // Initialize patterns from props
    $: {
        if (pattern) {
            // Initialize beat pattern
            beatPattern = Array(16)
                .fill(null)
                .map(() => []);
            pattern.beat.forEach((event) => {
                const time = parseTimeString(event.time);
                if (time >= 0 && time < 16) {
                    beatPattern[time].push(event.type);
                }
            });

            // Initialize bass pattern
            bassPattern = Array(8).fill('');
            pattern.bass.forEach((event) => {
                const time = parseTimeString(event.time);
                if (time >= 0 && time < 8) {
                    bassPattern[time] = event.note;
                }
            });

            // Initialize melody pattern
            melodyPattern = Array(8).fill('');
            pattern.melody.forEach((event) => {
                const time = parseTimeString(event.time);
                if (time >= 0 && time < 8) {
                    melodyPattern[time] = event.note;
                }
            });
        }
    }

    function parseTimeString(timeStr: string): number {
        const parts = timeStr.split(':').map(Number);
        const [, beat, subdivision] = parts;
        return beat * 4 + subdivision;
    }

    function toggleBeatCell(index: number, type: 'kick' | 'snare' | 'hihat') {
        const cell = beatPattern[index];
        const typeIndex = cell.indexOf(type);
        if (typeIndex >= 0) {
            cell.splice(typeIndex, 1);
        } else {
            cell.push(type);
        }
        beatPattern = [...beatPattern];
    }

    function cycleBassNote(index: number) {
        const bassNotes = ['', 'C2', 'D2', 'E2', 'F2', 'G2', 'A2', 'B2', 'C3'];
        const currentNote = bassPattern[index] || '';
        const currentIndex = bassNotes.indexOf(currentNote);
        const nextIndex = (currentIndex + 1) % bassNotes.length;
        bassPattern[index] = bassNotes[nextIndex];
        bassPattern = [...bassPattern];
    }

    function cycleMelodyNote(index: number) {
        const melodyNotes = ['', 'C4', 'D4', 'E4', 'F4', 'G4', 'A4', 'B4', 'C5'];
        const currentNote = melodyPattern[index] || '';
        const currentIndex = melodyNotes.indexOf(currentNote);
        const nextIndex = (currentIndex + 1) % melodyNotes.length;
        melodyPattern[index] = melodyNotes[nextIndex];
        melodyPattern = [...melodyPattern];
    }

    function savePattern() {
        const beatEvents: BeatEvent[] = [];
        beatPattern.forEach((cell, index) => {
            cell.forEach((type) => {
                const measure = Math.floor(index / 16);
                const beat = Math.floor((index % 16) / 4);
                const subdivision = index % 4;
                beatEvents.push({
                    time: `${measure}:${beat}:${subdivision}`,
                    type: type as 'kick' | 'snare' | 'hihat',
                });
            });
        });

        const bassEvents: BassEvent[] = [];
        bassPattern.forEach((note, index) => {
            if (note) {
                const beat = index * 0.5;
                bassEvents.push({
                    time: `0:${beat}:0`,
                    note,
                });
            }
        });

        const melodyEvents: MelodyEvent[] = [];
        melodyPattern.forEach((note, index) => {
            if (note) {
                const beat = index * 0.5;
                melodyEvents.push({
                    time: `0:${beat}:0`,
                    note,
                });
            }
        });

        const newPattern: Pattern = {
            beat: beatEvents,
            bass: bassEvents,
            melody: melodyEvents,
        };

        dispatch('save', { section, pattern: newPattern });
    }
</script>

<style>
    .pattern-editor-backdrop {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.8);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2000;
    }

    .pattern-editor {
        background: rgba(0, 0, 0, 0.9);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 8px;
        width: 90vw;
        max-width: 800px;
        max-height: 80vh;
        overflow: hidden;
        display: flex;
        flex-direction: column;
    }

    .editor-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.2);
    }

    .editor-header h3 {
        margin: 0;
        color: rgba(255, 255, 255, 0.9);
        font-size: 1.2rem;
    }

    .close-btn {
        background: none;
        border: none;
        color: rgba(255, 255, 255, 0.7);
        font-size: 1.5rem;
        cursor: pointer;
        padding: 0;
        width: 30px;
        height: 30px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .close-btn:hover {
        color: rgba(255, 255, 255, 0.9);
    }

    .editor-content {
        flex: 1;
        overflow: auto;
        padding: 1rem;
    }

    .pattern-tabs {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }

    .tab-btn {
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.7);
        border: 1px solid rgba(255, 255, 255, 0.3);
        padding: 0.5rem 1rem;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.3s;
    }

    .tab-btn.active {
        background: rgba(100, 108, 255, 0.3);
        border-color: #646cff;
        color: white;
    }

    .pattern-grid {
        background: rgba(255, 255, 255, 0.05);
        border-radius: 4px;
        padding: 1rem;
    }

    .editor-actions {
        display: flex;
        gap: 1rem;
        padding: 1rem;
        border-top: 1px solid rgba(255, 255, 255, 0.2);
        justify-content: flex-end;
    }

    .btn-primary,
    .btn-secondary {
        padding: 0.5rem 1rem;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
        transition: all 0.3s;
    }

    .btn-primary {
        background: #646cff;
        color: white;
        border: 1px solid #646cff;
    }

    .btn-primary:hover {
        background: #5a5fcf;
    }

    .btn-secondary {
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.9);
        border: 1px solid rgba(255, 255, 255, 0.3);
    }

    .btn-secondary:hover {
        background: rgba(255, 255, 255, 0.2);
    }

    /* Simple Sequencer Styles */
    .simple-sequencer {
        margin-bottom: 16px;
    }

    .sequencer-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 16px;
        padding-bottom: 8px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.2);
    }

    .sequencer-header h4 {
        margin: 0;
        color: rgba(255, 255, 255, 0.9);
        font-size: 16px;
        font-weight: 600;
    }

    .pattern-length {
        color: rgba(255, 255, 255, 0.6);
        font-size: 12px;
        font-weight: 500;
    }

    .sequencer-grid {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 8px;
        padding: 16px;
        margin-bottom: 12px;
    }

    .step-numbers {
        display: flex;
        gap: 4px;
        margin-bottom: 12px;
        padding-left: 80px; /* Space for instrument names */
    }

    .step-number {
        width: 32px;
        height: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 11px;
        color: rgba(255, 255, 255, 0.6);
        font-weight: 500;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 4px;
    }

    .instrument-row {
        display: flex;
        align-items: center;
        margin-bottom: 8px;
    }

    .instrument-name {
        width: 70px;
        font-size: 12px;
        color: rgba(255, 255, 255, 0.8);
        font-weight: 500;
        flex-shrink: 0;
    }

    .step-buttons {
        display: flex;
        gap: 4px;
        flex: 1;
    }

    .step-button {
        width: 32px;
        height: 32px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.7);
        border-radius: 6px;
        cursor: pointer;
        transition: all 0.2s ease;
        font-size: 12px;
        font-weight: 500;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .step-button:hover {
        background: rgba(255, 255, 255, 0.2);
        border-color: rgba(255, 255, 255, 0.5);
        transform: translateY(-1px);
    }

    .step-button.active {
        background: #646cff;
        border-color: #646cff;
        color: white;
        box-shadow: 0 2px 8px rgba(100, 108, 255, 0.3);
    }

    .step-button.kick.active {
        background: #ff6b6b;
        border-color: #ff6b6b;
        box-shadow: 0 2px 8px rgba(255, 107, 107, 0.3);
    }

    .step-button.snare.active {
        background: #4ecdc4;
        border-color: #4ecdc4;
        box-shadow: 0 2px 8px rgba(78, 205, 196, 0.3);
    }

    .step-button.hihat.active {
        background: #ffe66d;
        border-color: #ffe66d;
        box-shadow: 0 2px 8px rgba(255, 230, 109, 0.3);
        color: #333;
    }

    .step-button.bass.active {
        background: #a8e6cf;
        border-color: #a8e6cf;
        box-shadow: 0 2px 8px rgba(168, 230, 207, 0.3);
        color: #333;
    }

    .step-button.melody.active {
        background: #ffb3ba;
        border-color: #ffb3ba;
        box-shadow: 0 2px 8px rgba(255, 179, 186, 0.3);
        color: #333;
    }

    .note-legend {
        text-align: center;
        font-size: 11px;
        color: rgba(255, 255, 255, 0.6);
        font-style: italic;
    }
</style>
