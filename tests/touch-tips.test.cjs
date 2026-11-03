"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { create } = require("../js/touch-tips.js");

class Element {
    attributes = new Map();
    listeners = new Map();
    writes = [];
    _text = "";
    _hidden = false;
    _scroll = 0;
    get textContent() { return this._text; }
    set textContent(value) { this._text = value; this.writes.push(["text", value]); }
    set innerHTML(_) { throw new Error("Tutorial text must never use innerHTML."); }
    get hidden() { return this._hidden; }
    set hidden(value) {
        this._hidden = value;
        this.writes.push(["hidden", value]);
        // Simulate a browser losing its layout-dependent scroll offset when hidden.
        if (value) this._scroll = 0;
    }
    get scrollTop() { return this._scroll; }
    set scrollTop(value) { this._scroll = value; this.writes.push(["scroll", value]); }
    setAttribute(name, value) { this.attributes.set(name, value); this.writes.push([name, value]); }
    getAttribute(name) { return this.attributes.get(name) ?? null; }
    removeAttribute(name) { this.attributes.delete(name); this.writes.push([name, null]); }
    addEventListener(name, handler) {
        if (!this.listeners.has(name)) this.listeners.set(name, new Set());
        this.listeners.get(name).add(handler);
    }
    click() {
        const event = { prevented: false, preventDefault() { this.prevented = true; } };
        for (const handler of this.listeners.get("click") || []) handler(event);
        return event;
    }
}

function setup() {
    const elements = Object.fromEntries(["card", "hide", "show", "full", "reveal", "player"]
        .map(name => [name, new Element()]));
    const root = { querySelector: selector => elements[selector.slice("[data-tip-".length, -1)] || null };
    const toggles = [];
    const api = create({ root, onToggle: open => toggles.push(open) });
    function update(changes = {}) {
        api.update({ id: "intro", text: "Walk to the doorway.", player: 1, shown: 20, ...changes });
    }
    function resetWrites() {
        for (const element of Object.values(elements)) element.writes.length = 0;
    }
    return { ...elements, root, api, toggles, update, resetWrites };
}

test("starts empty with both controls hidden and stable accessible note semantics", () => {
    const s = setup();
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, true);
    assert.equal(s.card.getAttribute("role"), "note");
    assert.equal(s.full.getAttribute("aria-hidden"), "true");
    assert.equal(s.reveal.getAttribute("aria-hidden"), "true");
    assert.deepEqual(s.toggles, []);
});

test("shows the full sizing text with only the requested typewriter prefix visible", () => {
    const s = setup();
    s.update({ shown: 4 });
    assert.equal(s.card.hidden, false);
    assert.equal(s.show.hidden, true);
    assert.equal(s.full.textContent, "Walk to the doorway.");
    assert.equal(s.reveal.textContent, "Walk");
    assert.equal(s.player.textContent, "Player 1");
    assert.equal(s.card.getAttribute("aria-label"), "Player 1: Walk to the doorway.");
    assert.deepEqual(s.toggles, []);
});

test("repeated frame updates do not touch DOM or reset the reader's scroll", () => {
    const s = setup();
    s.update({ shown: 4 });
    s.card.scrollTop = 73;
    s.resetWrites();
    for (let i = 0; i < 120; i += 1) s.update({ shown: 4 });
    assert.equal(s.card.scrollTop, 73);
    for (const name of ["card", "show", "full", "reveal", "player"]) {
        assert.deepEqual(s[name].writes, [], `${name} unexpectedly changed`);
    }
});

test("typewriter progress updates only visible text, leaving the accessible label and layout stable", () => {
    const s = setup();
    s.update({ shown: 4 });
    s.card.scrollTop = 43;
    s.resetWrites();
    s.update({ shown: 7 });
    assert.deepEqual(s.reveal.writes, [["text", "Walk to"]]);
    assert.deepEqual(s.full.writes, []);
    assert.deepEqual(s.player.writes, []);
    assert.deepEqual(s.card.writes, []);
    assert.equal(s.card.scrollTop, 43);
});

test("dismissal survives per-frame updates and reopening restores scroll", () => {
    const s = setup();
    s.update({ shown: 4 });
    s.card.scrollTop = 64;
    assert.equal(s.hide.click().prevented, true);
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, false);
    s.resetWrites();
    for (let i = 0; i < 60; i += 1) s.update({ shown: 12 });
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, false);
    assert.deepEqual(s.card.writes, []);
    assert.deepEqual(s.toggles, [false]);
    // Model the temporary layout while both flex children are visible: a
    // premature scroll restoration gets clamped before the Tip button hides.
    const scroll = Object.getOwnPropertyDescriptor(Element.prototype, "scrollTop");
    Object.defineProperty(s.card, "scrollTop", {
        get() { return scroll.get.call(this); },
        set(value) { scroll.set.call(this, s.show.hidden ? value : Math.min(value, 23)); }
    });
    assert.equal(s.show.click().prevented, true);
    assert.equal(s.card.hidden, false);
    assert.equal(s.show.hidden, true);
    assert.equal(s.card.scrollTop, 64);
    assert.deepEqual(s.toggles, [false, true]);
});

