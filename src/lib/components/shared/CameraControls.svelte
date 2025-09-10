<!-- This component doesn't render anything, it just handles keyboard events -->

<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let enabled: boolean = true;
    export let simulationType: string = '';
    export let controlMode: 'camera' | 'simulation' = 'camera';

    const pressedKeys = new Set<string>();
    let animationFrameId: number | null = null;

    function handleKeyDown(event: KeyboardEvent) {
        if (!enabled) return;

        // Check if user is focused on a form element - if so, don't process camera controls
        const activeElement = document.activeElement;
        const isInputFocused =
            activeElement &&
            (activeElement.tagName === 'INPUT' ||
                activeElement.tagName === 'TEXTAREA' ||
                activeElement.tagName === 'SELECT' ||
                (activeElement as HTMLElement).contentEditable === 'true');

        if (isInputFocused) {
            return; // Let the form element handle the keyboard input
        }

        // Fullscreen toggle is now handled globally in App.svelte

        if (event.key === '/') {
            event.preventDefault();
            // Dispatch event for parent to handle GUI toggle
            dispatch('toggleGui');
            return;
        }

        if (event.key === ' ') {
            event.preventDefault();
            // Dispatch event for parent to handle pause toggle
            dispatch('togglePause');
            return;
        }

        // Handle camera controls based on control mode
        const cameraKeys = [
            'w',
            'a',
            's',
            'd',
            'arrowup',
            'arrowdown',
            'arrowleft',
            'arrowright',
            'q',
            'e',
            'c',
        ];
        if (cameraKeys.includes(event.key.toLowerCase()) && !isInputFocused) {
            event.preventDefault();
            pressedKeys.add(event.key.toLowerCase());
            console.log(
                'Key pressed:',
                event.key.toLowerCase(),
                'simulationType:',
                simulationType,
                'controlMode:',
                controlMode
            );
        }
    }

    function handleKeyUp(event: KeyboardEvent) {
        const cameraKeys = [
            'w',
            'a',
            's',
            'd',
            'arrowup',
            'arrowdown',
            'arrowleft',
            'arrowright',
            'q',
            'e',
            'c',
        ];
        if (cameraKeys.includes(event.key.toLowerCase())) {
            pressedKeys.delete(event.key.toLowerCase());
        }
    }

    async function panCamera(deltaX: number, deltaY: number) {
        try {
            await invoke('pan_camera', { deltaX, deltaY });
        } catch (e) {
            console.error('Failed to pan camera:', e);
        }
    }

    async function zoomCamera(delta: number) {
        try {
            await invoke('zoom_camera', { delta });
        } catch (e) {
            console.error('Failed to zoom camera:', e);
        }
    }

    async function resetCamera() {
        try {
            await invoke('reset_camera');
        } catch (e) {
            console.error('Failed to reset camera:', e);
        }
    }

    // Camera update loop for smooth movement
    function updateCamera() {
        if (!enabled) {
            animationFrameId = requestAnimationFrame(updateCamera);
            return;
        }

        let moved = false;

        // Camera controls
        const panAmount = 0.1;
        let deltaX = 0;
        let deltaY = 0;

        if (pressedKeys.has('w') || pressedKeys.has('arrowup')) {
            deltaY += panAmount;
            moved = true;
        }
        if (pressedKeys.has('s') || pressedKeys.has('arrowdown')) {
            deltaY -= panAmount;
            moved = true;
        }
        if (pressedKeys.has('a') || pressedKeys.has('arrowleft')) {
            deltaX -= panAmount;
            moved = true;
        }
        if (pressedKeys.has('d') || pressedKeys.has('arrowright')) {
            deltaX += panAmount;
            moved = true;
        }

        // Apply combined movement if any keys are pressed
        if (moved) {
            panCamera(deltaX, deltaY);
        }

        // Handle Q/E for zoom
        if (pressedKeys.has('q')) {
            zoomCamera(-0.05);
            moved = true;
        }
        if (pressedKeys.has('e')) {
            zoomCamera(0.05);
            moved = true;
        }

        if (pressedKeys.has('c')) {
            resetCamera();
            moved = true;
        }

        // Always schedule the next frame to keep the loop running
        animationFrameId = requestAnimationFrame(updateCamera);
    }

    onMount(() => {
        // Set up keyboard listeners for camera control
        document.addEventListener('keydown', handleKeyDown);
        document.addEventListener('keyup', handleKeyUp);

        // Start camera update loop immediately so camera controls work even when paused
        if (animationFrameId === null) {
            animationFrameId = requestAnimationFrame(updateCamera);
        }
    });

    onDestroy(() => {
        // Remove keyboard event listeners
        document.removeEventListener('keydown', handleKeyDown);
        document.removeEventListener('keyup', handleKeyUp);

        // Stop camera update loop
        if (animationFrameId !== null) {
            cancelAnimationFrame(animationFrameId);
            animationFrameId = null;
        }
    });
</script>
