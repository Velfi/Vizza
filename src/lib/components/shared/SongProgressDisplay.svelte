<div class="song-progress-container">
    <div class="progress-header">
        <h4>Song Progress</h4>
        {#if showSectionSelection}
            <div class="section-selection">
                <span class="selection-label">Edit Section:</span>
                <div class="section-buttons">
                    {#each ['A', 'B', 'C'] as section}
                        <button
                            class="section-btn"
                            class:active={selectedSection === section}
                            class:playing={currentSection === section}
                            on:click={() => selectSection(section)}
                            type="button"
                        >
                            {section}
                        </button>
                    {/each}
                </div>
            </div>
        {/if}
    </div>

    <!-- SVG Progress Display -->
    <svg class="progress-svg" viewBox="0 0 400 120" xmlns="http://www.w3.org/2000/svg">
        <!-- Background -->
        <rect
            x="0"
            y="0"
            width="400"
            height="120"
            fill="#1a1a1a"
            stroke="#333"
            stroke-width="2"
            rx="8"
        ></rect>

        <!-- Section blocks -->
        {#each songStructure.split('') as section, index}
            {@const sectionWidth = 400 / songStructure.length}
            {@const sectionX = index * sectionWidth}
            {@const isCurrentSection = section === currentSection}
            {@const isSelectedSection = selectedSection === section}
            {@const sectionProgressWidth = isCurrentSection ? sectionProgress * sectionWidth : 0}

            <!-- Section background -->
            <rect
                x={sectionX}
                y="20"
                width={sectionWidth}
                height="80"
                fill={isCurrentSection ? '#2a4a6b' : isSelectedSection ? '#4a2a6b' : '#2a2a2a'}
                stroke={isCurrentSection ? '#4a8acc' : isSelectedSection ? '#8a4acc' : '#444'}
                stroke-width={isCurrentSection ? '2' : isSelectedSection ? '2' : '1'}
                rx="4"
            ></rect>

            <!-- Section progress fill -->
            {#if isCurrentSection && sectionProgressWidth > 0}
                <rect
                    x={sectionX}
                    y="20"
                    width={sectionProgressWidth}
                    height="80"
                    fill="#4a8acc"
                    opacity="0.7"
                    rx="4"
                ></rect>
            {/if}

            <!-- Section label -->
            <text
                x={sectionX + sectionWidth / 2}
                y="70"
                text-anchor="middle"
                fill={isCurrentSection ? '#ffffff' : isSelectedSection ? '#e0a0ff' : '#888'}
                font-family="monospace"
                font-size="16"
                font-weight={isCurrentSection ? 'bold' : isSelectedSection ? 'bold' : 'normal'}
            >
                {section}
            </text>

            <!-- Section number -->
            <text
                x={sectionX + sectionWidth / 2}
                y="45"
                text-anchor="middle"
                fill={isCurrentSection ? '#ffffff' : isSelectedSection ? '#e0a0ff' : '#666'}
                font-family="monospace"
                font-size="12"
            >
                {index + 1}
            </text>

            <!-- Selection indicator -->
            {#if isSelectedSection}
                <circle cx={sectionX + sectionWidth / 2} cy="95" r="3" fill="#8a4acc"></circle>
            {/if}
        {/each}

        <!-- Progress indicator line -->
        <line
            x1={totalProgress * 400}
            y1="0"
            x2={totalProgress * 400}
            y2="120"
            stroke="#ff6b6b"
            stroke-width="3"
            opacity="0.8"
        ></line>

        <!-- Time indicators -->
        <text x="10" y="15" fill="#888" font-family="monospace" font-size="10">
            Section: {currentSection} ({Math.round(sectionProgress * 100)}%)
        </text>
        <text x="300" y="15" fill="#888" font-family="monospace" font-size="10">
            Total: {Math.round(totalProgress * 100)}%
        </text>
    </svg>

    <!-- Beat grid visualization -->
    <div class="beat-grid">
        <div class="beat-grid-label">Beat Pattern</div>
        <div class="beat-grid-container">
            {#each Array(16) as _, beatIndex}
                {@const beatProgress = (beatIndex + 1) / 16}
                {@const isActive =
                    totalProgress >= beatProgress - 0.0625 && totalProgress < beatProgress}
                <div
                    class="beat-dot"
                    class:active={isActive}
                    style="left: {beatProgress * 100}%"
                ></div>
            {/each}
        </div>
    </div>

    <!-- Section Management Controls -->
    <div class="section-management">
        <div class="section-management-header">
            <h5>Section {selectedSection} Controls</h5>
            <span class="section-status">
                {currentSection === selectedSection ? 'Playing' : 'Ready'}
            </span>
        </div>

        <div class="section-controls">
            <!-- Octave Range -->
            <div class="control-group">
                <RangeSlider
                    label="Octave Range"
                    min={0}
                    max={8}
                    step={1}
                    lowValue={sectionSettings[selectedSection]?.lowestOctave || 3}
                    highValue={sectionSettings[selectedSection]?.highestOctave || 5}
                    lowLabel="Low"
                    highLabel="High"
                    allowOverlap={false}
                    on:change={(e) =>
                        handleOctaveRangeChange(
                            selectedSection,
                            e.detail.lowValue,
                            e.detail.highValue
                        )}
                />
            </div>

            <!-- Pattern Selection -->
            <div class="control-group">
                <label for="pattern-{selectedSection}" class="control-label">Pattern:</label>
                <select
                    id="pattern-{selectedSection}"
                    value={selectedPatterns[selectedSection] || ''}
                    on:change={(e) =>
                        handlePatternChange(selectedSection, (e.target as HTMLSelectElement).value)}
                    class="select-input"
                >
                    <option value="">Custom</option>
                    {#each presetPatterns as pattern}
                        <option value={pattern.id}>{pattern.name}</option>
                    {/each}
                </select>
            </div>

            <!-- Pattern Actions -->
            <div class="pattern-actions">
                <button
                    type="button"
                    class="btn-small"
                    on:click={() => regenerateSection(selectedSection)}
                >
                    Regenerate
                </button>
                <button
                    type="button"
                    class="btn-small"
                    on:click={() => openPatternEditor(selectedSection)}
                >
                    {showPatternEditor ? 'Close Editor' : 'Edit Pattern'}
                </button>
                <button type="button" class="btn-small" on:click={() => savePatternAsPreset()}>
                    Save as Preset
                </button>
            </div>

            <!-- Inline Pattern Editor -->
            {#if showPatternEditor}
                <div class="inline-pattern-editor">
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

                    <!-- Simple Beat Sequencer -->
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
                    {/if}

                    <!-- Simple Bass Sequencer -->
                    {#if activeTab === 'bass'}
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
                                    >Click to cycle through notes: C2 → D2 → E2 → F2 → G2 → A2 → B2
                                    → C3 → (empty)</span
                                >
                            </div>
                        </div>
                    {/if}

                    <!-- Simple Melody Sequencer -->
                    {#if activeTab === 'melody'}
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
                                    >Click to cycle through notes: C4 → D4 → E4 → F4 → G4 → A4 → B4
                                    → C5 → (empty)</span
                                >
                            </div>
                        </div>
                    {/if}

                    <!-- Editor Actions -->
                    <div class="editor-actions">
                        <button type="button" class="btn-secondary" on:click={closePatternEditor}>
                            Cancel
                        </button>
                        <button type="button" class="btn-primary" on:click={savePattern}>
                            Save Pattern
                        </button>
                    </div>
                </div>
            {/if}
        </div>
    </div>
</div>

<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { musicSettings } from '../../stores/musicStore';
    import musicEngine from '../../audio/musicEngine';
    import type {
        SectionSettings,
        PresetPattern,
        Pattern,
        BeatEvent,
        BassEvent,
        MelodyEvent,
    } from '../../audio/musicEngine';
    import RangeSlider from './RangeSlider.svelte';

    const dispatch = createEventDispatcher();

    export let songStructure: string = '';
    export let currentSection: string = '';
    export let sectionProgress: number = 0; // 0-1 progress within current section
    export let totalProgress: number = 0; // 0-1 progress through entire song
    export let selectedSection: string = 'A'; // Section selected for editing
    export let showSectionSelection: boolean = true; // Whether to show section selection UI

    let animationFrame: number;
    let sectionSettings: Record<string, SectionSettings> = {};
    let presetPatterns: PresetPattern[] = [];
    let selectedPatterns: Record<string, string> = {};

    // Pattern editing state
    let showPatternEditor: boolean = false;
    let activeTab: 'beat' | 'bass' | 'melody' = 'beat';
    let beatPattern: string[][] = [];
    let bassPattern: string[] = [];
    let melodyPattern: string[] = [];
    let patternLength: number = 16; // Will be set from music engine

    // Initialize section settings and refresh when global scale changes
    $: {
        // Create dependency on global scale so this block re-runs when it changes
        const _globalScale = $musicSettings.scale;
        void _globalScale;
        ['A', 'B', 'C'].forEach((section) => {
            const settings = musicEngine.getSectionSettings(section);
            if (settings) {
                sectionSettings[section] = settings;
            } else {
                // Initialize with default values if section doesn't exist yet
                sectionSettings[section] = {
                    scale: $musicSettings.scale,
                    lowestOctave: $musicSettings.lowestOctave,
                    highestOctave: $musicSettings.highestOctave,
                    pattern: { beat: [], bass: [], melody: [] },
                };
            }
        });
        presetPatterns = musicEngine.getPresetPatterns();
    }

    // Update progress display - just trigger re-render, don't recalculate
    function updateProgress() {
        // Continue animation to keep the display updating
        animationFrame = requestAnimationFrame(updateProgress);
    }

    function selectSection(section: string) {
        selectedSection = section;
        dispatch('sectionSelected', { section });
    }

    function handleOctaveRangeChange(section: string, lowOctave: number, highOctave: number) {
        musicEngine.setSectionOctaves(section, lowOctave, highOctave);
        sectionSettings[section] =
            musicEngine.getSectionSettings(section) || sectionSettings[section];
        sectionSettings = { ...sectionSettings };
    }

    function handlePatternChange(section: string, patternId: string) {
        if (patternId) {
            musicEngine.loadPresetPattern(section, patternId);
            selectedPatterns[section] = patternId;
            selectedPatterns = { ...selectedPatterns };
        } else {
            selectedPatterns[section] = '';
            selectedPatterns = { ...selectedPatterns };
        }
        sectionSettings[section] =
            musicEngine.getSectionSettings(section) || sectionSettings[section];
        sectionSettings = { ...sectionSettings };
    }

    function regenerateSection(section: string) {
        const current = sectionSettings[section];
        if (current) {
            musicEngine.setSectionScale(section, current.scale);
            sectionSettings[section] =
                musicEngine.getSectionSettings(section) || sectionSettings[section];
            sectionSettings = { ...sectionSettings };
        }
    }

    function openPatternEditor(section: string) {
        showPatternEditor = true;
        loadPatternIntoEditor(section);
    }

    function closePatternEditor() {
        showPatternEditor = false;
    }

    function loadPatternIntoEditor(section: string) {
        const settings = sectionSettings[section];
        if (!settings) return;

        // Get pattern length from music engine
        patternLength = musicEngine.getPatternLength();

        // Calculate array sizes based on actual pattern length
        const beatSubdivisions = patternLength * 16; // 16 subdivisions per measure
        const noteSubdivisions = patternLength * 8; // 8 subdivisions per measure

        // Reset patterns with correct sizes
        beatPattern = Array(beatSubdivisions)
            .fill(null)
            .map(() => []);
        bassPattern = Array(noteSubdivisions).fill('');
        melodyPattern = Array(noteSubdivisions).fill('');

        // Load beat pattern
        settings.pattern.beat.forEach((event) => {
            const time = parseBeatTimeString(event.time);
            const beatIndex = Math.floor(time / 4); // 4 subdivisions per beat
            if (beatIndex >= 0 && beatIndex < beatSubdivisions) {
                beatPattern[beatIndex].push(event.type);
            }
        });

        // Load bass pattern
        settings.pattern.bass.forEach((event) => {
            const time = parseNoteTimeString(event.time);
            const beatIndex = Math.floor(time);
            if (beatIndex >= 0 && beatIndex < noteSubdivisions) {
                bassPattern[beatIndex] = event.note;
            }
        });

        // Load melody pattern
        settings.pattern.melody.forEach((event) => {
            const time = parseNoteTimeString(event.time);
            const beatIndex = Math.floor(time);
            if (beatIndex >= 0 && beatIndex < noteSubdivisions) {
                melodyPattern[beatIndex] = event.note;
            }
        });
    }

    function parseBeatTimeString(timeStr: string): number {
        const parts = timeStr.split(':').map(Number);
        const [measure, beat, subdivision] = parts;
        // Beat patterns use 16 subdivisions per measure (4 beats × 4 subdivisions)
        return measure * 16 + beat * 4 + subdivision;
    }

    function parseNoteTimeString(timeStr: string): number {
        const parts = timeStr.split(':').map(Number);
        const [measure, beat, subdivision] = parts;
        // Bass and melody patterns use 8 subdivisions per measure (4 beats × 2 subdivisions)
        // subdivision is either 0 or 2, so we divide by 2
        return measure * 8 + beat * 2 + subdivision / 2;
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
                const measure = Math.floor(index / 16); // 16 subdivisions per measure
                const beatInMeasure = index % 16;
                const beat = Math.floor(beatInMeasure / 4);
                const subdivision = beatInMeasure % 4;
                beatEvents.push({
                    time: `${measure}:${beat}:${subdivision}`,
                    type: type as 'kick' | 'snare' | 'hihat',
                });
            });
        });

        const bassEvents: BassEvent[] = [];
        bassPattern.forEach((note, index) => {
            if (note) {
                const measure = Math.floor(index / 8); // 8 subdivisions per measure
                const beatInMeasure = index % 8;
                const beat = Math.floor(beatInMeasure / 2);
                const subdivision = (beatInMeasure % 2) * 2;
                bassEvents.push({
                    time: `${measure}:${beat}:${subdivision}`,
                    note: note,
                });
            }
        });

        const melodyEvents: MelodyEvent[] = [];
        melodyPattern.forEach((note, index) => {
            if (note) {
                const measure = Math.floor(index / 8); // 8 subdivisions per measure
                const beatInMeasure = index % 8;
                const beat = Math.floor(beatInMeasure / 2);
                const subdivision = (beatInMeasure % 2) * 2;
                melodyEvents.push({
                    time: `${measure}:${beat}:${subdivision}`,
                    note: note,
                });
            }
        });

        const newPattern: Pattern = {
            beat: beatEvents,
            bass: bassEvents,
            melody: melodyEvents,
        };

        musicEngine.setSectionPattern(selectedSection, newPattern);
        sectionSettings[selectedSection] =
            musicEngine.getSectionSettings(selectedSection) || sectionSettings[selectedSection];
        sectionSettings = { ...sectionSettings };
        closePatternEditor();
    }

    function savePatternAsPreset() {
        const pattern = sectionSettings[selectedSection]?.pattern;
        if (pattern) {
            const patternName = prompt('Enter pattern name:');
            if (patternName) {
                const customPattern: PresetPattern = {
                    id: `custom-${Date.now()}`,
                    name: patternName,
                    beat: [...pattern.beat],
                    bass: [...pattern.bass],
                    melody: [...pattern.melody],
                };
                musicEngine.saveCustomPattern(customPattern);
                presetPatterns = musicEngine.getPresetPatterns();
            }
        }
    }

    onMount(() => {
        updateProgress();
    });

    onDestroy(() => {
        if (animationFrame) {
            cancelAnimationFrame(animationFrame);
        }
    });

    // Reactive statement to start/stop animation based on music state
    $: if ($musicSettings.enabled) {
        if (!animationFrame) {
            updateProgress();
        }
    } else {
        if (animationFrame) {
            cancelAnimationFrame(animationFrame);
            animationFrame = 0;
        }
    }
</script>

<style>
    .song-progress-container {
        margin: 20px 0;
        padding: 15px;
        background: #1a1a1a;
        border-radius: 8px;
        border: 1px solid #333;
    }

    .progress-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 15px;
    }

    .song-progress-container h4 {
        margin: 0;
        color: #fff;
        font-size: 16px;
        font-weight: 600;
    }

    .section-selection {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .selection-label {
        color: #888;
        font-size: 12px;
        font-weight: 500;
    }

    .section-buttons {
        display: flex;
        gap: 5px;
    }

    .section-btn {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.7);
        padding: 4px 8px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s ease;
        font-size: 12px;
        font-weight: 500;
        min-width: 24px;
        text-align: center;
    }

    .section-btn:hover {
        background: rgba(255, 255, 255, 0.2);
        color: rgba(255, 255, 255, 0.9);
    }

    .section-btn.active {
        background: rgba(138, 74, 204, 0.3);
        border-color: #8a4acc;
        color: #e0a0ff;
        font-weight: 600;
    }

    .section-btn.playing {
        background: rgba(74, 138, 204, 0.3);
        border-color: #4a8acc;
        color: #ffffff;
        font-weight: 600;
    }

    .section-btn.active.playing {
        background: rgba(138, 74, 204, 0.5);
        border-color: #8a4acc;
        color: #e0a0ff;
        box-shadow: 0 0 8px rgba(138, 74, 204, 0.4);
    }

    .progress-svg {
        width: 100%;
        height: 120px;
        display: block;
        margin-bottom: 15px;
    }

    .beat-grid {
        margin-top: 10px;
    }

    .beat-grid-label {
        color: #888;
        font-size: 12px;
        margin-bottom: 8px;
        font-family: monospace;
    }

    .beat-grid-container {
        position: relative;
        height: 20px;
        background: #2a2a2a;
        border-radius: 10px;
        overflow: hidden;
    }

    .beat-dot {
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%);
        width: 8px;
        height: 8px;
        background: #444;
        border-radius: 50%;
        transition: all 0.1s ease;
    }

    .beat-dot.active {
        background: #ff6b6b;
        box-shadow: 0 0 8px #ff6b6b;
        transform: translate(-50%, -50%) scale(1.2);
    }

    /* Section Management Styles */
    .section-management {
        margin-top: 20px;
        padding: 15px;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 8px;
    }

    .section-management-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 15px;
    }

    .section-management-header h5 {
        margin: 0;
        color: rgba(255, 255, 255, 0.9);
        font-size: 14px;
        font-weight: 600;
    }

    .section-status {
        font-size: 12px;
        color: rgba(76, 175, 80, 0.8);
        font-weight: 500;
    }

    .section-controls {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .control-group {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .control-label {
        font-size: 12px;
        color: rgba(255, 255, 255, 0.8);
        font-weight: 500;
    }

    .pattern-actions {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
    }

    .btn-small {
        background: rgba(255, 255, 255, 0.1);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.3);
        padding: 6px 12px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.3s;
        font-size: 11px;
        font-weight: 500;
    }

    .btn-small:hover {
        background: rgba(255, 255, 255, 0.2);
    }

    .select-input {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.9);
        padding: 6px 8px;
        border-radius: 4px;
        font-family: inherit;
        font-size: 12px;
    }

    .select-input:focus {
        outline: none;
        border-color: #646cff;
        background: rgba(255, 255, 255, 0.15);
    }

    /* Responsive design */
    @media (max-width: 768px) {
        .song-progress-container {
            margin: 15px 0;
            padding: 10px;
        }

        .progress-svg {
            height: 100px;
        }

        .beat-grid-container {
            height: 16px;
        }

        .beat-dot {
            width: 6px;
            height: 6px;
        }

        .pattern-actions {
            flex-direction: column;
        }

        .btn-small {
            width: 100%;
        }
    }

    /* Inline Pattern Editor Styles */
    .inline-pattern-editor {
        margin-top: 8px;
        padding: 8px;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 4px;
        max-height: 200px;
        overflow-y: auto;
    }

    .pattern-tabs {
        display: flex;
        gap: 8px;
        margin-bottom: 16px;
    }

    .tab-btn {
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.7);
        border: 1px solid rgba(255, 255, 255, 0.3);
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s;
        font-size: 12px;
        font-weight: 500;
    }

    .tab-btn:hover {
        background: rgba(255, 255, 255, 0.15);
        color: white;
    }

    .tab-btn.active {
        background: #646cff;
        color: white;
        border-color: #646cff;
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

    /* Responsive design for sequencer */
    @media (max-width: 768px) {
        .sequencer-grid {
            padding: 12px;
        }

        .step-button {
            width: 28px;
            height: 28px;
            font-size: 10px;
        }

        .step-number {
            width: 28px;
            height: 20px;
            font-size: 10px;
        }

        .instrument-name {
            width: 60px;
            font-size: 11px;
        }

        .step-numbers {
            padding-left: 60px;
        }

        .note-legend {
            font-size: 10px;
        }
    }

    @media (max-width: 480px) {
        .step-button {
            width: 24px;
            height: 24px;
            font-size: 9px;
        }

        .step-number {
            width: 24px;
            height: 18px;
            font-size: 9px;
        }

        .instrument-name {
            width: 50px;
            font-size: 10px;
        }

        .step-numbers {
            padding-left: 50px;
        }
    }

    /* Editor Actions */
    .editor-actions {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
        margin-top: 16px;
        padding-top: 16px;
        border-top: 1px solid rgba(255, 255, 255, 0.1);
    }

    .btn-primary {
        background: #646cff;
        color: white;
        border: 1px solid #646cff;
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s;
        font-size: 12px;
        font-weight: 500;
    }

    .btn-primary:hover {
        background: #5a5fcf;
        border-color: #5a5fcf;
    }

    .btn-secondary {
        background: rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.8);
        border: 1px solid rgba(255, 255, 255, 0.3);
        padding: 8px 16px;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s;
        font-size: 12px;
        font-weight: 500;
    }

    .btn-secondary:hover {
        background: rgba(255, 255, 255, 0.15);
        color: white;
    }

    /* Responsive design for pattern editor */
    @media (max-width: 768px) {
        .pattern-tabs {
            flex-wrap: wrap;
        }

        .editor-actions {
            flex-direction: column;
        }
    }
</style>
