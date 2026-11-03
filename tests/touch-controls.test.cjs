"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const source = fs.readFileSync(path.join(__dirname, "../js/touch-controls.js"), "utf8");

class Clock {
    now = 0;
    next = 1;
    tasks = new Map();
    schedule(fn, at) {
        const id = this.next++;
        this.tasks.set(id, { fn, at });
        return id;
    }
    advance(ms) {
        const until = this.now + ms;
        for (;;) {
            const pending = [...this.tasks.entries()].filter(([, task]) => task.at <= until)
                .sort((a, b) => a[1].at - b[1].at || a[0] - b[0]);
            if (!pending.length) break;
            const [id, task] = pending[0];
            this.now = task.at;
            this.tasks.delete(id);
            task.fn(this.now);
        }
        this.now = until;
    }
}

class Target {
    listeners = new Map();
    addEventListener(type, fn) {
        if (!this.listeners.has(type)) this.listeners.set(type, new Set());
        this.listeners.get(type).add(fn);
    }
    removeEventListener(type, fn) { this.listeners.get(type)?.delete(fn); }
    emit(type, fields = {}) {
        const event = { type, target: this, defaultPrevented: false,
            preventDefault() { this.defaultPrevented = true; }, ...fields };
        for (const fn of this.listeners.get(type) || []) fn(event);
        return event;
    }
}

function setup({ ready = true, dispatchKeyboardEvents = true } = {}) {
    const clock = new Clock();
    const window = new Target();
    const document = new Target();
    const root = new Target();
    const events = [];
    const actions = [];
    const interruptions = [];
    const buttons = [];
    document.visibilityState = "visible";
    document.elementFromPoint = () => document.pointed;
    root.contains = button => buttons.includes(button);
    window.performance = { now: () => clock.now };
    window.setTimeout = (fn, ms) => clock.schedule(fn, clock.now + ms);
    window.clearTimeout = id => clock.tasks.delete(id);
    window.requestAnimationFrame = fn => clock.schedule(fn, (Math.floor(clock.now / 16) + 1) * 16);
    window.cancelAnimationFrame = id => clock.tasks.delete(id);
    window.KeyboardEvent = class {
        constructor(type, props) { this.type = type; Object.assign(this, props); }
    };
    const canvas = {
        ownerDocument: document,
        onkeydown: ready ? function () {} : null,
        focus() { this.focused = true; },
        dispatchEvent(event) { events.push({ ...event, time: clock.now }); }
    };
    vm.runInNewContext(source, { window });
    const api = window.InfestationTouch.create({ canvas, root, dispatchKeyboardEvents,
        onAction: action => actions.push(action), onInterrupt: () => interruptions.push(clock.now) });

    function button(dataset) {
        const attributes = new Map();
        const classes = new Set();
        const captures = new Set();
        const element = {
            dataset, disabled: false,
            classList: {
                toggle(name, value) { value ? classes.add(name) : classes.delete(name); },
                remove(name) { classes.delete(name); },
                contains(name) { return classes.has(name); }
            },
            setAttribute(name, value) { attributes.set(name, value); },
            getAttribute(name) { return attributes.get(name) ?? null; },
            closest(selector) {
                if (selector === "button[data-direction]" && !dataset.direction) return null;
                return dataset.direction || dataset.code ? this : null;
            },
            setPointerCapture(id) { captures.add(id); },
            hasPointerCapture(id) { return captures.has(id); },
            releasePointerCapture(id) {
                captures.delete(id);
                window.emit("lostpointercapture", { pointerId: id, target: this });
            }
        };
        buttons.push(element);
        return element;
    }
    function down(element, pointerId = 1) {
        return root.emit("pointerdown", { target: element, pointerId, pointerType: "touch", button: 0 });
    }
    function up(pointerId = 1) { return window.emit("pointerup", { pointerId }); }
    function move(element, pointerId = 1) {
        document.pointed = element;
        window.emit("pointermove", { pointerId, clientX: 10, clientY: 10 });
    }
    function keys() { return events.map(event => `${event.type}:${event.code}`); }
    return { clock, window, document, root, canvas, events, actions, interruptions, api, button, down, up, move, keys };
}

