/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

const COL_WIDTH = 16;
const ROW_HEIGHT = 22;
const ROW_FONT = "20px Courier";
const RIGHT_MARGIN_WIDTH = COL_WIDTH * 1;
const LEFT_MARGIN_WIDTH = COL_WIDTH * 1;

/* ===== GLOBAL VARIABLES ===== */

// Variables set in the `start()` function
let canv, ctx;

// Viewport controls
// ...

function drawRow(x, y, annot_row) {
    // Set values that are the same across all the bells
    const bells = annot_row.row().to_string();
    const highlights = annot_row.highlights();
    const text_baseline = y + ROW_HEIGHT * 0.75;
    const right = x + COL_WIDTH * bells.length;
    ctx.font = ROW_FONT;
    // Call string
    ctx.textAlign = "right";
    ctx.fillText(annot_row.call_str(), x - LEFT_MARGIN_WIDTH, text_baseline);
    // Bells
    ctx.textAlign = "center";
    for (let i = 0; i < bells.length; i++) {
        if (highlights[i] != 0) {
            ctx.fillStyle = "#5b5";
            ctx.fillRect(x + COL_WIDTH * i, y, COL_WIDTH, ROW_HEIGHT);
        }
        ctx.fillStyle = "black";
        ctx.fillText(bells[i], x + COL_WIDTH * (i + 0.5), text_baseline);
    }
    // Ruleoff
    if (annot_row.is_ruleoff()) {
        ctx.beginPath();
        ctx.moveTo(x, y + ROW_HEIGHT);
        ctx.lineTo(right, y + ROW_HEIGHT);
        ctx.stroke();
    }
    // Method string
    ctx.textAlign = "left";
    ctx.fillText(annot_row.method_str(), x + bells.length * COL_WIDTH + RIGHT_MARGIN_WIDTH, text_baseline);
}

function onWindowResize() {
    // Set the canvas size according to its new on-screen size
    var rect = canv.getBoundingClientRect();
    canv.width = rect.width * dpr;
    canv.height = rect.height * dpr;

    // Request a frame to be drawn
    draw();
}

function draw() {
    // Clear the screen and correct for HDPI displays
    ctx.save();
    ctx.clearRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);

    ctx.beginPath();
    ctx.arc(100, 100, 80, 0, Math.PI * 2);
    ctx.stroke();

    const frag = Frag.example();
    for (let i = 0; i < frag.len(); i++) {
        const annot_row = frag.get_row(i);
        drawRow(200, 200 + ROW_HEIGHT * i, annot_row);
    }

    // Reset the canvas' transform matrix so that the next frame is rendered correctly
    ctx.restore();
}

function start() {
    canv = document.getElementById("comp-canvas");
    ctx = canv.getContext("2d");
    
    // Correctly handle window resizing
    window.addEventListener('resize', onWindowResize);
    onWindowResize();

    window.requestAnimationFrame(draw);
}
