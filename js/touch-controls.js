/* Touch controls with explicit hero input, plus an optional keyboard adapter. */
(function (window) {
    "use strict";

    const MIN_PRESS_MS = 80;
    const DIRECTIONS = {
        p1: { up: "ArrowUp", down: "ArrowDown", left: "ArrowLeft", right: "ArrowRight" },
        p2: { up: "KeyW", down: "KeyS", left: "KeyA", right: "KeyD" }
    };
    const BUTTON_SELECTOR = "button[data-code], button[data-direction]";
    const PRIORITY_KEYS = new Set(["Space", "KeyU", "KeyR", "Escape", "Enter"]);
    // Shared with game/src/input.rs. Low 15 bits are held input; bits 16..30
    // are fresh presses. The engine consumes one snapshot per frame, keeping
    // touch input separate from physical keys even when both are used together.
    const INPUT_BITS = Object.freeze({
        ArrowUp: 1 << 0, ArrowDown: 1 << 1, ArrowLeft: 1 << 2, ArrowRight: 1 << 3,
        KeyW: 1 << 4, KeyS: 1 << 5, KeyA: 1 << 6, KeyD: 1 << 7,
        ShiftRight: 1 << 8, ShiftLeft: 1 << 9, Space: 1 << 10,
        KeyU: 1 << 11, KeyR: 1 << 12, Escape: 1 << 13, Enter: 1 << 14
    });

    function keyInfo(code) {
        const named = {
            ArrowUp: ["ArrowUp", 38], ArrowDown: ["ArrowDown", 40],
            ArrowLeft: ["ArrowLeft", 37], ArrowRight: ["ArrowRight", 39],
            Space: [" ", 32], Enter: ["Enter", 13], Escape: ["Escape", 27],
            Backspace: ["Backspace", 8], Tab: ["Tab", 9],
            ShiftLeft: ["Shift", 16, 1], ShiftRight: ["Shift", 16, 2]
        };
        if (named[code]) return named[code];
        if (/^Key[A-Z]$/.test(code)) return [code.slice(3).toLowerCase(), code.charCodeAt(3)];
        if (/^Digit[0-9]$/.test(code)) return [code.slice(5), code.charCodeAt(5)];
        throw new Error("Unsupported touch-control key: " + code);
    }

    function movementPlayer(code) {
        for (const player of ["p1", "p2"]) {
            if (Object.values(DIRECTIONS[player]).includes(code)) return player;
        }
        return null;
    }

    function create({ canvas, root, onAction = function () {}, onInterrupt = function () {},
        dispatchKeyboardEvents = true }) {
        if (!canvas || !root) throw new Error("Touch controls require a canvas and a root element.");
        const document = canvas.ownerDocument;
        const pointers = new Map();
        const lanes = new Map();
        const downCounts = new Map();
        const buttonCounts = new Map();
        const listeners = [];
        let player = "p1";
        let sync = false;
        let destroyed = false;
        let pressedBits = 0;

        function ready() {
            return !destroyed && typeof canvas.onkeydown === "function";
        }

        function dispatchKey(type, code) {
            if (!dispatchKeyboardEvents || typeof canvas.onkeydown !== "function") return;
            const [key, keyCode, location = 0] = keyInfo(code);
            const shiftKey = downCounts.has("ShiftLeft") || downCounts.has("ShiftRight");
            const event = new window.KeyboardEvent(type, {
                code, key: shiftKey && /^Key/.test(code) ? key.toUpperCase() : key,
                keyCode, which: keyCode, location, shiftKey,
                bubbles: true, cancelable: true, composed: true, repeat: false
            });
            // Some browsers ignore legacy fields in the KeyboardEvent initializer.
            if (event.keyCode !== keyCode) Object.defineProperty(event, "keyCode", { value: keyCode });
            if (event.which !== keyCode) Object.defineProperty(event, "which", { value: keyCode });
            canvas.dispatchEvent(event);
        }

        function acquireKey(code) {
            const count = downCounts.get(code) || 0;
            downCounts.set(code, count + 1);
            if (count === 0) {
                pressedBits |= INPUT_BITS[code] || 0;
                dispatchKey("keydown", code);
            }
        }

        function releaseKey(code) {
            const count = downCounts.get(code);
            if (count === undefined) return;
            if (count > 1) downCounts.set(code, count - 1);
            else {
                downCounts.delete(code);
                dispatchKey("keyup", code);
            }
        }

        function showPressed(button, pressed) {
            if (!button) return;
            const count = (buttonCounts.get(button) || 0) + (pressed ? 1 : -1);
            if (count > 0) buttonCounts.set(button, count);
            else buttonCounts.delete(button);
            button.classList.toggle("is-pressed", count > 0);
            button.setAttribute("aria-pressed", String(count > 0));
        }

        function finishWhenReleased(lane, cycle) {
            if (lane.current !== cycle || cycle.owners.size || cycle.frames < 2) return;
            const remaining = MIN_PRESS_MS - (window.performance.now() - cycle.startedAt);
            if (remaining > 0) {
                // Clock precision can leave an on-time callback just short of
                // the minimum. Always retry so a released press cannot stick.
                window.clearTimeout(cycle.timer);
                cycle.timer = window.setTimeout(function () {
                    finishWhenReleased(lane, cycle);
                }, Math.max(1, Math.ceil(remaining)));
                return;
            }
            window.clearTimeout(cycle.timer);
            window.cancelAnimationFrame(cycle.frame);
            releaseKey(cycle.code);
            for (const modifier of cycle.modifiers) releaseKey(modifier);
            lane.current = null;
            // Leave a complete animation frame with the key up. This preserves
            // separate quick taps and lets the game's held-input timers reset.
            lane.gap = window.requestAnimationFrame(function () {
                lane.gap = window.requestAnimationFrame(function () {
                    lane.gap = null;
                    startNext(lane);
                });
            });
        }

        function startNext(lane) {
            if (lane.current || lane.gap !== null || !lane.queue.length) return;
            const cycle = lane.queue.shift();
            lane.current = cycle;
            cycle.startedAt = window.performance.now();
            for (const modifier of cycle.modifiers) acquireKey(modifier);
            acquireKey(cycle.code);
            cycle.timer = window.setTimeout(function () {
                finishWhenReleased(lane, cycle);
            }, MIN_PRESS_MS);
            function countFrame() {
                cycle.frames += 1;
                if (cycle.frames < 2) cycle.frame = window.requestAnimationFrame(countFrame);
                else finishWhenReleased(lane, cycle);
            }
            cycle.frame = window.requestAnimationFrame(countFrame);
        }

        function press(code, button, source) {
            if (!ready()) return null;
            keyInfo(code);
            // Wait, undo and scene changes take precedence over movement.
            // Keep this action's own lane so two fingers, or a quick re-tap,
            // retain the same ownership and minimum-press guarantees.
            if (PRIORITY_KEYS.has(code)) releaseOtherControls(code);
            const movingPlayer = movementPlayer(code);
            // Directions share one lane per player: sliding or pressing two
            // arrows never sends a diagonal chord to the game.
            const laneName = movingPlayer || code;
            if (!lanes.has(laneName)) lanes.set(laneName, { current: null, queue: [], gap: null });
            const lane = lanes.get(laneName);
            let cycle = lane.queue.length ? lane.queue[lane.queue.length - 1] : lane.current;
            if (!cycle || cycle.code !== code || !cycle.owners.size) {
                cycle = { code, owners: new Set(), modifiers: new Set(), frames: 0,
                    startedAt: 0, timer: null, frame: null };
                lane.queue.push(cycle);
            }
            const action = { lane, cycle, button, released: false };
            cycle.owners.add(action);
            if (sync && movingPlayer) {
                const modifier = movingPlayer === "p1" ? "ShiftRight" : "ShiftLeft";
                if (!cycle.modifiers.has(modifier)) {
                    cycle.modifiers.add(modifier);
                    if (lane.current === cycle) acquireKey(modifier);
                }
            }
            showPressed(button, true);
            canvas.focus({ preventScroll: true });
            startNext(lane);
            onAction({ code, player: movingPlayer || player, sync: sync && !!movingPlayer, source });
            return action;
        }

        function release(action) {
            if (!action || action.released) return;
            action.released = true;
            action.cycle.owners.delete(action);
            showPressed(action.button, false);
            finishWhenReleased(action.lane, action.cycle);
        }

        function findButton(target, directionsOnly = false) {
            const button = target && target.closest && target.closest(
                directionsOnly ? "button[data-direction]" : BUTTON_SELECTOR
            );
            return button && root.contains(button) && !button.disabled &&
                button.getAttribute("aria-disabled") !== "true" ? button : null;
        }

        function codeFor(button) {
            return button.dataset.direction ? DIRECTIONS[player][button.dataset.direction] : button.dataset.code;
        }

        function pointerDown(event) {
            const button = findButton(event.target);
            if (!button || (event.pointerType === "mouse" && event.button !== 0)) return;
            event.preventDefault();
            if (pointers.has(event.pointerId)) return;
            const action = press(codeFor(button), button, "pointer");
            if (!action) return;
            pointers.set(event.pointerId, {
                capture: button, button, action, directional: !!button.dataset.direction
            });
            button.setPointerCapture(event.pointerId);
        }

        function pointerMove(event) {
            const pointer = pointers.get(event.pointerId);
            if (!pointer || !pointer.directional) return;
            event.preventDefault();
            const button = findButton(document.elementFromPoint(event.clientX, event.clientY), true);
            if (button === pointer.button) return;
            release(pointer.action);
            pointer.button = button;
            pointer.action = button ? press(codeFor(button), button, "pointer") : null;
        }

        function pointerUp(event) {
            const pointer = pointers.get(event.pointerId);
            if (!pointer) return;
            event.preventDefault();
            // Remove first: the normal lostpointercapture following pointerup
            // must not cancel another finger's held button.
            pointers.delete(event.pointerId);
            release(pointer.action);
        }

        function pointerInterrupted(event) {
            if (pointers.has(event.pointerId)) interrupt();
        }

        function keyboardClick(event) {
            const button = findButton(event.target);
            if (!button || event.detail !== 0) return;
            event.preventDefault();
            const action = press(codeFor(button), button, "keyboard");
            release(action);
        }

        function releaseOtherControls(keepCode) {
            // A cancelled press must not linger until the engine's next poll.
            pressedBits &= keepCode ? INPUT_BITS[keepCode] || 0 : 0;
            const captured = [];
            const keptButtons = new Set();
            for (const [id, pointer] of pointers) {
                if (keepCode && pointer.action && pointer.action.cycle.code === keepCode) {
                    keptButtons.add(pointer.action.button);
                } else {
                    captured.push([id, pointer]);
                    pointers.delete(id);
                }
            }
            for (const [name, lane] of lanes) {
                if (name === keepCode) continue;
                window.cancelAnimationFrame(lane.gap);
                if (lane.current) {
                    window.clearTimeout(lane.current.timer);
                    window.cancelAnimationFrame(lane.current.frame);
                }
                lane.current = null;
                lane.queue.length = 0;
                lanes.delete(name);
            }
            // Release ordinary keys before their modifiers.
            const codes = Array.from(downCounts.keys()).sort(function (a, b) {
                return Number(a.startsWith("Shift")) - Number(b.startsWith("Shift"));
            });
            for (const code of codes) {
                if (code === keepCode) continue;
                downCounts.delete(code);
                dispatchKey("keyup", code);
            }
            for (const button of buttonCounts.keys()) {
                if (keptButtons.has(button)) continue;
                button.classList.remove("is-pressed");
                button.setAttribute("aria-pressed", "false");
                buttonCounts.delete(button);
            }
            for (const [id, pointer] of captured) {
                if (pointer.capture.hasPointerCapture(id)) pointer.capture.releasePointerCapture(id);
            }
        }

        function releaseAll() {
            releaseOtherControls(null);
        }

        function interrupt() {
            releaseAll();
            onInterrupt();
        }

        function releasePointerHolds() {
            // Changing hero/mode should stop an old held direction, but a very
            // quick first hero tap still needs to reach two game frames before
            // its key is released. Already-released taps keep that guarantee.
            for (const [id, pointer] of pointers) {
                pointers.delete(id);
                release(pointer.action);
                if (pointer.capture.hasPointerCapture(id)) pointer.capture.releasePointerCapture(id);
            }
        }

        function listen(target, name, handler) {
            target.addEventListener(name, handler, { passive: false });
            listeners.push([target, name, handler]);
        }

        listen(root, "pointerdown", pointerDown);
        listen(root, "click", keyboardClick);
        listen(window, "pointermove", pointerMove);
        listen(window, "pointerup", pointerUp);
        listen(window, "pointercancel", pointerInterrupted);
        listen(window, "lostpointercapture", pointerInterrupted);
        listen(window, "blur", interrupt);
        listen(window, "pagehide", interrupt);
        listen(document, "visibilitychange", function () {
            if (document.visibilityState === "hidden") interrupt();
        });

        return {
            inputBits() {
                let heldBits = 0;
                for (const code of downCounts.keys()) heldBits |= INPUT_BITS[code] || 0;
                const snapshot = (heldBits | (pressedBits << 16)) >>> 0;
                pressedBits = 0;
                return snapshot;
            },
            setPlayer(value) {
                if (!DIRECTIONS[value]) throw new Error("Unknown player: " + value);
                if (player !== value) releasePointerHolds();
                player = value;
            },
            setSync(value) {
                value = !!value;
                if (sync !== value) releasePointerHolds();
                sync = value;
            },
            releaseAll,
            tap(code) { release(press(code, null, "api")); },
            destroy() {
                releaseAll();
                destroyed = true;
                for (const [target, name, handler] of listeners) target.removeEventListener(name, handler);
                listeners.length = 0;
            }
        };
    }

    window.InfestationTouch = Object.freeze({ create });
})(window);
