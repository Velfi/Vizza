declare module '*.svelte' {
    import { SvelteComponentTyped } from 'svelte';
    export default class Component extends SvelteComponentTyped<
        Record<string, unknown>,
        Record<string, unknown>,
        Record<string, unknown>
    > {}
}
