/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

// Display config
const COL_WIDTH = 16;
const ROW_HEIGHT = 22;
const RIGHT_MARGIN_WIDTH = COL_WIDTH * 1;
const LEFT_MARGIN_WIDTH = COL_WIDTH * 1;

const BACKGROUND_COL = "white";
const FOREGROUND_COL = "black";

const ROW_FONT = "20px monospace";
const BELL_NAMES = "1234567890ETABCDFGHJKLMNPQRSUVWXYZ";
const RULEOFF_LINE_WIDTH = 1;
const MUSIC_COL = "#5b5";
const LEFTOVER_ROW_OPACITY = 0.4;
const MUSIC_ONIONSKIN_OPACITY = 0.13;

const FALSE_ROW_GROUP_COLS = ["#f00", "#dd0", "#0b0", "#0bf", "#55f", "#f0f"];
const FALSE_ROW_GROUP_NOTCH_WIDTH = 0.3;
const FALSE_ROW_GROUP_NOTCH_HEIGHT = 0.3;
const FALSE_ROW_GROUP_LINE_WIDTH = 3;
const FALSE_COUNT_COL_FALSE = "red";
const FALSE_COUNT_COL_TRUE = "green";

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
let mouse_coords = {x: 0, y: 0};

// Viewport controls
let viewport = {x: 0, y: 0, w: 100, h: 100};

// Stuff that shouldn't be glovars but currently are
let lines = {
    0: [1.5, "red"],
    7: [2.5, "blue"],
};

/* ===== DRAWING CODE ===== */

function draw_row(x, y, row) {
    // Don't draw if the row is going to be off the screen
    if (y < viewport.y - viewport.h / 2 - VIEW_CULLING_EXTRA_SIZE
        || y > viewport.y + viewport.h / 2 + VIEW_CULLING_EXTRA_SIZE) {
        return;
    }
    // Calculate some useful values
    const stage = comp.stage();
    const text_baseline = y + ROW_HEIGHT * 0.75;
    const right = x + COL_WIDTH * stage;
    // Set the font for the entire row
    ctx.font = ROW_FONT;
    // Call string
    if (row.call_str) {
        ctx.textAlign = "right";
        ctx.fillStyle = FOREGROUND_COL;
        ctx.fillText(row.call_str, x - LEFT_MARGIN_WIDTH, text_baseline);
    }
    // Bells
    ctx.textAlign = "center";
    for (let b = 0; b < stage; b++) {
        // Music highlighting
        if (row.music_highlights && row.music_highlights[b].length > 0) {
            // If some music happened in the part we're currently viewing, then set the alpha to 1,
            // otherwise make an 'onionskinning' effect of the music from other parts
            ctx.globalAlpha = row.music_highlights[b].includes(current_part)
                ? 1
                : 1 - Math.pow(1 - MUSIC_ONIONSKIN_OPACITY, row.music_highlights[b].length);
            ctx.fillStyle = MUSIC_COL;
            ctx.fillRect(x + COL_WIDTH * b, y, COL_WIDTH, ROW_HEIGHT);
        }
        // Text
        const bell_index = row.expanded_rows[current_part][b];
        if (!lines[bell_index]) {
            ctx.globalAlpha = row.is_leftover ? LEFTOVER_ROW_OPACITY : 1;
            ctx.fillStyle = FOREGROUND_COL;
            ctx.fillText(BELL_NAMES[bell_index], x + COL_WIDTH * (b + 0.5), text_baseline);
        }
    }
    ctx.globalAlpha = 1;
    // Ruleoff
    if (row.is_lead_end) {
        const ruleoff_y = Math.round(y + ROW_HEIGHT) - 0.5;
        ctx.beginPath();
        ctx.moveTo(x, ruleoff_y);
        ctx.lineTo(right, ruleoff_y);
        ctx.strokeStyle = FOREGROUND_COL;
        ctx.lineWidth = RULEOFF_LINE_WIDTH;
        ctx.stroke();
    }
    // Method string
    if (row.method_str) {
        ctx.textAlign = "left";
        ctx.fillStyle = FOREGROUND_COL;
        ctx.fillText(
            row.method_str,
            x + stage * COL_WIDTH + RIGHT_MARGIN_WIDTH,
            text_baseline
        );
    }
}

function draw_falseness_indicator(x, min_y, max_y, notch_width, notch_height) {
    ctx.beginPath();
    ctx.moveTo(x + notch_width, min_y);
    ctx.lineTo(x, min_y + notch_height);
    ctx.lineTo(x, max_y - notch_height);
    ctx.lineTo(x + notch_width, max_y);
    ctx.stroke();
}

