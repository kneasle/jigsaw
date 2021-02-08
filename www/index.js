/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

// Row display consts
const COL_WIDTH = 16;
const ROW_HEIGHT = 22;
const ROW_FONT = "20px Courier";
const RIGHT_MARGIN_WIDTH = COL_WIDTH * 1;
const LEFT_MARGIN_WIDTH = COL_WIDTH * 1;

// How many pixels off the edge of the screen the viewport culling will happen
const VIEW_CULLING_EXTRA_SIZE = 20;

/* ===== GLOBAL VARIABLES ===== */

// Variables set in the `start()` function
let canv, ctx;
let start_time;

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
    window.requestAnimationFrame(frame);
}

function draw() {
    // Clear the screen and correct for HDPI displays
    ctx.save();
    ctx.clearRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);

    // Calculate viewport for row culling 
    const viewport = {
        l: 0, t: 0,
        r: canv.getBoundingClientRect().width,
        b: canv.getBoundingClientRect().height,
    };

    const elapsed_time = (Date.now() - start_time) / 1000;

    const frag = Frag.example();
    for (let i = 0; i < frag.len(); i++) {
        const annot_row = frag.get_row(i);
        const y = 200 - elapsed_time * 50 + ROW_HEIGHT * i;
        // Only draw the row if it's actually on the screen
        if (y < viewport.t - VIEW_CULLING_EXTRA_SIZE || y > viewport.b + VIEW_CULLING_EXTRA_SIZE) {
            continue;
        }
        // Draw the row if it hasn't been culled
        drawRow(100, y, annot_row);
    }

    // Reset the canvas' transform matrix so that the next frame is rendered correctly
    ctx.restore();
}

function frame() {
    draw();

    // window.requestAnimationFrame(frame);
}

function start() {
    canv = document.getElementById("comp-canvas");
    ctx = canv.getContext("2d");

    start_time = Date.now();
    
    // Correctly handle window resizing
    window.addEventListener('resize', onWindowResize);
    onWindowResize();

    window.requestAnimationFrame(frame);
}
