/* ===== GLOBAL VALUES ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

/* === Commonly used HTML elements === */

// Transpose box
const elem_transpose_box = document.getElementById("transpose-box");
const elem_transpose_input = document.getElementById("transpose-input");
const elem_transpose_message = document.getElementById("transpose-message");
// Stats
const elem_part_len = document.getElementById("part-len");
const elem_num_parts = document.getElementById("num-parts");
const elem_num_rows = document.getElementById("num-rows");
const elem_falseness_info = document.getElementById("falseness-info");
// Part heads
const elem_part_head_input = document.getElementById("part-head-input");
const elem_part_head_list = document.getElementById("part-head");
const elem_part_head_message = document.getElementById("part-head-message");
const elem_part_head_is_group = document.getElementById("part-head-group-message");
// Right sidebar
const elem_right_sidebar = document.getElementById("right-sidebar");
const elem_sections = find_section_fold_elems(["general", "methods", "calls", "music"]);

const elem_num_methods = document.getElementById("num-methods");
const elem_method_box = document.getElementById("method-list");
const elem_selected_method = document.getElementById("selected-method");

const elem_num_calls = document.getElementById("num-calls");
const elem_call_readout = document.getElementById("call-readout");
const elem_selected_call = document.getElementById("selected-call");
// Templates
const template_method_entry = document.getElementById("template-method-entry");
// Canvas elements
const canv = document.getElementById("comp-canvas");
const ctx = canv.getContext("2d");

/* ===== CONSTANTS ===== */

const BELL_NAMES = "1234567890ETABCDFGHJKLMNPQRSUVWXYZ";

// IDs of mouse buttons
const BTN_LEFT = 0;
const BTN_RIGHT = 1;
const BTN_MIDDLE = 2;

// Cookie names
const COOKIE_NAME_VIEW = "view";

// How many pixels off the edge of the screen the viewport culling will happen
const VIEW_CULLING_EXTRA_SIZE = 20;

const FOLD_BUTTON_TRIANGLE_CLOSED = "▶";
const FOLD_BUTTON_TRIANGLE_OPEN = "▼";

/* ===== DISPLAY CONSTANTS ===== */

const COL_WIDTH = 12; // px
const ROW_HEIGHT = 16; // px
const FOLD_COL_WIDTH = COL_WIDTH * 1;
const FALSENESS_COL_WIDTH = COL_WIDTH * 1;
const FRAG_BBOX_EXTRA_HEIGHT = ROW_HEIGHT * (5 / 16);

const FOREGROUND_COL = "black";
const ERROR_COL = "red";

const BACKGROUND_COL = "white";
const GRID_COL = "#eee";
const GRID_SIZE = 200; // px

const DRAW_FRAG_LINK_LINES = true;
const FRAG_LINK_WIDTH = 1.5; // px
const FRAG_LINK_MIN_OPACITY = 0.15;
const FRAG_LINK_OPACITY_FALLOFF = 0.001;
const FRAG_LINK_SELECTED_WIDTH_MULTIPLIER = 2; // as a multiple of FRAG_LINK_WIDTH
const FRAG_LINK_SELECTION_DIST = 20; // px

const ROW_FONT = "monospace";
const ROW_FONT_SIZE_MULTIPLIER = 0.9; // as a multiple of ROW_HEIGHT
const ROW_FONT_BASELINE_LEVEL = 0.32; // moves font baseline down to make it look central
const UNPROVEN_ROW_OPACITY = 0.3;
const RULEOFF_LINE_WIDTH = 1; // px
const MUSIC_COL = "#5b5";
const MUSIC_ONIONSKIN_OPACITY = 0.6;

const FALSE_ROW_GROUP_NOTCH_WIDTH = 0.3;
const FALSE_ROW_GROUP_NOTCH_HEIGHT = 0.3; // multiple of the falseness margin width
const FALSE_ROW_GROUP_LINE_WIDTH = 2.5; // px
const FALSE_COUNT_COL_FALSE = "red";
const FALSE_COUNT_COL_TRUE = "green";

// Debug settings
const DBG_PROFILE_SERIALISE_STATE = false; // profile `sync_derived_state` in `start`?
const DBG_LOG_STATE_TRANSITIONS = false; // log to console whenever the UI changes state

/* ===== GLOBAL VARIABLES ===== */

