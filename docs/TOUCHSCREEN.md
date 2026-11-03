# Touchscreen guide

Infestation's web version supports touchscreen, keyboard and controller play.
Build and serve it from source using the [README instructions](../README.md).

## Playing

The game opens automatically when loading finishes. Begin on the game's title
screen with a tap, key press or controller confirmation. A keyboard or pointer
gesture allows the browser to start audio. The game is turn based, so there is
no need to rush between moves.

Help blocks gameplay from every input device. Closing it suppresses controls
that were already held, so they cannot cause an unintended move. If loading
takes longer than expected, the page shows a warning and remains ready to
recover when loading finishes. A graphics failure leaves a visible error
message with recovery instructions, including if it happens after play begins.

| Control | Action |
| --- | --- |
| Directional pad | Move the selected hero. Tap for an immediate step or hold to repeat at the saved D-pad delay. You can slide between directions. The center is inactive. |
| Wait | Pass a turn without moving. In two-hero rooms, it supplies a wait for both heroes. |
| Play / Enter room / Close | Contextual confirmation on the title, map or code screen. During gameplay, Continue is on the win card. Settings and menu items accept direct taps without an extra Select button. |
| Undo | Take back a move. |
| Restart | Tap twice within two seconds to start the room again. |
| Blue / orange portrait | Choose the hero controlled by the directional pad. A matching ring marks that hero on the board. Shown only in two-hero rooms. |
| Move together | Queue directions for a simultaneous two-hero turn. Shown only in two-hero rooms. |
| Menu (☰) | Open the game's menu, including On-screen controls: shown / hidden. |
| Fullscreen (⛶) | Enter or leave fullscreen when supported by the browser. |
| Help (?) | Open the guide for your current input method. |

The game's own menu buttons and map rooms also accept direct taps. Drag the
map to explore it. The pad can highlight menu rows and adjust sliders;
keyboard and gamepad confirmation remain available. Touch users choose a row
by tapping it directly.
Restart, Wait and Undo share the same style in the right-hand column, from top
to bottom. Unavailable actions become disabled without moving the other buttons.
Undo stays in its normal position after winning or losing. Wait never becomes
a Continue button.

### Input-aware instructions

Phones start in touch mode, with the on-screen pad visible unless you have
saved a preference to hide it. Keyboard and
controller keycaps, control legends and button prompts are hidden in touch
mode. Tutorial notes instead describe the D-pad, hero portraits and
**Move together**. Restart confirmation and the title screen use touch wording.
The Help guide also follows the active input method.

On a device with more than one input method, touch or pen input selects touch
mode; a physical keyboard, mouse or freshly used controller selects its mode.
Touch input is tracked separately from physical keyboard input. An idle
connected controller, a previously held input or small stick drift cannot steal
touch mode. Without a saved preference, the dock starts visible on touch
devices and hidden on desktops. **On-screen controls: shown / hidden** in
the game menu sets and saves your visibility preference. Changing input methods updates the
instructions without resizing the puzzle. Hiding the dock alone does not
change touch instructions into keyboard instructions, and showing it selects
touch instructions.

### Settings and control layout

Open the game menu with **☰**. Drag the **Music**, **Sound**, or **Screen shake**
bar to adjust it, or tap a point on the bar. Values keep their existing eleven
steps from 0 through 10. A slider stays attached to the gesture that started
on it, so releasing outside the bar cannot activate another menu item.

Tap **On-screen controls: shown / hidden** to show or hide the pad. The choice
is saved with the other settings and restored on your next visit. The **☰**
menu stays accessible when the pad is hidden, so you can always show it again.
With keyboard or controller navigation, left selects hidden and right selects
shown. Choosing shown also selects touch instructions.

Tap **D-pad: left** or **D-pad: right** to swap the directional pad and the
Restart / Wait / Undo column. The hero selectors follow the D-pad in two-hero
rooms. This preference is saved with the game's settings and restored on the
next visit; existing saves keep the D-pad on the left until you change it.
Swapping sides keeps the dock and game area the same size.

