# Undoable restarts

Restart begins a fresh attempt while keeping the previous attempt in the
room's undo history. The existing Undo and Restart controls work with both
keyboard and controller input, including the usual restart confirmation.

After restarting, Undo first takes back any moves in the new attempt. The
next Undo restores the position immediately before the restart. Keep undoing
to return through that attempt's earlier moves and any earlier restarts.
This also restores a won or lost position, including the cause of a loss.

Restarting before making a move does not add an empty undo step. It still
clears a synchronized move waiting for the other hero. A Wait counts as a
move, even when the board looks unchanged.

The move counter and exported solution describe the current attempt only.
Restoring an earlier attempt restores its move history too. Old animations
and buffered input do not resume when an attempt is restored.

Undo history lasts while the room remains open. Leaving the room or
reloading the game starts a new undo history; completed-room progress
continues to use the existing save system.

## Regression tests

Run the game-state, scene, and original puzzle-scenario tests:

```sh
cargo test --locked -p infestation -p scenario_tests --lib
```

On Ubuntu, native tests require `libasound2-dev`, `libudev-dev`, and
`pkg-config`.