// Global variable of the `link` that the user is 'selecting'.  This is recalculated every time the
// mouse moves, and is then cached and used in rendering and when deciding which fragments to join.
let selected_link = undefined;
// Variables which will used to sync with the Rust code (in 99% of the code, these should be treated
// as immutable - they should only be mutated from `sync_derived_state`, `sync_derived_state` and
// `start` or when allowed to get out-of-sync whilst the user either pans or drags fragments).
let comp, derived_state, view;
// Mouse variables that the browser should keep track of but doesn't
let mouse_coords = { x: 0, y: 0 };

/* THINGS THAT SHOULD BE USER CONFIG BUT CURRENTLY ARE GLOBAL VARS */

// What widths and colours should be assigned to bells.  There is no way to render both bell names
// and lines at the same time (because IMO it looks awful)
let bell_lines = {
    0: [1, "red"],
    7: [2, "blue"],
};

/* ===== DRAWING CODE ===== */

function draw_row(x, y, row) {
    const v = view_rect();
    // Don't draw if the row is going to be off the screen
    if (y < v.min_y - VIEW_CULLING_EXTRA_SIZE || y > v.max_y + VIEW_CULLING_EXTRA_SIZE) {
        return;
    }
    const opacity = row.is_proved === true ? 1 : UNPROVEN_ROW_OPACITY;
    // Calculate some useful values
    const stage = derived_state.stage;
    const text_baseline =
        y + ROW_HEIGHT * (0.5 + ROW_FONT_BASELINE_LEVEL * ROW_FONT_SIZE_MULTIPLIER);
    const right = x + COL_WIDTH * stage;
    // Set the font for the entire row
    ctx.font = `${Math.round(ROW_HEIGHT * ROW_FONT_SIZE_MULTIPLIER)}px ${ROW_FONT}`;
    // Bells
    ctx.textAlign = "center";
    for (let b = 0; b < stage; b++) {
        // Music highlighting
        if (row.music_highlights && row.music_highlights[b].length > 0) {
            // If some music happened in the part we're currently viewing, then set the alpha to 1,
            // otherwise make an 'onionskinning' effect of the music from other parts
            ctx.globalAlpha =
                (row.music_highlights[b].includes(view.current_part)
                    ? 1
                    : 1 -
                      Math.pow(
                          1 - MUSIC_ONIONSKIN_OPACITY,
                          row.music_highlights[b].length / derived_state.part_heads.rows.length
                      )) * opacity;
            ctx.fillStyle = MUSIC_COL;
            ctx.fillRect(x + COL_WIDTH * b, y, COL_WIDTH, ROW_HEIGHT);
        }
        // Text
        const bell_index = row.rows[view.current_part][b];
        const is_folded = row.fold ? !row.fold.is_open : false;
        const line = bell_lines[bell_index];
        if (!line || is_folded) {
            ctx.globalAlpha = opacity;
            ctx.fillStyle = line ? line[1] : FOREGROUND_COL;
            ctx.fillText(BELL_NAMES[bell_index], x + COL_WIDTH * (b + 0.5), text_baseline);
        }
    }
    // All the annotations should be rendered with the foreground colour, and have the opacity
    // applied to them
    ctx.globalAlpha = opacity;
    ctx.fillStyle = FOREGROUND_COL;
    // Call string
    if (row.call_strings) {
        ctx.textAlign = "right";
        ctx.fillText(
            row.call_strings[view.current_part],
            x - FALSENESS_COL_WIDTH - FOLD_COL_WIDTH,
            text_baseline
        );
    }
    // Fold triangle
    if (row.fold) {
        // We make the triangle use up 80% of the available space
        const tri_radius = Math.min(ROW_HEIGHT, FOLD_COL_WIDTH) * 0.4;
        // For simplicity, we render the triangle using canvas translation and rotation (and
        // therefore we have to save/restore the old transformation matrix so that the effects only
        // apply to the triangle).
        ctx.save();
        ctx.translate(x - FALSENESS_COL_WIDTH - FOLD_COL_WIDTH / 2, y + ROW_HEIGHT / 2);
        if (row.fold.is_open) ctx.rotate(Math.PI / 2);
        // Draw the triangle pointing right
        ctx.beginPath();
        ctx.moveTo(tri_radius, 0);
        for (let i = 1; i <= 3; i++) {
            const angle = (i / 3.0) * (Math.PI * 2);
            ctx.lineTo(tri_radius * Math.cos(angle), tri_radius * Math.sin(angle));
        }
        ctx.fill();
        // Reset the canvas
        ctx.restore();
    }
    // Method string
    if (row.method_string) {
        ctx.textAlign = "left";
        ctx.fillText(row.method_string, x + stage * COL_WIDTH + FALSENESS_COL_WIDTH, text_baseline);
    }
    ctx.globalAlpha = 1;
    // Ruleoff
    if (row.is_ruleoff) {
        // Subtracting the tiny number here encourages the line rounding to round down rather than
        // up and thus (where possible) avoid rendering the next row's music highlights on top of
        // the ruleoff.
        //
        // TODO: Render all the highlights in one pass before rendering the foreground.
        const ruleoff_y = round_line_coord(y + ROW_HEIGHT - 0.00001);
        ctx.beginPath();
        ctx.moveTo(x, ruleoff_y);
        ctx.lineTo(right, ruleoff_y);
        ctx.strokeStyle = FOREGROUND_COL;
        ctx.lineWidth = RULEOFF_LINE_WIDTH;
        ctx.stroke();
    }
}

