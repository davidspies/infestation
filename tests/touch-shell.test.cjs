"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
const path = require("node:path");
const repo = path.join(__dirname, "..");
const InputMode = require("../js/input-mode.js");
const Dock = require("../js/touch-state.js");
const Tips = require("../js/touch-tips.js");

// Exercise the assembled page and its actual miniquad key handlers. The DOM
// model provides focus and event delivery; graphics and physical pads are not
// needed to verify which inputs cross the page/game boundary.
class Event {
    constructor(type, fields = {}) {
        this.type = type;
        this.isTrusted = false;
        this.defaultPrevented = false;
        this.stopped = false;
        Object.assign(this, fields);
    }
    preventDefault() { this.defaultPrevented = true; }
    stopImmediatePropagation() { this.stopped = true; }
}

class Element {
    constructor(id) {
        this.id = id;
        this.hidden = false;
        this.disabled = false;
        this.open = false;
        this.dataset = {};
        this.attributes = new Map();
        this.listeners = new Map();
        this.classes = new Set();
        this.classList = {
            toggle: (name, on) => on ? this.classes.add(name) : this.classes.delete(name),
            remove: name => this.classes.delete(name)
        };
    }
    addEventListener(name, fn) {
        if (!this.listeners.has(name)) this.listeners.set(name, []);
        this.listeners.get(name).push(fn);
    }
    emit(name, fields = {}) {
        const event = fields instanceof Event ? fields : new Event(name, { target: this, ...fields });
        for (const fn of this.listeners.get(name) || []) {
            fn(event);
            if (event.stopped) break;
        }
        return event;
    }
    dispatchEvent(event) {
        event.target = this;
        this.ownerDocument.emit(event.type, event);
        if (!event.stopped) {
            this.emit(event.type, event);
            this[`on${event.type}`]?.(event);
        }
        return !event.defaultPrevented;
    }
    setAttribute(name, value) { this.attributes.set(name, value); }
    getAttribute(name) { return this.attributes.get(name) ?? null; }
    removeAttribute(name) { this.attributes.delete(name); }
    focus() {
        if (this.ownerDocument.activeElement === this) return;
        this.ownerDocument.activeElement?.emit("blur");
        this.ownerDocument.activeElement = this;
    }
    showModal() {
        this.open = true;
        this.ownerDocument.getElementById("close-help").focus();
    }
    close() { this.open = false; this.emit("close"); }
    closest() { return null; }
    querySelector(selector) {
        return this.ownerDocument.getElementById(selector.startsWith("#") ? selector.slice(1) : `${this.id}:${selector}`);
    }
    querySelectorAll(selector) {
        if (selector === "[data-player]") return ["p1", "p2"].map(player => {
            const element = this.ownerDocument.getElementById(player);
            element.dataset.player = player;
            return element;
        });
        if (selector === "[data-direction]") return ["up", "down", "left", "right"].map(direction => {
            const element = this.ownerDocument.getElementById(direction);
            element.dataset.direction = direction;
            return element;
        });
        return [];
    }
}

