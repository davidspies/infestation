/* The touch dock follows explicit game state, not the contents of the canvas. */
(function (window, factory) {
    "use strict";
    const api = factory();
    if (typeof module === "object" && module.exports) module.exports = api;
    if (window) window.InfestationDock = api;
})(typeof window === "undefined" ? null : window, function () {
    "use strict";

    // Keep these values aligned with game/src/touch_ui.rs.
    const SCREEN = Object.freeze({ loading: 0, title: 1, map: 2, playing: 3,
        won: 4, lost: 5, menu: 6, code: 7, transition: 8 });
    const ACTION = Object.freeze({ direction: 1, wait: 2, undo: 4, restart: 8, confirm: 16 });
    const CONTEXT = {
        [SCREEN.title]: ["Play", "Begin exploring the keep."],
        [SCREEN.map]: ["Enter room", "Choose a room on the map."],
        [SCREEN.code]: ["Close", "Return to the game."]
    };

    function create({ root, controls, onPlayerChange = function () {}, onSyncChange = function () {} }) {
        function required(selector) {
            const element = root.querySelector(selector);
            if (!element) throw new Error("Touch dock is missing " + selector + ".");
            return element;
        }
        const heroes = required("#controller-meta");
        const players = Array.from(root.querySelectorAll("[data-player]"));
        const directions = Array.from(root.querySelectorAll("[data-direction]"));
        const playActions = required("#play-actions");
        const contextActions = required("#context-actions");
        const confirm = required("#context-confirm");
        const contextLabel = required("#context-label");
        const contextHint = required("#context-hint");
        const syncButton = required("#sync");
        const syncLabel = required("#sync-label");
        const wait = required('[data-code="Space"]');
        const undo = required('[data-code="KeyU"]');
        const restart = required('[data-code="KeyR"]');
        const note = root.querySelector("#dock-note");
        let state = { room: "", screen: SCREEN.loading, players: 0, actions: 0 };
        let available = false;
        let player = "p1";
        let sync = false;
        let inputEpoch = 0;
        let dpadRight = false;

        function interrupt() {
            // The engine also clears pending synchronized moves on a new epoch.
            // Releasing browser keys alone cannot cancel those buffered moves.
            controls.releaseAll();
            inputEpoch = (inputEpoch + 1) >>> 0;
        }

        function render() {
            const playing = state.screen === SCREEN.playing;
            const twoHeroes = state.players === 2;
            const context = CONTEXT[state.screen];
            const enabled = flag => available && Boolean(state.actions & flag);
            // A room's INITIAL player count owns this row's geometry. Outcomes,
            // menus and temporary disabled actions must not collapse it.
            heroes.hidden = !twoHeroes;
            playActions.hidden = Boolean(context) || state.screen === SCREEN.menu;
            contextActions.hidden = !context;
            for (const button of directions) button.disabled = !enabled(ACTION.direction);
            wait.disabled = !playing || !enabled(ACTION.wait);
            undo.disabled = !enabled(ACTION.undo);
            restart.disabled = !enabled(ACTION.restart);
            confirm.disabled = !context || !enabled(ACTION.confirm);
            if (context) {
                contextLabel.textContent = context[0];
                contextHint.textContent = context[1];
                confirm.setAttribute("aria-label", context[0]);
                // Enter copies a code before closing its dialog. The explicit
                // Close action uses Escape; the game's Copy button stays usable.
                confirm.dataset.code = state.screen === SCREEN.code ? "Escape" : "Enter";
            }
            for (const button of players) {
                const selected = button.dataset.player === player;
                button.classList.toggle("selected", selected);
                button.setAttribute("aria-pressed", String(selected));
                button.disabled = !available || !playing || !twoHeroes;
            }
            syncButton.disabled = !available || !playing || !twoHeroes;
            syncButton.setAttribute("aria-pressed", String(sync));
            syncLabel.textContent = "Move together";
            if (note) note.textContent = sync ? "Choose a direction for each hero."
                : "Tap to move. Think as long as you like.";
            // Menus always use the ordinary arrow keys, irrespective of which
            // hero was selected before pausing. The selection returns on resume.
            controls.setPlayer(playing && twoHeroes ? player : "p1");
            controls.setSync(playing && twoHeroes && sync);
        }

        function update(next) {
            const normalized = {
                room: String(next.room || ""),
                screen: Number(next.screen),
                players: Number(next.players),
                actions: Number(next.actions) >>> 0
            };
            if (Object.keys(normalized).every(key => normalized[key] === state[key])) return;
            const changedRoom = normalized.room !== state.room || normalized.players !== state.players;
            if (changedRoom || normalized.screen !== state.screen) interrupt();
            if (changedRoom || normalized.players !== 2) {
                player = "p1";
                sync = false;
            }
            state = normalized;
            render();
        }

        for (const button of players) {
            button.addEventListener("click", () => {
                if (button.disabled || button.dataset.player === player) return;
                player = button.dataset.player;
                // setPlayer releases held pointer ownership without throwing
                // away the minimum press needed for a quick synchronized tap.
                // Do not change epoch: the first hero's queued move must survive.
                render();
                onPlayerChange(player);
            });
        }
        syncButton.addEventListener("click", () => {
            if (syncButton.disabled) return;
            sync = !sync;
            if (!sync) interrupt();
            render();
            onSyncChange(sync);
        });

        render();
        return {
            update,
            interrupt,
            setDpadRight(value) {
                value = Boolean(value);
                if (dpadRight === value) return;
                // Stop a held gesture before its button moves to the other side.
                interrupt();
                dpadRight = value;
                root.classList.toggle("dpad-right", value);
            },
            setAvailable(value) {
                value = Boolean(value);
                if (available === value) return;
                available = value;
                interrupt();
                render();
            },
            selectedPlayer() {
                return available && state.screen === SCREEN.playing && state.players === 2
                    ? (player === "p2" ? 2 : 1) : 0;
            },
            inputEpoch() { return inputEpoch; }
        };
    }

    return Object.freeze({ create, SCREEN, ACTION });
});