test("quick touch remains down for 80 ms and releases without native mouse events", () => {
    const s = setup();
    const up = s.button({ direction: "up" });
    assert.equal(s.down(up).defaultPrevented, true);
    s.clock.advance(5);
    s.up();
    assert.equal(up.classList.contains("is-pressed"), false);
    s.clock.advance(74);
    assert.deepEqual(s.keys(), ["keydown:ArrowUp"]);
    s.clock.advance(1);
    assert.deepEqual(s.keys(), ["keydown:ArrowUp", "keyup:ArrowUp"]);
    assert.equal(s.events[0].keyCode, 38);
    assert.equal(s.events[0].which, 38);
    assert.equal(s.events[0].key, "ArrowUp");
    assert.equal(s.canvas.focused, true);
});

test("explicit touch input preserves tap timing without dispatching physical keyboard events", () => {
    const s = setup({ dispatchKeyboardEvents: false });
    s.down(s.button({ direction: "up" }));
    assert.equal(s.api.inputBits(), (1 << 0) | (1 << 16));
    assert.equal(s.api.inputBits(), 1 << 0, "fresh edges are consumed once");
    s.up();
    s.clock.advance(79);
    assert.equal(s.api.inputBits(), 1 << 0);
    s.clock.advance(1);
    assert.equal(s.api.inputBits(), 0);
    assert.deepEqual(s.keys(), [], "touch press/release must not alter physical key state");
});

test("a cancelled touch edge disappears while a new same-key tap remains fresh", () => {
    const s = setup({ dispatchKeyboardEvents: false });
    s.api.tap("KeyU");
    const first = s.api.inputBits();
    s.api.releaseAll();
    s.api.tap("KeyU");
    assert.equal(s.api.inputBits(), first, "a fresh press survives even without a released poll");
    s.api.tap("ArrowRight");
    s.api.releaseAll();
    assert.equal(s.api.inputBits(), 0, "cancelled presses must not reappear on a later poll");
    s.clock.advance(200);
    assert.equal(s.api.inputBits(), 0);
    assert.deepEqual(s.keys(), []);
});

test("explicit touch snapshots distinguish both heroes, sync modifiers and action buttons", () => {
    for (const [code, bit] of [
        ["ArrowUp", 0], ["ArrowDown", 1], ["ArrowLeft", 2], ["ArrowRight", 3],
        ["KeyW", 4], ["KeyS", 5], ["KeyA", 6], ["KeyD", 7],
        ["Space", 10], ["KeyU", 11], ["KeyR", 12], ["Escape", 13], ["Enter", 14]
    ]) {
        const s = setup({ dispatchKeyboardEvents: false });
        s.api.tap(code);
        assert.equal(s.api.inputBits(), ((1 << bit) | (1 << (bit + 16))) >>> 0, code);
        assert.deepEqual(s.keys(), []);
    }
    for (const [player, direction, modifier] of [["p1", 0, 8], ["p2", 4, 9]]) {
        const s = setup({ dispatchKeyboardEvents: false });
        s.api.setPlayer(player);
        s.api.setSync(true);
        s.down(s.button({ direction: "up" }));
        const bits = (1 << direction) | (1 << modifier);
        assert.equal(s.api.inputBits(), (bits | (bits << 16)) >>> 0);
        s.up();
        s.clock.advance(80);
        assert.equal(s.api.inputBits(), 0);
    }
});

test("a held control stays down for the engine's own repeat and releases immediately after the minimum", () => {
    const s = setup();
    const left = s.button({ direction: "left" });
    s.down(left);
    s.clock.advance(500);
    assert.deepEqual(s.keys(), ["keydown:ArrowLeft"]);
    assert.equal(left.getAttribute("aria-pressed"), "true");
    s.up();
    assert.deepEqual(s.keys(), ["keydown:ArrowLeft", "keyup:ArrowLeft"]);
    assert.equal(s.events[1].time, 500);
});

test("a re-tap before the first minimum expires produces two separate presses", () => {
    const s = setup();
    const right = s.button({ direction: "right" });
    s.down(right);
    s.clock.advance(5);
    s.up();
    s.clock.advance(5);
    s.down(right, 2);
    s.clock.advance(5);
    s.up(2);
    s.clock.advance(300);
    assert.deepEqual(s.keys(), [
        "keydown:ArrowRight", "keyup:ArrowRight", "keydown:ArrowRight", "keyup:ArrowRight"
    ]);
    assert.ok(s.events[2].time > s.events[1].time);
    assert.ok(s.events[3].time - s.events[2].time >= 80);
});