Drag **D-pad delay** to adjust how long a held direction waits before its first
repeat and between later repeats. The default is **0.30 seconds**, with values
from **0.10 to 0.60 seconds** in 0.05-second steps. Move the slider right for
slower repeats or left for faster repeats; the displayed value is the delay in
seconds. A fresh tap or direction change still moves immediately. The setting
is saved for the next visit and applies to touch D-pad movement in rooms.
Keyboard/controller movement, Wait, Undo, and menu/map navigation keep their
existing repeat timing. Turn animations can make movement slower than the
selected repeat cadence.

On smaller screens, swipe the settings list outside a slider to scroll to
the remaining options. A mouse wheel also scrolls, and D-pad or keyboard
navigation brings the focused row into view.

### Two heroes and Move together

For an ordinary move, leave Move together off and select the hero to move. For a
simultaneous turn, turn Move together on, select hero 1 and tap a direction, then select
hero 2 and tap their direction. Both heroes can then take their queued turns
together. Release the direction button before switching heroes.

Move together uses the game's synchronized-input behavior. Turning it off
clears queued future moves. Switching portraits preserves queued moves, so
each hero can contribute a direction to the same turn. With the direction
buttons released, Wait supplies waiting inputs for both heroes.

An idle connected controller does not change which hero the touch pad controls.
Physical controls always keep their original routing: the first controller
moves the blue hero, while the keyboard or second controller moves the orange
hero. Without a controller, arrow keys move the blue hero and WASD moves the
orange hero. Touching the screen changes the instructions without transferring
a held keyboard direction to a different hero.

## Layout and room tips

The bottom dock is 151 CSS pixels high in solo rooms, with 44-pixel-high primary
controls. Two-hero rooms add one 44-pixel utility row and a 6-pixel gap. The
layout uses the room's initial hero count, so a hero dying, an outcome card or
a tip appearing cannot resize it. The controls fill the available phone width,
with 10-pixel side padding and a 562-pixel maximum inner width on larger screens.
On landscape screens up to 600 pixels high, the dock moves to a 272-pixel-wide
side column. The surrounding page also accounts for device safe areas.

When the game uses its portrait layout and the touch controls are ready and
visible, level scenes reclaim the space used by the original lower instruction
panel. The level's title and status header stay visible. The removed panel's
buttons also stop accepting clicks, so they cannot intercept touches on the
enlarged board.

Room tips appear as an independent overlay near the bottom of the game area.
They can cover part of the puzzle, but they do not consume board layout space.
Scroll a long tip, tap **−** to minimize it, and tap **Room tip** to reopen it.
The full text determines the card's size before the typewriter effect finishes.
Showing, hiding, changing or revealing tip text never changes the board's
position or scale at a fixed viewport and controls state.

A tip retains its dismissal and scroll position while temporarily suppressed,
such as behind a menu. A different room, text or speaker starts a new tip.
Tips are suppressed during transitions, outcome cards, loading, errors and
Help. Touching or scrolling a tip releases held movement.

Hiding the touch controls restores the game's own portrait panel. Rotating the
device or toggling the dock can therefore change the board layout. In the
game's landscape layout, its existing status and field-guide panel remains,
while duplicate control rows are omitted when the touch dock is visible.
The game's layout follows the available canvas size, which can differ from
the device's orientation after the dock takes its share of the screen. Native
builds keep the original HUD and tutorial presentation.

## Source map

