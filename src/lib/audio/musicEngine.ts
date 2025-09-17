type ScaleName = string;
type SongStructure = string;
type GeneratorType = 'beat' | 'bass' | 'melody' | 'pad';

interface BeatEvent {
    time: string;
    type: 'kick' | 'snare' | 'hihat';
}

interface BassEvent {
    time: string;
    note: string;
}

interface MelodyEvent {
    time: string;
    note: string;
}

interface Pattern {
    beat: BeatEvent[];
    bass: BassEvent[];
    melody: MelodyEvent[];
}

interface SectionSettings {
    scale: ScaleName;
    lowestOctave: number;
    highestOctave: number;
    pattern: Pattern;
}

interface PresetPattern {
    id: string;
    name: string;
    beat: BeatEvent[];
    bass: BassEvent[];
    melody: MelodyEvent[];
}

interface GeneratorSettings {
    beatDensity: number; // 0-1
    beatVolume: number; // dB
    bassComplexity: number; // 0-1
    bassVolume: number; // dB
    melodyMovement: number; // 0-1
    melodyVolume: number; // dB
    patternLength: number; // measures (8, 16, or 32)
}

function mulberry32(seed: number) {
    let t = seed >>> 0;
    return function () {
        t += 0x6d2b79f5;
        let r = Math.imul(t ^ (t >>> 15), 1 | t);
        r ^= r + Math.imul(r ^ (r >>> 7), 61 | r);
        return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
    };
}

const songStructures: SongStructure[] = [
    'ABAB',
    'AABB',
    'ABAC',
    'AAAB',
    'AABC',
    'ABBB',
    'ABBC',
    'ABCC',
];

// Preset patterns
const presetPatterns: PresetPattern[] = [
    {
        id: 'ambient-1',
        name: 'Ambient Drift (2 measures)',
        beat: [
            { time: '0:0:0', type: 'kick' },
            { time: '0:2:0', type: 'kick' },
            { time: '1:0:0', type: 'kick' },
            { time: '1:2:0', type: 'kick' },
            { time: '0:0:2', type: 'hihat' },
            { time: '0:1:2', type: 'hihat' },
            { time: '0:2:2', type: 'hihat' },
            { time: '0:3:2', type: 'hihat' },
            { time: '1:0:2', type: 'hihat' },
            { time: '1:1:2', type: 'hihat' },
            { time: '1:2:2', type: 'hihat' },
            { time: '1:3:2', type: 'hihat' },
        ],
        bass: [
            { time: '0:0:0', note: 'C2' },
            { time: '0:2:0', note: 'G2' },
            { time: '0:3:0', note: 'A2' },
            { time: '1:0:0', note: 'C2' },
            { time: '1:2:0', note: 'G2' },
            { time: '1:3:0', note: 'A2' },
        ],
        melody: [
            { time: '0:0:0', note: 'C4' },
            { time: '0:1:0', note: 'E4' },
            { time: '0:2:0', note: 'G4' },
            { time: '0:3:0', note: 'A4' },
            { time: '1:0:0', note: 'C4' },
            { time: '1:1:0', note: 'E4' },
            { time: '1:2:0', note: 'G4' },
            { time: '1:3:0', note: 'A4' },
        ],
    },
    {
        id: 'minimal-1',
        name: 'Minimal Pulse (2 measures)',
        beat: [
            { time: '0:0:0', type: 'kick' },
            { time: '0:2:0', type: 'kick' },
            { time: '0:1:0', type: 'snare' },
            { time: '0:3:0', type: 'snare' },
            { time: '1:0:0', type: 'kick' },
            { time: '1:2:0', type: 'kick' },
            { time: '1:1:0', type: 'snare' },
            { time: '1:3:0', type: 'snare' },
        ],
        bass: [
            { time: '0:0:0', note: 'D2' },
            { time: '0:2:0', note: 'A2' },
            { time: '1:0:0', note: 'D2' },
            { time: '1:2:0', note: 'A2' },
        ],
        melody: [
            { time: '0:0:0', note: 'D4' },
            { time: '0:2:0', note: 'A4' },
            { time: '1:0:0', note: 'D4' },
            { time: '1:2:0', note: 'A4' },
        ],
    },
    {
        id: 'complex-1',
        name: 'Complex Layers (2 measures)',
        beat: [
            { time: '0:0:0', type: 'kick' },
            { time: '0:1:0', type: 'kick' },
            { time: '0:2:0', type: 'kick' },
            { time: '0:3:0', type: 'kick' },
            { time: '1:0:0', type: 'kick' },
            { time: '1:1:0', type: 'kick' },
            { time: '1:2:0', type: 'kick' },
            { time: '1:3:0', type: 'kick' },
            { time: '0:0:2', type: 'hihat' },
            { time: '0:1:2', type: 'hihat' },
            { time: '0:2:2', type: 'hihat' },
            { time: '0:3:2', type: 'hihat' },
            { time: '1:0:2', type: 'hihat' },
            { time: '1:1:2', type: 'hihat' },
            { time: '1:2:2', type: 'hihat' },
            { time: '1:3:2', type: 'hihat' },
            { time: '0:1:0', type: 'snare' },
            { time: '0:3:0', type: 'snare' },
            { time: '1:1:0', type: 'snare' },
            { time: '1:3:0', type: 'snare' },
        ],
        bass: [
            { time: '0:0:0', note: 'C2' },
            { time: '0:0:2', note: 'E2' },
            { time: '0:1:0', note: 'F2' },
            { time: '0:1:2', note: 'G2' },
            { time: '0:2:0', note: 'A2' },
            { time: '0:2:2', note: 'C3' },
            { time: '0:3:0', note: 'D3' },
            { time: '0:3:2', note: 'E3' },
            { time: '1:0:0', note: 'C2' },
            { time: '1:0:2', note: 'E2' },
            { time: '1:1:0', note: 'F2' },
            { time: '1:1:2', note: 'G2' },
            { time: '1:2:0', note: 'A2' },
            { time: '1:2:2', note: 'C3' },
            { time: '1:3:0', note: 'D3' },
            { time: '1:3:2', note: 'E3' },
        ],
        melody: [
            { time: '0:0:0', note: 'C4' },
            { time: '0:0:2', note: 'E4' },
            { time: '0:1:0', note: 'F4' },
            { time: '0:1:2', note: 'G4' },
            { time: '0:2:0', note: 'A4' },
            { time: '0:2:2', note: 'C5' },
            { time: '0:3:0', note: 'D5' },
            { time: '0:3:2', note: 'E5' },
            { time: '1:0:0', note: 'C4' },
            { time: '1:0:2', note: 'E4' },
            { time: '1:1:0', note: 'F4' },
            { time: '1:1:2', note: 'G4' },
            { time: '1:2:0', note: 'A4' },
            { time: '1:2:2', note: 'C5' },
            { time: '1:3:0', note: 'D5' },
            { time: '1:3:2', note: 'E5' },
        ],
    },
];

