/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

// Display config
const COL_WIDTH = 16;
const ROW_HEIGHT = 22;
const RIGHT_MARGIN_WIDTH = COL_WIDTH * 1;
const LEFT_MARGIN_WIDTH = COL_WIDTH * 1;

const ROW_FONT = "20px monospace";
const BELL_NAMES = "1234567890ETABCDFGHJKLMNPQRSUVWXYZ";
const MUSIC_OPACITY = 0.2;
const LEFTOVER_ROW_ALPHA = 0.4;

const FALSE_ROW_GROUP_COLS = ["#f00", "#dd0", "#0b0", "#0bf", "#55f", "#f0f"];
const FALSE_ROW_GROUP_NOTCH_WIDTH = 0.3;
const FALSE_ROW_GROUP_NOTCH_HEIGHT = 0.3;
const FALSE_ROW_GROUP_LINE_WIDTH = 3;

// How many pixels off the edge of the screen the viewport culling will happen
const VIEW_CULLING_EXTRA_SIZE = 20;

/* ===== GLOBAL VARIABLES ===== */

// Variables set in the `start()` function
let canv, ctx;
// The comp being edited
let comp;
let derived_state;
// The part index being viewed
let current_part = 0;

// Mouse variables that the browser should keep track of but doesn't
let mouse_coords = { x: 0, y: 0 };

// Viewport controls
let viewport = { x: 0, y: 0, w: 100, h: 100 };

/* ===== DRAWING CODE ===== */

function drawRow(x, y, row) {
    // Don't draw if the row is going to be off the screen
    if (y < viewport.y - viewport.h / 2 - VIEW_CULLING_EXTRA_SIZE
     || y > viewport.y + viewport.h / 2 + VIEW_CULLING_EXTRA_SIZE) {
        return;
    }
    // Calculate some useful values
    const stage = comp.stage();
    const text_baseline = y + ROW_HEIGHT * 0.75;
    const right = x + COL_WIDTH * stage;
    // Call string
    ctx.font = ROW_FONT;
    if (row.call_str) {
        ctx.textAlign = "right";
        ctx.fillText(row.call_str, x - LEFT_MARGIN_WIDTH, text_baseline);
    }
    // Bells
    ctx.textAlign = "center";
    for (let b = 0; b < stage; b++) {
        ctx.fillStyle = "#5b5";
        ctx.globalAlpha = 1 - Math.pow(1 - MUSIC_OPACITY, row.music_highlights[b]);
        // Music highlighting
        if (row.music_highlights[b] > 0) {
            ctx.fillRect(x + COL_WIDTH * b, y, COL_WIDTH, ROW_HEIGHT);
        }
        ctx.globalAlpha = row.is_leftover ? LEFTOVER_ROW_ALPHA : 1;
        // Text
        ctx.fillStyle = "black";
        ctx.fillText(
            BELL_NAMES[row.expanded_rows[current_part][b]],
            x + COL_WIDTH * (b + 0.5),
            text_baseline
        );
    }
    // Ruleoff
    if (row.is_lead_end) {
        ctx.beginPath();
        ctx.moveTo(x, y + ROW_HEIGHT);
        ctx.lineTo(right, y + ROW_HEIGHT);
        ctx.stroke();
    }
    // Method string
    if (row.method_str) {
        ctx.textAlign = "left";
        ctx.fillText(
            row.method_str,
            x + stage * COL_WIDTH + RIGHT_MARGIN_WIDTH,
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

function drawFrag(x, y, frag) {
    // Rows
    for (let i = 0; i < frag.exp_rows.length; i++) {
        drawRow(x, y + ROW_HEIGHT * i, frag.exp_rows[i]);
    }
    // Falseness
    ctx.lineWidth = FALSE_ROW_GROUP_LINE_WIDTH;
    for (let i = 0; i < frag.false_row_ranges.length; i++) {
        const range = frag.false_row_ranges[i];
        // Draw the lines
        ctx.strokeStyle = FALSE_ROW_GROUP_COLS[range.group % FALSE_ROW_GROUP_COLS.length];
        drawFalsenessIndicator(
            x + LEFT_MARGIN_WIDTH * -0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            LEFT_MARGIN_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
        drawFalsenessIndicator(
            x + comp.stage() * COL_WIDTH + RIGHT_MARGIN_WIDTH * 0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            - RIGHT_MARGIN_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
    }
}

function draw() {
    // Clear the screen and correct for HDPI displays
    ctx.save();
    ctx.clearRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);
    // Move so that the camera's origin is in the centre of the screen
    ctx.translate(Math.round(viewport.w / 2), Math.round(viewport.h / 2));
    ctx.translate(Math.round(-viewport.x), Math.round(-viewport.y));

    for (let f = 0; f < derived_state.annot_frags.length; f++) {
        drawFrag(comp.frag_x(f), comp.frag_y(f), derived_state.annot_frags[f]);
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

/* ===== HUD CODE ===== */

function onPartHeadChange(evt) {
    // Update which part to display, and update the screen
    current_part = parseInt(evt.target.value);
    requestFrame();
}

function updatePartHeadList() {
    let ph_list = document.getElementById("part-head");
    // Clear the existing children
    ph_list.innerHTML = '';
    // Add the new part heads
    for (var i = 0; i < comp.num_parts(); i++) {
        let new_opt = document.createElement("option");
        new_opt.value = i.toString();
        new_opt.innerText = "#" + (i + 1).toString() + ": " + comp.part_head_str(i);
        ph_list.appendChild(new_opt);
    }
}

/* ===== STARTUP CODE ===== */

function start() {
    canv = document.getElementById("comp-canvas");
    ctx = canv.getContext("2d");

    comp = Comp.example();
    derived_state = JSON.parse(comp.derived_state());
    
    // Bind event listeners to all the things we need
    window.addEventListener("resize", onWindowResize);
    window.addEventListener("mousemove", onMouseMove);
    document.getElementById("part-head").addEventListener("change", onPartHeadChange);

    // Force a load of updates to make sure that things are initialised
    onWindowResize();
    updatePartHeadList();

    requestFrame();
}
