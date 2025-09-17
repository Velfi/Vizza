import { writable } from 'svelte/store';
import musicEngine, { type ScaleName, type GeneratorType } from '../audio/musicEngine';

export interface MusicSettings {
    // Audio settings
    enabled: boolean;
    volume: number;
    tempo: number;
    scale: ScaleName;
    randomness: number;
    reverb: number;
    delayMix: number;
    lowestOctave: number;
    highestOctave: number;
    seed: number;

    // Generator settings
    generators: Record<GeneratorType, boolean>;
    beatDensity: number;
    beatVolume: number;
    bassComplexity: number;
    bassVolume: number;
    melodyMovement: number;
    melodyVolume: number;

    // Centralized playback state
    isPlaying: boolean;
    isPaused: boolean;
    audioContextSuspended: boolean;
    currentSection: string;
    songStructure: string;
    sectionProgress: number;
    totalProgress: number;
    currentLoopCount: number;
    currentStructureIndex: number;
    structureTransitionInterval: number;
    structureTransitionNotification: string;
}

const defaultSettings: MusicSettings = {
    // Audio settings
    enabled: false,
    volume: 0.4,
    tempo: 40,
    scale: 'lydian',
    randomness: 0.3,
    reverb: 0.15,
    delayMix: 0.15,
    lowestOctave: 3,
    highestOctave: 5,
    seed: 12345,

    // Generator settings
    generators: {
        beat: true,
        bass: true,
        melody: true,
        pad: true,
    },
    beatDensity: 0.3,
    beatVolume: -20,
    bassComplexity: 0.3,
    bassVolume: -15,
    melodyMovement: 0.4,
    melodyVolume: -18,

    // Centralized playback state
    isPlaying: false,
    isPaused: false,
    audioContextSuspended: true,
    currentSection: '',
    songStructure: '',
    sectionProgress: 0,
    totalProgress: 0,
    currentLoopCount: 0,
    currentStructureIndex: 0,
    structureTransitionInterval: 8,
    structureTransitionNotification: '',
};

// Create the writable store
export const musicSettings = writable<MusicSettings>(defaultSettings);

// Local storage key for persistence
const STORAGE_KEY = 'vizza.music.v3';

// Initialize from localStorage on first load
function loadFromStorage(): MusicSettings {
    if (typeof window === 'undefined') return defaultSettings;

    try {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored) {
            const parsed = JSON.parse(stored);
            // Merge with defaults to handle missing properties
            return { ...defaultSettings, ...parsed };
        }
    } catch (e) {
        console.error('Failed to load music settings from storage:', e);
    }

    return defaultSettings;
}

// Save to localStorage
function saveToStorage(settings: MusicSettings): void {
    if (typeof window === 'undefined') return;

    try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch (e) {
        console.error('Failed to save music settings to storage:', e);
    }
}

// Apply settings to the music engine (only audio settings, not playback state)
async function applyToEngine(settings: MusicSettings): Promise<void> {
    musicEngine.setVolume(settings.volume);
    musicEngine.setTempo(settings.tempo);
    musicEngine.setScale(settings.scale);
    musicEngine.setReverb(settings.reverb);
    musicEngine.setDelayMix(settings.delayMix);
    musicEngine.setLowestOctave(settings.lowestOctave);
    musicEngine.setHighestOctave(settings.highestOctave);
    musicEngine.setSeed(settings.seed);

    // Apply generator settings
    musicEngine.setGeneratorEnabled('beat', settings.generators.beat);
    musicEngine.setGeneratorEnabled('bass', settings.generators.bass);
    musicEngine.setGeneratorEnabled('melody', settings.generators.melody);
    musicEngine.setGeneratorEnabled('pad', settings.generators.pad);
    musicEngine.setBeatDensity(settings.beatDensity);
    musicEngine.setBeatVolume(settings.beatVolume);
    musicEngine.setBassComplexity(settings.bassComplexity);
    musicEngine.setBassVolume(settings.bassVolume);
    musicEngine.setMelodyMovement(settings.melodyMovement);
    musicEngine.setMelodyVolume(settings.melodyVolume);
    musicEngine.setStructureTransitionInterval(settings.structureTransitionInterval);
}

// Initialize store with saved settings
const initialSettings = loadFromStorage();
musicSettings.set(initialSettings);
applyToEngine(initialSettings);

// Progress update interval
let progressInterval: number | null = null;

// Track previous settings to only apply changes
let previousSettings: MusicSettings = initialSettings;