function draw_falseness_indicator(unrounded_x, min_y, max_y, notch_width, notch_height) {
    const x = round_line_coord(unrounded_x);
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
    for (let i = 0; i < frag.rows.length; i++) {
        draw_row(x, y + ROW_HEIGHT * i, frag.rows[i]);
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
        for (let i = 0; i < frag.rows.length; i++) {
            const ind = frag.rows[i].rows[view.current_part].findIndex((x) => x == l);
            ctx.lineTo(round_line_coord(x + (ind + 0.5) * COL_WIDTH), y + ROW_HEIGHT * (i + 0.5));
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
            x + FALSENESS_COL_WIDTH * -0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            FALSENESS_COL_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
        draw_falseness_indicator(
            x + derived_state.stage * COL_WIDTH + FALSENESS_COL_WIDTH * 0.5,
            y + ROW_HEIGHT * range.start,
            y + ROW_HEIGHT * (range.end + 1),
            -FALSENESS_COL_WIDTH * FALSE_ROW_GROUP_NOTCH_WIDTH,
            ROW_HEIGHT * FALSE_ROW_GROUP_NOTCH_HEIGHT
        );
    }
    // Link group lines
    ctx.lineWidth = FRAG_LINK_WIDTH;
    if (frag.link_groups.top !== undefined) {
        const line_y = round_line_coord(rect.min_y);
        ctx.strokeStyle = group_col(frag.link_groups.top);
        ctx.beginPath();
        ctx.moveTo(rect.min_x, line_y);
        ctx.lineTo(rect.max_x, line_y);
        ctx.stroke();
    }
    if (frag.link_groups.bottom !== undefined) {
        const line_y = round_line_coord(rect.max_y);
        ctx.strokeStyle = group_col(frag.link_groups.bottom);
        ctx.beginPath();
        ctx.moveTo(rect.min_x, line_y);
        ctx.lineTo(rect.max_x, line_y);
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
        ctx.moveTo(round_line_coord(x), v.min_y);
        ctx.lineTo(round_line_coord(x), v.max_y);
        ctx.stroke();
    }
    // Horizontal bars
    for (let y = Math.ceil(v.min_y / GRID_SIZE) * GRID_SIZE; y < v.max_y; y += GRID_SIZE) {
        ctx.beginPath();
        ctx.moveTo(v.min_x, round_line_coord(y));
        ctx.lineTo(v.max_x, round_line_coord(y));
        ctx.stroke();
    }
}

function draw_link(link, is_selected) {
    const l = frag_link_line(link);
    // Calculate the opacity of the line from its length
    const length = Math.sqrt(Math.pow(l.to_x - l.from_x, 2) + Math.pow(l.to_y - l.from_y, 2));
    ctx.globalAlpha =
        FRAG_LINK_MIN_OPACITY +
        (1 - FRAG_LINK_MIN_OPACITY) * Math.exp(-length * FRAG_LINK_OPACITY_FALLOFF);
    // Draw the line
    ctx.strokeStyle = group_col(link.group);
    ctx.lineWidth = FRAG_LINK_WIDTH * (is_selected ? FRAG_LINK_SELECTED_WIDTH_MULTIPLIER : 1);
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
    for (let f = 0; f < derived_state.frags.length; f++) {
        draw_frag(derived_state.frags[f]);
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
        derived_state.frags[frag_being_dragged].x += e.offsetX - mouse_coords.x;
        derived_state.frags[frag_being_dragged].y += e.offsetY - mouse_coords.y;
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
            log_state_transition("Idle", `Dragging(${frag.index})`);
        }
    }
}