class MusicEngine {
    private ctx: AudioContext | null = null;
    private master: GainNode | null = null;
    private out: GainNode | null = null;
    private delay: DelayNode | null = null;
    private feedback: GainNode | null = null;
    private convolver: ConvolverNode | null = null;
    private delayMixGain: GainNode | null = null;

    // Instruments
    private kick: GainNode | null = null;
    private snare: GainNode | null = null;
    private hihat: GainNode | null = null;
    private bass: GainNode | null = null;
    private padInstrument: GainNode | null = null;

    // Audio generation parameters (no state management)
    private volume = 0.4;
    private tempo = 40; // bpm
    private scale: ScaleName = 'lydian';
    private reverb = 0.15;
    private delayMix = 0.15;
    private lowestOctave = 2; // lowest octave (inclusive)
    private highestOctave = 4; // highest octave (inclusive)
    private seed = 12345;
    private rand = mulberry32(12345);
    private schedulerId: number | null = null;

    // New generator system
    private generators: Record<GeneratorType, boolean> = {
        beat: true,
        bass: true,
        melody: true,
        pad: true,
    };
    private generatorSettings: GeneratorSettings = {
        beatDensity: 0.8,
        beatVolume: 0,
        bassComplexity: 0.3,
        bassVolume: -15,
        melodyMovement: 0.4,
        melodyVolume: -18,
        patternLength: 16, // 16 measures by default
    };
    private songStructure: string = '';
    private sectionSettings: Record<string, SectionSettings> = {};
    private customPatterns: Record<string, PresetPattern> = {};
    private currentLoopCount: number = 0;
    private structureTransitionInterval: number = 8; // Change structure every 8 loops
    private currentStructureIndex: number = 0;

    private get now() {
        return this.ctx ? this.ctx.currentTime : 0;
    }

    private ensureContext() {
        if (!this.ctx) {
            this.ctx = new (window.AudioContext || (window as any).webkitAudioContext)();
            this.master = this.ctx.createGain();
            this.master.gain.value = this.volume;
            this.out = this.ctx.createGain();

            // Delay + feedback
            this.delay = this.ctx.createDelay(2.5);
            this.delay.delayTime.value = 0.4;
            this.feedback = this.ctx.createGain();
            this.feedback.gain.value = 0.25;

            // Simple generated IR for reverb
            this.convolver = this.ctx.createConvolver();
            this.convolver.buffer = this.makeImpulseResponse(this.ctx, 2.5, this.reverb);

            // Create instruments
            this.createInstruments();

            // Routing: master -> [dry to out] and [wet to delay/reverb] -> out -> destination
            const dry = this.ctx.createGain();
            dry.gain.value = 1.0;

            const wet = this.ctx.createGain();
            wet.gain.value = 0.6;

            this.master.connect(dry).connect(this.out);
            this.master.connect(wet);
            wet.connect(this.delay!).connect(this.feedback!).connect(this.delay!); // feedback loop
            wet.connect(this.convolver!);

            this.delayMixGain = this.ctx.createGain();
            this.delayMixGain.gain.value = this.delayMix;
            const revMix = this.ctx.createGain();
            revMix.gain.value = this.reverb;

            this.delay!.connect(this.delayMixGain).connect(this.out);
            this.convolver!.connect(revMix).connect(this.out);

            this.out!.connect(this.ctx.destination);
        }
    }

    private createInstruments() {
        if (!this.ctx) return;

        // Beat instruments
        this.kick = this.ctx.createGain();
        this.kick.gain.value = this.dbToGain(this.generatorSettings.beatVolume);
        this.kick.connect(this.master!);

        this.snare = this.ctx.createGain();
        this.snare.gain.value = this.dbToGain(this.generatorSettings.beatVolume - 5);
        this.snare.connect(this.master!);

        this.hihat = this.ctx.createGain();
        this.hihat.gain.value = this.dbToGain(this.generatorSettings.beatVolume - 10);
        this.hihat.connect(this.master!);

        // Bass instrument
        this.bass = this.ctx.createGain();
        this.bass.gain.value = this.dbToGain(this.generatorSettings.bassVolume);
        this.bass.connect(this.master!);

        // Pad instrument
        this.padInstrument = this.ctx.createGain();
        this.padInstrument.gain.value = this.dbToGain(this.generatorSettings.melodyVolume);
        this.padInstrument.connect(this.master!);
    }