function setup({ touchDefault = false } = {}) {
    const elements = new Map();
    const document = new Element("document");
    document.visibilityState = "visible";
    document.getElementById = id => {
        if (!elements.has(id)) {
            const element = new Element(id);
            element.ownerDocument = document;
            elements.set(id, element);
        }
        return elements.get(id);
    };
    const get = document.getElementById;
    document.querySelector = () => get("menu");
    document.body = get("body");
    document.body.dataset.wasmPath = "test.wasm";
    document.documentElement = {};
    get("loading-label").textContent = "Loading game…";
    const canvas = get("glcanvas");
    canvas.parentElement = get("game-container");
    canvas.focus(); // The original loader does this before initialization.
    const window = new Element("window");
    window.matchMedia = query => ({ matches: query.includes("coarse") ? touchDefault : !touchDefault });
    window.dispatchEvent = event => window.emit(event.type, event);
    window.KeyboardEvent = Event;
    let now = 0, nextTimer = 1;
    const timers = new Map();
    window.performance = { now: () => now };
    window.setTimeout = (callback, ms) => {
        const id = nextTimer++;
        timers.set(id, { callback, at: now + ms });
        return id;
    };
    window.clearTimeout = id => timers.delete(id);
    window.requestAnimationFrame = callback => window.setTimeout(callback, 16);
    window.cancelAnimationFrame = window.clearTimeout;
    function advance(ms) {
        const end = now + ms;
        for (;;) {
            const next = [...timers.entries()].filter(([, timer]) => timer.at <= end)
                .sort((a, b) => a[1].at - b[1].at || a[0] - b[0])[0];
            if (!next) break;
            const [id, timer] = next;
            now = timer.at;
            timers.delete(id);
            timer.callback();
        }
        now = end;
    }
    const pads = [];
    const plugins = [];
    const keyDowns = [];
    const heldKeys = new Set();
    let audioResumes = 0;
    let controls;
    const context = {
        console: { warn() {}, error() {} }, document, window,
        navigator: { maxTouchPoints: touchDefault ? 5 : 0, getGamepads: () => pads },
        InfestationInputMode: InputMode, InfestationDock: Dock, InfestationTips: Tips,
        setTimeout: window.setTimeout, clearTimeout: window.clearTimeout,
        Event, ResizeObserver: class { observe() {} },
        miniquad_add_plugin: plugin => plugins.push(plugin), load() {}, UTF8ToString: () => "",
        into_sapp_keycode: code => ({ Tab: 258, Enter: 257, Space: 32, Escape: 256, ArrowRight: 262, KeyU: 85 })[code],
        wasm_exports: {
            key_down: (...args) => { keyDowns.push(args); heldKeys.add(args[0]); },
            key_up: code => heldKeys.delete(code), key_press() {}
        },
        audio_context: { state: "suspended", resume() { audioResumes += 1; return Promise.resolve(); } },
        canvas
    };
    vm.createContext(context);
    vm.runInContext(fs.readFileSync(path.join(repo, "js/touch-controls.js"), "utf8"), context);
    context.InfestationTouch = {
        create(options) { controls = window.InfestationTouch.create(options); return controls; }
    };
    vm.runInContext(fs.readFileSync(path.join(repo, "js/quad-gamepad.js"), "utf8"), context);
    vm.runInContext(fs.readFileSync(path.join(repo, "js/touch-shell.js"), "utf8"), context);
    const loader = fs.readFileSync(path.join(repo, "js/gl.js"), "utf8");
    vm.runInContext(loader.slice(loader.indexOf("canvas.onkeydown ="), loader.indexOf("canvas.onkeyup =")), context);
    vm.runInContext(loader.slice(loader.indexOf("canvas.onkeyup ="), loader.indexOf("canvas.onkeypress =")), context);
    const imports = { env: {} };
    plugins.forEach(plugin => plugin.register_plugin(imports));
    function key(code, type = "keydown", fields = {}) {
        const event = new Event(type, { code, isTrusted: true, repeat: false, ...fields });
        document.activeElement.dispatchEvent(event);
        return event;
    }
    // App::tick uses this shell/engine contract: a frame stays blocked if the
    // page was obscured before polling, even when that poll closes the overlay.
    function poll() {
        const before = imports.env.infestation_input_blocked();
        const count = imports.env.sapp_gamepad_count();
        const after = imports.env.infestation_input_blocked();
        return { blocked: Boolean(before || after), count };
    }
    function pad(buttons = [], index = 0, axes = [0, 0]) {
        pads[index] = { connected: true, id: "Xbox", axes,
            buttons: Array.from({ length: 17 }, (_, button) => ({ pressed: buttons.includes(button) })) };
    }
    function loadGame() { poll(); }
    return { get, document, window, canvas, pads, pad, imports: imports.env, controls,
        key, poll, loadGame, advance, keyDowns, heldKeys,
        get audioResumes() { return audioResumes; },
        get epoch() { return imports.env.infestation_touch_input_epoch(); } };
}

test("loading blocks game input and permits Tab to leave the canvas", () => {
    const s = setup();
    assert.equal(s.imports.infestation_input_blocked(), 1);
    assert.equal(s.key("Tab").defaultPrevented, false);
    assert.equal(s.key("Enter").defaultPrevented, true);
    assert.deepEqual(s.keyDowns, []);
    assert.equal(s.get("loading").hidden, false);
});

for (const touchDefault of [false, true]) {
    test(`readiness automatically reveals and focuses the game (${touchDefault})`, () => {
        const s = setup({ touchDefault });
        assert.equal(s.poll().blocked, true); // Readiness itself consumes this frame.
        assert.equal(s.get("loading").hidden, true);
        assert.equal(s.imports.infestation_input_blocked(), 0);
        assert.equal(s.document.activeElement.id, "glcanvas");
        assert.equal(s.get("menu").disabled, false);
        assert.equal(s.imports.infestation_touch_controls_visible(), Number(touchDefault));
        assert.equal(s.poll().blocked, false);
    });
}