test("two fingers on the same key retain it until both release", () => {
    const s = setup();
    const wait = s.button({ code: "Space" });
    s.down(wait, 1);
    s.down(wait, 2);
    s.up(1);
    s.clock.advance(120);
    assert.deepEqual(s.keys(), ["keydown:Space"]);
    assert.equal(wait.classList.contains("is-pressed"), true);
    s.up(2);
    assert.deepEqual(s.keys(), ["keydown:Space", "keyup:Space"]);
    assert.equal(wait.classList.contains("is-pressed"), false);
});

test("player and sync selection map directions and keep the modifier through keyup", () => {
    const s = setup();
    s.api.setPlayer("p2");
    s.api.setSync(true);
    s.down(s.button({ direction: "up" }));
    s.up();
    s.clock.advance(100);
    assert.deepEqual(s.keys(), ["keydown:ShiftLeft", "keydown:KeyW", "keyup:KeyW", "keyup:ShiftLeft"]);
    assert.equal(s.events[1].shiftKey, true);
    assert.equal(s.events[1].key, "W");
    assert.equal(s.events[2].shiftKey, true);
    assert.equal(s.events[3].shiftKey, false);
    assert.equal(s.actions[0].player, "p2");
    assert.equal(s.actions[0].sync, true);
    assert.equal(s.actions[0].source, "pointer");
});

test("switching player preserves an already queued sync move and maps the next player separately", () => {
    const s = setup();
    const left = s.button({ direction: "left" });
    s.api.setSync(true);
    s.down(left, 1);
    s.up(1);
    s.api.setPlayer("p2");
    s.down(left, 2);
    s.up(2);
    s.clock.advance(100);
    assert.ok(s.keys().includes("keydown:ShiftRight"));
    assert.ok(s.keys().includes("keydown:ArrowLeft"));
    assert.ok(s.keys().includes("keydown:ShiftLeft"));
    assert.ok(s.keys().includes("keydown:KeyA"));
    assert.equal(s.events.filter(event => event.type === "keyup").length, 4);
});

test("switching hero stops a held direction without interrupting a queued synchronized move", () => {
    const s = setup();
    const left = s.button({ direction: "left" });
    s.api.setSync(true);
    s.down(left, 1);
    s.clock.advance(100);
    s.api.setPlayer("p2");
    assert.deepEqual(s.keys(), ["keydown:ShiftRight", "keydown:ArrowLeft", "keyup:ArrowLeft", "keyup:ShiftRight"]);
    s.down(left, 2);
    s.up(2);
    s.clock.advance(100);
    assert.ok(s.keys().includes("keydown:KeyA"));
    assert.deepEqual(s.interruptions, []);
});

test("a very fast hero switch still delivers the first synchronized tap for its minimum duration", () => {
    const s = setup();
    const up = s.button({ direction: "up" });
    s.api.setSync(true);
    s.down(up, 1);
    s.clock.advance(5);
    s.api.setPlayer("p2");
    s.down(up, 2);
    s.up(2);
    assert.equal(s.keys().includes("keyup:ArrowUp"), false);
    s.clock.advance(100);
    for (const code of ["ArrowUp", "KeyW"]) {
        const events = s.events.filter(event => event.code === code);
        assert.deepEqual(events.map(event => event.type), ["keydown", "keyup"]);
        assert.ok(events[1].time - events[0].time >= 80);
    }
    assert.deepEqual(s.interruptions, []);
});

test("unexpected pointer cancellation notifies the engine bridge, deliberate release does not", () => {
    const s = setup();
    s.down(s.button({ direction: "up" }));
    s.window.emit("pointercancel", { pointerId: 1 });
    assert.deepEqual(s.interruptions, [0]);
    s.api.releaseAll();
    assert.deepEqual(s.interruptions, [0]);
});

test("sliding the D-pad gives distinct cardinal moves without overlapping directions", () => {
    const s = setup();
    const up = s.button({ direction: "up" });
    const right = s.button({ direction: "right" });
    s.down(up);
    s.clock.advance(10);
    s.move(right);
    s.clock.advance(150);
    s.up();
    s.clock.advance(100);
    assert.deepEqual(s.keys(), ["keydown:ArrowUp", "keyup:ArrowUp", "keydown:ArrowRight", "keyup:ArrowRight"]);
    assert.equal(up.classList.contains("is-pressed"), false);
    assert.equal(right.classList.contains("is-pressed"), false);
});

test("sliding outside the D-pad releases the move, then sliding back begins another", () => {
    const s = setup();
    const left = s.button({ direction: "left" });
    s.down(left);
    s.clock.advance(100);
    s.move(null);
    assert.deepEqual(s.keys(), ["keydown:ArrowLeft", "keyup:ArrowLeft"]);
    s.clock.advance(40);
    s.move(left);
    assert.equal(s.keys().at(-1), "keydown:ArrowLeft");
});

