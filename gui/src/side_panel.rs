//! Drawing code for the GUI's side panel

use std::{collections::HashSet, rc::Rc};

use eframe::egui::{self, Color32, Ui};
use jigsaw_comp::{
    full::{self, FullState, MusicGroupInner},
    spec::part_heads,
};
use jigsaw_utils::types::RowSource;

use crate::{Action, CompAction};

pub(crate) fn draw(
    ctx: &egui::CtxRef,
    state: &FullState,
    part_head_str: &str,
    push_action: impl FnMut(Action),
) -> HashSet<RowSource> {
    egui::SidePanel::right("side_panel")
        .show(ctx, |ui| {
            draw_panel_contents(ui, state, part_head_str, push_action)
        })
        .inner
}

fn draw_panel_contents(
    ui: &mut Ui,
    full_state: &FullState,
    part_head_str: &str,
    push_action: impl FnMut(Action),
) -> HashSet<RowSource> {
    const PANEL_SPACE: f32 = 5.0; // points

    let mut rows_to_highlight = HashSet::<RowSource>::new();

    ui.heading("Jigsaw");

    // General info
    let part_len = full_state.stats.part_len;
    let num_parts = full_state.part_heads.len();
    ui.label(format!(
        "{} rows * {} parts = {} rows",
        part_len,
        num_parts,
        part_len * num_parts
    ));

    ui.add_space(PANEL_SPACE);

    // Create a scrollable panel for the rest of the dropdowns
    egui::ScrollArea::auto_sized().show(ui, |panels_ui| {
        // Parts panel
        let part_panel_title = format!("Parts ({})", full_state.part_heads.len());
        let r = egui::CollapsingHeader::new(part_panel_title)
            .id_source("Parts")
            .show(panels_ui, |ui| {
                draw_parts_panel(ui, full_state, part_head_str, push_action)
            });
        // Add space only when the panel is open
        if r.body_response.is_some() {
            panels_ui.add_space(PANEL_SPACE);
        }

        // Methods panel
        let method_panel_title = format!("Methods ({})", full_state.methods.len());
        let r = egui::CollapsingHeader::new(method_panel_title)
            .id_source("Methods")
            .show(panels_ui, |ui| draw_method_panel(ui, full_state));
        // Add space only when the panel is open
        if r.body_response.is_some() {
            panels_ui.add_space(PANEL_SPACE);
        }

        // Calls panel
        let r = panels_ui.collapsing("Calls", |ui| {
            ui.label("14 LE -");
            ui.label("1234 LE s");
        });
        // Add space only when the panel is open
        if r.body_response.is_some() {
            panels_ui.add_space(PANEL_SPACE);
        }

        // Music panel
        let music = &full_state.music;
        let label = format!("Music ({}/{})", music.total_count(), music.max_count());
        egui::CollapsingHeader::new(label)
            .id_source("Music")
            .show(panels_ui, |ui| {
                draw_music_ui(ui, music.groups(), &mut rows_to_highlight);
            });
    });

    rows_to_highlight
}

fn draw_parts_panel(
    ui: &mut Ui,
    full_state: &FullState,
    part_head_str: &str,
    mut push_action: impl FnMut(Action),
) {
    let mut part_head_str_mut = part_head_str.to_owned();
    // Part head input
    ui.text_edit_singleline(&mut part_head_str_mut);

    // Add an action to update the app's `part_head_str` if the user changed the string
    if part_head_str_mut != part_head_str {
        push_action(Action::SetPartHeadString(part_head_str_mut));
    }

    // Parse the user's input
    let parse_result = full_state.part_heads.try_reparse(part_head_str);
    match parse_result {
        // If the part heads changed, then replace them as another undo step
        Ok(part_heads::ReparseOk::DifferentRows(new_phs)) => {
            push_action(Action::Comp(CompAction::SetPartHeads(new_phs)));
        }
        // No effect if the part heads haven't changed
        Ok(part_heads::ReparseOk::SameRows) => {}
        // In the case of an error, display that error to the user
        Err(e) => {
            let err_label = egui::Label::new(e.to_string()).text_color(Color32::RED);
            ui.label(err_label);
        }
    }

    // Add a warning if the parts don't form a group
    if !full_state.part_heads.is_group() {
        ui.label("Parts don't form a group!");
    }

    // Part list
    ui.separator();
    for r in full_state.part_heads.rows() {
        ui.label(r.to_string());
    }
}

fn draw_method_panel(ui: &mut Ui, full_state: &FullState) {
    for (i, method) in full_state.methods.iter().enumerate() {
        left_then_right(
            ui,
            // The main label sticks to the left
            |left_ui| {
                left_ui.label(format!(
                    "(#{}, {}): {}",
                    i,
                    method.shorthand(),
                    method.name()
                ))
            },
            |right_ui| {
                if method.num_rows == 0 {
                    // Because we're in a right-to-left block, the buttons are added from right
                    // to left (which feels like the reverse order)
                    if right_ui.button("del").clicked() {
                        println!(
                            "Can't delete methods.  Even {}, good though it is!",
                            method.name()
                        );
                    }
                    if right_ui.button("edit").clicked() {
                        println!(
                            "Can't edit methods.  Even {}, good though it is!",
                            method.name()
                        );
                    }
                } else {
                    // If the method is used, then display either 'x rows' or 'x/y rows',
                    // depending on whether or not all the method's rows are muted
                    let label_text = if method.num_proved_rows == method.num_rows {
                        format!("{} rows", method.num_rows,)
                    } else {
                        format!("{}/{} rows", method.num_proved_rows, method.num_rows,)
                    };
                    right_ui.label(label_text);
                }
            },
        );
    }
}

/// Recursively creates the GUI for a set of `MusicGroup`s
fn draw_music_ui(
    ui: &mut Ui,
    musics: &[Rc<full::MusicGroup>],
    rows_to_highlight: &mut HashSet<RowSource>,
) {
    for m in musics {
        draw_music_group_ui(m, ui, rows_to_highlight);
    }
}

/// Recursively creates the GUI for a single `MusicGroup`
fn draw_music_group_ui(
    group: &full::MusicGroup,
    ui: &mut Ui,
    rows_to_highlight: &mut HashSet<RowSource>,
) {
    let full::MusicGroup {
        name,
        max_count,
        inner,
    } = group;

    let response = match inner {
        MusicGroupInner::Leaf { rows_matched } => {
            left_then_right(
                ui,
                |left_ui| left_ui.label(name),
                |right_ui| right_ui.label(format!("{}/{}", rows_matched.len(), max_count)),
            )
            .response // Get the response from the entire horizontal layout
        }
        MusicGroupInner::Group { sub_groups, count } => {
            let label = format!("{} ({}/{})", name, count, max_count);
            egui::CollapsingHeader::new(label)
                .id_source(name)
                .show(ui, |sub_ui| {
                    draw_music_ui(sub_ui, sub_groups, rows_to_highlight)
                })
                .header_response
        }
    };

    // If this is being hovered, then highlight every row matched by any of its descendants
    if response.hovered() {
        group.add_row_sources(rows_to_highlight);
    }
}

/// Helper function to draw two pieces of GUI, one aligned left and one aligned right
fn left_then_right<L, R>(
    ui: &mut Ui,
    left: impl FnOnce(&mut Ui) -> L,
    right: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<(L, R)> {
    ui.horizontal(|left_ui| {
        let left_res = left(left_ui);
        let right_res = left_ui.with_layout(egui::Layout::right_to_left(), right);
        (left_res, right_res.inner)
    })
}
