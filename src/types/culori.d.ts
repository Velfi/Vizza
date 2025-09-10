declare module 'culori' {
    export function interpolate(colors: string[], space?: string): (t: number) => string;
    export function formatHex(color: unknown): string;
    export function rgb(color: string): { r: number; g: number; b: number; alpha?: number };
}