function draw_frag(x, y, frag) {
    // Rows
    for (let i = 0; i < frag.exp_rows.length; i++) {
        draw_row(x, y + ROW_HEIGHT * i, frag.exp_rows[i]);
    }
    // Lines
    for (let l in lines) {
        const width = lines[l][0];
        const col = lines[l][1];
        ctx.beginPath();
        for (let i = 0; i < frag.exp_rows.length; i++) {
            const ind = frag.exp_rows[i].expanded_rows[current_part].findIndex((x) => x == l);
            ctx.lineTo(x + (ind + 0.5) * COL_WIDTH, y + ROW_HEIGHT * (i + 0.5));
        }
        ctx.lineWidth = width;
        ctx.strokeStyle = col;
        ctx.stroke();
    }
    // Falseness
    ctx.lineWidth = FALSE_ROW_GROUP_LINE_WIDTH;
    for (let i = 0; i < frag.false_row_ranges.length; i++) {
        const range = frag.false_row_ranges[i];
        // Draw the lines
        ctx.strokeStyle = FALSE_ROW_GROUP_COLS[range.group % FALSE_ROW_GROUP_COLS.length];
        draw_falseness_indicator(
            x + LEFT_MARGIN_WIDTH * -0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            LEFT_MARGIN_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
        draw_falseness_indicator(
            x + comp.stage() * COL_WIDTH + RIGHT_MARGIN_WIDTH * 0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            -RIGHT_MARGIN_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
    }
}

function draw() {
    // Clear then fill the screen and correct for HDPI displays
    ctx.save();
    ctx.clearRect(0, 0, canv.width, canv.height);
    ctx.fillStyle = BACKGROUND_COL;
    ctx.fillRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);
    // Move so that the camera's origin is in the centre of the screen
    ctx.translate(Math.round(viewport.w / 2), Math.round(viewport.h / 2));
    ctx.translate(Math.round(-viewport.x), Math.round(-viewport.y));

    for (let f = 0; f < derived_state.annot_frags.length; f++) {
        draw_frag(comp.frag_x(f), comp.frag_y(f), derived_state.annot_frags[f]);
    }

    // Reset the canvas' transform matrix so that the next frame is rendered correctly
    ctx.restore();
}

function frame() {
    draw();

    // window.requestAnimationFrame(frame);
}

// Request for a frame to be rendered
function request_frame() {
    window.requestAnimationFrame(frame);
}

/* ===== EVENT LISTENERS ===== */

function on_window_resize() {
    // Set the canvas size according to its new on-screen size
    var rect = canv.getBoundingClientRect();
    canv.width = rect.width * dpr;
    canv.height = rect.height * dpr;

    viewport.w = rect.width;
    viewport.h = rect.height;

    // Request a frame to be drawn
    request_frame();
}

function on_mouse_move(e) {
    if (e.offsetX == 0 && e.offsetY == 0) {
        return;
    }
    if (is_button(e, 0)) {
        viewport.x -= e.offsetX - mouse_coords.x;
        viewport.y -= e.offsetY - mouse_coords.y;
    }

    mouse_coords.x = e.offsetX;
    mouse_coords.y = e.offsetY;

    request_frame();
}

function is_button(e, button) {
    // Deal with Safari being ideosyncratic
    const button_mask = (e.buttons === undefined ? e.which : e.buttons);
    return (button_mask & (1 << button)) != 0;
}

/* ===== HUD CODE ===== */

function on_part_head_change(evt) {
    // Update which part to display, and update the screen
    current_part = parseInt(evt.target.value);
    request_frame();
}

function update_hud() {
    document.getElementById("row-counter").innerText = derived_state.num_rows.toString();
    const falseness_info = document.getElementById("falseness-info");
    falseness_info.innerText = derived_state.num_false_rows === 0
        ? "true"
        : derived_state.num_false_rows.toString() + " false";
    falseness_info.style.color = derived_state.num_false_rows === 0
        ? FALSE_COUNT_COL_TRUE
        : FALSE_COUNT_COL_FALSE;
}

function update_part_head_list() {
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
    window.addEventListener("resize", on_window_resize);
    window.addEventListener("mousemove", on_mouse_move);
    document.getElementById("part-head").addEventListener("change", on_part_head_change);

    // Force a load of updates to make sure that things are initialised
    on_window_resize();
    update_part_head_list();
    update_hud();

    request_frame();
}