for (const [code, keyCode] of [["Enter", 257], ["Space", 32], ["ArrowRight", 262]]) {
    test(`the first physical ${code} reaches the game directly and enables audio`, () => {
        const s = setup();
        s.loadGame();
        assert.equal(s.audioResumes, 0);
        assert.equal(s.key(code).stopped, false);
        assert.equal(s.keyDowns.length, 1);
        assert.equal(s.keyDowns[0][0], keyCode);
        assert.equal(s.heldKeys.has(keyCode), true);
        assert.equal(s.audioResumes, 1);
        s.key(code, "keyup");
        assert.equal(s.heldKeys.size, 0);
    });
}

test("a key held during loading cannot carry repeats into the ready game", () => {
    const s = setup();
    s.key("Space");
    s.loadGame();
    assert.equal(s.get("loading").hidden, true);
    assert.equal(s.key("Space", "keydown", { repeat: true }).defaultPrevented, true);
    assert.deepEqual(s.keyDowns, []);
    s.key("Space", "keyup");
    s.key("Space");
    assert.equal(s.keyDowns.length, 1);
    assert.equal(s.keyDowns[0][0], 32);
});

test("keys on other focused HTML controls retain their normal behavior", () => {
    const s = setup();
    s.loadGame();
    s.get("original-source-link").focus();
    assert.equal(s.key("Enter").defaultPrevented, false);
    assert.deepEqual(s.keyDowns, []);
    assert.equal(s.audioResumes, 0);
});

for (const button of [0, 1, 9]) {
    test(`controller button ${button} closes Help only on a fresh, consumed press`, () => {
        const s = setup();
        s.pad([button]);
        assert.equal(s.poll().blocked, true);
        assert.equal(s.get("loading").hidden, true);
        assert.equal(s.poll().blocked, false);
        s.get("show-help").emit("click");
        assert.equal(s.poll().blocked, true);
        assert.equal(s.get("help").open, true); // The controller button remains held.
        s.pad([]);
        s.poll();
        s.pad([button]);
        assert.equal(s.poll().blocked, true);
        assert.equal(s.get("help").open, false);
        assert.equal(s.poll().blocked, false);
    });
}

test("first readiness preserves Help focus and does not treat an already-held pad button as closing it", () => {
    const s = setup();
    s.get("show-help").emit("click");
    s.pad([0]);
    assert.equal(s.poll().blocked, true);
    assert.equal(s.get("loading").hidden, true);
    assert.equal(s.get("help").open, true);
    assert.equal(s.document.activeElement.id, "close-help");
    assert.equal(s.poll().blocked, true);
    assert.equal(s.get("help").open, true);
    s.pad([]);
    s.poll();
    s.pad([0]);
    assert.equal(s.poll().blocked, true);
    assert.equal(s.get("help").open, false);
    assert.equal(s.document.activeElement.id, "glcanvas");
});

test("readiness in a hidden tab defers initial focus until the page becomes visible", () => {
    const s = setup();
    s.document.visibilityState = "hidden";
    s.get("other-control").focus();
    assert.equal(s.poll().blocked, true);
    assert.equal(s.get("loading").hidden, true);
    assert.equal(s.document.activeElement.id, "other-control");
    s.document.visibilityState = "visible";
    s.document.emit("visibilitychange");
    assert.equal(s.imports.infestation_input_blocked(), 0);
    assert.equal(s.document.activeElement.id, "glcanvas");
    s.get("other-control").focus();
    s.document.emit("visibilitychange");
    assert.equal(s.document.activeElement.id, "other-control"); // Initial focus runs only once.
});

for (const touchDefault of [false, true]) {
    test(`Help gates gameplay and interrupts independently of dock visibility (${touchDefault})`, () => {
        const s = setup({ touchDefault });
        s.loadGame();
        const before = s.epoch;
        s.get("show-help").emit("click");
        assert.ok(s.epoch > before);
        assert.equal(s.imports.infestation_input_blocked(), 1);
        s.pad([2], 0, [1, 0]); // Undo plus right stick: neither closes Help.
        assert.equal(s.poll().blocked, true);
        assert.equal(s.get("help").open, true);
        const inHelp = s.epoch;
        s.get("help-done").emit("click");
        assert.ok(s.epoch > inHelp);
        assert.equal(s.imports.infestation_input_blocked(), 0);
        assert.equal(s.document.activeElement.id, "glcanvas");
    });
}

test("opening Help clears a held canvas key and consumes the release delivered to the dialog", () => {
    const s = setup();
    s.loadGame();
    s.key("ArrowRight");
    assert.equal(s.heldKeys.has(262), true);
    s.get("show-help").emit("click");
    assert.equal(s.heldKeys.has(262), false);
    assert.equal(s.key("ArrowRight", "keyup").defaultPrevented, true);
    s.get("help-done").emit("click");
    assert.equal(s.heldKeys.size, 0);
    s.key("ArrowRight");
    assert.equal(s.heldKeys.has(262), true);
});