    private dbToGain(db: number): number {
        return Math.pow(10, db / 20);
    }

    private generateSongStructure() {
        // Cycle through song structures instead of random selection
        const structure = songStructures[this.currentStructureIndex];
        this.songStructure = structure;

        console.log(
            `Generating song structure: ${structure} (index: ${this.currentStructureIndex})`
        );

        // Generate settings for each unique section
        const uniqueSections = [...new Set(this.songStructure.split(''))];
        this.sectionSettings = {};
        uniqueSections.forEach((section) => {
            this.sectionSettings[section] = {
                scale: this.scale,
                lowestOctave: this.lowestOctave,
                highestOctave: this.highestOctave,
                pattern: {
                    beat: this.generateBeatPattern(),
                    bass: this.generateBassPattern(this.scale, this.lowestOctave),
                    melody: this.generateMelodyPattern(
                        this.scale,
                        this.lowestOctave,
                        this.highestOctave
                    ),
                },
            };
        });
    }

    private advanceToNextStructure() {
        this.currentStructureIndex = (this.currentStructureIndex + 1) % songStructures.length;
        console.log(
            `Advancing to next song structure: ${songStructures[this.currentStructureIndex]} (index: ${this.currentStructureIndex})`
        );
        this.generateSongStructure();
    }

    private generateBeatPattern(): BeatEvent[] {
        const events: BeatEvent[] = [];
        const measures = this.generatorSettings.patternLength;
        const totalBeats = measures * 16; // 16 beats per measure

        for (let i = 0; i < totalBeats; i++) {
            const measure = Math.floor(i / 16);
            const beatInMeasure = i % 16;
            const time = `${measure}:${Math.floor(beatInMeasure / 4)}:${beatInMeasure % 4}`;

            // Kick pattern - more reliable, with some variation
            if (i % 4 === 0) {
                // Add some variation - occasionally skip kicks
                if (this.rand() > 0.1) {
                    events.push({ time, type: 'kick' });
                }
            }

            // Snare pattern - more reliable, with some variation
            if (i % 8 === 4) {
                // Add some variation - occasionally skip snares
                if (this.rand() > 0.15) {
                    events.push({ time, type: 'snare' });
                }
            }

            // Hihat pattern - use higher density with more variation
            if (this.rand() < Math.max(0.2, this.generatorSettings.beatDensity * 0.8)) {
                events.push({ time, type: 'hihat' });
            }

            // Add some additional kick variations for longer patterns
            if (measures >= 8 && i % 16 === 8 && this.rand() < 0.3) {
                events.push({ time, type: 'kick' });
            }

            // Add some additional snare variations for longer patterns
            if (measures >= 16 && i % 32 === 16 && this.rand() < 0.4) {
                events.push({ time, type: 'snare' });
            }
        }

        console.log(
            `Generated beat pattern with ${events.length} events over ${measures} measures:`,
            events
        );
        return events;
    }

    private generateBassPattern(
        scale: ScaleName = this.scale,
        lowestOctave: number = this.lowestOctave
    ): BassEvent[] {
        const events: BassEvent[] = [];
        const scaleNotes = this.getScaleNotes(scale, lowestOctave, 2); // Bass octave
        const measures = this.generatorSettings.patternLength;
        const totalSubdivisions = measures * 8; // 8 subdivisions per measure

        // Safety check for empty scale notes
        if (scaleNotes.length === 0) {
            console.warn(
                'No scale notes generated for scale:',
                scale,
                'lowestOctave:',
                lowestOctave
            );
            return events;
        }

        let currentNoteIndex = Math.floor(scaleNotes.length / 2);
        let lastNoteTime = -1;

        for (let i = 0; i < totalSubdivisions; i++) {
            const measure = Math.floor(i / 8);
            const beatInMeasure = i % 8;
            const beat = Math.floor(beatInMeasure / 2); // 0, 0, 1, 1, 2, 2, 3, 3
            const subdivision = (beatInMeasure % 2) * 2; // 0, 2, 0, 2, 0, 2, 0, 2
            const time = `${measure}:${beat}:${subdivision}`;

            // Base probability for bass notes
            let noteProbability = 0.3 + this.generatorSettings.bassComplexity * 0.4;

            // Increase probability for downbeats
            if (beatInMeasure % 4 === 0) {
                noteProbability += 0.3;
            }

            // Decrease probability if we just played a note (avoid too many consecutive notes)
            if (i - lastNoteTime < 2) {
                noteProbability *= 0.5;
            }

            if (this.rand() < noteProbability) {
                // Random walk for note selection with some structure
                if (this.rand() < 0.7) {
                    // Small steps (more musical)
                    currentNoteIndex += this.rand() < 0.5 ? 1 : -1;
                } else {
                    // Occasional larger jumps
                    currentNoteIndex += Math.floor(this.rand() * 5) - 2;
                }

                currentNoteIndex = Math.max(0, Math.min(scaleNotes.length - 1, currentNoteIndex));
                const note = scaleNotes[currentNoteIndex];

                // Additional safety check
                if (note && typeof note === 'string') {
                    events.push({ time, note });
                    lastNoteTime = i;
                } else {
                    console.warn('Invalid note generated:', note, 'at index:', currentNoteIndex);
                }
            }
        }

        console.log(
            `Generated bass pattern with ${events.length} events over ${measures} measures`
        );
        return events;
    }

