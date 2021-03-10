/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

// IDs of mouse buttons
const BTN_LEFT = 0;
const BTN_RIGHT = 1;
const BTN_MIDDLE = 2;

// Cookie names
const COOKIE_NAME_VIEW = "view";

// How many pixels off the edge of the screen the viewport culling will happen
const VIEW_CULLING_EXTRA_SIZE = 20;

/* ===== DISPLAY CONSTANTS ===== */

const COL_WIDTH = 16;
const ROW_HEIGHT = 22;
const FALSENESS_BAR_WIDTH = COL_WIDTH * 1;
const FRAG_BBOX_EXTRA_HEIGHT = FALSENESS_BAR_WIDTH * 0.5;

const FOREGROUND_COL = "black";
const BACKGROUND_COL = "white";
const GRID_COL = "#eee";
const GRID_SIZE = 200;

const ROW_FONT = "20px monospace";
const BELL_NAMES = "1234567890ETABCDFGHJKLMNPQRSUVWXYZ";
const RULEOFF_LINE_WIDTH = 1;
const MUSIC_COL = "#5b5";
const LEFTOVER_ROW_OPACITY = 0.4;
const MUSIC_ONIONSKIN_OPACITY = 0.13;

const FALSE_ROW_GROUP_NOTCH_WIDTH = 0.3;
const FALSE_ROW_GROUP_NOTCH_HEIGHT = 0.3;
const FALSE_ROW_GROUP_LINE_WIDTH = 3;
const FALSE_COUNT_COL_FALSE = "red";
const FALSE_COUNT_COL_TRUE = "green";

/* ===== GLOBAL VARIABLES ===== */

// Variables set in the `start()` function
let canv, ctx;
// Variables which will used to sync with the Rust code (in 90% of the code, these should be treated
// as immutable).
let comp, derived_state, view;
// Mouse variables that the browser should keep track of but doesn't
let mouse_coords = {x: 0, y: 0};
// Things that should be user config but currently are global vars
let bell_lines = {
    0: [1.5, "red"],
    7: [2.5, "blue"],
};

/* ===== DRAWING CODE ===== */