for (const code of ["Escape", "Enter", "Space"]) {
    test(`${code} closes Help without activating gameplay after focus returns`, () => {
        const s = setup();
        s.loadGame();
        s.get("show-help").emit("click");
        assert.equal(s.key(code).defaultPrevented, true);
        assert.equal(s.get("help").open, false);
        assert.equal(s.key(code, "keydown", { repeat: true }).defaultPrevented, true);
        s.key(code, "keyup");
        assert.deepEqual(s.keyDowns, []);
    });
}

test("native dialog cancellation restores the ready game's focus", () => {
    const s = setup();
    s.loadGame();
    s.get("show-help").emit("click");
    const event = s.get("help").emit("cancel");
    assert.equal(event.defaultPrevented, true);
    assert.equal(s.get("help").open, false);
    assert.equal(s.document.activeElement.id, "glcanvas");
    assert.equal(s.imports.infestation_input_blocked(), 0);
});

test("moving focus from the canvas releases only keys the game actually received", () => {
    const s = setup();
    s.loadGame();
    s.key("ArrowRight");
    s.get("tip").focus();
    assert.equal(s.heldKeys.size, 0);
    s.key("KeyU"); // A key on a different HTML element never enters game state.
    s.canvas.focus();
    s.key("KeyU", "keyup");
    assert.equal(s.keyDowns.length, 1);
    s.key("ArrowRight"); // Fresh press recovers even if the original keyup was lost.
    assert.equal(s.heldKeys.has(262), true);
});

test("backgrounding blocks input, clears held keys, and interrupts even with the dock hidden", () => {
    const s = setup();
    s.loadGame();
    s.key("ArrowRight");
    const before = s.epoch;
    s.document.visibilityState = "hidden";
    s.document.emit("visibilitychange");
    assert.equal(s.imports.infestation_input_blocked(), 1);
    assert.equal(s.heldKeys.size, 0);
    assert.ok(s.epoch > before);
    s.document.visibilityState = "visible";
    s.document.emit("visibilitychange");
    assert.equal(s.imports.infestation_input_blocked(), 0);
    assert.equal(s.key("ArrowRight", "keydown", { repeat: true }).defaultPrevented, true);
});

test("a load finishing after the warning automatically enables the game and controls", () => {
    const s = setup({ touchDefault: true });
    s.advance(45000);
    assert.match(s.get("load-note").textContent, /keep waiting/);
    assert.equal(s.get("loading").hidden, false);
    assert.equal(s.get("loading-label").textContent, "Loading game…");
    s.poll();
    assert.equal(s.get("loading").hidden, true);
    assert.equal(s.imports.infestation_input_blocked(), 0);
    assert.equal(s.get("menu").disabled, false);
    assert.equal(s.imports.infestation_touch_controls_visible(), 1);
});

test("a fatal graphics error cannot be cleared by readiness or the slow-load timer", () => {
    const s = setup();
    s.canvas.emit("webglcontextcreationerror", { statusMessage: "No context" });
    s.advance(45000);
    s.pad([0]);
    s.poll();
    assert.equal(s.get("loading-label").textContent, "Game couldn’t start");
    assert.match(s.get("load-note").textContent, /WebGL/);
    assert.equal(s.get("loading").hidden, false);
    assert.equal(s.imports.infestation_input_blocked(), 1);
});

test("losing graphics after startup reopens a persistent error overlay and blocks input", () => {
    const s = setup({ touchDefault: true });
    s.loadGame();
    s.canvas.emit("webglcontextlost");
    assert.equal(s.get("loading").hidden, false);
    assert.equal(s.get("loading-label").textContent, "Game interrupted");
    assert.match(s.get("load-note").textContent, /Reload/);
    assert.equal(s.imports.infestation_input_blocked(), 1);
    assert.equal(s.get("menu").disabled, true);
    assert.equal(s.imports.infestation_touch_controls_visible(), 0);
    s.key("ArrowRight");
    s.advance(45000);
    s.poll();
    assert.deepEqual(s.keyDowns, []);
    assert.equal(s.get("loading").hidden, false);
});

test("touch inputs use the separate engine channel without synthesizing keyboard state", () => {
    const s = setup({ touchDefault: true });
    s.loadGame();
    s.controls.tap("ArrowRight");
    const first = s.imports.infestation_touch_input();
    assert.equal(first & 0xffff, 1 << 3);
    assert.equal(first >>> 16, 1 << 3);
    assert.equal(s.imports.infestation_touch_input(), 1 << 3);
    assert.deepEqual(s.keyDowns, []);
    assert.equal(s.heldKeys.size, 0);
    s.advance(100);
    assert.equal(s.imports.infestation_touch_input(), 0);
});