// Subscribe to changes and apply them to the engine
musicSettings.subscribe(async (settings) => {
    // Only apply settings to engine if they actually changed (not just progress updates)
    const settingsChanged =
        previousSettings.volume !== settings.volume ||
        previousSettings.tempo !== settings.tempo ||
        previousSettings.scale !== settings.scale ||
        previousSettings.reverb !== settings.reverb ||
        previousSettings.delayMix !== settings.delayMix ||
        previousSettings.lowestOctave !== settings.lowestOctave ||
        previousSettings.highestOctave !== settings.highestOctave ||
        previousSettings.seed !== settings.seed ||
        previousSettings.generators.beat !== settings.generators.beat ||
        previousSettings.generators.bass !== settings.generators.bass ||
        previousSettings.generators.melody !== settings.generators.melody ||
        previousSettings.generators.pad !== settings.generators.pad ||
        previousSettings.beatDensity !== settings.beatDensity ||
        previousSettings.beatVolume !== settings.beatVolume ||
        previousSettings.bassComplexity !== settings.bassComplexity ||
        previousSettings.bassVolume !== settings.bassVolume ||
        previousSettings.melodyMovement !== settings.melodyMovement ||
        previousSettings.melodyVolume !== settings.melodyVolume ||
        previousSettings.structureTransitionInterval !== settings.structureTransitionInterval;

    if (settingsChanged) {
        await applyToEngine(settings);
        saveToStorage(settings);
        previousSettings = { ...settings };
    }

    // Handle progress updates based on playback state
    if (settings.isPlaying && !settings.audioContextSuspended) {
        if (!progressInterval) {
            progressInterval = setInterval(() => {
                musicSettings.update((current) => {
                    const newSection = musicEngine.getCurrentSection();
                    const newSongStructure = musicEngine.getSongStructure();
                    const newSectionProgress = musicEngine.getSectionProgress();
                    const newTotalProgress = musicEngine.getTotalProgress();
                    const newLoopCount = musicEngine.getCurrentLoopCount();
                    const newStructureIndex = musicEngine.getCurrentStructureIndex();

                    // Check for structure transitions
                    let notification = current.structureTransitionNotification;
                    if (newStructureIndex !== current.currentStructureIndex) {
                        notification = `Structure changed to: ${newSongStructure}`;
                        // Clear notification after 3 seconds
                        setTimeout(() => {
                            musicSettings.update((state) => ({
                                ...state,
                                structureTransitionNotification: '',
                            }));
                        }, 3000);
                    }

                    return {
                        ...current,
                        currentSection: newSection,
                        songStructure: newSongStructure,
                        sectionProgress: newSectionProgress,
                        totalProgress: newTotalProgress,
                        currentLoopCount: newLoopCount,
                        currentStructureIndex: newStructureIndex,
                        structureTransitionNotification: notification,
                    };
                });
            }, 100);
        }
    } else {
        if (progressInterval) {
            clearInterval(progressInterval);
            progressInterval = null;
        }
    }
});

