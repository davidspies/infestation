"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { create, SCREEN, ACTION } = require("../js/touch-state.js");

class Element {
    attributes = new Map();
    listeners = new Map();
    classes = new Set();
    dataset = {};
    writes = [];
    _hidden = false;
    _disabled = false;
    _text = "";
    classList = {
        toggle: (name, value) => {
            value ? this.classes.add(name) : this.classes.delete(name);
            this.writes.push(["class", name, value]);
        }
    };
    get hidden() { return this._hidden; }
    set hidden(value) { this._hidden = value; this.writes.push(["hidden", value]); }
    get disabled() { return this._disabled; }
    set disabled(value) { this._disabled = value; this.writes.push(["disabled", value]); }
    get textContent() { return this._text; }
    set textContent(value) { this._text = value; this.writes.push(["text", value]); }
    setAttribute(name, value) { this.attributes.set(name, value); this.writes.push([name, value]); }
    getAttribute(name) { return this.attributes.get(name) ?? null; }
    addEventListener(name, fn) {
        if (!this.listeners.has(name)) this.listeners.set(name, new Set());
        this.listeners.get(name).add(fn);
    }
    click() { for (const fn of this.listeners.get("click") || []) fn({ target: this }); }
}

function setup() {
    const elements = Object.fromEntries([
        "controller-meta", "play-actions", "context-actions", "context-confirm",
        "context-label", "context-hint", "sync", "sync-label", "dock-note",
        "Space", "KeyU", "KeyR", "p1", "p2", "up", "down", "left", "right"
    ].map(name => [name, new Element()]));
    const players = [elements.p1, elements.p2];
    players.forEach((element, i) => { element.dataset.player = "p" + (i + 1); });
    const directions = [elements.up, elements.down, elements.left, elements.right];
    const root = Object.assign(new Element(), {
        querySelector(selector) {
            const key = selector.startsWith("#") ? selector.slice(1)
                : selector.slice('[data-code="'.length, -2);
            return elements[key] || null;
        },
        querySelectorAll(selector) { return selector === "[data-player]" ? players : directions; }
    });
    const controls = {
        releases: 0, player: "p1", sync: false,
        releaseAll() { this.releases += 1; },
        setPlayer(value) { this.player = value; },
        setSync(value) { this.sync = value; }
    };
    const changes = [];
    const api = create({ root, controls,
        onPlayerChange: player => changes.push(["player", player]),
        onSyncChange: sync => changes.push(["sync", sync]) });
    function room(changes = {}) {
        api.update({ room: "Planks", screen: SCREEN.playing, players: 1,
            actions: ACTION.direction | ACTION.wait, ...changes });
    }
    function resetWrites() {
        for (const element of [root, ...Object.values(elements)]) element.writes.length = 0;
    }
    return { root, elements, directions, controls, changes, api, room, resetWrites };
}

test("loading offers no active game actions or premature two-hero controls", () => {
    const s = setup();
    assert.equal(s.elements["controller-meta"].hidden, true);
    assert.equal(s.elements["context-actions"].hidden, true);
    for (const name of ["Space", "KeyR", "KeyU", "context-confirm", "p1", "p2", "up"]) {
        assert.equal(s.elements[name].disabled, true, name);
    }
    assert.equal(s.api.selectedPlayer(), 0);
});

test("a solo puzzle has directions and Wait without OK or hero controls", () => {
    const s = setup();
    s.room();
    s.api.setAvailable(true);
    assert.equal(s.elements["controller-meta"].hidden, true);
    assert.equal(s.elements["play-actions"].hidden, false);
    assert.equal(s.elements["context-actions"].hidden, true);
    assert.equal(s.elements["context-confirm"].disabled, true);
    assert.equal(s.elements.Space.disabled, false);
    assert.equal(s.elements.KeyU.disabled, true);
    assert.equal(s.elements.KeyR.disabled, true);
    for (const direction of s.directions) assert.equal(direction.disabled, false);
    assert.equal(s.api.selectedPlayer(), 0);
});

