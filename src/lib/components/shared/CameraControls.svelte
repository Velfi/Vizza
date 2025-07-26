<!-- This component doesn't render anything, it just handles keyboard events -->

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher } from 'svelte';

  async function toggleFullscreen() {
    try {
      await invoke('toggle_fullscreen');
    } catch (error) {
      console.error('Failed to toggle fullscreen:', error);
    }
  }

  const dispatch = createEventDispatcher();

  export let enabled: boolean = true;
  export let simulationType: string = '';
  export let controlMode: 'camera' | 'simulation' = 'camera';

  // Debug logging for control mode changes
  $: {
    console.log(
      'CameraControls: controlMode changed to:',
      controlMode,
      'simulationType:',
      simulationType
    );
  }

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

    // Check for fullscreen toggle shortcuts
    // Windows/Linux: Alt+Enter, macOS: Cmd+F
    if ((event.key === 'Enter' && event.altKey) || (event.key === 'f' && event.metaKey)) {
      event.preventDefault();
      // Handle fullscreen toggle directly
      toggleFullscreen();
      return;
    }

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

    // Debug: log pressed keys occasionally
    if (pressedKeys.size > 0 && Math.random() < 0.01) {
      // Log ~1% of the time when keys are pressed
      console.log(
        'Pressed keys:',
        Array.from(pressedKeys),
        'simulationType:',
        simulationType,
        'controlMode:',
        controlMode
      );
    }

    let moved = false;

    if (controlMode === 'camera') {
      // Regular camera controls
      const panAmount = 0.1;
      let deltaX = 0;
      let deltaY = 0;

      if (pressedKeys.has('w') || pressedKeys.has('arrowup')) {
        console.log('CAMERA MODE: W/Up pressed - panning camera up');
        deltaY += panAmount;
        moved = true;
      }
      if (pressedKeys.has('s') || pressedKeys.has('arrowdown')) {
        console.log('CAMERA MODE: S/Down pressed - panning camera down');
        deltaY -= panAmount;
        moved = true;
      }
      if (pressedKeys.has('a') || pressedKeys.has('arrowleft')) {
        console.log('CAMERA MODE: A/Left pressed - panning camera left');
        deltaX -= panAmount;
        moved = true;
      }
      if (pressedKeys.has('d') || pressedKeys.has('arrowright')) {
        console.log('CAMERA MODE: D/Right pressed - panning camera right');
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
    } else {
      // Simulation controls - let the backend handle mode switching
      const panAmount = 0.1;
      let deltaX = 0;
      let deltaY = 0;

      if (pressedKeys.has('w') || pressedKeys.has('arrowup')) {
        console.log('SIM MODE: W/Up pressed - sending to backend');
        deltaY += panAmount;
        moved = true;
      }
      if (pressedKeys.has('s') || pressedKeys.has('arrowdown')) {
        console.log('SIM MODE: S/Down pressed - sending to backend');
        deltaY -= panAmount;
        moved = true;
      }
      if (pressedKeys.has('a') || pressedKeys.has('arrowleft')) {
        console.log('SIM MODE: A/Left pressed - sending to backend');
        deltaX -= panAmount;
        moved = true;
      }
      if (pressedKeys.has('d') || pressedKeys.has('arrowright')) {
        console.log('SIM MODE: D/Right pressed - sending to backend');
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
