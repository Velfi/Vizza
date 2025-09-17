<div class="section-manager">
    <h4>Section Management</h4>

    <div class="sections-container">
        {#each sections as section}
            <div class="section-card" class:active={currentSection === section}>
                <div class="section-header">
                    <h5>Section {section}</h5>
                    <span class="section-status"
                        >{currentSection === section ? 'Playing' : 'Ready'}</span
                    >
                </div>

                <div class="section-controls">
                    <!-- Octave Range -->
                    <div class="control-group">
                        <label for="octave-{section}" class="control-label">Octave Range:</label>
                        <div class="octave-controls">
                            <input
                                type="range"
                                min="0"
                                max="7"
                                step="1"
                                value={sectionSettings[section]?.lowestOctave || 3}
                                on:input={(e) =>
                                    handleLowestOctaveChange(
                                        section,
                                        parseInt((e.target as HTMLInputElement).value)
                                    )}
                                class="slider"
                            />
                            <span class="octave-label"
                                >Low: {sectionSettings[section]?.lowestOctave || 3}</span
                            >
                        </div>
                        <div class="octave-controls">
                            <input
                                type="range"
                                min="0"
                                max="8"
                                step="1"
                                value={sectionSettings[section]?.highestOctave || 5}
                                on:input={(e) =>
                                    handleHighestOctaveChange(
                                        section,
                                        parseInt((e.target as HTMLInputElement).value)
                                    )}
                                class="slider"
                            />
                            <span class="octave-label"
                                >High: {sectionSettings[section]?.highestOctave || 5}</span
                            >
                        </div>
                    </div>

                    <!-- Pattern Selection -->
                    <div class="control-group">
                        <label for="pattern-{section}" class="control-label">Pattern:</label>
                        <select
                            id="pattern-{section}"
                            value={selectedPatterns[section] || ''}
                            on:change={(e) =>
                                handlePatternChange(section, (e.target as HTMLSelectElement).value)}
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
                            on:click={() => regenerateSection(section)}
                        >
                            Regenerate
                        </button>
                        <button
                            type="button"
                            class="btn-small"
                            on:click={() => openPatternEditor(section)}
                        >
                            Edit Pattern
                        </button>
                        <button
                            type="button"
                            class="btn-small"
                            on:click={() => savePattern(section)}
                        >
                            Save Pattern
                        </button>
                    </div>
                </div>
            </div>
        {/each}
    </div>
</div>

<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import musicEngine from '../../audio/musicEngine';
    import type { SectionSettings, PresetPattern } from '../../audio/musicEngine';
    import { musicSettings } from '../../stores/musicStore';

    const dispatch = createEventDispatcher();

    export let currentSection: string = '-';

    const sections = ['A', 'B', 'C'];
    let sectionSettings: Record<string, SectionSettings> = {};
    let presetPatterns: PresetPattern[] = [];
    let selectedPatterns: Record<string, string> = {};

    // Initialize section settings and refresh when global scale changes
    $: {
        // Create dependency on global scale so this block re-runs when it changes
        const _globalScale = $musicSettings.scale;
        void _globalScale;
        sections.forEach((section) => {
            const settings = musicEngine.getSectionSettings(section);
            if (settings) {
                sectionSettings[section] = settings;
            }
        });
        presetPatterns = musicEngine.getPresetPatterns();
    }

    function handleLowestOctaveChange(section: string, octave: number) {
        const current = sectionSettings[section];
        if (current) {
            musicEngine.setSectionOctaves(section, octave, current.highestOctave);
            sectionSettings[section] =
                musicEngine.getSectionSettings(section) || sectionSettings[section];
            sectionSettings = { ...sectionSettings };
        }
    }

    function handleHighestOctaveChange(section: string, octave: number) {
        const current = sectionSettings[section];
        if (current) {
            musicEngine.setSectionOctaves(section, current.lowestOctave, octave);
            sectionSettings[section] =
                musicEngine.getSectionSettings(section) || sectionSettings[section];
            sectionSettings = { ...sectionSettings };
        }
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
        dispatch('openPatternEditor', { section, pattern: sectionSettings[section]?.pattern });
    }

    function savePattern(section: string) {
        const pattern = sectionSettings[section]?.pattern;
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
</script>

<style>
    .section-manager {
        margin: 1rem 0;
    }

    .section-manager h4 {
        margin: 0 0 1rem 0;
        color: rgba(255, 255, 255, 0.9);
        font-size: 1.1rem;
        font-weight: 600;
    }

    .sections-container {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1rem;
    }

    .section-card {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 8px;
        padding: 1rem;
        transition: all 0.3s ease;
    }

    .section-card.active {
        border-color: rgba(76, 175, 80, 0.8);
        background: rgba(76, 175, 80, 0.1);
    }

    .section-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }

    .section-header h5 {
        margin: 0;
        color: rgba(255, 255, 255, 0.9);
        font-size: 1rem;
        font-weight: 600;
    }

    .section-status {
        font-size: 0.8rem;
        color: rgba(76, 175, 80, 0.8);
        font-weight: 500;
    }

    .section-controls {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .control-group {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .control-label {
        font-size: 0.85rem;
        color: rgba(255, 255, 255, 0.8);
        font-weight: 500;
    }

    .octave-controls {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .octave-label {
        font-size: 0.8rem;
        color: rgba(255, 255, 255, 0.7);
        min-width: 60px;
    }

    .pattern-actions {
        display: flex;
        gap: 0.5rem;
        flex-wrap: wrap;
    }

    .btn-small {
        background: rgba(255, 255, 255, 0.1);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.3);
        padding: 0.25rem 0.5rem;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.3s;
        font-size: 0.75rem;
        font-weight: 500;
    }

    .btn-small:hover {
        background: rgba(255, 255, 255, 0.2);
    }

    .select-input {
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.9);
        padding: 0.25rem 0.5rem;
        border-radius: 4px;
        font-family: inherit;
        font-size: 0.85rem;
    }

    .select-input:focus {
        outline: none;
        border-color: #646cff;
        background: rgba(255, 255, 255, 0.15);
    }

    .slider {
        flex: 1;
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.3);
        color: rgba(255, 255, 255, 0.9);
        padding: 0.25rem 0;
        border-radius: 4px;
        accent-color: #646cff;
    }

    .slider:focus {
        outline: none;
        border-color: #646cff;
    }

    @media (max-width: 768px) {
        .sections-container {
            grid-template-columns: 1fr;
        }

        .pattern-actions {
            flex-direction: column;
        }

        .btn-small {
            width: 100%;
        }
    }
</style>