test("changing available actions updates buttons without canceling input", () => {
    const s = setup();
    s.room();
    s.api.setAvailable(true);
    const epoch = s.api.inputEpoch();
    const releases = s.controls.releases;
    s.room({ actions: ACTION.direction | ACTION.wait | ACTION.undo | ACTION.restart });
    assert.equal(s.elements.KeyU.disabled, false);
    assert.equal(s.elements.KeyR.disabled, false);
    // Action flags can change independently within the same scene.
    s.room({ actions: ACTION.direction | ACTION.wait | ACTION.undo });
    assert.equal(s.elements.KeyU.disabled, false);
    assert.equal(s.elements.KeyR.disabled, true);
    assert.equal(s.api.inputEpoch(), epoch);
    assert.equal(s.controls.releases, releases);
});

for (const screen of [SCREEN.won, SCREEN.lost]) {
    test(`outcome ${screen} cancels held Wait and keeps Undo in the same action stack`, () => {
        const s = setup();
        s.room({ players: 2 });
        s.api.setAvailable(true);
        const epoch = s.api.inputEpoch();
        const releases = s.controls.releases;
        s.room({ players: 2, screen, actions: ACTION.undo | ACTION.restart });
        assert.equal(s.api.inputEpoch(), epoch + 1);
        assert.equal(s.controls.releases, releases + 1);
        assert.equal(s.elements.Space.disabled, true);
        assert.equal(s.elements.KeyU.disabled, false);
        assert.equal(s.elements.KeyR.disabled, false);
        assert.equal(s.elements["play-actions"].hidden, false);
        assert.equal(s.elements["context-actions"].hidden, true);
        assert.equal(s.elements["controller-meta"].hidden, false);
        assert.equal(s.elements.p2.disabled, true);
        for (const direction of s.directions) assert.equal(direction.disabled, true);
        assert.equal(s.api.selectedPlayer(), 0);
    });
}

test("hero selection and queued sync survive switching heroes and ordinary moves", () => {
    const s = setup();
    s.room({ players: 2 });
    s.api.setAvailable(true);
    assert.equal(s.elements["controller-meta"].hidden, false);
    assert.equal(s.api.selectedPlayer(), 1);
    s.elements.sync.click();
    const epoch = s.api.inputEpoch();
    s.elements.p2.click();
    assert.equal(s.api.inputEpoch(), epoch);
    assert.equal(s.api.selectedPlayer(), 2);
    assert.equal(s.controls.player, "p2");
    assert.equal(s.controls.sync, true);
    assert.equal(s.elements.p1.getAttribute("aria-pressed"), "false");
    assert.equal(s.elements.p2.getAttribute("aria-pressed"), "true");
    assert.equal(s.elements["sync-label"].textContent, "Move together");
    s.room({ players: 2, actions: 15 });
    assert.equal(s.api.inputEpoch(), epoch);
    assert.equal(s.api.selectedPlayer(), 2);
    assert.deepEqual(s.changes, [["sync", true], ["player", "p2"]]);
});

test("turning Move together off cancels the queued synchronized move", () => {
    const s = setup();
    s.room({ players: 2 });
    s.api.setAvailable(true);
    s.elements.sync.click();
    const epoch = s.api.inputEpoch();
    const releases = s.controls.releases;
    s.elements.sync.click();
    assert.equal(s.controls.sync, false);
    assert.equal(s.elements.sync.getAttribute("aria-pressed"), "false");
    assert.equal(s.api.inputEpoch(), epoch + 1);
    assert.equal(s.controls.releases, releases + 1);
});