function draw_row(x, y, row) {
    const v = view_rect();
    // Don't draw if the row is going to be off the screen
    if (y < v.min_y - VIEW_CULLING_EXTRA_SIZE || y > v.max_y + VIEW_CULLING_EXTRA_SIZE) {
        return;
    }
    // Calculate some useful values
    const stage = derived_state.stage;
    const text_baseline = y + ROW_HEIGHT * 0.75;
    const right = x + COL_WIDTH * stage;
    // Set the font for the entire row
    ctx.font = ROW_FONT;
    // Call string
    if (row.call_str) {
        ctx.textAlign = "right";
        ctx.fillStyle = FOREGROUND_COL;
        ctx.fillText(row.call_str, x - FALSENESS_BAR_WIDTH, text_baseline);
    }
    // Bells
    ctx.textAlign = "center";
    for (let b = 0; b < stage; b++) {
        // Music highlighting
        if (row.music_highlights && row.music_highlights[b].length > 0) {
            // If some music happened in the part we're currently viewing, then set the alpha to 1,
            // otherwise make an 'onionskinning' effect of the music from other parts
            ctx.globalAlpha = row.music_highlights[b].includes(view.current_part)
                ? 1
                : 1 - Math.pow(1 - MUSIC_ONIONSKIN_OPACITY, row.music_highlights[b].length);
            ctx.fillStyle = MUSIC_COL;
            ctx.fillRect(x + COL_WIDTH * b, y, COL_WIDTH, ROW_HEIGHT);
        }
        // Text
        const bell_index = row.rows[view.current_part][b];
        if (!bell_lines[bell_index]) {
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
            row.method_str.name,
            x + stage * COL_WIDTH + FALSENESS_BAR_WIDTH,
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

function draw_frag(frag) {
    const x = frag.x;
    const y = frag.y;
    const rect = frag_bbox(frag);
    // Background box (to overlay over the grid)
    ctx.fillStyle = BACKGROUND_COL;
    ctx.fillRect(rect.min_x, rect.min_y, rect.w, rect.h);
    // Rows
    for (let i = 0; i < frag.exp_rows.length; i++) {
        draw_row(x, y + ROW_HEIGHT * i, frag.exp_rows[i]);
    }
    // Lines
    for (let l in bell_lines) {
        const width = bell_lines[l][0];
        const col = bell_lines[l][1];
        ctx.beginPath();
        for (let i = 0; i < frag.exp_rows.length; i++) {
            const ind = frag.exp_rows[i].rows[view.current_part].findIndex((x) => x == l);
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
            x + FALSENESS_BAR_WIDTH * -0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            FALSENESS_BAR_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
        draw_falseness_indicator(
            x + derived_state.stage * COL_WIDTH + FALSENESS_BAR_WIDTH * 0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            -FALSENESS_BAR_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
    }
}

function draw_grid() {
    const v = view_rect();
    // Calculate the local-space boundary of the viewport
    ctx.strokeStyle = GRID_COL;
    // Vertical bars
    for (let x = Math.ceil(v.min_x / GRID_SIZE) * GRID_SIZE; x < v.max_x; x += GRID_SIZE) {
        ctx.beginPath();
        ctx.moveTo(x + 0.5, v.min_y);
        ctx.lineTo(x + 0.5, v.max_y);
        ctx.stroke();
    }
    // Horizontal bars
    for (let y = Math.ceil(v.min_y / GRID_SIZE) * GRID_SIZE; y < v.max_y; y += GRID_SIZE) {
        ctx.beginPath();
        ctx.moveTo(v.min_x, y + 0.5);
        ctx.lineTo(v.max_x, y + 0.5);
        ctx.stroke();
    }
}

function draw() {
    // Clear the screen and correct for HDPI displays
    ctx.save();
    ctx.fillStyle = BACKGROUND_COL;
    ctx.fillRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);
    const v = view_rect();
    // Move so that the camera's origin is in the centre of the screen
    ctx.translate(Math.round(v.w / 2), Math.round(v.h / 2));
    ctx.translate(Math.round(-v.c_x), Math.round(-v.c_y));
    // Draw background grid
    draw_grid();
    // Draw all the fragments
    for (let f = 0; f < derived_state.annot_frags.length; f++) {
        draw_frag(derived_state.annot_frags[f]);
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
    // Request a frame to be drawn
    request_frame();
}

function on_mouse_move(e) {
    // Early return if no change has been made
    if (e.offsetX == 0 && e.offsetY == 0) {
        return;
    }
    if (is_button_pressed(e, BTN_MIDDLE)) {
        // Move the camera in the JS version
        view.view_x -= e.offsetX - mouse_coords.x;
        view.view_y -= e.offsetY - mouse_coords.y;
        request_frame();
    }
    mouse_coords.x = e.offsetX;
    mouse_coords.y = e.offsetY;
}

function on_mouse_up(e) {
    if (get_button(e) == BTN_MIDDLE) {
        // Only update the new view and sync when the user releases the button.  This makes sure
        // that we don't write cookies whenever the user moves their mouse.
        comp.set_view_loc(view.view_x, view.view_y);
        sync_view();
    }
}

function on_key_down(e) {
    // 'a' to add the first lead of Plain Bob as a new fragment to the comp
    if (e.key === 'a') {
        comp.add_frag();
        on_comp_change();
    }
    // 's' to [s]plit a fragment into two
    if (e.key === 's') {
        const hov_loc = mouse_hover_location();
        // Only try to split if the user is actually hovering over a fragment
        if (hov_loc.frag) {
            const split_index = Math.round(hov_loc.frag.row);
            // Make sure there's a 10px gap between the BBoxes of the two fragments (the `+ 1` takes
            // into account the existence of the leftover row)
            const new_y = derived_state.annot_frags[hov_loc.frag.index].y
                + (split_index + 1) * ROW_HEIGHT
                + FRAG_BBOX_EXTRA_HEIGHT * 2 + 10;

            // Split the fragment, and store the error
            const err = comp.split_frag(
                hov_loc.frag.index,
                split_index,
                new_y
            );
            // If the split failed, then log the error.  Otherwise, resync the composition with
            // `on_comp_change`.
            if (err) {
                console.error("Error splitting fragment: " + err);
            } else {
                on_comp_change();
            }
        }
    }
    // ctrl-z to undo
    if (e.key === 'z' && e.ctrlKey) {
        comp.undo();
        on_comp_change();
    }
    // shift-ctrl-Z or ctrl-y to redo
    if ((e.key === 'Z' && e.ctrlKey) || (e.key === 'y' && e.ctrlKey)) {
        comp.redo();
        on_comp_change();
    }
}

/* ===== HUD CODE ===== */

function on_part_head_change(evt) {
    // Update which part to display (indirectly so that we avoid divergence between Rust's
    // datastructures and their JS counterparts).
    comp.set_current_part(parseInt(evt.target.value));
    sync_view();
    request_frame();
}

function update_hud() {
    const stats = derived_state.stats;
    // Populate row counter
    const part_len = stats.part_len;
    const num_parts = derived_state.part_heads.length;
    document.getElementById("part-len").innerText = part_len.toString();
    document.getElementById("num-parts").innerText = num_parts.toString();
    document.getElementById("num-rows").innerText = (part_len * num_parts).toString();
    // Populate the falseness summary
    const falseness_info = document.getElementById("falseness-info");
    const num_false_rows = stats.num_false_rows;
    const num_false_groups = stats.num_false_groups;
    const is_true = num_false_rows === 0;
    falseness_info.innerText = is_true
        ? "true"
        : num_false_rows.toString() + " false rows in " + num_false_groups.toString() + " groups";
    falseness_info.style.color = is_true ? FALSE_COUNT_COL_TRUE : FALSE_COUNT_COL_FALSE;
    // Set the part chooser to the value specified in `view`
    document.getElementById("part-head").value = view.current_part;
}

function update_part_head_list() {
    let ph_list = document.getElementById("part-head");
    // Clear the existing children
    ph_list.innerHTML = '';
    // Add the new part heads
    for (var i = 0; i < derived_state.part_heads.length; i++) {
        let new_opt = document.createElement("option");
        new_opt.value = i.toString();
        let str = "#" + (i + 1).toString() + ": ";
        for (const j of derived_state.part_heads[i]) {
            str += BELL_NAMES[j];
        }
        new_opt.innerText = str;
        ph_list.appendChild(new_opt);
    }
}

/* ===== STARTUP CODE ===== */

function init_comp() {
    // Initialise the composition
    comp = Comp.example();
    // Read saved values from cookies
    let view = getCookie(COOKIE_NAME_VIEW);
    if (view) {
        comp.set_view_from_json(view);
    }
    // Update JS's local copies of the variables
    sync_derived_state();
    sync_view();
}

function start() {
    init_comp();
    // Set up the canvas variables
    canv = document.getElementById("comp-canvas");
    ctx = canv.getContext("2d");
    // Bind event listeners to all the things we need
    canv.addEventListener("mousemove", on_mouse_move);
    canv.addEventListener("mouseup", on_mouse_up);
    document.addEventListener("keydown", on_key_down);
    window.addEventListener("resize", on_window_resize);
    document.getElementById("part-head").addEventListener("change", on_part_head_change);
    // Force a load of updates to initialise the display
    on_window_resize();
    update_part_head_list();
    update_hud();
    request_frame();
}

/* ===== UTILITY FUNCTIONS/GETTERS ===== */

function get_button(e) {
    // Correct for the fact that `e.button` and `e.buttons` assign the buttons in different orders.
    // **FACEPALM**
    switch (e.button) {
        case 0:
            return BTN_LEFT;
        case 1:
            return BTN_MIDDLE;
        case 2:
            return BTN_RIGHT;
        default:
            console.warning("Unknown button value ", e.button);
    }
}

function is_button_pressed(e, button) {
    // Deal with Safari being ideosyncratic
    const button_mask = (e.buttons === undefined ? e.which : e.buttons);
    return (button_mask & (1 << button)) != 0;
}

// Returns the fragment underneath the cursor, along with a more precise indication of which
// row/column the mouse is hovering over
function mouse_hover_location() {
    // First, transform the mouse coords out of screen space and into world space
    const world_x = mouse_coords.x - canv.width / 2 + view.view_x;
    const world_y = mouse_coords.y - canv.height / 2 + view.view_y;
    // Now, perform a raycast through the fragments to detect any collisions.  We do this in the
    // opposite order to that which they are rendered, so that in the case of overlap the topmost
    // frag takes precidence.
    let hovered_frag = undefined;
    for (let i = derived_state.annot_frags.length - 1; i >= 0; i--) {
        const frag = derived_state.annot_frags[i];
        const bbox = frag_bbox(frag);
        // Skip this frag if the mouse is outside its bbox
        if (world_x < bbox.min_x || world_x > bbox.max_x
            || world_y < bbox.min_y || world_y > bbox.max_y) {
            continue;
        }
        // If we get to this point, this must be the topmost fragment that we are hovering over so
        // we calculate the row/col coordinate and break the loop so that hovered_frag doesn't get
        // overwritten.
        hovered_frag = {
            index: i,
            row: (world_y - frag.y) / ROW_HEIGHT,
            col: (world_x - frag.x) / COL_WIDTH,
        };
        break;
    }
    // Package the data and return
    return {
        world_x: world_x,
        world_y: world_y,
        frag: hovered_frag,
    };
}

function frag_bbox(f) {
    return new_rect(
        f.x - FALSENESS_BAR_WIDTH,
        f.y - FRAG_BBOX_EXTRA_HEIGHT,
        derived_state.stage * COL_WIDTH + FALSENESS_BAR_WIDTH * 2,
        f.exp_rows.length * ROW_HEIGHT + FRAG_BBOX_EXTRA_HEIGHT * 2,
    );
}

function view_rect() {
    return rect_from_centre(
        view.view_x,
        view.view_y,
        canv.width / devicePixelRatio,
        canv.height / devicePixelRatio,
    );
}

// Rect constructors
function rect_from_centre(c_x, c_y, w, h) {
    return new_rect(c_x - w / 2, c_y - h / 2, w, h);
}

function new_rect(x, y, w, h) {
    return {
        c_x: x + w / 2, c_y: y + h / 2,
        w: w, h: h,
        min_x: x,
        max_x: x + w,
        min_y: y,
        max_y: y + h
    };
}

// Called whenever the composition changes, and generates the necessary updates in order to get the
// user's view in sync with the new composition
function on_comp_change() {
    sync_derived_state();
    update_hud();
    request_frame();
}

function sync_derived_state() {
    derived_state = JSON.parse(comp.ser_derived_state());
}

// Make sure that both the local copy of `view` and the cookie are syncronised with Rust's view
// struct (which is regarded as the ground truth).
function sync_view() {
    const v = comp.ser_view();
    setCookie(COOKIE_NAME_VIEW, v);
    view = JSON.parse(v);
}
