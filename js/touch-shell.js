/* Touch page and explicit visibility bridge to the game's adaptive HUD. */
(() => {
  "use strict";
  const canvas = document.getElementById("glcanvas");
  const app = document.getElementById("app");
  const loading = document.getElementById("loading");
  const help = document.getElementById("help");
  const toast = document.getElementById("toast");
  const menuButton = document.querySelector('.topbar [data-code="Escape"]');
  const tipRoot = document.getElementById("level-tip");
  const inputMode = InfestationInputMode.create({
    touchDefault: InfestationInputMode.prefersTouch({
      coarsePointer: window.matchMedia("(pointer: coarse)").matches,
      finePointer: window.matchMedia("(any-pointer: fine)").matches,
      maxTouchPoints: navigator.maxTouchPoints || 0
    }),
    onChange() {
      // Release dock holds before physical input takes over.
      // Prompt changes alone must not reset a fresh key/stick action, discard
      // synchronized moves, or resize the puzzle.
      if (presentedMode === "touch" && inputMode.mode() !== "touch") controls.releaseAll();
      syncInputPresentation();
      syncPresentation();
    }
  });
  let controlsVisible = inputMode.controlsVisible();
  let presentedMode = inputMode.mode();
  let toastTimer;
  let ready = false;
  let loadFailed = false;
  let initialFocusPending = true;
  let engineTipVisible = false;
  let wasInputBlocked = true;
  const heldGameKeys = new Set();
  const consumedKeys = new Set();
  let previousHelpButtons = new Map();

  function showToast(message, duration = 2600) {
    clearTimeout(toastTimer);
    toast.textContent = message;
    toast.hidden = false;
    toastTimer = setTimeout(() => { toast.hidden = true; }, duration);
  }
  function resumeAudio() {
    // Called by real gestures; synthetic key events alone do not activate audio.
    if (typeof audio_context !== "undefined" && audio_context?.state === "suspended") {
      audio_context.resume().catch(error => {
        console.warn("Audio could not resume:", error);
        showToast("Sound is paused. Tap the game to enable it.");
      });
    }
  }
  const controls = InfestationTouch.create({
    canvas,
    root: document.body,
    dispatchKeyboardEvents: false,
    onInterrupt() { dock.interrupt(); },
    onAction(action) {
      resumeAudio();
      syncTipVisibility();
      if (action.code === "KeyR") {
        showToast("Restart uses two taps within 2 seconds to confirm.", 2000);
      }
    }
  });
  const dock = InfestationDock.create({
    root: document.getElementById("controller"),
    controls,
    onPlayerChange() {
      resumeAudio();
      focusGame();
    },
    onSyncChange(enabled) {
      resumeAudio();
      if (enabled) showToast("Choose a direction for each hero to move together.", 3600);
      focusGame();
    }
  });
  const tips = InfestationTips.create({
    root: tipRoot,
    onToggle(open) {
      dock.interrupt();
      if (open) tipRoot.querySelector("[data-tip-card]").focus({ preventScroll: true });
      else focusGame();
    }
  });
  // Reading or scrolling a tip must not leave a held direction running.
  tipRoot.addEventListener("pointerdown", () => dock.interrupt());
  function syncTipVisibility() {
    tips.setSuppressed(!engineTipVisible || inputBlocked() || !controlsVisible);
  }
  function inputBlocked() {
    return !ready || loadFailed || help.open
      || document.visibilityState === "hidden";
  }
  function releasePhysicalKeys() {
    // Keyup follows focus. A canvas key released over a dialog or another
    // button must not stay down in miniquad. Ignore its later repeats until
    // release (or a new physical press if the browser lost that release).
    for (const code of heldGameKeys) {
      consumedKeys.add(code);
      canvas.dispatchEvent(new window.KeyboardEvent("keyup", { code }));
    }
    heldGameKeys.clear();
  }
  function syncPresentation() {
    loading.hidden = ready && !loadFailed;
    const blocked = inputBlocked();
    if (blocked !== wasInputBlocked) {
      wasInputBlocked = blocked;
      releasePhysicalKeys();
      // Shell overlays own their input boundary even when the dock is hidden.
      dock.interrupt();
    }
    const interactive = !blocked;
    dock.setAvailable(interactive && controlsVisible);
    menuButton.disabled = !interactive;
    syncTipVisibility();
  }
  function syncInputPresentation() {
    controlsVisible = inputMode.controlsVisible();
    const mode = inputMode.mode();
    presentedMode = mode;
    document.body.dataset.inputMode = mode;
    app.classList.toggle("controls-hidden", !controlsVisible);
    canvas.setAttribute("aria-label", mode === "touch"
      ? "Game board. Use the direction buttons to move, Wait to pass a turn, and Undo to take back a move."
      : mode === "controller" ? "Game board. Use your controller. Matching button prompts appear in the game."
        : "Game board. Use the arrow keys or WASD to move, Space to wait, and U to undo.");
  }
  // Real input chooses instructions; dock input has its own engine channel.
  document.addEventListener("pointerdown", event => {
    if (event.pointerType === "mouse" && event.target.closest?.(".topbar")) return;
    inputMode.pointer(event, Boolean(event.target.closest?.("#controller")));
  }, { capture: true, passive: true });
  function focusGame() { canvas.focus({ preventScroll: true }); }
  function restoreFocus() {
    if (!inputBlocked()) {
      focusGame();
      initialFocusPending = false;
    }
  }
  function closeHelp() { help.close(); syncPresentation(); restoreFocus(); }
  function consumeKey(event) {
    event.preventDefault();
    event.stopImmediatePropagation();
  }
  document.addEventListener("keydown", event => {
    inputMode.keyboard(event);
    if (!event.isTrusted) return;
    if (consumedKeys.has(event.code)) {
      if (event.repeat) { consumeKey(event); return; }
      consumedKeys.delete(event.code);
    }
    const confirm = event.code === "Enter" || event.code === "Space";
    if (help.open) {
      if (event.code === "Escape" || (confirm
        && (event.target.id === "close-help" || event.target.id === "help-done"))) {
        consumedKeys.add(event.code);
        consumeKey(event);
        closeHelp();
      }
      return;
    }
    if (inputBlocked()) {
      if (event.target === canvas) {
        // Let Tab leave the canvas while loading; its own handler traps Tab.
        event.stopImmediatePropagation();
        if (event.code !== "Tab") {
          consumedKeys.add(event.code);
          event.preventDefault();
        }
      }
      return;
    }
    if (event.target === canvas) {
      resumeAudio();
      heldGameKeys.add(event.code);
    }
  }, { capture: true });
  document.addEventListener("keyup", event => {
    if (!event.isTrusted) return;
    if (consumedKeys.delete(event.code)) {
      consumeKey(event);
      return;
    }
    if (heldGameKeys.delete(event.code) && event.target !== canvas) {
      canvas.dispatchEvent(new window.KeyboardEvent("keyup", { code: event.code }));
    }
  }, { capture: true });
  canvas.addEventListener("blur", releasePhysicalKeys);
  window.addEventListener("blur", releasePhysicalKeys);
  function pollHelpButtons(pads) {
    const next = new Map();
    let confirmed = false;
    for (let index = 0; index < Math.min(pads.length, 2); index += 1) {
      if (!pads[index]?.connected) continue;
      const buttons = [0, 1, 9].map(button => Boolean(pads[index].buttons?.[button]?.pressed));
      const previous = previousHelpButtons.get(index);
      if (buttons.some((pressed, button) => pressed && !previous?.[button])) confirmed = true;
      next.set(index, buttons);
    }
    previousHelpButtons = next;
    if (!confirmed || !ready || loadFailed || document.visibilityState === "hidden") return;
    // The engine samples the blocking flag before and after this gamepad poll,
    // consuming the activation frame and suppressing the hold until release.
    if (help.open) closeHelp();
  }
  document.getElementById("show-help").addEventListener("click", () => {
    help.showModal();
    syncPresentation();
  });
  document.getElementById("close-help").addEventListener("click", closeHelp);
  document.getElementById("help-done").addEventListener("click", closeHelp);
  help.addEventListener("cancel", event => { event.preventDefault(); closeHelp(); });
  help.addEventListener("close", () => { syncPresentation(); restoreFocus(); });
  document.addEventListener("visibilitychange", () => {
    syncPresentation();
    if (initialFocusPending) restoreFocus();
  });
  const fullscreen = document.getElementById("fullscreen");
  if (!document.documentElement.requestFullscreen) fullscreen.hidden = true;
  fullscreen.addEventListener("click", async () => {
    dock.interrupt();
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
      focusGame();
    } catch (error) {
      console.warn("Fullscreen was unavailable:", error);
      showToast("Fullscreen isn’t available in this browser.");
    }
  });
  document.addEventListener("fullscreenchange", () => {
    fullscreen.setAttribute("aria-label", document.fullscreenElement ? "Exit fullscreen" : "Enter fullscreen");
  });
  canvas.addEventListener("pointerdown", event => {
    focusGame();
    resumeAudio();
    // Mouse/pen releases outside the canvas must still finish a slider drag.
    // Touch events already remain associated with their original target.
    if (event.isPrimary && event.button === 0
      && (event.pointerType === "mouse" || event.pointerType === "pen")) {
      canvas.setPointerCapture(event.pointerId);
    }
  });
  // These gestures belong to the canvas rather than a dock button. Notify the
  // engine too, so an interrupted slider drag cannot resume with a later touch.
  for (const event of ["pointercancel", "touchcancel"]) {
    canvas.addEventListener(event, () => dock.interrupt());
  }
  new ResizeObserver(() => {
    dock.interrupt();
    window.dispatchEvent(new Event("resize"));
  }).observe(canvas.parentElement);
  function showLoadFailure(message) {
    loadFailed = true;
    engineTipVisible = false;
    tips.clear();
    document.getElementById("loading-label").textContent = ready
      ? "Game interrupted" : "Game couldn’t start";
    document.getElementById("load-note").textContent = message;
    syncPresentation();
  }
  canvas.addEventListener("webglcontextlost", () => {
    dock.interrupt();
    showLoadFailure("The graphics context was lost. Reload the page to continue.");
  });
  canvas.addEventListener("webglcontextcreationerror", event => {
    console.warn("WebGL initialization failed:", event.statusMessage);
    showLoadFailure("WebGL isn’t available in this browser. Open the game in a browser with hardware graphics enabled.");
  });
  window.addEventListener("error", event => {
    console.error("Infestation error:", event.message);
    if (!ready) showLoadFailure("The game couldn’t initialize. Try reloading in Chrome or Safari.");
  });
  // First poll happens once App has loaded sprites/audio and starts running.
  miniquad_add_plugin({
    name: "infestation_touch_page",
    version: 1,
    register_plugin(importObject) {
      importObject.env.infestation_input_mode = () => inputMode.code();
      importObject.env.infestation_input_blocked = () => Number(inputBlocked());
      importObject.env.infestation_touch_input = () => controls.inputBits();
      // The game reclaims its generic instruction panel only while a usable
      // replacement is visible. Hide/show and readiness take effect next frame.
      importObject.env.infestation_touch_controls_visible = () =>
        Number(controlsVisible && ready && !loadFailed);
      importObject.env.infestation_touch_state = (roomPtr, roomLen, screen, players, actions) => {
        dock.update({ room: UTF8ToString(roomPtr, roomLen), screen, players, actions });
      };
      importObject.env.infestation_touch_layout = dpadRight => {
        dock.setDpadRight(Boolean(dpadRight));
      };
      importObject.env.infestation_touch_visibility = preference => {
        inputMode.setControlsPreference(preference < 0 ? null : Boolean(preference));
      };
      importObject.env.infestation_touch_selected_player = () => dock.selectedPlayer();
      importObject.env.infestation_touch_input_epoch = () => dock.inputEpoch();
      importObject.env.infestation_touch_tip = (roomPtr, roomLen, textPtr, textLen, player, shown, visible) => {
        engineTipVisible = Boolean(visible);
        syncTipVisibility();
        tips.update({
          id: UTF8ToString(roomPtr, roomLen),
          text: UTF8ToString(textPtr, textLen),
          player,
          shown
        });
      };
      const count = importObject.env.sapp_gamepad_count;
      importObject.env.sapp_gamepad_count = function () {
        const connected = count();
        const pads = navigator.getGamepads?.() || [];
        inputMode.gamepads(pads);
        // Record buttons held during loading without letting them close Help.
        pollHelpButtons(pads);
        if (!ready) {
          ready = true;
          syncInputPresentation();
          syncPresentation();
          restoreFocus();
        }
        return connected;
      };
    }
  });
  setTimeout(() => {
    if (!ready && !loadFailed) {
      document.getElementById("load-note").textContent =
        "Loading is taking longer than expected. You can keep waiting or reload.";
    }
  }, 45000);
  syncInputPresentation();
  syncPresentation();
  load(document.body.dataset.wasmPath);
})();