test("menus preserve room geometry and selection but navigate with unsynced arrow keys", () => {
    const s = setup();
    s.room({ players: 2 });
    s.api.setAvailable(true);
    s.elements.p2.click();
    s.elements.sync.click();
    const epoch = s.api.inputEpoch();
    s.room({ players: 2, screen: SCREEN.menu, actions: ACTION.direction | ACTION.confirm });
    assert.equal(s.api.inputEpoch(), epoch + 1);
    assert.equal(s.elements["controller-meta"].hidden, false);
    assert.equal(s.elements["context-actions"].hidden, true);
    assert.equal(s.elements["play-actions"].hidden, true);
    assert.equal(s.elements["context-confirm"].disabled, true);
    assert.equal(s.elements.p2.disabled, true);
    assert.equal(s.controls.player, "p1");
    assert.equal(s.controls.sync, false);
    assert.equal(s.api.selectedPlayer(), 0);
    s.room({ players: 2 });
    assert.equal(s.elements["controller-meta"].hidden, false);
    assert.equal(s.elements["context-actions"].hidden, true);
    assert.equal(s.elements["play-actions"].hidden, false);
    assert.equal(s.api.selectedPlayer(), 2);
    assert.equal(s.controls.sync, true);
});

test("a different room resets hero selection and solo rooms remove the utility row", () => {
    const s = setup();
    s.room({ players: 2 });
    s.api.setAvailable(true);
    s.elements.p2.click();
    s.elements.sync.click();
    const epoch = s.api.inputEpoch();
    s.room({ room: "Another duo", players: 2 });
    assert.equal(s.api.inputEpoch(), epoch + 1);
    assert.equal(s.api.selectedPlayer(), 1);
    assert.equal(s.controls.sync, false);
    s.room();
    assert.equal(s.elements["controller-meta"].hidden, true);
    assert.equal(s.elements.p2.disabled, true);
    s.elements.p2.click();
    s.elements.sync.click();
    assert.equal(s.controls.player, "p1");
    assert.equal(s.controls.sync, false);
});

for (const [screen, label] of [[SCREEN.title, "Play"], [SCREEN.map, "Enter room"],
    [SCREEN.code, "Close"]]) {
    test(`context ${screen} offers ${label} only when confirmation is available`, () => {
        const s = setup();
        s.api.setAvailable(true);
        s.api.update({ room: "", screen, players: 0, actions: ACTION.confirm });
        assert.equal(s.elements["context-label"].textContent, label);
        assert.equal(s.elements["context-confirm"].getAttribute("aria-label"), label);
        assert.equal(s.elements["context-confirm"].dataset.code, screen === SCREEN.code ? "Escape" : "Enter");
        assert.equal(s.elements["context-confirm"].disabled, false);
        assert.equal(s.elements["context-actions"].hidden, false);
        assert.equal(s.elements["play-actions"].hidden, true);
        s.api.update({ room: "", screen, players: 0, actions: 0 });
        assert.equal(s.elements["context-confirm"].disabled, true);
    });
}

test("closing a code dialog hides menu confirmation and the map restores Enter", () => {
    const s = setup();
    s.api.setAvailable(true);
    s.api.update({ room: "", screen: SCREEN.code, players: 0, actions: ACTION.confirm });
    assert.equal(s.elements["context-confirm"].dataset.code, "Escape");
    s.api.update({ room: "", screen: SCREEN.menu, players: 0, actions: ACTION.direction | ACTION.confirm });
    assert.equal(s.elements["context-actions"].hidden, true);
    assert.equal(s.elements["context-confirm"].disabled, true);
    s.api.update({ room: "", screen: SCREEN.map, players: 0, actions: ACTION.direction | ACTION.confirm });
    assert.equal(s.elements["context-label"].textContent, "Enter room");
    assert.equal(s.elements["context-confirm"].dataset.code, "Enter");
});

test("settings have no Select or gameplay action buttons even if a confirm flag is supplied", () => {
    const s = setup();
    s.api.setAvailable(true);
    s.api.update({ room: "", screen: SCREEN.menu, players: 0,
        actions: ACTION.direction | ACTION.confirm });
    assert.equal(s.elements["context-actions"].hidden, true);
    assert.equal(s.elements["play-actions"].hidden, true);
    assert.equal(s.elements["context-confirm"].disabled, true);
    for (const direction of s.directions) assert.equal(direction.disabled, false);
});