for (const interrupt of ["pointercancel", "lostpointercapture", "blur", "pagehide", "hidden"]) {
    test(`${interrupt} immediately releases all keys and discards pending taps`, () => {
        const s = setup();
        const right = s.button({ direction: "right" });
        s.api.setSync(true);
        s.down(right, 1);
        s.up(1);
        s.down(right, 2);
        if (interrupt === "hidden") {
            s.document.visibilityState = "hidden";
            s.document.emit("visibilitychange");
        } else s.window.emit(interrupt, { pointerId: 2 });
        assert.deepEqual(s.keys(), ["keydown:ShiftRight", "keydown:ArrowRight", "keyup:ArrowRight", "keyup:ShiftRight"]);
        const count = s.events.length;
        s.clock.advance(1000);
        assert.equal(s.events.length, count);
        assert.equal(right.getAttribute("aria-pressed"), "false");
    });
}

test("normal lost capture after pointerup leaves another pointer's hold intact", () => {
    const s = setup();
    const wait = s.button({ code: "Space" });
    const undo = s.button({ code: "KeyU" });
    s.down(wait, 1);
    s.down(undo, 2);
    s.clock.advance(100);
    s.up(1);
    s.window.emit("lostpointercapture", { pointerId: 1 });
    assert.equal(s.keys().includes("keyup:KeyU"), false);
    assert.equal(undo.getAttribute("aria-pressed"), "true");
    s.up(2);
    assert.equal(s.keys().at(-1), "keyup:KeyU");
});

test("keyboard activation pulses once while the subsequent ordinary pointer click is ignored", () => {
    const s = setup();
    const confirm = s.button({ code: "Enter" });
    s.root.emit("click", { target: confirm, detail: 0 });
    s.root.emit("click", { target: confirm, detail: 1 });
    s.clock.advance(100);
    assert.deepEqual(s.keys(), ["keydown:Enter", "keyup:Enter"]);
    assert.equal(s.actions.length, 1);
    assert.equal(s.actions[0].source, "keyboard");
});

test("nothing is dispatched before the game's keyboard handler is initialized", () => {
    const s = setup({ ready: false });
    s.down(s.button({ code: "Enter" }));
    s.api.tap("Escape");
    s.clock.advance(100);
    assert.deepEqual(s.events, []);
    assert.deepEqual(s.actions, []);
    s.canvas.onkeydown = function () {};
    s.api.tap("Escape");
    s.clock.advance(100);
    assert.deepEqual(s.keys(), ["keydown:Escape", "keyup:Escape"]);
});

test("fixed action buttons keep their mapping for either player and report API actions", () => {
    const s = setup();
    s.api.setPlayer("p2");
    s.api.setSync(true);
    s.down(s.button({ code: "KeyU" }));
    s.up();
    s.api.tap("KeyR");
    s.clock.advance(100);
    assert.equal(s.events.filter(event => event.code.startsWith("Shift")).length, 0);
    assert.equal(s.actions[1].code, "KeyR");
    assert.equal(s.actions[1].source, "api");
    assert.equal(s.actions[1].sync, false);
});

test("disabled buttons and UI toggles are untouched; destroy removes event handlers", () => {
    const s = setup();
    const button = s.button({ code: "Space" });
    button.disabled = true;
    s.down(button);
    const toggle = s.button({ sync: "" });
    s.down(toggle);
    assert.deepEqual(s.events, []);
    assert.equal(toggle.getAttribute("aria-pressed"), null);
    button.disabled = false;
    s.down(button);
    s.api.destroy();
    s.down(button);
    s.api.tap("Enter");
    s.clock.advance(1000);
    assert.deepEqual(s.keys(), ["keydown:Space", "keyup:Space"]);
});

test("Wait releases a held direction before its keydown and discards queued movement", () => {
    const s = setup();
    s.down(s.button({ direction: "right" }), 1);
    s.clock.advance(100);
    s.down(s.button({ direction: "up" }), 2);
    s.down(s.button({ code: "Space" }), 3);
    assert.deepEqual(s.keys(), ["keydown:ArrowRight", "keyup:ArrowRight", "keydown:Space"]);
    s.up(3);
    s.clock.advance(1000);
    assert.deepEqual(s.keys(), ["keydown:ArrowRight", "keyup:ArrowRight", "keydown:Space", "keyup:Space"]);
    assert.deepEqual(s.interruptions, []);
});

