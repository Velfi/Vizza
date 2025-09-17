<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
    class="dialog-backdrop"
    role="button"
    tabindex="0"
    on:click={() => dispatch('close')}
    on:keydown={(e) => e.key === 'Escape' && dispatch('close')}
>
    <div class="dialog" role="document" on:click|stopPropagation>
        <h3 id="music-title">Ambient Music</h3>

        <form on:submit|preventDefault>
            <!-- Main Controls + Progress -->
            <div class="top-grid">
                <fieldset>
                    <legend>Music Controls</legend>
                    {#if audioContextSuspended}
                        <div class="control-group">
                            <button type="button" class="btn-primary" on:click={handleStartAudio}>
                                Initialize Audio
                            </button>
                            <span class="status-text">Click to enable audio playback</span>
                        </div>
                    {:else}
                        <div class="control-group music-controls">
                            <button
                                type="button"
                                class="btn-music"
                                class:active={isPlaying}
                                on:click={handleStart}
                            >
                                ▶️ Start
                            </button>
                            <button type="button" class="btn-music" on:click={handleStop}>
                                ⏹️ Stop
                            </button>
                            <button type="button" class="btn-music" on:click={handlePause}>
                                ⏸️ Pause
                            </button>
                            <button type="button" class="btn-music" on:click={handleRestart}>
                                🔄 Restart
                            </button>
                        </div>
                        <!-- Actions -->
                        <div class="control-group actions-group">
                            <Button variant="default" on:click={() => randomize()}>Randomize</Button
                            >
                            <Button variant="default" on:click={() => regeneratePatterns()}
                                >Regenerate</Button
                            >
                            <Button variant="default" on:click={() => forceStructureTransition()}
                                >Next Structure</Button
                            >
                            <Button variant="default" on:click={() => saveLocal()}>Save</Button>
                            <Button variant="default" on:click={() => loadLocal()}>Load</Button>
                        </div>
                    {/if}
                    <div class="control-group">
                        <span class="status-text"
                            >Current Section: <strong>{currentSection}</strong></span
                        >
                    </div>
                    <div class="control-group">
                        <span class="status-text"
                            >Song Structure: <strong>{songStructure}</strong></span
                        >
                    </div>
                    <div class="control-group">
                        <span class="status-text">Status: <strong>{musicStatus}</strong></span>
                    </div>
                    <div class="control-group">
                        <span class="status-text"
                            >Loop Count: <strong>{currentLoopCount}</strong></span
                        >
                    </div>
                    <div class="control-group">
                        <span class="status-text"
                            >Structure Index: <strong
                                >{currentStructureIndex + 1}/{songStructures.length}</strong
                            ></span
                        >
                    </div>
                    <div class="control-group">
                        <span class="status-text"
                            >Next Transition: <strong
                                >{structureTransitionInterval -
                                    (currentLoopCount % structureTransitionInterval)} loops</strong
                            ></span
                        >
                    </div>
                    {#if structureTransitionNotification}
                        <div class="control-group notification">
                            <span class="status-text notification-text"
                                >🎵 {structureTransitionNotification}</span
                            >
                        </div>
                    {/if}
                </fieldset>

                <!-- Song Progress Display -->
                <div class="song-card">
                    <SongProgressDisplay
                        {songStructure}
                        {currentSection}
                        {sectionProgress}
                        {totalProgress}
                        {selectedSection}
                        showSectionSelection={true}
                        on:sectionSelected={handleSectionSelected}
                        on:openPatternEditor={handleOpenPatternEditor}
                    />
                </div>
            </div>

            <!-- Audio + Effects + Pitch Range -->
            <div class="sound-grid">
                <fieldset class="card">
                    <legend>Audio Settings</legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Volume:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={volume}
                                    on:input={handleVolumeChange}
                                    class="slider"
                                />
                                <span class="val">{volume.toFixed(2)}</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Tempo:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="30"
                                    max="140"
                                    step="1"
                                    value={tempo}
                                    on:input={handleTempoChange}
                                    class="slider"
                                />
                                <span class="val">{tempo.toFixed(0)} bpm</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Scale:</span>
                            <select
                                value={scale}
                                on:change={handleScaleChange}
                                class="select-input"
                            >
                                <option value="pentatonic">Pentatonic</option>
                                <option value="lydian">Lydian</option>
                                <option value="aeolian">Aeolian</option>
                                <option value="whole">Whole tone</option>
                                <option value="chromatic">Chromatic</option>
                                <option value="blues">Blues</option>
                                <option value="dorian">Dorian</option>
                                <option value="harmonicMinor">Harmonic Minor</option>
                                <option value="ionian">Ionian</option>
                                <option value="locrian">Locrian</option>
                                <option value="major">Major</option>
                                <option value="melodicMinor">Melodic Minor</option>
                                <option value="minor">Minor</option>
                                <option value="mixolydian">Mixolydian</option>
                                <option value="phrygian">Phrygian</option>
                            </select>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Randomness:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={randomness}
                                    on:input={handleRandomnessChange}
                                    class="slider"
                                />
                                <span class="val">{randomness.toFixed(2)}</span>
                            </div>
                        </div>
                    </div>
                </fieldset>

                <!-- Effects Settings -->
                <fieldset class="card">
                    <legend>Effects</legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Reverb:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="0.95"
                                    step="0.01"
                                    value={reverb}
                                    on:input={handleReverbChange}
                                    class="slider"
                                />
                                <span class="val">{reverb.toFixed(2)}</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Delay Mix:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={delayMix}
                                    on:input={handleDelayMixChange}
                                    class="slider"
                                />
                                <span class="val">{delayMix.toFixed(2)}</span>
                            </div>
                        </div>
                    </div>
                </fieldset>

                <!-- Pitch Range -->
                <fieldset class="card span-2">
                    <legend>Pitch Range</legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Lowest Octave:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="7"
                                    step="1"
                                    value={lowestOctave}
                                    on:input={handleLowestOctaveChange}
                                    class="slider"
                                />
                                <span class="val">{lowestOctave}</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Highest Octave:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="8"
                                    step="1"
                                    value={highestOctave}
                                    on:input={handleHighestOctaveChange}
                                    class="slider"
                                />
                                <span class="val">{highestOctave}</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Seed:</span>
                            <input
                                type="number"
                                min="1"
                                step="1"
                                value={seed}
                                on:change={handleSeedChange}
                                class="number-input"
                            />
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Structure Transition:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="1"
                                    max="32"
                                    step="1"
                                    value={structureTransitionInterval}
                                    on:input={handleStructureTransitionChange}
                                    class="slider"
                                />
                                <span class="val">{structureTransitionInterval} loops</span>
                            </div>
                        </div>
                    </div>
                </fieldset>
            </div>

            <!-- Generators -->
            <div class="gen-grid">
                <fieldset class="card">
                    <legend>
                        Beat Generator
                        <button
                            type="button"
                            class="toggle-btn"
                            class:active={generators.beat}
                            on:click={() => toggleGenerator('beat')}
                        >
                            {generators.beat ? 'ON' : 'OFF'}
                        </button>
                    </legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Density:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={beatDensity}
                                    on:input={handleBeatDensityChange}
                                    class="slider"
                                />
                                <span class="val">{(beatDensity * 100).toFixed(0)}%</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Volume:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="-40"
                                    max="-10"
                                    step="1"
                                    value={beatVolume}
                                    on:input={handleBeatVolumeChange}
                                    class="slider"
                                />
                                <span class="val">{beatVolume}dB</span>
                            </div>
                        </div>
                    </div>
                </fieldset>

                <!-- Bass Generator -->
                <fieldset class="card">
                    <legend>
                        Bass Generator
                        <button
                            type="button"
                            class="toggle-btn"
                            class:active={generators.bass}
                            on:click={() => toggleGenerator('bass')}
                        >
                            {generators.bass ? 'ON' : 'OFF'}
                        </button>
                    </legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Complexity:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={bassComplexity}
                                    on:input={handleBassComplexityChange}
                                    class="slider"
                                />
                                <span class="val">{(bassComplexity * 100).toFixed(0)}%</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Volume:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="-40"
                                    max="-5"
                                    step="1"
                                    value={bassVolume}
                                    on:input={handleBassVolumeChange}
                                    class="slider"
                                />
                                <span class="val">{bassVolume}dB</span>
                            </div>
                        </div>
                    </div>
                </fieldset>

                <!-- Melody/Pad Generator -->
                <fieldset class="card">
                    <legend>
                        Melody/Pad Generator
                        <button
                            type="button"
                            class="toggle-btn"
                            class:active={generators.melody}
                            on:click={() => toggleGenerator('melody')}
                        >
                            {generators.melody ? 'ON' : 'OFF'}
                        </button>
                    </legend>
                    <div class="settings-grid">
                        <div class="setting-item">
                            <span class="setting-label">Movement:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={melodyMovement}
                                    on:input={handleMelodyMovementChange}
                                    class="slider"
                                />
                                <span class="val">{(melodyMovement * 100).toFixed(0)}%</span>
                            </div>
                        </div>
                        <div class="setting-item">
                            <span class="setting-label">Volume:</span>
                            <div class="slider-container">
                                <input
                                    type="range"
                                    min="-40"
                                    max="-5"
                                    step="1"
                                    value={melodyVolume}
                                    on:input={handleMelodyVolumeChange}
                                    class="slider"
                                />
                                <span class="val">{melodyVolume}dB</span>
                            </div>
                        </div>
                    </div>
                </fieldset>
            </div>
        </form>
    </div>
</div>

{#if showPatternEditor}
    <PatternEditor
        section={editingSection}
        pattern={editingPattern || { beat: [], bass: [], melody: [] }}
        on:close={() => (showPatternEditor = false)}
        on:save={handlePatternSave}
    />
{/if}

<script lang="ts">
    import { createEventDispatcher, onMount } from 'svelte';
    import Button from './Button.svelte';
    import PatternEditor from './PatternEditor.svelte';
    import SongProgressDisplay from './SongProgressDisplay.svelte';
    import { musicSettings, musicActions } from '../../stores/musicStore';
    import musicEngine from '../../audio/musicEngine';
    import type { ScaleName, GeneratorType, Pattern } from '../../audio/musicEngine';

    // Available song structures
    const songStructures = ['ABAB', 'AABB', 'ABAC', 'AAAB', 'AABC', 'ABBB', 'ABBC', 'ABCC'];

    const dispatch = createEventDispatcher();

    // All state comes from the centralized store
    $: ({
        // Audio settings
        volume,
        tempo,
        scale,
        randomness,
        reverb,
        delayMix,
        lowestOctave,
        highestOctave,
        seed,
        generators,
        beatDensity,
        beatVolume,
        bassComplexity,
        bassVolume,
        melodyMovement,
        melodyVolume,
        structureTransitionInterval,

        // Centralized playback state
        isPlaying,
        isPaused,
        audioContextSuspended,
        currentSection,
        songStructure,
        sectionProgress,
        totalProgress,
        currentLoopCount,
        currentStructureIndex,
        structureTransitionNotification,
    } = $musicSettings);

    // Pattern editor state
    let showPatternEditor = false;
    let editingSection = '';
    let editingPattern: Pattern | null = null;

    // Section selection state
    let selectedSection = 'A';

    // Derived music status for display
    $: musicStatus = audioContextSuspended
        ? 'Audio Not Initialized'
        : isPlaying && !isPaused
          ? 'Playing'
          : isPaused
            ? 'Paused'
            : 'Stopped';

    // Initialize audio context when modal opens
    onMount(async () => {
        await musicActions.initializeAudioContext();
    });

    async function handleStartAudio() {
        await musicActions.initializeAudioContext();
    }

    async function handleStart() {
        await musicActions.start();
    }

    function handleStop() {
        musicActions.stop();
    }

    function handlePause() {
        musicActions.pause();
    }

    async function handleRestart() {
        await musicActions.restart();
    }

    function handleVolumeChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setVolume(parseFloat(target.value));
    }

    function handleTempoChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setTempo(parseInt(target.value));
    }

    function handleScaleChange(event: Event) {
        const target = event.target as HTMLSelectElement;
        musicActions.setScale(target.value as ScaleName);
    }

    function handleRandomnessChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setRandomness(parseFloat(target.value));
    }

    function handleReverbChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setReverb(parseFloat(target.value));
    }

    function handleDelayMixChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setDelayMix(parseFloat(target.value));
    }

    function handleLowestOctaveChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setLowestOctave(parseInt(target.value));
    }

    function handleHighestOctaveChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setHighestOctave(parseInt(target.value));
    }

    function handleSeedChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setSeed(parseInt(target.value));
    }

    function handleStructureTransitionChange(event: Event) {
        const target = event.target as HTMLInputElement;
        const newInterval = parseInt(target.value);
        musicActions.setStructureTransitionInterval(newInterval);
    }

    function forceStructureTransition() {
        musicActions.forceStructureTransition();
    }

    // Generator control handlers
    function toggleGenerator(type: GeneratorType) {
        musicActions.setGeneratorEnabled(type, !generators[type]);
    }

    function handleBeatDensityChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setBeatDensity(parseFloat(target.value));
    }

    function handleBeatVolumeChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setBeatVolume(parseInt(target.value));
    }

    function handleBassComplexityChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setBassComplexity(parseFloat(target.value));
    }

    function handleBassVolumeChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setBassVolume(parseInt(target.value));
    }

    function handleMelodyMovementChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setMelodyMovement(parseFloat(target.value));
    }

    function handleMelodyVolumeChange(event: Event) {
        const target = event.target as HTMLInputElement;
        musicActions.setMelodyVolume(parseInt(target.value));
    }

    function randomize() {
        musicActions.randomize();
    }

    function regeneratePatterns() {
        musicActions.regeneratePatterns();
    }

    function saveLocal() {
        // Settings are automatically saved to localStorage via the store
        console.log('Music settings saved automatically');
    }

    function loadLocal() {
        // Settings are automatically loaded from localStorage via the store
        console.log('Music settings loaded automatically');
    }

    function handleSectionSelected(event: CustomEvent) {
        selectedSection = event.detail.section;
    }

    function handleOpenPatternEditor(event: CustomEvent) {
        editingSection = event.detail.section;
        editingPattern = event.detail.pattern;
        showPatternEditor = true;
    }

    function handlePatternSave(event: CustomEvent) {
        const { section, pattern } = event.detail;
        musicEngine.setSectionPattern(section, pattern);
        showPatternEditor = false;
        editingSection = '';
        editingPattern = null;
    }