test("saved D-pad side restores and changing it cancels input before the layout moves", () => {
    const s = setup();
    assert.equal(s.root.classes.has("dpad-right"), false);
    const epoch = s.api.inputEpoch();
    const releases = s.controls.releases;
    s.api.setDpadRight(true);
    assert.equal(s.root.classes.has("dpad-right"), true);
    assert.equal(s.api.inputEpoch(), epoch + 1);
    assert.equal(s.controls.releases, releases + 1);
    s.api.setDpadRight(false);
    assert.equal(s.root.classes.has("dpad-right"), false);
    assert.equal(s.api.inputEpoch(), epoch + 2);
    assert.equal(s.controls.releases, releases + 2);
});

test("D-pad side survives room, menu, outcome and control-visibility changes", () => {
    const s = setup();
    s.api.setDpadRight(true);
    s.room({ players: 2 });
    s.api.setAvailable(true);
    s.room({ players: 2, screen: SCREEN.menu, actions: ACTION.direction });
    assert.equal(s.root.classes.has("dpad-right"), true);
    s.room({ players: 2, screen: SCREEN.lost, actions: ACTION.undo | ACTION.restart });
    assert.equal(s.root.classes.has("dpad-right"), true);
    s.room({ room: "Another room", players: 1 });
    s.api.setAvailable(false);
    s.api.setAvailable(true);
    assert.equal(s.root.classes.has("dpad-right"), true);
});

test("repeated saved-layout publications do not rewrite the DOM or cancel input", () => {
    const s = setup();
    s.api.setDpadRight(true);
    s.room({ players: 2 });
    s.api.setAvailable(true);
    s.resetWrites();
    const epoch = s.api.inputEpoch();
    const releases = s.controls.releases;
    for (let i = 0; i < 120; i += 1) s.api.setDpadRight(true);
    assert.equal(s.api.inputEpoch(), epoch);
    assert.equal(s.controls.releases, releases);
    assert.deepEqual(s.root.writes, []);
    for (const element of Object.values(s.elements)) assert.deepEqual(element.writes, []);
});

test("loading, help and hidden controls disable input without changing room geometry", () => {
    const s = setup();
    s.room({ players: 2, actions: 15 });
    assert.equal(s.elements.Space.disabled, true);
    s.api.setAvailable(true);
    s.elements.p2.click();
    const epoch = s.api.inputEpoch();
    s.api.setAvailable(false);
    assert.equal(s.api.inputEpoch(), epoch + 1);
    assert.equal(s.elements["controller-meta"].hidden, false);
    assert.equal(s.elements["play-actions"].hidden, false);
    assert.equal(s.elements.KeyU.disabled, true);
    assert.equal(s.elements.KeyR.disabled, true);
    assert.equal(s.elements.Space.disabled, true);
    assert.equal(s.api.selectedPlayer(), 0);
    s.api.setAvailable(true);
    assert.equal(s.api.selectedPlayer(), 2);
    assert.equal(s.elements.Space.disabled, false);
});

test("repeated frame publications do not alter the DOM or cancel a held control", () => {
    const s = setup();
    s.room({ players: 2, actions: 15 });
    s.api.setAvailable(true);
    s.resetWrites();
    const epoch = s.api.inputEpoch();
    const releases = s.controls.releases;
    for (let i = 0; i < 120; i += 1) {
        s.room({ players: 2, actions: 15 });
        s.api.setAvailable(true);
    }
    assert.equal(s.api.inputEpoch(), epoch);
    assert.equal(s.controls.releases, releases);
    for (const element of Object.values(s.elements)) assert.deepEqual(element.writes, []);
});

test("an explicit interruption cancels browser holds and engine pending moves", () => {
    const s = setup();
    s.room({ players: 2 });
    s.api.setAvailable(true);
    const epoch = s.api.inputEpoch();
    const releases = s.controls.releases;
    s.api.interrupt();
    assert.equal(s.api.inputEpoch(), epoch + 1);
    assert.equal(s.controls.releases, releases + 1);
});