// Action functions for updating settings
export const musicActions = {
    updateSettings: (updates: Partial<MusicSettings>) => {
        musicSettings.update((current) => ({ ...current, ...updates }));
    },

    // Centralized playback control
    async start(): Promise<void> {
        await musicEngine.initializeAudioContext();
        const audioContextSuspended = musicEngine.getAudioContextState() === 'suspended';

        if (!audioContextSuspended) {
            await musicEngine.start();
            musicSettings.update((state) => ({
                ...state,
                isPlaying: true,
                isPaused: false,
                audioContextSuspended: false,
                enabled: true,
            }));
        } else {
            musicSettings.update((state) => ({
                ...state,
                audioContextSuspended: true,
            }));
        }
    },

    stop(): void {
        musicEngine.stop();
        musicSettings.update((state) => ({
            ...state,
            isPlaying: false,
            isPaused: false,
            enabled: false,
        }));
    },

    async restart(): Promise<void> {
        // Reset song position and stop current playback
        musicEngine.resetSongPosition();
        musicSettings.update((state) => ({
            ...state,
            isPlaying: false,
            isPaused: false,
            currentSection: '',
            songStructure: '',
            sectionProgress: 0,
            totalProgress: 0,
            currentLoopCount: 0,
            currentStructureIndex: 0,
            structureTransitionNotification: '',
        }));

        // Wait a moment then restart
        setTimeout(async () => {
            await musicEngine.initializeAudioContext();
            const audioContextSuspended = musicEngine.getAudioContextState() === 'suspended';

            if (!audioContextSuspended) {
                await musicEngine.start();
                musicSettings.update((state) => ({
                    ...state,
                    isPlaying: true,
                    isPaused: false,
                    audioContextSuspended: false,
                    enabled: true,
                }));
            }
        }, 100);
    },

    pause(): void {
        // For now, we'll stop the music since we don't have pause functionality in the engine
        musicEngine.stop();
        musicSettings.update((state) => ({
            ...state,
            isPlaying: false,
            isPaused: true,
            enabled: false,
        }));
    },

    async initializeAudioContext(): Promise<void> {
        await musicEngine.initializeAudioContext();
        const audioContextSuspended = musicEngine.getAudioContextState() === 'suspended';
        musicSettings.update((state) => ({
            ...state,
            audioContextSuspended,
        }));
    },

    toggleEnabled: async () => {
        musicSettings.update((current) => {
            if (current.isPlaying) {
                musicEngine.stop();
                return { ...current, enabled: false, isPlaying: false, isPaused: false };
            } else {
                // This will be handled by the start() method
                return { ...current, enabled: true };
            }
        });
    },

    setVolume: (volume: number) => {
        musicSettings.update((current) => ({ ...current, volume }));
    },

    setTempo: (tempo: number) => {
        musicSettings.update((current) => ({ ...current, tempo }));
    },

    setScale: (scale: ScaleName) => {
        musicSettings.update((current) => ({ ...current, scale }));
    },

    setRandomness: (randomness: number) => {
        musicSettings.update((current) => ({ ...current, randomness }));
    },

    setReverb: (reverb: number) => {
        musicSettings.update((current) => ({ ...current, reverb }));
    },

    setDelayMix: (delayMix: number) => {
        musicSettings.update((current) => ({ ...current, delayMix }));
    },

    setLowestOctave: (lowestOctave: number) => {
        musicSettings.update((current) => ({ ...current, lowestOctave }));
    },

    setHighestOctave: (highestOctave: number) => {
        musicSettings.update((current) => ({ ...current, highestOctave }));
    },

    setSeed: (seed: number) => {
        musicSettings.update((current) => ({ ...current, seed }));
    },

    // Generator control actions
    setGeneratorEnabled: (type: GeneratorType, enabled: boolean) => {
        musicSettings.update((current) => ({
            ...current,
            generators: { ...current.generators, [type]: enabled },
        }));
    },

    setBeatDensity: (density: number) => {
        musicSettings.update((current) => ({ ...current, beatDensity: density }));
    },

    setBeatVolume: (volume: number) => {
        musicSettings.update((current) => ({ ...current, beatVolume: volume }));
    },

    setBassComplexity: (complexity: number) => {
        musicSettings.update((current) => ({ ...current, bassComplexity: complexity }));
    },

    setBassVolume: (volume: number) => {
        musicSettings.update((current) => ({ ...current, bassVolume: volume }));
    },

    setMelodyMovement: (movement: number) => {
        musicSettings.update((current) => ({ ...current, melodyMovement: movement }));
    },

    setMelodyVolume: (volume: number) => {
        musicSettings.update((current) => ({ ...current, melodyVolume: volume }));
    },

    setStructureTransitionInterval: (interval: number) => {
        musicEngine.setStructureTransitionInterval(interval);
        musicSettings.update((current) => ({ ...current, structureTransitionInterval: interval }));
    },

    forceStructureTransition: () => {
        musicEngine.forceStructureTransition();
        const newStructureIndex = musicEngine.getCurrentStructureIndex();
        const newSongStructure = musicEngine.getSongStructure();
        musicSettings.update((state) => ({
            ...state,
            currentStructureIndex: newStructureIndex,
            songStructure: newSongStructure,
            structureTransitionNotification: `Structure manually changed to: ${newSongStructure}`,
        }));
        // Clear notification after 3 seconds
        setTimeout(() => {
            musicSettings.update((state) => ({ ...state, structureTransitionNotification: '' }));
        }, 3000);
    },

    regeneratePatterns: () => {
        musicEngine.regeneratePatterns();
        musicSettings.update((state) => ({
            ...state,
            currentSection: '',
            songStructure: '',
            sectionProgress: 0,
            totalProgress: 0,
            currentLoopCount: 0,
            currentStructureIndex: 0,
        }));
    },

    resetSongPosition: () => {
        musicEngine.resetSongPosition();
        musicSettings.update((state) => ({
            ...state,
            currentSection: '',
            songStructure: '',
            sectionProgress: 0,
            totalProgress: 0,
            currentLoopCount: 0,
            currentStructureIndex: 0,
        }));
    },

    randomize: () => {
        const newSettings: Partial<MusicSettings> = {
            seed: Math.floor(Math.random() * 1e9),
            randomness: Math.min(1, Math.max(0, 0.3 + (Math.random() - 0.5) * 0.2)),
            tempo: Math.round(50 + Math.random() * 50),
            reverb: Math.min(0.95, Math.max(0, 0.15 + (Math.random() - 0.5) * 0.2)),
            delayMix: Math.min(1, Math.max(0, 0.15 + (Math.random() - 0.5) * 0.2)),
            lowestOctave: Math.floor(2 + Math.random() * 3), // 2-4
            highestOctave: Math.floor(3 + Math.random() * 3), // 3-5
            // Randomize generator settings
            beatDensity: Math.random(),
            beatVolume: -30 + Math.random() * 20, // -30 to -10 dB
            bassComplexity: Math.random(),
            bassVolume: -25 + Math.random() * 15, // -25 to -10 dB
            melodyMovement: Math.random(),
            melodyVolume: -25 + Math.random() * 15, // -25 to -10 dB
        };

        console.log('Randomizing settings:', newSettings);
        musicSettings.update((current) => ({ ...current, ...newSettings }));
    },

    resetToDefaults: () => {
        musicSettings.set(defaultSettings);
    },

    // Get current settings synchronously (for components that need it)
    getCurrentSettings: (): MusicSettings => {
        let current: MusicSettings = defaultSettings;
        musicSettings.subscribe((value) => (current = value))();
        return current;
    },
};