| File | Responsibility |
| --- | --- |
| `index.html` | Web page, control buttons, loading/error status, Help and room-tip markup. |
| `touch.css` | Compact responsive dock, safe areas and independent tip overlay. |
| `js/touch-controls.js` | Pointer input, hero selection, Sync modifiers and independently polled touch actions. |
| `js/touch-shell.js` | Loading, audio activation, visibility, fullscreen, Help, keyboard focus and the JavaScript side of the game bridge. |
| `js/touch-state.js` | Scene-aware action availability, hero controls, contextual labels and input cancellation. |
| `js/touch-tips.js` | Tip identity, typewriter text, dismissal, suppression and scroll restoration. |
| `js/input-mode.js` | Active touch, keyboard and controller detection, excluding synthetic events and stale controller input. |
| `game/src/touch_ui.rs` | WebAssembly imports for touch input, browser overlay blocking, dock state, settings, selected hero, cancellation and authored tip text, with native fallbacks. |
| `game/src/input.rs` | Independent touch and physical input sources, original keyboard/controller routing and held-input suppression. |
| `game/src/scenes/level.rs` | Level layout adjustment, action availability and publication of the current room tip. |
| `game/src/render/board.rs` | Matching selection ring on the touch-controlled hero. |
| `game/src/scenes/menu.rs` | Draggable sliders, scrollable settings, and visibility and control-side settings. |
| `game/src/settings.rs` | Saved audio, screen-shake, visibility, control-side, touch repeat delay and map preferences. |
| `game/src/render/ui.rs` | Reclaims the portrait instruction panel's space for the board. |
| `game/src/scenes/level/hud.rs` | Omits the replaced panel and its pointer targets. |
| `game/src/app.rs` | Publishes scene state and coordinates tip visibility and input cancellation with scenes, menus and transitions. |
| `tests/touch-controls.test.cjs` | Input lifecycle and queue regression tests. |
| `tests/touch-tips.test.cjs` | Tip content, visibility, identity and scroll regression tests. |
| `tests/touch-state.test.cjs` | Hero-row gating, contextual actions, outcome states and cancellation regression tests. |
| `tests/input-mode.test.cjs` | Initial device choice, real input transitions, synthetic input and controller activity regression tests. |
| `tests/touch-shell.test.cjs` | Automatic startup, modal input isolation, focus/key-release handling and slow-load recovery using the actual browser modules. |

Structured tutorial notes support an optional `touch` field alongside their
keyboard/controller variants. All shipped device-specific notes have authored
touch instructions. Old/custom structured notes without this field use generic
touch-control guidance in touch mode; plain gameplay notes are unchanged.

The pointer adapter keeps short taps active for at least 80 milliseconds and
two animation frames so the game's polling loop can see them. It separates
rapid taps, tracks ownership when several fingers share a button, and
serializes directions per hero to avoid unintended direction chords. Undo,
Restart, Menu and confirmation cancel buffered touch movement. Cancellation,
focus loss, backgrounding and resizing release held input. A monotonically
increasing input epoch also tells the engine to discard queued future moves
when the scene changes or interaction is interrupted, while preserving the
currently resolved turn and its animation. Changing available Undo/Restart
flags alone does not cancel input or disrupt the two-tap Restart confirmation.

The page publishes held touch actions separately from physical keys. The engine
keeps independent directional repeat timers and hero ownership for those sources.
Browser overlays block engine updates even when the touch pad is hidden, while
drawing and readiness checks continue. The frame that closes an overlay through
controller confirmation stays blocked, preventing the same press from also
activating the game.

The web page and WebAssembly binary should be built and served from the same
revision. The updated game imports the state, visibility and tip callbacks registered
by the page. `./serve.sh` builds the binary at the path used by `index.html`.

## Validation and browser limits

Run the regression tests from the repository root. The Rust build also prepares
the browser runtime used by the shell integration tests:

```sh
cargo test --locked -p infestation -p scenario_tests --lib
node --test tests/*.test.cjs
```

Build and run the web game with the normal source workflow:

```sh
rustup target add wasm32-unknown-unknown
./serve.sh
```

For a device check, exercise quick taps, held movement, direction changes,
Undo, the two-tap restart, hero switching and Move together. In portrait gameplay, open
and minimize a long tip and step onto or away from a tutorial trigger: the
board should stay at the same position and scale. Check rotating, hiding and
showing controls, opening menus, and returning after the browser has been
backgrounded. On a hybrid device, switch between touch, a physical keyboard or
mouse, and a controller; verify both the displayed prompts and hero routing.
Check the native version with `cargo run -r` when changing the
shared Rust HUD code.

Also open a fresh page and check that loading goes directly to the game. Use
the keyboard or controller to begin from the title screen. Hold a direction
while opening Help, release it inside Help, then return to the game;
the hero should stay still until a fresh input. In a two-hero room with a
controller connected, keep a physical arrow held while touching the board and
check that the keyboard continues controlling the same hero.

The game still needs WebAssembly and a working WebGL context. Browsers without
usable hardware graphics support may be unable to start it; reload after a
lost graphics context. Fullscreen depends on browser support, and audio needs
a real user gesture. Automated DOM/input tests do not establish rendering
performance or compatibility on a physical phone.

Settings scroll on short screens; portrait mode gives the panel more space.
Completed rooms and settings use this site's browser storage. Other browsers
or hosting origins have separate saved progress.
