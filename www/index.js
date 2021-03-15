/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

const BELL_NAMES = "1234567890ETABCDFGHJKLMNPQRSUVWXYZ";

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
const ERROR_COL = "red";

const BACKGROUND_COL = "white";
const GRID_COL = "#eee";
const GRID_SIZE = 200;

const DRAW_FRAG_LINK_LINES = true;
const FRAG_LINK_WIDTH = 2;
const FRAG_LINK_MIN_OPACITY = 0.15;
const FRAG_LINK_OPACITY_FALLOFF = 0.001;
const SELECTED_LINK_WIDTH_MULTIPLIER = 2;
const FRAG_LINK_SELECTION_DIST = 20;

const ROW_FONT = "20px monospace";
const UNPROVEN_ROW_OPACITY = 0.3;
const RULEOFF_LINE_WIDTH = 1;
const MUSIC_COL = "#5b5";
const MUSIC_ONIONSKIN_OPACITY = 0.13;

const FALSE_ROW_GROUP_NOTCH_WIDTH = 0.3;
const FALSE_ROW_GROUP_NOTCH_HEIGHT = 0.3;
const FALSE_ROW_GROUP_LINE_WIDTH = 3;
const FALSE_COUNT_COL_FALSE = "red";
const FALSE_COUNT_COL_TRUE = "green";

// Debug settings
const DBG_PROFILE_SERIALISE_STATE = false; // profile `sync_derived_state` in `start`?
const DBG_LOG_STATE_TRANSITIONS = false; // log to console whenever the UI changes state

/* ===== GLOBAL VARIABLES ===== */