function on_mouse_up(e) {
    // If we have just released a fragment, then update Rust's 'ground truth' and force a resync
    // of JS's local copy of the state.  Also let go of whatever we were dragging.
    if (comp.is_state_dragging() && get_button(e) === BTN_LEFT) {
        const released_frag = derived_state.frags[comp.frag_being_dragged()];
        comp.finish_dragging(released_frag.x, released_frag.y);
        on_comp_change();
        log_state_transition("Dragging", "Idle");
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
        const selected_method = parseInt(elem_selected_method.value);
        // Add more rows to the composition
        if (e.key === "a" || e.key === "A") {
            // Decide whether or not to insert a full course
            const adding_full_course = e.key === "A";
            // Decide how to add the rows
            if (
                frag !== undefined &&
                Math.floor(frag.row) == derived_state.frags[frag.index].rows.length - 1
            ) {
                // Case 1: we're hovering over the leftover row of a fragment.  In this case, we
                // add the new chunk onto the end of the existing one
                comp.extend_frag(frag.index, selected_method, adding_full_course);
                on_comp_change();
            } else {
                // Case 2: we're not hovering over the end of a fragment, so we add the course and
                // switch to transposing mode so that the user can decide what row to start with
                const new_frag_ind = comp.add_frag(
                    cursor_pos.x,
                    cursor_pos.y,
                    selected_method,
                    adding_full_course
                );
                on_comp_change();
                // Immediately enter transposing mode to let the user specify what course they wanted
                start_transposition(new_frag_ind, 0);
            }
            // TODO: Figure out why this is necessary...
            e.preventDefault();
        }
        // 'cut' a fragment into two at the mouse location
        if (e.key === "x" && frag) {
            const split_index = frag.source_range.start;
            // Make sure there's a 10px gap between the BBoxes of the two fragments (we add 1 to
            // `split_index` to take into account the existence of the leftover row)
            const new_y =
                derived_state.frags[frag.index].y +
                (Math.floor(frag.row) + 1) * ROW_HEIGHT +
                FRAG_BBOX_EXTRA_HEIGHT * 2 +
                10;
            // Split the fragment, and store the error string
            const err = comp.split_frag(frag.index, split_index, new_y);
            // If the split failed, then log the error.  Otherwise, resync the composition with
            // `on_comp_change`.
            if (err) {
                console.warn("Error splitting fragment: " + err);
            } else {
                on_comp_change();
            }
        }
        // mute/unmute a fragment
        if (e.key === "s" && frag) {
            comp.toggle_frag_mute(frag.index);
            on_comp_change();
        }
        // solo/unsolo a fragment
        if (e.key === "S" && frag) {
            comp.toggle_frag_solo(frag.index);
            on_comp_change();
        }
        // transpose a fragment by its start row
        if (e.key === "t" && frag) {
            start_transposition(frag.index, 0);
            // Prevent this event causing the user to type 't' into the newly focussed transposition box
            e.preventDefault();
        }
        // transpose a fragment by the hovered row
        if (e.key === "T" && frag) {
            start_transposition(frag.index, frag.source_range.start);
            // Prevent this event causing the user to type 'T' into the newly focussed transposition box
            e.preventDefault();
        }
        // delete the fragment under the cursor
        if (e.key === "d" && frag) {
            comp.delete_frag(frag.index);
            on_comp_change();
        }
        // join two fragments if we're hovering the link between them
        if (e.key === "c" && selected_link !== undefined) {
            const link_to_join = derived_state.frag_links[selected_link];
            comp.join_frags(link_to_join.from, link_to_join.to);
            on_comp_change();
        }
        // set call under the cursor
        if (e.key === "e" && frag) {
            const err = comp.set_call(
                frag.index,
                frag.source_range.start,
                parseInt(elem_selected_call.value)
            );
            if (err) {
                console.warn("Error setting call: " + err);
            } else {
                on_comp_change();
            }
        }
        // Fold the fragment under the cursor
        if (e.key === "f" && frag) {
            comp.toggle_lead_fold(frag.index, frag.source_range.start);
            on_comp_change();
        }
        // reset the composition
        if (e.key === "R") {
            comp.reset();
            on_comp_change();
        }
        // ctrl-z or simply z to undo (of course)
        if (e.key === "z") {
            comp.undo();
            on_comp_change();
        }
        // shift-ctrl-Z or Z or ctrl-y to redo
        if (e.key === "Z" || (e.key === "y" && e.ctrlKey)) {
            comp.redo();
            on_comp_change();
        }
    }
}