</script>

<style>
    @import '../../shared-theme.css';

    .dialog-backdrop {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.8);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .dialog {
        background: rgba(0, 0, 0, 0.9);
        padding: 2rem;
        border-radius: 8px;
        min-width: 600px;
        max-width: 90vw;
        max-height: 85vh;
        overflow-y: auto;
        border: 1px solid rgba(255, 255, 255, 0.2);
        backdrop-filter: blur(10px);
        -webkit-backdrop-filter: blur(10px);
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    }

    .dialog h3 {
        margin-top: 0;
        margin-bottom: 1.5rem;
        color: rgba(255, 255, 255, 0.9);
        font-size: 1.5rem;
        font-weight: 600;
    }

    /* Layout grids */
    .top-grid {
        display: grid;
        grid-template-columns: 1.1fr 1fr;
        gap: 1rem;
        align-items: start;
        margin-bottom: 1rem;
    }

    .sound-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
        margin-bottom: 1rem;
    }

    .gen-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 1rem;
        margin-bottom: 1rem;
    }

    .card {
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid rgba(255, 255, 255, 0.08);
    }

    .song-card {
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 4px;
        padding: 0.5rem;
    }

    .span-2 {
        grid-column: span 2;
    }

    fieldset {
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 4px;
        padding: 1rem;
        margin-bottom: 1rem;
    }

    legend {
        font-weight: bold;
        padding: 0 0.5rem;
        color: rgba(255, 255, 255, 0.9);
    }

    .control-group {
        margin-bottom: 0.5rem;
        display: flex;
        gap: 0.5rem;
        align-items: center;
        flex-wrap: wrap;
    }

    /* Settings grid layout matching app patterns */
    .settings-grid {
        display: grid;
        grid-template-columns: 1fr auto;
        gap: 0.15rem 0.3rem;
        width: 100%;
    }

    .setting-item {
        display: contents;
    }

    .setting-label {
        font-weight: 500;
        color: rgba(255, 255, 255, 0.9);
        padding: 0.5rem 0;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        text-align: right;
        padding-right: 1rem;
    }

    .setting-item:last-child .setting-label {
        border-bottom: none;
    }

    .slider-container {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        min-width: 200px;
    }

    .slider {
        flex: 1;
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.9);
        padding: 0.25rem 0;
        border-radius: 4px;
        font-family: inherit;
        font-size: 0.875rem;
        box-sizing: border-box;
        transition:
            border-color 0.2s ease,
            background-color 0.2s ease;
        accent-color: #646cff;
    }

    .slider:focus {
        outline: none;
        border-color: #646cff;
        background: rgba(255, 255, 255, 0.15);
        box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
    }

    .select-input {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.9);
        padding: 0.25rem 0.5rem;
        border-radius: 4px;
        font-family: inherit;
        font-size: 0.875rem;
        box-sizing: border-box;
        transition:
            border-color 0.2s ease,
            background-color 0.2s ease;
        min-width: 150px;
    }

    .select-input:focus {
        outline: none;
        border-color: #646cff;
        background: rgba(255, 255, 255, 0.15);
        box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
    }

    .number-input {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.9);
        padding: 0.25rem 0.5rem;
        border-radius: 4px;
        font-family: inherit;
        font-size: 0.875rem;
        box-sizing: border-box;
        transition:
            border-color 0.2s ease,
            background-color 0.2s ease;
        min-width: 120px;
    }

    .number-input:focus {
        outline: none;
        border-color: #646cff;
        background: rgba(255, 255, 255, 0.15);
        box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
    }

    .actions-group {
        margin-top: 1rem;
    }

    .val {
        opacity: 0.8;
        font-size: 0.85rem;
        color: rgba(255, 255, 255, 0.7);
        text-align: center;
        min-width: 60px;
        font-weight: 500;
    }

    .status-text {
        color: rgba(255, 255, 255, 0.8);
        font-size: 0.9rem;
        margin: 0.5rem 0;
    }

    .toggle-btn {
        background: rgba(255, 255, 255, 0.2);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.3);
        padding: 4px 8px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.3s;
        font-size: 0.75rem;
        font-weight: 500;
        margin-left: 0.5rem;
    }

    .toggle-btn:hover {
        background: rgba(255, 255, 255, 0.3);
    }

    .toggle-btn.active {
        background: rgba(76, 175, 80, 0.5);
        border-color: rgba(76, 175, 80, 0.8);
    }

    legend {
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;
    }

    .btn-primary {
        background: #646cff;
        color: white;
        border: 1px solid #646cff;
        padding: 0.5rem 1rem;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
        transition: all 0.3s;
    }

    .btn-primary:hover {
        background: #5a5fcf;
    }

    .music-controls {
        display: flex;
        gap: 0.5rem;
        flex-wrap: wrap;
        justify-content: center;
        margin: 1rem 0;
    }

    .btn-music {
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.9);
        border: 1px solid rgba(255, 255, 255, 0.3);
        padding: 0.5rem 1rem;
        border-radius: 6px;
        cursor: pointer;
        font-weight: 500;
        transition: all 0.3s;
        display: flex;
        align-items: center;
        gap: 0.25rem;
        min-width: 80px;
        justify-content: center;
    }

    .btn-music:hover {
        background: rgba(255, 255, 255, 0.2);
        border-color: rgba(255, 255, 255, 0.5);
    }

    .btn-music.active {
        background: rgba(76, 175, 80, 0.3);
        border-color: rgba(76, 175, 80, 0.8);
        color: #4caf50;
    }

    .btn-music:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    /* Notification styling */
    .notification {
        background: rgba(33, 150, 243, 0.1);
        border: 1px solid rgba(33, 150, 243, 0.3);
        border-radius: 4px;
        padding: 0.5rem;
        margin: 0.5rem 0;
        animation: fadeInOut 3s ease-in-out;
    }

    .notification-text {
        color: #2196f3;
        font-weight: 500;
    }

    @keyframes fadeInOut {
        0% {
            opacity: 0;
            transform: translateY(-10px);
        }
        20% {
            opacity: 1;
            transform: translateY(0);
        }
        80% {
            opacity: 1;
            transform: translateY(0);
        }
        100% {
            opacity: 0;
            transform: translateY(-10px);
        }
    }

    /* Responsive design */
    @media (max-width: 768px) {
        .dialog {
            min-width: 400px;
            padding: 1.5rem;
        }

        .top-grid {
            grid-template-columns: 1fr;
        }

        .sound-grid {
            grid-template-columns: 1fr;
        }

        .gen-grid {
            grid-template-columns: 1fr;
        }

        .dialog h3 {
            font-size: 1.25rem;
        }

        .settings-grid {
            grid-template-columns: 1fr;
            gap: 0.5rem;
        }

        .setting-label {
            text-align: left;
            padding-right: 0;
            border-bottom: none;
        }

        .music-controls {
            flex-direction: column;
            align-items: stretch;
        }

        .btn-music {
            min-width: auto;
            width: 100%;
        }
    }

    @media (max-width: 480px) {
        .dialog {
            min-width: 320px;
            padding: 1rem;
        }

        .control-group {
            flex-direction: column;
            align-items: stretch;
        }
    }
</style>