    private generateMelodyPattern(
        scale: ScaleName = this.scale,
        lowestOctave: number = this.lowestOctave,
        highestOctave: number = this.highestOctave
    ): MelodyEvent[] {
        const events: MelodyEvent[] = [];
        const scaleNotes = this.getScaleNotes(scale, lowestOctave, highestOctave);
        const measures = this.generatorSettings.patternLength;
        const totalSubdivisions = measures * 8; // 8 subdivisions per measure

        // Safety check for empty scale notes
        if (scaleNotes.length === 0) {
            console.warn(
                'No scale notes generated for melody scale:',
                scale,
                'lowestOctave:',
                lowestOctave,
                'highestOctave:',
                highestOctave
            );
            return events;
        }

        let currentIndex = Math.floor(scaleNotes.length / 2);
        let lastNoteTime = -1;
        let phraseStart = 0;

        for (let i = 0; i < totalSubdivisions; i++) {
            const measure = Math.floor(i / 8);
            const beatInMeasure = i % 8;
            const beat = Math.floor(beatInMeasure / 2); // 0, 0, 1, 1, 2, 2, 3, 3
            const subdivision = (beatInMeasure % 2) * 2; // 0, 2, 0, 2, 0, 2, 0, 2
            const time = `${measure}:${beat}:${subdivision}`;

            // Base probability for melody notes
            let noteProbability = 0.2 + this.generatorSettings.melodyMovement * 0.3;

            // Increase probability for downbeats
            if (beatInMeasure % 4 === 0) {
                noteProbability += 0.2;
            }

            // Decrease probability if we just played a note (avoid too many consecutive notes)
            if (i - lastNoteTime < 1) {
                noteProbability *= 0.3;
            }

            // Create phrases - longer patterns get more structured phrases
            const phraseLength = measures >= 16 ? 32 : measures >= 8 ? 16 : 8;
            if (i - phraseStart >= phraseLength) {
                phraseStart = i;
                // Reset to a more central note at phrase boundaries
                currentIndex = Math.floor(scaleNotes.length / 2) + Math.floor(this.rand() * 3) - 1;
                currentIndex = Math.max(0, Math.min(scaleNotes.length - 1, currentIndex));
            }

            if (this.rand() < noteProbability) {
                // Random walk with more musical structure
                if (this.rand() < 0.8) {
                    // Small steps (more musical)
                    currentIndex += this.rand() < 0.5 ? 1 : -1;
                } else if (this.rand() < 0.9) {
                    // Medium jumps
                    currentIndex += Math.floor(this.rand() * 3) - 1;
                } else {
                    // Occasional large jumps
                    currentIndex += Math.floor(this.rand() * 7) - 3;
                }

                currentIndex = Math.max(0, Math.min(scaleNotes.length - 1, currentIndex));
                const note = scaleNotes[currentIndex];

                // Additional safety check
                if (note && typeof note === 'string') {
                    events.push({ time, note });
                    lastNoteTime = i;
                } else {
                    console.warn('Invalid melody note generated:', note, 'at index:', currentIndex);
                }
            }
        }

        console.log(
            `Generated melody pattern with ${events.length} events over ${measures} measures`
        );
        return events;
    }

    private getScaleNotes(scale: ScaleName, lowestOctave: number, highestOctave: number): string[] {
        const scaleIntervals = {
            aeolian: [0, 2, 3, 5, 7, 8, 10],
            blues: [0, 3, 5, 6, 7, 10],
            chromatic: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            dorian: [0, 2, 3, 5, 7, 9, 10],
            harmonicMinor: [0, 2, 3, 5, 7, 8, 11],
            ionian: [0, 2, 4, 5, 7, 9, 11],
            locrian: [0, 1, 3, 5, 6, 8, 10],
            lydian: [0, 2, 4, 6, 7, 9, 11],
            major: [0, 2, 4, 5, 7, 9, 11],
            melodicMinor: [0, 2, 3, 5, 7, 9, 11],
            minor: [0, 3, 5, 7, 10],
            mixolydian: [0, 2, 4, 5, 7, 9, 10],
            pentatonic: [0, 3, 5, 7, 10],
            phrygian: [0, 1, 3, 5, 7, 8, 10],
            whole: [0, 2, 4, 6, 8, 10],
        };

        const intervals = scaleIntervals[scale as keyof typeof scaleIntervals];
        if (!intervals) {
            console.warn('Unknown scale:', scale, 'using pentatonic as fallback');
            return this.getScaleNotes('pentatonic', lowestOctave, highestOctave);
        }

        const notes: string[] = [];
        const noteNames = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

        for (let octave = lowestOctave; octave <= highestOctave; octave++) {
            for (const interval of intervals) {
                const noteName = noteNames[interval];
                if (noteName) {
                    notes.push(`${noteName}${octave}`);
                }
            }
        }

        return notes;
    }

    private playKick(time: number) {
        if (!this.ctx || !this.kick) {
            console.log('Cannot play kick: ctx or kick not available');
            return;
        }

        console.log('Playing kick at time', time, 'kick gain:', this.kick.gain.value);

        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();

        osc.type = 'sine';
        osc.frequency.setValueAtTime(60, time);
        osc.frequency.exponentialRampToValueAtTime(30, time + 0.1);

        gain.gain.setValueAtTime(0, time);
        gain.gain.linearRampToValueAtTime(1, time + 0.01);
        gain.gain.exponentialRampToValueAtTime(0.001, time + 0.3);

        osc.connect(gain).connect(this.kick);
        osc.start(time);
        osc.stop(time + 0.3);
    }

