// The game is played with a keyboard or controller. Browsers can't tell
// whether there's a keyboard, but anything with a mouse or trackpad (a
// computer, a tablet with a keyboard cover) almost always has one. Elsewhere
// (phones and most tablets) show a notice instead of loading the game.
function loadIfSupported(wasm_path) {
    if (window.matchMedia("(any-pointer: fine)").matches) {
        load(wasm_path);
        return;
    }
    const style = document.createElement("style");
    style.textContent = `
        .unsupported {
            box-sizing: border-box;
            height: 100%;
            padding: 24px;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            gap: 1rem;
            text-align: center;
            color: #a79fb5;
            font-family: system-ui, sans-serif;
            font-size: 1.1rem;
            line-height: 1.5;
        }
        .unsupported h1 {
            margin: 0 0 0.5rem;
            color: #f2c14e;
            font-size: 2.2rem;
            font-weight: 800;
            letter-spacing: 0.12em;
            text-transform: uppercase;
        }
        .unsupported p {
            margin: 0;
            max-width: 26rem;
        }
    `;
    document.head.append(style);
    const notice = document.createElement("div");
    notice.className = "unsupported";
    notice.innerHTML = `
        <h1>Infestation</h1>
        <p>This game needs a keyboard or controller, so it doesn't work on phones or tablets yet.</p>
        <p>Open this page on a computer to play.</p>
    `;
    document.body.replaceChildren(notice);
}