/* ===== TRANSPOSE MODE ===== */

function start_transposition(frag_index, row_index) {
    // Switch to the transposing state
    const current_first_row = comp.start_transposing(frag_index, row_index);
    log_state_transition("Idle", `Transposing(${frag_index}:${row_index})`);
    // Initialise the transpose box
    elem_transpose_box.style.display = "block";
    elem_transpose_box.style.left = mouse_coords.x.toString() + "px";
    elem_transpose_box.style.top = mouse_coords.y.toString() + "px";
    elem_transpose_input.value = current_first_row;
    elem_transpose_input.focus();
    // Initialise the error message
    on_transpose_box_change();
}

function on_transpose_box_change() {
    const row_err = comp.try_parse_transpose_row(elem_transpose_input.value);
    const success = row_err === "";
    elem_transpose_message.style.color = success ? FOREGROUND_COL : ERROR_COL;
    elem_transpose_message.innerText = success ? "Press 'enter' to finish." : row_err;
    // If the transposition was successful, then the composition we're viewing will have changed, so
    // we update the screen
    if (success) on_comp_change();
}

function on_transpose_box_key_down(e) {
    // Early return if the user pressed anything other than enter
    if (e.keyCode != 13) {
        return;
    }
    if (comp.is_state_transposing() && comp.finish_transposing(elem_transpose_input.value)) {
        log_state_transition("Transposing", "Idle");
        stop_transposing();
    }
}

function stop_transposing() {
    // Update the display to handle the changes
    elem_transpose_box.style.display = "none";
    on_comp_change();
}

/* ===== HUD CODE ===== */

// TODO: Clean these callbacks up

function on_part_change(evt) {
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
    const num_parts = derived_state.part_heads.rows.length;
    elem_part_len.innerText = part_len.toString();
    elem_num_parts.innerText = num_parts.toString();
    elem_num_rows.innerText = (part_len * num_parts).toString();

    // Populate the falseness summary
    const num_false_rows = stats.num_false_rows;
    const num_false_groups = stats.num_false_groups;
    const is_true = num_false_rows === 0;
    elem_falseness_info.innerText = is_true
        ? "true"
        : num_false_rows.toString() + " false rows in " + num_false_groups.toString() + " groups";
    elem_falseness_info.style.color = is_true ? FALSE_COUNT_COL_TRUE : FALSE_COUNT_COL_FALSE;

    // Update the part head display(s)
    elem_part_head_list.value = view.current_part;
    elem_part_head_input.value = derived_state.part_heads.spec;
    elem_part_head_message.innerText = `Parses to ${num_parts} part${num_parts == 1 ? "" : "s"}.`;
    elem_part_head_message.style.color = FOREGROUND_COL;
    elem_part_head_is_group.style.display = derived_state.part_heads.is_group ? "none" : "block";
}

function update_part_head_list() {
    // Clear the existing children
    elem_part_head_list.innerHTML = "";
    // Add the new part heads
    for (var i = 0; i < derived_state.part_heads.rows.length; i++) {
        // Generate the string for this option, following the format "#{index}: {row}"
        let str = "#" + (i + 1).toString() + ": ";
        for (const b of derived_state.part_heads.rows[i]) {
            str += BELL_NAMES[b];
        }
        // Add the new option to the part heads list
        let new_opt = document.createElement("option");
        new_opt.value = i.toString();
        new_opt.innerText = str;
        elem_part_head_list.appendChild(new_opt);
    }
    // Make sure the correct part head is selected
    elem_part_head_list.value = view.current_part;
}

function update_section_folds() {
    for (section in elem_sections) {
        const is_open = view.section_folds[section];
        if (is_open === undefined) {
            console.warn(`Section '${section}' has no foldedness value in 'view'.`);
            continue;
        }
        set_foldedness(is_open, elem_sections[section].fold_button, elem_sections[section].area);
    }
}

