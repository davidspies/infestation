/* Input prompts follow the device being used, not synthetic dock key events. */
(function (window, factory) {
    "use strict";
    const api = factory();
    if (typeof module === "object" && module.exports) module.exports = api;
    if (window) window.InfestationInputMode = api;
})(typeof window === "undefined" ? null : window, function () {
    "use strict";

    // Stable values consumed by game/src/touch_ui.rs.
    const MODE = Object.freeze({ keyboard: 0, touch: 1, controller: 2 });
    const STICK_THRESHOLD = 0.5;

    function prefersTouch({ coarsePointer, finePointer, maxTouchPoints }) {
        return Boolean(coarsePointer || (!finePointer && maxTouchPoints > 0));
    }

    function stickDirection(axes) {
        const x = Number(axes?.[0]) || 0;
        const y = Number(axes?.[1]) || 0;
        if (Math.max(Math.abs(x), Math.abs(y)) <= STICK_THRESHOLD) return null;
        if (Math.abs(x) >= Math.abs(y)) return x > 0 ? "right" : "left";
        return y > 0 ? "down" : "up";
    }

    function create({ touchDefault = false, onChange = function () {} } = {}) {
        let mode = touchDefault ? "touch" : "keyboard";
        let visible = touchDefault;
        let controlsPreference = null;
        let previousPads = new Map();

        function controlsVisible() {
            return visible;
        }

        function notify() {
            onChange({ mode, controlsVisible: controlsVisible() });
        }

        function use(next) {
            if (mode === next) return;
            mode = next;
            // Prompts can change without moving or resizing the puzzle. Only
            // the visibility setting changes the layout after startup.
            notify();
        }

        function pointer(event, onControls = false) {
            if (!event.isTrusted) return;
            if (event.pointerType === "touch" || event.pointerType === "pen") {
                use("touch");
            } else if (event.pointerType === "mouse" && event.button === 0
                && !event.sourceCapabilities?.firesTouchEvents) {
                // Clicking the on-screen pad is still using the touch UI.
                use(onControls ? "touch" : "keyboard");
            }
        }

        function keyboard(event) {
            // Dock buttons synthesize key events for the original engine. A
            // carried-over hardware repeat must not steal a new touch gesture.
            if (event.isTrusted && !event.repeat) use("keyboard");
        }

        function gamepads(pads) {
            const nextPads = new Map();
            let used = false;
            // The engine supports the first two controller slots.
            for (let index = 0; index < Math.min(pads.length, 2); index += 1) {
                const pad = pads[index];
                if (!pad?.connected) continue;
                const buttons = (pad.buttons || []).map(button => Boolean(button.pressed));
                const direction = stickDirection(pad.axes);
                const previous = previousPads.get(index);
                // A fresh press/deflection switches prompts. Connection alone,
                // stick drift, and a control already held during touch do not.
                if (buttons.some((pressed, button) => pressed && !previous?.buttons[button])
                    || (direction !== null && direction !== previous?.direction)) used = true;
                nextPads.set(index, { buttons, direction });
            }
            previousPads = nextPads;
            if (used) use("controller");
            else if (mode === "controller" && nextPads.size === 0) {
                use(touchDefault ? "touch" : "keyboard");
            }
        }

        function setControlsPreference(preference) {
            preference = preference == null ? null : Boolean(preference);
            // The engine publishes saved settings every frame. Applying the
            // same preference must not override a newer keyboard/pad gesture.
            if (preference === controlsPreference) return;
            controlsPreference = preference;
            visible = preference ?? touchDefault;
            // Choosing to show the pad opts into its matching instructions,
            // even if the device default had already made it visible.
            if (visible) mode = "touch";
            notify();
        }

        return {
            pointer,
            keyboard,
            gamepads,
            setControlsPreference,
            mode: () => mode,
            code: () => MODE[mode],
            controlsVisible
        };
    }

    return Object.freeze({ create, prefersTouch, MODE });
});
