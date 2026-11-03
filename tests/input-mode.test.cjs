"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { create, prefersTouch, MODE } = require("../js/input-mode.js");

const touch = { isTrusted: true, pointerType: "touch", button: 0 };
const mouse = { isTrusted: true, pointerType: "mouse", button: 0 };
const key = { isTrusted: true, repeat: false };

function pad({ connected = true, buttons = [], x = 0, y = 0 } = {}) {
    return { connected, buttons: buttons.map(pressed => ({ pressed })), axes: [x, y] };
}

test("phones start in touch mode while fine-pointer hybrids start in keyboard mode", () => {
    assert.equal(prefersTouch({ coarsePointer: true, finePointer: false, maxTouchPoints: 5 }), true);
    assert.equal(prefersTouch({ coarsePointer: false, finePointer: true, maxTouchPoints: 10 }), false);
    assert.equal(prefersTouch({ coarsePointer: false, finePointer: false, maxTouchPoints: 1 }), true);
    assert.equal(prefersTouch({ coarsePointer: false, finePointer: true, maxTouchPoints: 0 }), false);
    const mode = create({ touchDefault: true });
    assert.equal(mode.code(), MODE.touch);
    assert.equal(mode.controlsVisible(), true);
    mode.setControlsPreference(null);
    assert.equal(mode.controlsVisible(), true);
    const desktop = create();
    desktop.setControlsPreference(null);
    assert.equal(desktop.controlsVisible(), false);
    assert.equal(desktop.mode(), "keyboard");
});

test("real keyboard, touch, mouse and pen input change prompts without resizing the dock", () => {
    const mode = create({ touchDefault: true });
    mode.keyboard(key);
    assert.equal(mode.mode(), "keyboard");
    assert.equal(mode.controlsVisible(), true);
    mode.pointer(touch);
    assert.equal(mode.mode(), "touch");
    assert.equal(mode.controlsVisible(), true);
    mode.pointer(mouse);
    assert.equal(mode.mode(), "keyboard");
    mode.pointer({ ...touch, pointerType: "pen" });
    assert.equal(mode.mode(), "touch");
    assert.equal(mode.controlsVisible(), true);
    const desktop = create();
    desktop.pointer(touch);
    assert.equal(desktop.mode(), "touch");
    assert.equal(desktop.controlsVisible(), false);
});

test("synthetic dock keys and pointer events cannot switch away from touch", () => {
    const mode = create({ touchDefault: true });
    for (const code of ["ArrowUp", "Space", "KeyU", "KeyR", "Enter", "Escape", "ShiftRight"]) {
        mode.keyboard({ isTrusted: false, repeat: false, code });
    }
    mode.pointer({ ...mouse, isTrusted: false });
    assert.equal(mode.mode(), "touch");
    assert.equal(mode.controlsVisible(), true);
});

test("touch compatibility mouse events and right clicks do not change prompts", () => {
    const mode = create({ touchDefault: true });
    mode.pointer({ ...mouse, sourceCapabilities: { firesTouchEvents: true } });
    mode.pointer({ ...mouse, button: 2 });
    assert.equal(mode.mode(), "touch");
});

test("clicking the visible on-screen pad with a mouse uses touch instructions", () => {
    const mode = create();
    mode.pointer(mouse, true);
    assert.equal(mode.mode(), "touch");
    mode.keyboard({ isTrusted: false, repeat: false });
    assert.equal(mode.mode(), "touch");
});

test("idle controller connections and stick drift leave touch prompts alone", () => {
    const mode = create({ touchDefault: true });
    mode.gamepads([pad()]);
    mode.gamepads([pad({ x: 0.15, y: -0.25 }), null, pad({ connected: false })]);
    mode.gamepads([pad({ x: 0.5 })]);
    assert.equal(mode.mode(), "touch");
});

test("fresh controller buttons switch prompts but held buttons cannot steal touch", () => {
    const mode = create({ touchDefault: true });
    mode.gamepads([pad()]);
    mode.gamepads([pad({ buttons: [true] })]);
    assert.equal(mode.code(), MODE.controller);
    assert.equal(mode.controlsVisible(), true);
    mode.pointer(touch);
    mode.gamepads([pad({ buttons: [true] })]);
    assert.equal(mode.mode(), "touch");
    mode.gamepads([pad()]);
    mode.gamepads([pad({ buttons: [true] })]);
    assert.equal(mode.mode(), "controller");
});