for (const code of ["Space", "KeyU", "KeyR", "Escape", "Enter"]) {
    test(`${code} discards buffered moves before the priority action`, () => {
        const s = setup();
        const right = s.button({ direction: "right" });
        s.api.setSync(true);
        s.down(right, 1);
        s.up(1);
        s.clock.advance(10);
        s.down(right, 2);
        s.up(2);
        s.clock.advance(10);
        s.down(s.button({ code }), 3);
        s.up(3);
        s.clock.advance(1000);
        assert.deepEqual(s.keys(), [
            "keydown:ShiftRight", "keydown:ArrowRight", "keyup:ArrowRight", "keyup:ShiftRight",
            `keydown:${code}`, `keyup:${code}`
        ]);
        assert.equal(s.events[2].time, 20);
        assert.equal(s.events[4].time, 20);
        assert.equal(right.getAttribute("aria-pressed"), "false");
    });
}

test("two fingers holding Undo keep one held meta key while canceling other controls", () => {
    const s = setup();
    const undo = s.button({ code: "KeyU" });
    s.down(undo, 1);
    s.down(s.button({ direction: "up" }), 3);
    s.clock.advance(10);
    s.down(undo, 2);
    s.up(1);
    s.clock.advance(100);
    assert.equal(s.events.filter(event => event.type === "keydown" && event.code === "KeyU").length, 1);
    assert.equal(s.events.filter(event => event.type === "keyup" && event.code === "KeyU").length, 0);
    assert.equal(s.keys().includes("keyup:ArrowUp"), true);
    assert.equal(undo.getAttribute("aria-pressed"), "true");
    s.up(2);
    assert.equal(s.keys().at(-1), "keyup:KeyU");
    assert.equal(undo.getAttribute("aria-pressed"), "false");
});

test("quick repeated Undo preserves both minimum-length taps while canceling intervening moves", () => {
    const s = setup();
    const undo = s.button({ code: "KeyU" });
    s.down(undo, 1);
    s.up(1);
    s.clock.advance(10);
    s.down(s.button({ direction: "up" }), 3);
    s.down(undo, 2);
    s.up(2);
    s.clock.advance(1000);
    const undoEvents = s.events.filter(event => event.code === "KeyU");
    assert.deepEqual(undoEvents.map(event => event.type), ["keydown", "keyup", "keydown", "keyup"]);
    assert.ok(undoEvents[1].time - undoEvents[0].time >= 80);
    assert.ok(undoEvents[3].time - undoEvents[2].time >= 80);
    assert.equal(s.events.filter(event => event.type === "keydown" && event.code === "ArrowUp").length, 1);
});

for (const elapsed of [79.99, 79.5]) {
    test(`a release deadline measured as ${elapsed} ms retries until the tap can end`, () => {
        const s = setup({ dispatchKeyboardEvents: false });
        const right = s.button({ direction: "right" });
        // The timeout runs on schedule; the browser's exposed clock can be
        // rounded or jittered independently of the timer's actual deadline.
        s.window.performance.now = () => s.clock.now === 80 ? elapsed : s.clock.now;
        s.down(right);
        s.clock.advance(5);
        s.up();
        s.clock.advance(75);
        assert.equal(s.api.inputBits() & 0x7fff, 1 << 3, "minimum hold still applies");
        assert.equal(right.getAttribute("aria-pressed"), "false", "pointerup was processed");
        s.clock.advance(2);
        assert.equal(s.api.inputBits() & 0x7fff, 0, "retry releases the direction");
        s.clock.advance(60_000);
        assert.equal(s.api.inputBits(), 0);
        assert.equal(s.clock.tasks.size, 0);
    });
}

test("a fractional start time cannot strand a released direction at its deadline", () => {
    const s = setup({ dispatchKeyboardEvents: false });
    const right = s.button({ direction: "right" });
    s.clock.advance(48.2);
    s.down(right);
    s.clock.advance(5);
    s.up();
    s.clock.advance(75);
    assert.equal(s.clock.now, 128.2);
    assert.ok(s.clock.now - 48.2 < 80, "floating-point subtraction rounds below the duration");
    assert.equal(right.getAttribute("aria-pressed"), "false", "pointerup was processed");
    s.clock.advance(2);
    assert.equal(s.api.inputBits() & 0x7fff, 0, "an on-time callback must not latch the direction");
    s.clock.advance(60_000);
    assert.equal(s.api.inputBits(), 0);
    assert.equal(s.clock.tasks.size, 0);
});