function update_sidebar() {
    /* METHODS */

    // Set the number of methods in the title
    const num_methods = derived_state.methods.length;
    elem_num_methods.innerText = num_methods.toString();
    // Make sure that the method box contains the right number of HTML entries
    while (elem_method_box.children.length < num_methods) {
        // This is used as a capture for the 'onclick' event listener
        const index = elem_method_box.children.length;
        const new_entry = template_method_entry.cloneNode(true);
        // Remove the ID tag so it can't be confused with the template version when debugging
        new_entry.removeAttribute("id");
        // Attach callbacks
        new_entry.querySelector("#method-info-fold-btn").addEventListener("click", function () {
            comp.toggle_method_fold(index);
            on_comp_change();
        });
        const shorthand_input = new_entry.querySelector("#shorthand-input");
        shorthand_input.addEventListener("keyup", function () {
            comp.set_method_shorthand(index, shorthand_input.value);
            on_comp_change();
        });
        const name_input = new_entry.querySelector("#name-input");
        name_input.addEventListener("keyup", function () {
            comp.set_method_name(index, name_input.value);
            on_comp_change();
        });
        new_entry.querySelector("#delete-button").addEventListener("click", function () {
            const err = comp.remove_method(index);
            if (err) {
                console.warn("Error removing method: " + err);
            } else {
                on_comp_change();
            }
        });
        // Add it to the box
        elem_method_box.appendChild(new_entry);
    }
    while (elem_method_box.children.length > num_methods) {
        elem_method_box.removeChild(elem_method_box.lastChild);
    }
    // Now that we know that we have as many entries as methods, we can populate each entry in turn
    const method_entries = elem_method_box.children;
    for (let i = 0; i < num_methods; i++) {
        const m = derived_state.methods[i];
        const entry = method_entries[i];
        const is_used = m.num_rows !== 0;
        const is_open = comp.is_method_panel_open(i);
        // Populate title bar
        entry.querySelector("#name").innerText = m.name;
        entry.querySelector("#shorthand").innerText = `#${i}: ${m.shorthand}`;
        // Swap between row counter and delete if the method is never used
        const row_count = entry.querySelector("#row-count");
        const delete_btn = entry.querySelector("#delete-button");
        row_count.style.display = is_used ? "inline" : "none";
        delete_btn.style.display = is_used ? "none" : "inline";
        // Set row counter text
        row_count.innerText =
            (m.num_proved_rows === m.num_rows
                ? `${m.num_rows}`
                : `${m.num_proved_rows}/${m.num_rows}`) + " rows";
        // Set the foldedness
        set_foldedness(
            is_open,
            entry.querySelector("#method-info-fold-btn"),
            entry.querySelector("#method-info-area")
        );
        // Populate the fold-out part
        entry.querySelector("#shorthand-input").value = m.shorthand;
        entry.querySelector("#name-input").value = m.name;
        const pn = entry.querySelector("#place-notation-input");
        pn.value = m.place_not_string;
        pn.disabled = is_used;
    }

    /* CALL LIST */

    // Set the call count in the title
    elem_num_calls.innerText = derived_state.calls.length.toString();
    // Update the call count
    i = 0;
    elem_call_readout.innerText = derived_state.calls
        .map(function (c) {
            let unproved_count = c.count - c.proved_count;
            let unproved_str = unproved_count === 0 ? "" : ` (${unproved_count} muted)`;
            let s = `(#${i}) ${c.location} ${c.notation}: ${c.proved_count}${unproved_str}`;
            i += 1;
            return s;
        })
        .join("\n");
}