test("a fresh stick direction switches prompts without fighting newer keyboard input", () => {
    const mode = create({ touchDefault: true });
    mode.gamepads([pad({ x: 0.8 })]);
    assert.equal(mode.mode(), "controller");
    mode.keyboard(key);
    mode.gamepads([pad({ x: 0.9 })]);
    assert.equal(mode.mode(), "keyboard");
    mode.gamepads([pad({ x: -0.8 })]);
    assert.equal(mode.mode(), "controller");
    mode.pointer(touch);
    mode.gamepads([pad({ x: -0.8 })]);
    assert.equal(mode.mode(), "touch");
});

test("an old physical keyboard repeat cannot override a new touch gesture", () => {
    const mode = create();
    mode.keyboard(key);
    mode.pointer(touch);
    mode.keyboard({ ...key, repeat: true });
    assert.equal(mode.mode(), "touch");
    mode.keyboard(key);
    assert.equal(mode.mode(), "keyboard");
});

test("a second controller can select controller prompts and disconnect restores device default", () => {
    for (const touchDefault of [true, false]) {
        const mode = create({ touchDefault });
        mode.gamepads([null, pad({ buttons: [true] })]);
        assert.equal(mode.mode(), "controller");
        mode.gamepads([]);
        assert.equal(mode.mode(), touchDefault ? "touch" : "keyboard");
    }
});

test("hiding the dock retains touch prompts and stays hidden across device changes", () => {
    const mode = create({ touchDefault: true });
    mode.setControlsPreference(false);
    assert.equal(mode.controlsVisible(), false);
    assert.equal(mode.mode(), "touch");
    mode.pointer(touch);
    mode.gamepads([pad()]);
    assert.equal(mode.controlsVisible(), false);
    mode.keyboard(key);
    mode.pointer(touch);
    assert.equal(mode.controlsVisible(), false);
});

test("showing controls in settings opts into touch prompts and remains stable on dock interactions", () => {
    const mode = create();
    mode.setControlsPreference(true);
    assert.equal(mode.controlsVisible(), true);
    assert.equal(mode.mode(), "touch");
    mode.pointer(mouse, true);
    mode.keyboard({ isTrusted: false });
    assert.equal(mode.controlsVisible(), true);
    mode.keyboard(key);
    assert.equal(mode.controlsVisible(), true);
});

test("saved visibility overrides device defaults and can return to those defaults", () => {
    for (const touchDefault of [true, false]) {
        const mode = create({ touchDefault });
        mode.setControlsPreference(false);
        assert.equal(mode.controlsVisible(), false);
        mode.setControlsPreference(true);
        assert.equal(mode.controlsVisible(), true);
        assert.equal(mode.mode(), "touch");
        mode.setControlsPreference(null);
        assert.equal(mode.controlsVisible(), touchDefault);
    }
});

test("repeated saved visibility publication cannot override the active input device", () => {
    const changes = [];
    const mode = create({ touchDefault: true, onChange: state => changes.push(state) });
    mode.keyboard(key);
    // Explicitly saving shown selects touch even when the device default
    // already had the controls visible.
    mode.setControlsPreference(true);
    assert.equal(mode.mode(), "touch");
    mode.keyboard(key);
    for (let frame = 0; frame < 120; frame += 1) mode.setControlsPreference(true);
    assert.equal(mode.mode(), "keyboard");
    assert.equal(mode.controlsVisible(), true);
    assert.equal(changes.length, 3);
    mode.gamepads([pad({ buttons: [true] })]);
    for (let frame = 0; frame < 120; frame += 1) mode.setControlsPreference(true);
    assert.equal(mode.mode(), "controller");
    assert.equal(changes.length, 4);
    mode.setControlsPreference(false);
    for (let frame = 0; frame < 120; frame += 1) mode.setControlsPreference(false);
    assert.equal(mode.controlsVisible(), false);
    assert.equal(mode.mode(), "controller");
    assert.equal(changes.length, 5);
});

test("unchanged input does not repeatedly cancel or relayout the game", () => {
    const changes = [];
    const mode = create({ touchDefault: true, onChange: state => changes.push(state) });
    for (let frame = 0; frame < 120; frame += 1) {
        mode.pointer(touch);
        mode.keyboard({ isTrusted: false });
        mode.gamepads([pad()]);
    }
    assert.deepEqual(changes, []);
    mode.keyboard(key);
    for (let frame = 0; frame < 120; frame += 1) mode.keyboard(key);
    assert.deepEqual(changes, [{ mode: "keyboard", controlsVisible: true }]);
});

test("unsupported third and fourth controller slots cannot change prompts", () => {
    const mode = create({ touchDefault: true });
    mode.gamepads([null, null, pad({ buttons: [true] }), pad({ x: 0.9 })]);
    assert.equal(mode.mode(), "touch");
});