    private playSnare(time: number) {
        if (!this.ctx || !this.snare) return;

        const noise = this.ctx.createBufferSource();
        const buffer = this.ctx.createBuffer(1, this.ctx.sampleRate * 0.1, this.ctx.sampleRate);
        const data = buffer.getChannelData(0);

        for (let i = 0; i < data.length; i++) {
            data[i] = (Math.random() * 2 - 1) * 0.5;
        }

        noise.buffer = buffer;

        const gain = this.ctx.createGain();
        gain.gain.setValueAtTime(0, time);
        gain.gain.linearRampToValueAtTime(1, time + 0.01);
        gain.gain.exponentialRampToValueAtTime(0.001, time + 0.1);

        noise.connect(gain).connect(this.snare);
        noise.start(time);
        noise.stop(time + 0.1);
    }

    private playHihat(time: number) {
        if (!this.ctx || !this.hihat) return;

        const noise = this.ctx.createBufferSource();
        const buffer = this.ctx.createBuffer(1, this.ctx.sampleRate * 0.05, this.ctx.sampleRate);
        const data = buffer.getChannelData(0);

        for (let i = 0; i < data.length; i++) {
            data[i] = (Math.random() * 2 - 1) * 0.3;
        }

        noise.buffer = buffer;

        const gain = this.ctx.createGain();
        gain.gain.setValueAtTime(0, time);
        gain.gain.linearRampToValueAtTime(1, time + 0.001);
        gain.gain.exponentialRampToValueAtTime(0.001, time + 0.05);

        noise.connect(gain).connect(this.hihat);
        noise.start(time);
        noise.stop(time + 0.05);
    }

    private playBass(time: number, note: string) {
        if (!this.ctx || !this.bass) return;

        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        const filter = this.ctx.createBiquadFilter();

        osc.type = 'sine';
        osc.frequency.value = this.noteToFreq(this.noteToMidi(note));

        filter.type = 'lowpass';
        filter.frequency.value = 300;
        filter.Q.value = 2;

        gain.gain.setValueAtTime(0, time);
        gain.gain.linearRampToValueAtTime(1, time + 0.1);
        gain.gain.exponentialRampToValueAtTime(0.001, time + 2);

        osc.connect(filter).connect(gain).connect(this.bass);
        osc.start(time);
        osc.stop(time + 2);
    }

    private playPad(time: number, note: string) {
        if (!this.ctx || !this.padInstrument) return;

        const osc1 = this.ctx.createOscillator();
        const osc2 = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        const lfo = this.ctx.createOscillator();
        const lfoGain = this.ctx.createGain();

        const freq = this.noteToFreq(this.noteToMidi(note));

        osc1.type = 'triangle';
        osc2.type = 'sine';
        osc1.frequency.value = freq;
        osc2.frequency.value = freq;

        lfo.frequency.value = 0.1;
        lfoGain.gain.value = 10;

        gain.gain.setValueAtTime(0, time);
        gain.gain.linearRampToValueAtTime(1, time + 1.5);
        gain.gain.exponentialRampToValueAtTime(0.001, time + 3);

        lfo.connect(lfoGain);
        lfoGain.connect(osc1.detune);
        lfoGain.connect(osc2.detune);

        osc1.connect(gain).connect(this.padInstrument);
        osc2.connect(gain).connect(this.padInstrument);

        lfo.start(time);
        osc1.start(time);
        osc2.start(time);

        const stopTime = time + 3;
        lfo.stop(stopTime);
        osc1.stop(stopTime);
        osc2.stop(stopTime);
    }