function on_part_head_spec_change() {
    const parse_error = comp.parse_part_head_spec(elem_part_head_input.value);
    if (parse_error === "") {
        // Update the composition if the part heads parsed successfully (before updating this
        // display).  This will update `elem_part_head_message` with a success message.
        on_comp_change();
    } else {
        // If the parsing failed, then update `elem_part_head_message` to display the error to the
        // user
        elem_part_head_message.style.color = ERROR_COL;
        elem_part_head_message.innerText = parse_error;
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
    // Initialise the display
    on_comp_change();
}

function start() {
    init_comp();
    // Bind event listeners to all the things we need
    canv.addEventListener("mousemove", on_mouse_move);
    canv.addEventListener("mousedown", on_mouse_down);
    canv.addEventListener("mouseup", on_mouse_up);
    document.addEventListener("keydown", on_key_down);
    window.addEventListener("resize", on_window_resize);
    elem_part_head_list.addEventListener("change", on_part_change);
    elem_part_head_input.addEventListener("keyup", on_part_head_spec_change);
    elem_transpose_input.addEventListener("keyup", on_transpose_box_change);
    elem_transpose_input.addEventListener("keydown", on_transpose_box_key_down);
    // Bind event listeners for sidebar section folding
    for (s in elem_sections) {
        // We have to capture a copy of the string in the event listener, otherwise all the fold
        // buttons will fold the same section
        const section = `${s}`;
        elem_sections[section].fold_button.addEventListener("click", function () {
            if (!comp.toggle_section_fold(section)) {
                console.warn(`Section '${section}' doesn't exist.`);
                return;
            }
            sync_view();
            update_section_folds();
        });
    }
    // Update all the parts of the display to initialise them
    on_window_resize();
    update_section_folds();
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

function set_foldedness(is_open, triangle, area) {
    triangle.innerText = is_open ? FOLD_BUTTON_TRIANGLE_OPEN : FOLD_BUTTON_TRIANGLE_CLOSED;
    area.style.display = is_open ? "block" : "none";
}

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
    const button_mask = e.buttons === undefined ? e.which : e.buttons;
    return (button_mask & (1 << button)) != 0;
}

// Rounds a coordinate on a line of a given width so that:
// - A vertical or horizontal line going through this point overflows pixel boundaries by as little
//   as possible.
// - The rendered line will overflow pixel boundaries by the same amount on either side of that
//   horizontal or vertical line.
// The solution to this is to have the following cases:
// - `width` rounds down to an even number: round `coord` to pixel boundaries
// - `width` rounds down to an odd number:  round `coord` to pixel centres
function round_line_coord(coord, width) {
    // What the fractional part of our rounded coord should be
    const rounding_factor = Math.floor(width) === 0 ? 0 : 0.5;
    return Math.round(coord - rounding_factor) + rounding_factor;
}

function world_space_cursor_pos() {
    // First, transform the mouse coords out of screen space and into world space
    return {
        x: mouse_coords.x - canv.width / 2 + view.view_x,
        y: mouse_coords.y - canv.height / 2 + view.view_y,
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
    for (let i = derived_state.frags.length - 1; i >= 0; i--) {
        const frag = derived_state.frags[i];
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
        hov_frag.source_range = derived_state.frags[i].rows[Math.floor(hov_frag.row)].range;
        break;
    }
    return hov_frag;
}

function frag_bbox(f) {
    return new_rect(
        f.x - FALSENESS_COL_WIDTH,
        f.y - FRAG_BBOX_EXTRA_HEIGHT,
        derived_state.stage * COL_WIDTH + FALSENESS_COL_WIDTH * 2,
        f.rows.length * ROW_HEIGHT + FRAG_BBOX_EXTRA_HEIGHT * 2
    );
}

function view_rect() {
    return rect_from_centre(
        view.view_x,
        view.view_y,
        canv.width / devicePixelRatio,
        canv.height / devicePixelRatio
    );
}

function frag_link_line(link) {
    // Calculate bboxes of the frags we're joining
    const bbox_from = frag_bbox(derived_state.frags[link.from]);
    const bbox_to = frag_bbox(derived_state.frags[link.to]);
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
        if (link.from == link.to) continue;
        // Find which points the line will be drawn between
        const l = frag_link_line(link);
        // Calculate the direction vector of the line
        const d = {
            x: l.to_x - l.from_x,
            y: l.to_y - l.from_y,
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
        c_x: x + w / 2,
        c_y: y + h / 2,
        w: w,
        h: h,
        min_x: x,
        max_x: x + w,
        min_y: y,
        max_y: y + h,
    };
}

// Called whenever the composition changes, and generates the necessary updates in order to get the
// user's view in sync with the new composition.
function on_comp_change() {
    sync_derived_state();
    sync_view();
    update_hud();
    update_part_head_list();
    update_sidebar();
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

// Convert a list of sidebar section names into an object of:
// ```javascript
// { "<name>": { fold_button: <elem>, area: <elem> } }
// ```
function find_section_fold_elems(names) {
    let obj = {};
    for (n of names) {
        obj[n] = {
            fold_button: document.getElementById(`${n}-box-fold`),
            area: document.getElementById(`${n}-box-area`),
        };
    }
    return obj;
}

// Debug log that a state transition has occurred
function log_state_transition(from, to) {
    if (DBG_LOG_STATE_TRANSITIONS) {
        console.log(`Stage change: ${from} -> ${to}`);
    }
}