for (const changes of [
    { text: "Wait for the rat to approach." },
    { player: 2 },
    { id: "another-room" }
]) {
    test(`changing ${Object.keys(changes)[0]} gives a new note that opens at the top`, () => {
        const s = setup();
        s.update();
        s.card.scrollTop = 88;
        s.hide.click();
        s.update(changes);
        assert.equal(s.card.hidden, false);
        assert.equal(s.show.hidden, true);
        assert.equal(s.card.scrollTop, 0);
        assert.deepEqual(s.toggles, [false]);
    });
}

test("an omitted room id has a stable identity across updates", () => {
    const s = setup();
    s.api.update({ text: "Keep moving.", player: 1, shown: 5 });
    s.hide.click();
    s.api.update({ text: "Keep moving.", player: 1, shown: 12 });
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, false);
});

test("clear resets the note and dismissal without leaving a stale reopen button", () => {
    const s = setup();
    s.update();
    s.hide.click();
    s.api.clear();
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, true);
    assert.equal(s.full.textContent, "");
    assert.equal(s.reveal.textContent, "");
    assert.equal(s.player.textContent, "");
    assert.equal(s.card.getAttribute("aria-label"), null);
    s.resetWrites();
    s.api.clear();
    assert.deepEqual(s.card.writes, []);
    assert.deepEqual(s.full.writes, []);
    s.update();
    assert.equal(s.card.hidden, false);
    assert.equal(s.card.scrollTop, 0);
});

test("empty text clears the active note", () => {
    const s = setup();
    s.update();
    s.update({ text: "" });
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, true);
    assert.equal(s.full.textContent, "");
});

test("suppression preserves a visible note and its scroll while typewriting continues", () => {
    const s = setup();
    s.update({ shown: 3 });
    s.card.scrollTop = 46;
    s.api.setSuppressed(true);
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, true);
    s.update({ shown: 10 });
    s.api.setSuppressed(false);
    assert.equal(s.card.hidden, false);
    assert.equal(s.card.scrollTop, 46);
    assert.equal(s.reveal.textContent, "Walk to th");
    assert.deepEqual(s.toggles, []);
});

test("suppression preserves dismissal and suppresses the reopen button too", () => {
    const s = setup();
    s.update();
    s.hide.click();
    s.api.setSuppressed(true);
    assert.equal(s.show.hidden, true);
    s.update();
    s.show.click();
    s.api.setSuppressed(false);
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, false);
    assert.deepEqual(s.toggles, [false]);
});

test("new notes and clears respect an existing suppression state", () => {
    const s = setup();
    s.api.setSuppressed(true);
    s.update();
    assert.equal(s.card.hidden, true);
    s.api.clear();
    s.update({ id: "second-room" });
    assert.equal(s.card.hidden, true);
    assert.equal(s.show.hidden, true);
    s.api.setSuppressed(false);
    assert.equal(s.card.hidden, false);
});

test("typewriter counts Unicode code points without splitting supplementary characters", () => {
    const s = setup();
    s.update({ text: "A😀é🧙", shown: 2 });
    assert.equal(s.reveal.textContent, "A😀");
    s.update({ text: "A😀é🧙", shown: 4 });
    assert.equal(s.reveal.textContent, "A😀é🧙");
    s.update({ text: "Ae\u0301", shown: 2 });
    assert.equal(s.reveal.textContent, "Ae");
    s.update({ text: "Ae\u0301", shown: 3 });
    assert.equal(s.reveal.textContent, "Ae\u0301");
});

test("negative and excessive reveal counts are bounded and identical text avoids extra writes", () => {
    const s = setup();
    s.update({ text: "Tip", shown: -10 });
    assert.equal(s.reveal.textContent, "");
    s.update({ text: "Tip", shown: 1000 });
    assert.equal(s.reveal.textContent, "Tip");
    s.resetWrites();
    s.update({ text: "Tip", shown: 1001 });
    assert.deepEqual(s.reveal.writes, []);
});

test("authored text remains literal text and never becomes HTML", () => {
    const s = setup();
    const text = '<img src=x onerror="bad()"> & <b>literal</b>';
    s.update({ text, shown: text.length });
    assert.equal(s.full.textContent, text);
    assert.equal(s.reveal.textContent, text);
    assert.equal(s.card.getAttribute("aria-label"), "Player 1: " + text);
});

test("missing markup or invalid payloads give explicit errors", () => {
    assert.throws(() => create({ root: null }), /root element/);
    assert.throws(() => create({ root: { querySelector: () => null } }), /data-tip-card/);
    const s = setup();
    assert.throws(() => s.update({ player: 3 }), /player must be 1 or 2/);
    assert.throws(() => s.update({ text: null }), /text must be a string/);
    assert.throws(() => s.update({ shown: NaN }), /finite code-point count/);
});