    private noteToMidi(note: string): number {
        // Handle undefined, null, or empty note values
        if (!note || typeof note !== 'string') {
            console.warn('Invalid note value:', note, 'using default C4');
            return 60; // Default to C4
        }

        const noteMap: Record<string, number> = {
            C: 0,
            'C#': 1,
            D: 2,
            'D#': 3,
            E: 4,
            F: 5,
            'F#': 6,
            G: 7,
            'G#': 8,
            A: 9,
            'A#': 10,
            B: 11,
        };

        const match = note.match(/^([A-G]#?)(\d+)$/);
        if (!match) {
            console.warn('Invalid note format:', note, 'using default C4');
            return 60; // Default to C4
        }

        const [, noteName, octave] = match;
        const noteNumber = noteMap[noteName] || 0;
        const octaveNumber = parseInt(octave);

        return 12 + octaveNumber * 12 + noteNumber;
    }

    private makeImpulseResponse(ctx: AudioContext, duration: number, mix: number) {
        const rate = ctx.sampleRate;
        const length = Math.max(1, Math.floor(duration * rate));
        const ir = ctx.createBuffer(2, length, rate);
        for (let ch = 0; ch < 2; ch++) {
            const data = ir.getChannelData(ch);
            for (let i = 0; i < length; i++) {
                const t = i / length;
                data[i] = (Math.random() * 2 - 1) * Math.pow(1 - t, 2.0) * mix;
            }
        }
        return ir;
    }

    private noteToFreq(midi: number) {
        return 440 * Math.pow(2, (midi - 69) / 12);
    }

    private scheduleLoop() {
        if (!this.ctx) return;

        // Generate song structure if not exists
        if (this.songStructure.length === 0) {
            this.generateSongStructure();
        }

        const beat = 60 / this.tempo; // seconds per beat
        const horizon = this.now + beat * 4; // schedule ahead by 4 beats
        const currentTime = this.now;

        // Calculate current section based on time
        const sectionDuration = beat * 8; // 8 beats per section
        const currentSectionIndex =
            Math.floor(currentTime / sectionDuration) % this.songStructure.length;
        const currentSection = this.songStructure[currentSectionIndex];
        const sectionSettings = this.sectionSettings[currentSection];

        // Calculate the start time of the current section
        const currentSectionStartTime = Math.floor(currentTime / sectionDuration) * sectionDuration;

        // Track loops and trigger structure changes
        const totalSongDuration = sectionDuration * this.songStructure.length;
        const currentLoop = Math.floor(currentTime / totalSongDuration);

        // Check if we need to advance to the next structure
        if (currentLoop > this.currentLoopCount) {
            this.currentLoopCount = currentLoop;
            console.log(`Completed loop ${currentLoop}, checking for structure transition...`);

            // Check if we should transition to the next structure
            if (this.currentLoopCount % this.structureTransitionInterval === 0) {
                console.log(`Structure transition triggered at loop ${this.currentLoopCount}`);
                this.advanceToNextStructure();
            }
        }

        console.log(
            'Scheduler tick - Current time:',
            currentTime.toFixed(2),
            'Section:',
            currentSection,
            'Index:',
            currentSectionIndex,
            'Section start:',
            currentSectionStartTime.toFixed(2)
        );

        if (!sectionSettings) return;

        const pattern = sectionSettings.pattern;

        // Schedule beat events
        if (this.generators.beat) {
            console.log('Beat generator enabled, pattern has', pattern.beat.length, 'events');
            console.log('Current time:', currentTime, 'Horizon:', horizon);
            pattern.beat.forEach((event) => {
                const relativeEventTime = this.parseTimeString(event.time, beat);
                const absoluteEventTime = currentSectionStartTime + relativeEventTime;
                console.log(
                    'Event time (relative):',
                    relativeEventTime,
                    'Event time (absolute):',
                    absoluteEventTime,
                    'Current:',
                    currentTime,
                    'Horizon:',
                    horizon,
                    'In range:',
                    absoluteEventTime >= currentTime && absoluteEventTime < horizon
                );
                if (absoluteEventTime >= currentTime && absoluteEventTime < horizon) {
                    console.log('Scheduling beat event:', event.type, 'at time', absoluteEventTime);
                    switch (event.type) {
                        case 'kick':
                            this.playKick(absoluteEventTime);
                            break;
                        case 'snare':
                            this.playSnare(absoluteEventTime);
                            break;
                        case 'hihat':
                            this.playHihat(absoluteEventTime);
                            break;
                    }
                }
            });
        } else {
            console.log('Beat generator disabled');
        }

        // Schedule bass events
        if (this.generators.bass) {
            pattern.bass.forEach((event) => {
                const relativeEventTime = this.parseTimeString(event.time, beat);
                const absoluteEventTime = currentSectionStartTime + relativeEventTime;
                if (absoluteEventTime >= currentTime && absoluteEventTime < horizon) {
                    this.playBass(absoluteEventTime, event.note);
                }
            });
        }

        // Schedule melody events
        if (this.generators.melody) {
            pattern.melody.forEach((event) => {
                const relativeEventTime = this.parseTimeString(event.time, beat);
                const absoluteEventTime = currentSectionStartTime + relativeEventTime;
                if (absoluteEventTime >= currentTime && absoluteEventTime < horizon) {
                    this.playPad(absoluteEventTime, event.note);
                }
            });
        }
    }

    private parseTimeString(timeStr: string, beatDuration: number): number {
        // Parse time string like "0:0:0" (measure:beat:subdivision)
        const parts = timeStr.split(':').map(Number);
        const [measure, beat, subdivision] = parts;

        // Assuming 4/4 time signature
        const measureDuration = beatDuration * 4;
        const subdivisionDuration = beatDuration / 4;

        const result =
            measure * measureDuration + beat * beatDuration + subdivision * subdivisionDuration;
        return result;
    }

    private startScheduler() {
        if (this.schedulerId !== null) return;
        console.log('Starting music scheduler');
        const tick = () => {
            this.scheduleLoop();
            this.schedulerId = window.setTimeout(tick, 120);
        };
        this.schedulerId = window.setTimeout(tick, 0);
    }

    private stopScheduler() {
        if (this.schedulerId !== null) {
            window.clearTimeout(this.schedulerId);
            this.schedulerId = null;
        }
    }

    async start() {
        console.log('Music engine starting...');
        this.ensureContext();
        if (this.ctx && this.ctx.state === 'suspended') {
            await this.ctx.resume();
        }

        // Reset loop count when starting
        this.currentLoopCount = 0;
        this.currentStructureIndex = 0;

        // Restore master gain to current volume
        if (this.master) {
            this.master.gain.setValueAtTime(this.volume, this.now);
        }

        console.log('Music engine starting');
        this.initializeDefaultSections();
        this.startScheduler();
        console.log('Music engine started successfully');
    }

    stop() {
        console.log('Music engine stopping...');
        this.stopScheduler();

        // Immediately stop all sounds by setting master gain to 0
        if (this.master) {
            this.master.gain.setValueAtTime(0, this.now);
        }

        console.log('Music engine stopped - all sounds silenced');
    }

    resetSongPosition() {
        console.log('Resetting song position to beginning...');

        // Stop current playback
        this.stopScheduler();

        // Close the current audio context to reset time
        if (this.ctx) {
            this.ctx.close();
            this.ctx = null;
        }

        // Reset all audio nodes
        this.master = null;
        this.out = null;
        this.delay = null;
        this.feedback = null;
        this.convolver = null;
        this.delayMixGain = null;
        this.kick = null;
        this.snare = null;
        this.hihat = null;
        this.bass = null;
        this.padInstrument = null;

        // Reset song state
        this.currentLoopCount = 0;
        this.currentStructureIndex = 0;

        // Reset song structure to force regeneration
        this.songStructure = '';
        this.sectionSettings = {};

        console.log('Song position reset to beginning');
    }

    // Remove setEnabled method - state management is now handled by the store

    // Method to explicitly initialize audio context from user interaction
    async initializeAudioContext() {
        this.ensureContext();
        if (this.ctx && this.ctx.state === 'suspended') {
            await this.ctx.resume();
            console.log('Audio context resumed from user interaction');
        }
    }

    // Get the current state of the audio context
    getAudioContextState(): string {
        return this.ctx ? this.ctx.state : 'closed';
    }

    // Remove isEnabled method - state management is now handled by the store

    setVolume(v: number) {
        this.volume = Math.max(0, Math.min(1, v));
        if (this.master) this.master.gain.value = this.volume;
    }

    setTempo(bpm: number) {
        this.tempo = Math.max(20, Math.min(180, bpm));
    }

    setScale(name: ScaleName) {
        this.scale = name;
        // Propagate new scale to all sections and regenerate their patterns
        Object.keys(this.sectionSettings).forEach((section) => {
            this.setSectionScale(section, name);
        });
    }

    setReverb(v: number) {
        this.reverb = Math.max(0, Math.min(0.95, v));
        if (this.ctx && this.convolver) {
            this.convolver.buffer = this.makeImpulseResponse(this.ctx, 2.5, this.reverb);
        }
    }

    setDelayMix(v: number) {
        this.delayMix = Math.max(0, Math.min(1, v));
        if (this.delayMixGain) {
            this.delayMixGain.gain.value = this.delayMix;
        }
    }

    setSeed(s: number) {
        this.seed = Math.floor(s) || 1;
        this.rand = mulberry32(this.seed);
    }

    setLowestOctave(octave: number) {
        this.lowestOctave = Math.max(0, Math.min(7, octave));
        // Ensure highest octave is at least as high as lowest octave
        if (this.highestOctave < this.lowestOctave) {
            this.highestOctave = this.lowestOctave;
        }
    }

    setHighestOctave(octave: number) {
        this.highestOctave = Math.max(0, Math.min(8, octave));
        // Ensure lowest octave is at most as high as highest octave
        if (this.lowestOctave > this.highestOctave) {
            this.lowestOctave = this.highestOctave;
        }
    }

    // New generator control methods
    setGeneratorEnabled(type: GeneratorType, enabled: boolean) {
        this.generators[type] = enabled;
    }

    isGeneratorEnabled(type: GeneratorType): boolean {
        return this.generators[type];
    }

    setBeatDensity(density: number) {
        this.generatorSettings.beatDensity = Math.max(0, Math.min(1, density));
    }

    setBeatVolume(volume: number) {
        this.generatorSettings.beatVolume = Math.max(-40, Math.min(0, volume));
        if (this.kick) this.kick.gain.value = this.dbToGain(this.generatorSettings.beatVolume);
        if (this.snare)
            this.snare.gain.value = this.dbToGain(this.generatorSettings.beatVolume - 5);
        if (this.hihat)
            this.hihat.gain.value = this.dbToGain(this.generatorSettings.beatVolume - 10);
    }

    setBassComplexity(complexity: number) {
        this.generatorSettings.bassComplexity = Math.max(0, Math.min(1, complexity));
    }

    setBassVolume(volume: number) {
        this.generatorSettings.bassVolume = Math.max(-40, Math.min(0, volume));
        if (this.bass) this.bass.gain.value = this.dbToGain(this.generatorSettings.bassVolume);
    }

    setMelodyMovement(movement: number) {
        this.generatorSettings.melodyMovement = Math.max(0, Math.min(1, movement));
    }

    setMelodyVolume(volume: number) {
        this.generatorSettings.melodyVolume = Math.max(-40, Math.min(0, volume));
        if (this.padInstrument)
            this.padInstrument.gain.value = this.dbToGain(this.generatorSettings.melodyVolume);
    }

    setPatternLength(length: number) {
        // Only allow 8, 16, or 32 measures
        const validLengths = [8, 16, 32];
        if (validLengths.includes(length)) {
            this.generatorSettings.patternLength = length;
            console.log('Music engine: pattern length set to', length, 'measures');
        } else {
            console.warn('Invalid pattern length:', length, 'must be 8, 16, or 32');
        }
    }

    getPatternLength(): number {
        return this.generatorSettings.patternLength;
    }

    setStructureTransitionInterval(interval: number) {
        this.structureTransitionInterval = Math.max(1, Math.min(32, interval));
    }

    getStructureTransitionInterval(): number {
        return this.structureTransitionInterval;
    }

    getCurrentLoopCount(): number {
        return this.currentLoopCount;
    }

    getCurrentStructureIndex(): number {
        return this.currentStructureIndex;
    }

    getCurrentSongStructure(): string {
        return this.songStructure;
    }

    // Force transition to next structure
    forceStructureTransition() {
        console.log('Forcing structure transition...');
        this.advanceToNextStructure();
    }

    regeneratePatterns() {
        this.currentLoopCount = 0; // Reset loop count when regenerating
        this.currentStructureIndex = 0; // Reset to first structure
        this.generateSongStructure();
        // Also regenerate patterns for existing sections
        this.initializeDefaultSections();
        console.log('Music engine: patterns regenerated');
        console.log('Current sections:', Object.keys(this.sectionSettings));
        console.log('Section A pattern:', this.sectionSettings['A']?.pattern);
    }

    // Section management methods
    getSectionSettings(section: string): SectionSettings | null {
        return this.sectionSettings[section] || null;
    }

    setSectionSettings(section: string, settings: SectionSettings) {
        this.sectionSettings[section] = settings;
        console.log('Music engine: section', section, 'settings updated');
    }

    setSectionScale(section: string, scale: ScaleName) {
        if (this.sectionSettings[section]) {
            this.sectionSettings[section].scale = scale;
            // Regenerate patterns for this section
            this.sectionSettings[section].pattern = {
                beat: this.generateBeatPattern(),
                bass: this.generateBassPattern(scale, this.sectionSettings[section].lowestOctave),
                melody: this.generateMelodyPattern(
                    scale,
                    this.sectionSettings[section].lowestOctave,
                    this.sectionSettings[section].highestOctave
                ),
            };
        }
    }

    setSectionOctaves(section: string, lowestOctave: number, highestOctave: number) {
        if (this.sectionSettings[section]) {
            this.sectionSettings[section].lowestOctave = lowestOctave;
            this.sectionSettings[section].highestOctave = highestOctave;
            // Regenerate patterns for this section
            const scale = this.sectionSettings[section].scale;
            this.sectionSettings[section].pattern = {
                beat: this.generateBeatPattern(),
                bass: this.generateBassPattern(scale, lowestOctave),
                melody: this.generateMelodyPattern(scale, lowestOctave, highestOctave),
            };
        }
    }

    setSectionPattern(section: string, pattern: Pattern) {
        if (this.sectionSettings[section]) {
            this.sectionSettings[section].pattern = pattern;
        }
    }

    // Preset pattern methods
    getPresetPatterns(): PresetPattern[] {
        return [...presetPatterns, ...Object.values(this.customPatterns)];
    }

    loadPresetPattern(section: string, patternId: string) {
        const allPatterns = this.getPresetPatterns();
        const pattern = allPatterns.find((p) => p.id === patternId);
        if (pattern && this.sectionSettings[section]) {
            this.sectionSettings[section].pattern = {
                beat: [...pattern.beat],
                bass: [...pattern.bass],
                melody: [...pattern.melody],
            };
            console.log('Music engine: loaded preset pattern', patternId, 'for section', section);
        }
    }

    saveCustomPattern(pattern: PresetPattern) {
        this.customPatterns[pattern.id] = pattern;
        console.log('Music engine: saved custom pattern', pattern.id);
    }

    deleteCustomPattern(patternId: string) {
        delete this.customPatterns[patternId];
        console.log('Music engine: deleted custom pattern', patternId);
    }

    // Initialize default section settings
    initializeDefaultSections() {
        console.log('Initializing default sections...');
        const sections = ['A', 'B', 'C'];
        sections.forEach((section) => {
            if (!this.sectionSettings[section]) {
                const beatPattern = this.generateBeatPattern();
                const bassPattern = this.generateBassPattern(this.scale, this.lowestOctave);
                const melodyPattern = this.generateMelodyPattern(
                    this.scale,
                    this.lowestOctave,
                    this.highestOctave
                );
                console.log(
                    `Section ${section} patterns - Beat: ${beatPattern.length}, Bass: ${bassPattern.length}, Melody: ${melodyPattern.length}`
                );
                this.sectionSettings[section] = {
                    scale: this.scale,
                    lowestOctave: this.lowestOctave,
                    highestOctave: this.highestOctave,
                    pattern: {
                        beat: beatPattern,
                        bass: bassPattern,
                        melody: melodyPattern,
                    },
                };
            }
        });
        console.log('Default sections initialized');
    }

    // Get current song structure
    getSongStructure(): string {
        return this.songStructure;
    }

    // Get current section based on time
    getCurrentSection(): string {
        if (!this.ctx || this.songStructure.length === 0) return '';

        const currentTime = this.ctx.currentTime;
        const beat = 60 / this.tempo;
        const sectionDuration = beat * 8; // 8 beats per section
        const songStructureArray = this.songStructure.split('');
        const currentSectionIndex =
            Math.floor(currentTime / sectionDuration) % songStructureArray.length;
        return songStructureArray[currentSectionIndex] || '';
    }

    // Get progress within current section (0-1)
    getSectionProgress(): number {
        if (!this.ctx || this.songStructure.length === 0) return 0;

        const currentTime = this.ctx.currentTime;
        const beat = 60 / this.tempo;
        const sectionDuration = beat * 8; // 8 beats per section
        const currentSectionStartTime = Math.floor(currentTime / sectionDuration) * sectionDuration;
        const sectionElapsed = currentTime - currentSectionStartTime;
        return Math.min(sectionElapsed / sectionDuration, 1);
    }

    // Get total progress through song (0-1)
    getTotalProgress(): number {
        if (!this.ctx || this.songStructure.length === 0) return 0;

        const currentTime = this.ctx.currentTime;
        const beat = 60 / this.tempo;
        const sectionDuration = beat * 8; // 8 beats per section
        const songStructureArray = this.songStructure.split('');
        const totalSongDuration = sectionDuration * songStructureArray.length;
        const totalElapsed = currentTime % totalSongDuration;
        return totalElapsed / totalSongDuration;
    }

    // Get audio context
    getAudioContext(): AudioContext | null {
        return this.ctx;
    }

    // Get tempo
    getTempo(): number {
        return this.tempo;
    }
}

const musicEngine = new MusicEngine();
export default musicEngine;
export type {
    ScaleName,
    SongStructure,
    GeneratorType,
    GeneratorSettings,
    SectionSettings,
    PresetPattern,
    Pattern,
    BeatEvent,
    BassEvent,
    MelodyEvent,
};
