/* A stable, dismissible presenter for the game's authored tutorial notes. */
(function (window, factory) {
    "use strict";
    const api = factory();
    if (typeof module === "object" && module.exports) module.exports = api;
    if (window) window.InfestationTips = api;
})(typeof window === "undefined" ? null : window, function () {
    "use strict";

    function create({ root, onToggle = function () {} }) {
        if (!root || typeof root.querySelector !== "function") {
            throw new Error("Touch tips require a root element.");
        }
        function required(selector) {
            const element = root.querySelector(selector);
            if (!element) throw new Error("Touch tips are missing " + selector + ".");
            return element;
        }
        const card = required("[data-tip-card]");
        const hide = required("[data-tip-hide]");
        const show = required("[data-tip-show]");
        const full = required("[data-tip-full]");
        const reveal = required("[data-tip-reveal]");
        const label = required("[data-tip-player]");

        let note = null;
        let points = [];
        let revealedText = "";
        let dismissed = false;
        let suppressed = false;
        let savedScroll = 0;
        let cardVisible = !card.hidden;

        card.setAttribute("role", "note");
        full.setAttribute("aria-hidden", "true");
        reveal.setAttribute("aria-hidden", "true");

        function renderVisibility() {
            const nextVisible = !!note && !dismissed && !suppressed;
            if (cardVisible && !nextVisible) savedScroll = card.scrollTop;
            if (card.hidden === nextVisible) card.hidden = !nextVisible;
            const showVisible = !!note && dismissed && !suppressed;
            if (show.hidden === showVisible) show.hidden = !showVisible;
            // Restore only after both children have their final layout. If the
            // reopen button still occupies space, the browser can clamp this
            // offset against the temporary card size or apply scroll anchoring.
            if (!cardVisible && nextVisible) card.scrollTop = savedScroll;
            cardVisible = nextVisible;
        }

        function clear() {
            if (note) {
                note = null;
                points = [];
                revealedText = "";
                full.textContent = "";
                reveal.textContent = "";
                label.textContent = "";
                card.removeAttribute("aria-label");
            }
            dismissed = false;
            renderVisibility();
            savedScroll = 0;
        }

        function update({ id = null, text, player, shown }) {
            if (text === "") {
                clear();
                return;
            }
            if (typeof text !== "string") throw new TypeError("Tip text must be a string.");
            if (player !== 1 && player !== 2) throw new TypeError("Tip player must be 1 or 2.");
            if (!Number.isFinite(shown)) throw new TypeError("Tip shown must be a finite code-point count.");

            if (!note || note.id !== id || note.text !== text || note.player !== player) {
                note = { id, text, player };
                points = Array.from(text);
                dismissed = false;
                savedScroll = 0;
                full.textContent = text;
                label.textContent = "Player " + player;
                card.setAttribute("aria-label", "Player " + player + ": " + text);
                if (cardVisible) card.scrollTop = 0;
            }

            const nextText = points.slice(0, Math.max(0, Math.floor(shown))).join("");
            if (nextText !== revealedText) {
                reveal.textContent = nextText;
                revealedText = nextText;
            }
            renderVisibility();
        }

        hide.addEventListener("click", function (event) {
            if (!note || dismissed || suppressed) return;
            event.preventDefault();
            dismissed = true;
            renderVisibility();
            onToggle(false);
        });

        show.addEventListener("click", function (event) {
            if (!note || !dismissed || suppressed) return;
            event.preventDefault();
            dismissed = false;
            renderVisibility();
            onToggle(true);
        });

        clear();
        return {
            update,
            clear,
            setSuppressed(value) {
                suppressed = !!value;
                renderVisibility();
            }
        };
    }

    return Object.freeze({ create });
});