// Variables set in the `start()` function
let canv, ctx;
let transpose_box, transpose_input, transpose_message;
// Global variable of the `link` that the user is 'selecting'.  This is recalculated every time the
// mouse moves, and is then cached and used in rendering and when deciding which fragments to join.
let selected_link = undefined;
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
    const opacity = row.is_proved === true ? 1 : UNPROVEN_ROW_OPACITY;
    // Set the font for the entire row
    ctx.font = ROW_FONT;
    // Bells
    ctx.textAlign = "center";
    for (let b = 0; b < stage; b++) {
        // Music highlighting
        if (row.music_highlights && row.music_highlights[b].length > 0) {
            // If some music happened in the part we're currently viewing, then set the alpha to 1,
            // otherwise make an 'onionskinning' effect of the music from other parts
            ctx.globalAlpha = (
                row.music_highlights[b].includes(view.current_part)
                    ? 1
                    : 1 - Math.pow(1 - MUSIC_ONIONSKIN_OPACITY, row.music_highlights[b].length)
            ) * opacity;
            ctx.fillStyle = MUSIC_COL;
            ctx.fillRect(x + COL_WIDTH * b, y, COL_WIDTH, ROW_HEIGHT);
        }
        // Text
        const bell_index = row.rows[view.current_part][b];
        if (!bell_lines[bell_index]) {
            ctx.globalAlpha = opacity;
            ctx.fillStyle = FOREGROUND_COL;
            ctx.fillText(BELL_NAMES[bell_index], x + COL_WIDTH * (b + 0.5), text_baseline);
        }
    }
    ctx.globalAlpha = opacity;
    // Call string
    if (row.call_str) {
        ctx.textAlign = "right";
        ctx.fillStyle = FOREGROUND_COL;
        ctx.fillText(row.call_str, x - FALSENESS_BAR_WIDTH, text_baseline);
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
    const x = Math.round(frag.x);
    const y = Math.round(frag.y);
    const rect = frag_bbox(frag);
    // Background box (to overlay over the grid)
    ctx.fillStyle = BACKGROUND_COL;
    ctx.fillRect(rect.min_x, rect.min_y, rect.w, rect.h);
    // Rows
    for (let i = 0; i < frag.exp_rows.length; i++) {
        draw_row(x, y + ROW_HEIGHT * i, frag.exp_rows[i]);
    }
    // Draw Lines
    // Fade out the lines if this fragment isn't getting proven
    if (!frag.is_proved) {
        ctx.globalAlpha = UNPROVEN_ROW_OPACITY;
    }
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
    ctx.globalAlpha = 1;
    // Falseness
    ctx.lineWidth = FALSE_ROW_GROUP_LINE_WIDTH;
    for (let i = 0; i < frag.false_row_ranges.length; i++) {
        const range = frag.false_row_ranges[i];
        // Draw the lines
        ctx.strokeStyle = group_col(range.group);
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
    // Link group lines
    ctx.lineWidth = FRAG_LINK_WIDTH;
    if (frag.link_group_top !== undefined) {
        ctx.strokeStyle = group_col(frag.link_group_top);
        ctx.beginPath();
        ctx.moveTo(rect.min_x, rect.min_y);
        ctx.lineTo(rect.max_x, rect.min_y);
        ctx.stroke();
    }
    if (frag.link_group_bottom !== undefined) {
        ctx.strokeStyle = group_col(frag.link_group_bottom);
        ctx.beginPath();
        ctx.moveTo(rect.min_x, rect.max_y);
        ctx.lineTo(rect.max_x, rect.max_y);
        ctx.stroke();
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

function draw_link(link, is_selected) {
    const l = frag_link_line(link);
    // Calculate the opacity of the line from its length
    const length = Math.sqrt(Math.pow(l.to_x - l.from_x, 2) + Math.pow(l.to_y - l.from_y, 2));
    ctx.globalAlpha = FRAG_LINK_MIN_OPACITY
        + (1 - FRAG_LINK_MIN_OPACITY) * Math.exp(-length * FRAG_LINK_OPACITY_FALLOFF);
    // Draw the line
    ctx.strokeStyle = group_col(link.group);
    ctx.lineWidth = FRAG_LINK_WIDTH * (is_selected ? SELECTED_LINK_WIDTH_MULTIPLIER : 1);
    ctx.beginPath();
    ctx.moveTo(l.from_x, l.from_y);
    ctx.lineTo(l.to_x, l.to_y);
    ctx.stroke();
    // Reset global alpha for the next things to render
    ctx.globalAlpha = 1;
}

function draw() {
    /* TRANSFORM CANVAS SO THAT WE CAN USE WORLD SPACE COORDINATES */

    // Clear the screen and correct for HDPI displays
    ctx.save();
    ctx.fillStyle = BACKGROUND_COL;
    ctx.fillRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);
    const v = view_rect();
    // Move so that the camera's origin is in the centre of the screen
    ctx.translate(Math.round(v.w / 2), Math.round(v.h / 2));
    ctx.translate(Math.round(-v.c_x), Math.round(-v.c_y));

    /* DRAW EVERYTHING IN WORLD SPACE */

    // Draw background grid
    draw_grid();
    // Draw all the fragment links
    for (let i = 0; i < derived_state.frag_links.length; i++) {
        draw_link(derived_state.frag_links[i], i === selected_link);
    }
    // Draw all the fragments
    for (let f = 0; f < derived_state.annot_frags.length; f++) {
        draw_frag(derived_state.annot_frags[f]);
    }

    /* RESTORE CANVAS */

    // Reset the canvas' transform matrix so that the next frame is rendered correctly
    ctx.restore();
}

function frame() {
    draw();
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
    if (comp.is_state_idle()) {
        // Repaint the screen if the selected link has changed because we moved the mouse
        let closest_link = closest_frag_link_to_cursor();
        if (closest_link !== selected_link) {
            selected_link = closest_link;
            request_frame();
        }
    }
    // If we clicked on a fragment, then move it
    if (comp.is_state_dragging()) {
        const frag_being_dragged = comp.frag_being_dragged();
        // Note that in this case, we allow `derived_state` to get out of sync with Rust's ground
        // truth.  We do this for performance reasons; if we didn't, then the whole composition
        // would be reproved every time the mouse is moved causing horrendous lag.
        derived_state.annot_frags[frag_being_dragged].x += e.offsetX - mouse_coords.x;
        derived_state.annot_frags[frag_being_dragged].y += e.offsetY - mouse_coords.y;
        // Request a repaint because so that the new frag position appears on screen
        request_frame();
    }
    // Regardless of state, middle click should pan the camera
    if (is_button_pressed(e, BTN_MIDDLE)) {
        view.view_x -= e.offsetX - mouse_coords.x;
        view.view_y -= e.offsetY - mouse_coords.y;
        request_frame();
    }
    // Update the global variables storing the mouse coords (these will also be used to diff against
    // next time the mouse moves).
    mouse_coords.x = e.offsetX;
    mouse_coords.y = e.offsetY;
}

function on_mouse_down(e) {
    // Only handle mouse down events if the user is not already performing an action
    if (comp.is_state_idle()) {
        const frag = hovered_frag();
        // Left-clicking a fragment should switch the UI into the dragging state
        if (get_button(e) === BTN_LEFT && frag) {
            comp.start_dragging(frag.index);
            if (DBG_LOG_STATE_TRANSITIONS) {
                console.log(`State change: Idle -> Dragging(${frag.index})`);
            }
        }
    }
}

function on_mouse_up(e) {
    // If we have just released a fragment, then update Rust's 'ground truth' and force a resync
    // of JS's local copy of the state.  Also let go of whatever we were dragging.
    if (comp.is_state_dragging() && get_button(e) === BTN_LEFT) {
        const released_frag = derived_state.annot_frags[comp.frag_being_dragged()];
        comp.finish_dragging(released_frag.x, released_frag.y);
        on_comp_change();
        if (DBG_LOG_STATE_TRANSITIONS) {
            console.log("State change: Dragging -> Idle");
        }
    }
    if (get_button(e) === BTN_MIDDLE) {
        comp.set_view_coords(view.view_x, view.view_y);
        sync_view();
    }
}

function on_key_down(e) {
    if (comp.is_state_transposing()) {
        // Esc should exit transpose mode
        if (e.keyCode == 27) {
            comp.exit_transposing();
            stop_transposing();
        }
    }
    // Keyboard shortcuts can only be used if the UI is 'idle' - i.e. the user is not dragging or
    // transposing frags etc.
    if (comp.is_state_idle()) {
        // Detect which fragment is under the cursor
        const frag = hovered_frag();
        const cursor_pos = world_space_cursor_pos();
        // add a lead of Plain Bob as a new fragment to the comp
        if (e.key === 'a' || e.key === 'A') {
            const adding_full_course = e.key === 'A';
            const new_frag_ind = comp.add_frag(cursor_pos.x, cursor_pos.y, adding_full_course);
            on_comp_change();
            // Immediately enter transposing mode to let the user specify what course they wanted
            start_transposition(new_frag_ind, 0);
            e.preventDefault();
        }
        // 'cut' a fragment into two at the mouse location
        if (e.key === 'x' && frag) {
            const split_index = Math.round(frag.row);
            // Make sure there's a 10px gap between the BBoxes of the two fragments (we add 1 to
            // `split_index` to take into account the existence of the leftover row)
            const new_y = derived_state.annot_frags[frag.index].y
                + (split_index + 1) * ROW_HEIGHT
                + FRAG_BBOX_EXTRA_HEIGHT * 2 + 10;
            // Split the fragment, and store the error string
            const err = comp.split_frag(
                frag.index,
                split_index,
                new_y
            );
            // If the split failed, then log the error.  Otherwise, resync the composition with
            // `on_comp_change`.
            if (err) {
                console.warn("Error splitting fragment: " + err);
            } else {
                on_comp_change();
            }
        }
        // mute/unmute a fragment
        if (e.key === 's' && frag) {
            comp.toggle_frag_mute(frag.index);
            on_comp_change();
        }
        // solo/unsolo a fragment
        if (e.key === 'S' && frag) {
            comp.toggle_frag_solo(frag.index);
            on_comp_change();
        }
        // transpose a fragment by its start row
        if (e.key === 't' && frag) {
            start_transposition(frag.index, 0);
            // Prevent this event causing the user to type 't' into the newly focussed transposition box
            e.preventDefault()
        }
        // transpose a fragment by the hovered row
        if (e.key === 'T' && frag) {
            start_transposition(frag.index, Math.floor(frag.row));
            // Prevent this event causing the user to type 'T' into the newly focussed transposition box
            e.preventDefault()
        }
        // reset the composition (ye too dangerous I know but good enough for now)
        if (e.key === 'R') {
            comp.reset();
            on_comp_change();
        }
        // delete the fragment under the cursor (ye too dangerous I know but good enough for now)
        if (e.key === 'd' && frag) {
            comp.delete_frag(frag.index);
            on_comp_change();
        }
        // join the first frag 1 onto frag 0, but only if we aren't hovering a fragment
        if (e.key === 'j' && selected_link !== undefined) {
            const link_to_join = derived_state.frag_links[selected_link];
            comp.join_frags(link_to_join.from, link_to_join.to);
            on_comp_change();
        }
        // ctrl-z or simply z to undo (of course)
        if (e.key === 'z') {
            comp.undo();
            on_comp_change();
        }
        // shift-ctrl-Z or ctrl-y to redo
        if (e.key === 'Z' || (e.key === 'y' && e.ctrlKey)) {
            comp.redo();
            on_comp_change();
        }
    }
}

/* ===== TRANSPOSE MODE ===== */

function start_transposition(frag_index, row_index) {
    // Switch to the transposing state
    const current_first_row = comp.start_transposing(frag_index, row_index);
    if (DBG_LOG_STATE_TRANSITIONS) {
        console.log(`State change: Idle -> Transposing(${frag_index}:${row_index})`);
    }
    // Initialise the transpose box
    transpose_box.style.display = "block";
    transpose_box.style.left = mouse_coords.x.toString() + "px";
    transpose_box.style.top = mouse_coords.y.toString() + "px";
    transpose_input.value = current_first_row;
    transpose_input.focus();
    // Initialise the error message
    on_transpose_box_change();
}

function on_transpose_box_change() {
    const row_err = comp.row_parse_err(transpose_input.value);
    const success = row_err === "";
    transpose_message.style.color = success ? FOREGROUND_COL : ERROR_COL;
    transpose_message.innerText = success
        ? "Press 'enter' to transpose."
        : row_err;
}

function on_transpose_box_key_down(e) {
    // Early return if the user pressed anything other than enter
    if (e.keyCode != 13) {
        return;
    }
    if (comp.is_state_transposing() && comp.finish_transposing(transpose_input.value)) {
        if (DBG_LOG_STATE_TRANSITIONS) {
            console.log(`State change: Transposing -> Idle`);
        }
        stop_transposing();
    }
}

function stop_transposing() {
    // Update the display to handle the changes
    transpose_box.style.display = "none";
    on_comp_change();
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
    // Grab HTML elements we'll use a lot
    transpose_box = document.getElementById("transpose-box");
    transpose_input = document.getElementById("transpose-input");
    transpose_message = document.getElementById("transpose-message");
    // Bind event listeners to all the things we need
    canv.addEventListener("mousemove", on_mouse_move);
    canv.addEventListener("mousedown", on_mouse_down);
    canv.addEventListener("mouseup", on_mouse_up);
    document.addEventListener("keydown", on_key_down);
    window.addEventListener("resize", on_window_resize);
    document.getElementById("part-head").addEventListener("change", on_part_head_change);
    transpose_input.addEventListener("keyup", on_transpose_box_change);
    transpose_input.addEventListener("keydown", on_transpose_box_key_down);
    // Force a load of updates to initialise the display
    on_window_resize();
    update_part_head_list();
    update_hud();
    request_frame();

    // Time how long it takes to sync the derived state
    if (DBG_PROFILE_SERIALISE_STATE) {
        console.time("Sync derived state");
        for (let i = 0; i < 1000; i++) {
            sync_derived_state();
        }
        console.timeEnd("Sync derived state");
    }
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

function world_space_cursor_pos() {
    // First, transform the mouse coords out of screen space and into world space
    return {
        x: mouse_coords.x - canv.width / 2 + view.view_x,
        y: mouse_coords.y - canv.height / 2 + view.view_y
    };
}

// Returns the fragment underneath the cursor, along with a more precise indication of which
// row/column the mouse is hovering over
function hovered_frag() {
    let c = world_space_cursor_pos();
    // Now, perform a raycast through the fragments to detect any collisions.  We do this in the
    // opposite order to that which they are rendered, so that in the case of overlap the topmost
    // frag takes precidence.
    let hov_frag = undefined;
    for (let i = derived_state.annot_frags.length - 1; i >= 0; i--) {
        const frag = derived_state.annot_frags[i];
        const bbox = frag_bbox(frag);
        // Skip this frag if the mouse is outside its bbox
        if (c.x < bbox.min_x || c.x > bbox.max_x || c.y < bbox.min_y || c.y > bbox.max_y) {
            continue;
        }
        // If we get to this point, this must be the topmost fragment that we are hovering over so
        // we calculate the row/col coordinate and break the loop so that hovered_frag doesn't get
        // overwritten.
        hov_frag = {
            index: i,
            row: (c.y - frag.y) / ROW_HEIGHT,
            col: (c.x - frag.x) / COL_WIDTH,
        };
        break;
    }
    return hov_frag;
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

function frag_link_line(link) {
    // Calculate bboxes of the frags we're joining
    const bbox_from = frag_bbox(derived_state.annot_frags[link.from]);
    const bbox_to = frag_bbox(derived_state.annot_frags[link.to]);
    // If the boxes are offset left/right, then join them up using the shortest possible distance
    if (bbox_from.max_x < bbox_to.min_x) {
        var from_x = bbox_from.max_x;
        var to_x = bbox_to.min_x;
    } else if (bbox_from.min_x > bbox_to.max_x) {
        var from_x = bbox_from.min_x;
        var to_x = bbox_to.max_x;
    } else {
        var from_x = bbox_from.c_x;
        var to_x = bbox_to.c_x;
    }
    const from_y = bbox_from.max_y;
    const to_y = bbox_to.min_y;
    // Return the computed lines
    return {
        from_x: from_x,
        from_y: from_y,
        to_x: to_x,
        to_y: to_y,
    };
}

function closest_frag_link_to_cursor() {
    const c = world_space_cursor_pos();
    // Find nearest link to the cursor
    let best_distance = Infinity;
    let closest_link_ind = undefined;
    for (let i = 0; i < derived_state.frag_links.length; i++) {
        const link = derived_state.frag_links[i];
        // If the link goes to and from the same fragment, then reject it because it can't be joined
        if (link.from == link.to)
            continue;
        // Find which points the line will be drawn between
        const l = frag_link_line(link);
        // Calculate the direction vector of the line
        const d = {
            x: l.to_x - l.from_x,
            y: l.to_y - l.from_y
        };
        // Calculate the square length of the line
        const square_length = d.x * d.x + d.y * d.y;
        // Calculate how far along the line the closest point to the cursor should be (as a
        // proportion where 0 is `(from_x, from_y)` and 1 is `(to_x, to_y)`).
        //
        // This formula is equivalent to `((cursor - from) . d) / (d . d)`, where (d . d) is the
        // square length of `d`
        let lambda = ((c.x - l.from_x) * d.x + (c.y - l.from_y) * d.y) / square_length;
        // Clamp to 0 <= lambda <= 1 so that our closest point lies along the line _segment_ not the
        // entire line
        lambda = Math.max(0, Math.min(1, lambda));
        // Calculate the vector of (the point on the line segment represented by lambda) -> cursor
        let pt_to_cursor = {
            x: c.x - (l.from_x * (1 - lambda) + l.to_x * lambda),
            y: c.y - (l.from_y * (1 - lambda) + l.to_y * lambda),
        };
        // Calculate the length of pt_to_cursor
        let dist_from_cursor = Math.sqrt(
            pt_to_cursor.x * pt_to_cursor.x + pt_to_cursor.y * pt_to_cursor.y
        );
        // If we get a closer distance, then set this as the best link (so far)
        if (dist_from_cursor <= FRAG_LINK_SELECTION_DIST && dist_from_cursor < best_distance) {
            best_distance = dist_from_cursor;
            closest_link_ind = i;
        }
    }
    return closest_link_ind;
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
