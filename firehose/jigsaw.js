
let wasm;

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachegetUint8Memory0 = null;
function getUint8Memory0() {
    if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

let cachedTextEncoder = new TextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}
/**
* The complete state of the composition editor, complete with undo history and UI/view state.
*
* The general data-flow is:
* - User generates some input (keypress, mouse click, etc.)
* - JS reads this input and calls one of the `#[wasm_bindgen]` methods on `Comp`
* - These call some `self.make_*action*` function which runs a given closure on the existing
*   [`Spec`]
*   - This also handles the undo history (i.e. doesn't overwrite old copies, and deallocates
*     future redo steps that are now unreachable).
*   - Because the [`Spec`] has changed, we rebuild the [`DerivedState`] from this new [`Spec`].
*     This is necessary because JS can't access the [`Spec`] directly.
* - The following all happens during the call to the JS `on_comp_change()` method:
*   - After every edit, JS will call [`Comp::ser_derived_state`] which returns a JSON
*     serialisation of the current [`DerivedState`], which is parsed into a full-blown JS object
*     and the global `derived_state` variable is overwritten with this new value.
*   - The HUD UI (sidebar, etc.) are all updated to this new value
*   - A repaint is requested, so that the updated [`DerivedState`] gets fully rendered to the
*   screen.
*/
export class Comp {

