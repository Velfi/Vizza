declare module 'culori' {
  export function interpolate(colors: string[], space?: string): (t: number) => any;
  export function formatHex(color: any): string;
  export function rgb(color: string): any;
} 