/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

// Display config
const COL_WIDTH = 16;
const ROW_HEIGHT = 22;
const ROW_FONT = "20px Courier";
const RIGHT_MARGIN_WIDTH = COL_WIDTH * 1;
const LEFT_MARGIN_WIDTH = COL_WIDTH * 1;
const FALSE_ROW_GROUP_COLS = ["#f00", "#dd0", "#0b0", "#0bf", "#55f", "#f0f"];
const FALSE_ROW_GROUP_NOTCH_WIDTH = 0.3;
const FALSE_ROW_GROUP_NOTCH_HEIGHT = 0.3;
const FALSE_ROW_GROUP_LINE_WIDTH = 3;

// How many pixels off the edge of the screen the viewport culling will happen
const VIEW_CULLING_EXTRA_SIZE = 20;

/* ===== GLOBAL VARIABLES ===== */

// Variables set in the `start()` function
let canv, ctx;

// Mouse variables that the browser should keep track of but doesn't
let mouse_coords = { x: 0, y: 0 };

// Viewport controls
let viewport = { x: 0, y: 0, w: 100, h: 100 };

/* ===== DRAWING CODE ===== */

function drawRow(x, y, annot_row) {
    // Don't draw if the row is going to be off the screen
    if (y < viewport.y - viewport.h / 2 - VIEW_CULLING_EXTRA_SIZE
     || y > viewport.y + viewport.h / 2 + VIEW_CULLING_EXTRA_SIZE) {
        return;
    }
    // Set values that are the same across all the bells
    const bells = annot_row.row().to_string();
    const hl_ranges = annot_row.highlight_ranges();
    const method_str = annot_row.method_str();
    const call_str = annot_row.call_str();
    const text_baseline = y + ROW_HEIGHT * 0.75;
    const right = x + COL_WIDTH * bells.length;
    ctx.font = ROW_FONT;
    // Call string
    if (call_str) {
        ctx.textAlign = "right";
        ctx.fillText(call_str, x - LEFT_MARGIN_WIDTH, text_baseline);
    }
    // Highlighting
    ctx.fillStyle = "#5b5";
    for (let i = 0; i < hl_ranges.length; i += 2) {
        const start = hl_ranges[i];
        const end = hl_ranges[i + 1];
        ctx.fillRect(x + COL_WIDTH * start, y, COL_WIDTH * (end - start), ROW_HEIGHT);
    }
    // Bells
    ctx.textAlign = "center";
    ctx.fillStyle = "black";
    for (let i = 0; i < bells.length; i++) {
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
    if (method_str) {
        ctx.textAlign = "left";
        ctx.fillText(
            method_str,
            x + bells.length * COL_WIDTH + RIGHT_MARGIN_WIDTH,
            text_baseline
        );
    }
}

function drawFalsenessIndicator(x, min_y, max_y, notch_width, notch_height) {
    ctx.beginPath();
    ctx.moveTo(x + notch_width, min_y);
    ctx.lineTo(x, min_y + notch_height);
    ctx.lineTo(x, max_y - notch_height);
    ctx.lineTo(x + notch_width, max_y);
    ctx.stroke();
}

function draw() {
    // Clear the screen and correct for HDPI displays
    ctx.save();
    ctx.clearRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);
    // Move so that the camera's origin is in the centre of the screen
    ctx.translate(viewport.w / 2, viewport.h / 2);
    ctx.translate(-viewport.x, -viewport.y);

    const frag = Frag.example();
    // Rows
    for (let i = 0; i < frag.len(); i++) {
        drawRow(frag.x, frag.y + ROW_HEIGHT * i, frag.get_row(i));
    }
    // Falseness
    ctx.lineWidth = FALSE_ROW_GROUP_LINE_WIDTH;
    const false_row_groups = frag.false_row_groups();
    for (let i = 0; i < false_row_groups.length; i += 3) {
        // Unpack data from the silly way we have to send it through WASM
        const start = false_row_groups[i + 0];
        const end = false_row_groups[i + 1];
        const group = false_row_groups[i + 2];
        // Draw the lines
        ctx.strokeStyle = FALSE_ROW_GROUP_COLS[group % FALSE_ROW_GROUP_COLS.length];
        drawFalsenessIndicator(
            frag.x + LEFT_MARGIN_WIDTH * -0.5,
            frag.y + ROW_HEIGHT * start,
            frag.y + ROW_HEIGHT * end,
            LEFT_MARGIN_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
        drawFalsenessIndicator(
            frag.x + frag.num_bells() * COL_WIDTH + RIGHT_MARGIN_WIDTH * 0.5,
            frag.y + ROW_HEIGHT * start,
            frag.y + ROW_HEIGHT * end,
            - RIGHT_MARGIN_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
    }

    // Reset the canvas' transform matrix so that the next frame is rendered correctly
    ctx.restore();
}

function frame() {
    draw();

    // window.requestAnimationFrame(frame);
}

// Request for a frame to be rendered
function requestFrame() {
    window.requestAnimationFrame(frame);
}

/* ===== EVENT LISTENERS ===== */

function onWindowResize() {
    // Set the canvas size according to its new on-screen size
    var rect = canv.getBoundingClientRect();
    canv.width = rect.width * dpr;
    canv.height = rect.height * dpr;

    viewport.w = rect.width;
    viewport.h = rect.height;

    // Request a frame to be drawn
    requestFrame();
}

function onMouseMove(e) {
    if (e.offsetX == 0 && e.offsetY == 0) {
        return;
    }
    if (isButton(e, 0)) {
        viewport.x -= e.offsetX - mouse_coords.x;
        viewport.y -= e.offsetY - mouse_coords.y;
    }

    mouse_coords.x = e.offsetX;
    mouse_coords.y = e.offsetY;

    requestFrame();
}

function isButton(e, button) {
    // Deal with Safari being ideosyncratic
    const button_mask = (e.buttons === undefined ? e.which : e.buttons);
    return (button_mask & (1 << button)) != 0;
}

/* ===== STARTUP CODE ===== */

function start() {
    canv = document.getElementById("comp-canvas");
    ctx = canv.getContext("2d");
    
    // Bind event listeners to all the things we need
    window.addEventListener('resize', onWindowResize);
    window.addEventListener('mousemove', onMouseMove);

    // Generate a window resize event on startup to make sure the display gets initialised properly
    onWindowResize();

    requestFrame();
}