    static __wrap(ptr) {
        const obj = Object.create(Comp.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_comp_free(ptr);
    }
    /**
    * Create an example composition
    * @returns {Comp}
    */
    static example() {
        var ret = wasm.comp_example();
        return Comp.__wrap(ret);
    }
    /**
    * Attempt to parse a new part head specification [`String`].  If it successfully parses then
    * update the part head list as a new edit (returning `""`), otherwise return a [`String`]
    * summarising the issue with the parsing.
    * @param {string} s
    * @returns {string}
    */
    parse_part_head_spec(s) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            var ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            wasm.comp_parse_part_head_spec(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Return a JSON serialisation of the derived state
    * @returns {string}
    */
    ser_derived_state() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.comp_ser_derived_state(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Return a JSON serialisation of the current view settings
    * @returns {string}
    */
    ser_view() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.comp_ser_view(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} json
    */
    set_view_from_json(json) {
        var ptr0 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.comp_set_view_from_json(this.ptr, ptr0, len0);
    }
    /**
    * Returns `true` if the editor is in [`State::Idle`]
    * @returns {boolean}
    */
    is_state_idle() {
        var ret = wasm.comp_is_state_idle(this.ptr);
        return ret !== 0;
    }
    /**
    * Returns `true` if the editor is in [`State::Dragging`]
    * @returns {boolean}
    */
    is_state_dragging() {
        var ret = wasm.comp_is_state_dragging(this.ptr);
        return ret !== 0;
    }
    /**
    * Returns the index of the [`Frag`] being dragged, `panic!`ing if the UI is not in
    * [`State::Dragging`].
    * @returns {number}
    */
    frag_being_dragged() {
        var ret = wasm.comp_frag_being_dragged(this.ptr);
        return ret >>> 0;
    }
    /**
    * Moves the UI into [`State::Dragging`], `panic!`ing if we start in any state other than
    * [`State::Idle`]
    * @param {number} frag_ind
    */
    start_dragging(frag_ind) {
        wasm.comp_start_dragging(this.ptr, frag_ind);
    }
    /**
    * Called to exit [`State::Dragging`].  This moves the [`Frag`] the user was dragging to the
    * provided coords (as a new undo step), and returns to [`State::Idle`].  This `panic!`s if
    * called from any state other than [`State::Dragging`].
    * @param {number} new_x
    * @param {number} new_y
    */
    finish_dragging(new_x, new_y) {
        wasm.comp_finish_dragging(this.ptr, new_x, new_y);
    }
    /**
    * Returns `true` if the editor is in [`State::Transposing`]
    * @returns {boolean}
    */
    is_state_transposing() {
        var ret = wasm.comp_is_state_transposing(this.ptr);
        return ret !== 0;
    }
    /**
    * Moves the editor into [`State::Transposing`] the [`Frag`] at `frag_ind`.  This returns the
    * string representation of the first [`Row`] of that [`Frag`], to initialise the
    * transposition input box.  This `panic!`s if called from any state other than
    * [`State::Idle`].
    * @param {number} frag_ind
    * @param {number} row_ind
    * @returns {string}
    */
    start_transposing(frag_ind, row_ind) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.comp_start_transposing(retptr, this.ptr, frag_ind, row_ind);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Attempt to parse a [`String`] into a [`Row`] of the correct [`Stage`] for this `Comp`, to
    * be used in [`State::Transposing`].  There are two possible outcomes:
    * - **The string corresponds to a valid [`Row`]**: This parsed [`Row`] is used to modify
    *   the temporary [`Spec`] contained with in the [`State::Transposing`] enum.  The
    *   [`DerivedState`] is updated and `""` is returned.
    * - **The string creates a parse error**:  No modification is made, and a [`String`]
    *   representing the error is returned.
    * This `panic!`s if called from any state other than [`State::Transposing`].
    * @param {string} row_str
    * @returns {string}
    */
    try_parse_transpose_row(row_str) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            var ptr0 = passStringToWasm0(row_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            wasm.comp_try_parse_transpose_row(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Called to exit [`State::Transposing`], saving the changes.  If `row_str` parses to a valid
    * [`Row`] then this commits the desired transposition and returns the editor to
    * [`State::Idle`] (returning `true`), otherwise no change occurs and this returns `false`.
    * This `panic!`s if called from any state other than [`State::Transposing`].
    * @param {string} row_str
    * @returns {boolean}
    */
    finish_transposing(row_str) {
        var ptr0 = passStringToWasm0(row_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        var ret = wasm.comp_finish_transposing(this.ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
    * Called to exit [`State::Transposing`], **without** saving the changes.  This `panic!`s if
    * called from any state other than [`State::Transposing`].
    */
    exit_transposing() {
        wasm.comp_exit_transposing(this.ptr);
    }
    /**
    * Returns `true` if the editor is in [`State::EditingMethod`]
    * @returns {boolean}
    */
    is_state_editing_method() {
        var ret = wasm.comp_is_state_editing_method(this.ptr);
        return ret !== 0;
    }
    /**
    * Starts editing the [`MethodSpec`] at a given index
    * @param {number} index
    */
    start_editing_method(index) {
        wasm.comp_start_editing_method(this.ptr, index);
    }
    /**
    * Starts editing a new [`MethodSpec`], which will get added at the end
    */
    start_editing_new_method() {
        wasm.comp_start_editing_new_method(this.ptr);
    }
    /**
    * Return all the information required for JS to update the method edit box, serialised as
    * JSON
    * @returns {string}
    */
    method_edit_state() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.comp_method_edit_state(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Sets both the name and shorthand of the method being edited
    * @param {string} new_name
    * @param {string} new_shorthand
    */
    set_method_names(new_name, new_shorthand) {
        var ptr0 = passStringToWasm0(new_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        var ptr1 = passStringToWasm0(new_shorthand, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        wasm.comp_set_method_names(this.ptr, ptr0, len0, ptr1, len1);
    }
    /**
    * Sets the place notatation string in the method edit box, and reparses to generate a new
    * error.  Called whenever the user types into the method box
    * @param {string} new_pn
    */
    set_method_pn(new_pn) {
        var ptr0 = passStringToWasm0(new_pn, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.comp_set_method_pn(this.ptr, ptr0, len0);
    }
    /**
    * Exit method editing mode, without commiting any of the changes
    */
    exit_method_edit() {
        wasm.comp_exit_method_edit(this.ptr);
    }
    /**
    * Exit method editing mode, commiting the new method to the composition if valid.  This
    * returns `false` if no change occured
    * @returns {boolean}
    */
    finish_editing_method() {
        var ret = wasm.comp_finish_editing_method(this.ptr);
        return ret !== 0;
    }
    /**
    */
    undo() {
        wasm.comp_undo(this.ptr);
    }
    /**
    */
    redo() {
        wasm.comp_redo(this.ptr);
    }
    /**
    * See [`Spec::extend_frag_end`] for docs
    * @param {number} frag_ind
    * @param {number} method_ind
    * @param {boolean} add_course
    */
    extend_frag(frag_ind, method_ind, add_course) {
        wasm.comp_extend_frag(this.ptr, frag_ind, method_ind, add_course);
    }
    /**
    * See [`Spec::add_frag`] for docs
    * @param {number} x
    * @param {number} y
    * @param {number} method_ind
    * @param {boolean} add_course
    * @returns {number}
    */
    add_frag(x, y, method_ind, add_course) {
        var ret = wasm.comp_add_frag(this.ptr, x, y, method_ind, add_course);
        return ret >>> 0;
    }
    /**
    * Deletes a [`Frag`]ment by index.
    * @param {number} frag_ind
    */
    delete_frag(frag_ind) {
        wasm.comp_delete_frag(this.ptr, frag_ind);
    }
    /**
    * See [`Spec::join_frags`] for docs.
    * @param {number} frag_1_ind
    * @param {number} frag_2_ind
    */
    join_frags(frag_1_ind, frag_2_ind) {
        wasm.comp_join_frags(this.ptr, frag_1_ind, frag_2_ind);
    }
    /**
    * Splits a given [`Frag`]ment into two fragments, returning `""` on success and an error
    * string on failure. `split_index` refers to the first row of the 2nd fragment (so row
    * #`split_index` will also be the new leftover row of the 1st subfragment).
    * @param {number} frag_ind
    * @param {number} split_index
    * @param {number} new_y
    * @returns {string}
    */
    split_frag(frag_ind, split_index, new_y) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.comp_split_frag(retptr, this.ptr, frag_ind, split_index, new_y);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Replace the call at the end of a composition.  Calls are referenced by their index, and any
    * negative number will correspond to removing a call.  See [`Spec::set_call`] for more docs.
    * @param {number} frag_ind
    * @param {number} row_ind
    * @param {number} call_ind
    * @returns {string}
    */
    set_call(frag_ind, row_ind, call_ind) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.comp_set_call(retptr, this.ptr, frag_ind, row_ind, call_ind);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Toggle whether or not a given [`Frag`] is muted
    * @param {number} frag_ind
    */
    toggle_frag_mute(frag_ind) {
        wasm.comp_toggle_frag_mute(this.ptr, frag_ind);
    }
    /**
    * [`Frag`] soloing ala FL Studio; this has two cases:
    * 1. `frag_ind` is the only unmuted [`Frag`], in which case we unmute everything
    * 2. `frag_ind` isn't the only unmuted [`Frag`], in which case we mute everything except it
    * @param {number} frag_ind
    */
    toggle_frag_solo(frag_ind) {
        wasm.comp_toggle_frag_solo(this.ptr, frag_ind);
    }
    /**
    * Toggles the lead folding at a given **on screen** row index.  This doesn't update the undo
    * history.
    * @param {number} frag_ind
    * @param {number} on_screen_row_ind
    */
    toggle_lead_fold(frag_ind, on_screen_row_ind) {
        wasm.comp_toggle_lead_fold(this.ptr, frag_ind, on_screen_row_ind);
    }
    /**
    * Remove a method from the list, if it doesn't appear in the composition
    * @param {number} method_ind
    * @returns {string}
    */
    remove_method(method_ind) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.comp_remove_method(retptr, this.ptr, method_ind);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Change the shorthand name of a method
    * @param {number} method_ind
    * @param {string} new_name
    */
    set_method_shorthand(method_ind, new_name) {
        var ptr0 = passStringToWasm0(new_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.comp_set_method_shorthand(this.ptr, method_ind, ptr0, len0);
    }
    /**
    * Change the full name of a method (without causing an undo history
    * @param {number} method_ind
    * @param {string} new_name
    */
    set_method_name(method_ind, new_name) {
        var ptr0 = passStringToWasm0(new_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.comp_set_method_name(this.ptr, method_ind, ptr0, len0);
    }
    /**
    * Resets the composition to the example
    */
    reset() {
        wasm.comp_reset(this.ptr);
    }
    /**
    * Moves the view's camera to a given location
    * @param {number} new_cam_x
    * @param {number} new_cam_y
    */
    set_view_coords(new_cam_x, new_cam_y) {
        wasm.comp_set_view_coords(this.ptr, new_cam_x, new_cam_y);
    }
    /**
    * Sets the current part being viewed
    * @param {number} new_part
    */
    set_current_part(new_part) {
        wasm.comp_set_current_part(this.ptr, new_part);
    }
    /**
    * Toggles the foldedness of the method section, returning `false` if no section with that
    * name exists.
    * @param {string} section_name
    * @returns {boolean}
    */
    toggle_section_fold(section_name) {
        var ptr0 = passStringToWasm0(section_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        var ret = wasm.comp_toggle_section_fold(this.ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
    * Toggles the foldedness of a specific method panel
    * @param {number} method_ind
    */
    toggle_method_fold(method_ind) {
        wasm.comp_toggle_method_fold(this.ptr, method_ind);
    }
    /**
    * Returns whether or not a given method info panel is open
    * @param {number} method_ind
    * @returns {boolean}
    */
    is_method_panel_open(method_ind) {
        var ret = wasm.comp_is_method_panel_open(this.ptr, method_ind);
        return ret !== 0;
    }
}

async function load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

async function init(input) {
    if (typeof input === 'undefined') {
        input = new URL('jigsaw_bg.wasm', import.meta.url);
    }
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_new_59cb74e423758ede = function() {
        var ret = new Error();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_stack_558ba5917b466edd = function(arg0, arg1) {
        var ret = getObject(arg1).stack;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_error_4bb6c2a97407129a = function(arg0, arg1) {
        try {
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(arg0, arg1);
        }
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }



    const { instance, module } = await load(await input, imports);

    wasm = instance.exports;
    init.__wbindgen_wasm_module = module;

    return wasm;
}

export default init;

