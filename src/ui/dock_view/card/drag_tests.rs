use super::*;

#[test]
fn actual_drag_emits_one_reorder_on_release_not_launch_on_all_edges() {
    for edge in [
        DockEdge::Top,
        DockEdge::Bottom,
        DockEdge::Left,
        DockEdge::Right,
    ] {
        for outcome in ["drop", "outside", "escape"] {
            let ctx = egui::Context::default();
            let config = Config::defaults("home".into());
            let mut cards: Vec<(uuid::Uuid, egui::Rect)> = Vec::new();
            let mut commands = Vec::new();
            for step in 0..5 {
                let mut events = Vec::new();
                if step > 0 {
                    let point = if step == 4 && outcome == "outside" {
                        egui::pos2(-20.0, -20.0)
                    } else if step < 3 {
                        cards[0].1.center() + egui::vec2(if step == 2 { 15.0 } else { 0.0 }, 0.0)
                    } else {
                        cards[2].1.center()
                            + if edge.is_horizontal() {
                                egui::vec2(5.0, 0.0)
                            } else {
                                egui::vec2(0.0, 5.0)
                            }
                    };
                    events.push(egui::Event::PointerMoved(point));
                    if step == 1 || step == 4 {
                        events.push(egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed: step == 1,
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                    if step == 4 && outcome == "escape" {
                        events.push(egui::Event::Key {
                            key: egui::Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                }
                cards.clear();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(600.0, 600.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        let layout = if edge.is_horizontal() {
                            egui::Layout::left_to_right(egui::Align::Center)
                        } else {
                            egui::Layout::top_down(egui::Align::Center)
                        };
                        ui.with_layout(layout, |ui| {
                            for item in &config.items {
                                cards.push((item.id, render(ui, 56.0, None, item, &mut commands)));
                            }
                            crate::ui::dock_drag::finish(ui, edge, &cards, &mut commands);
                        });
                    },
                );
                output.textures_delta.clear();
                if step < 4 {
                    assert!(commands.is_empty());
                }
            }
            if outcome == "drop" {
                assert_eq!(commands.len(), 1);
                assert!(
                    matches!(commands[0], Command::Reorder { id, before: None } if id == config.items[0].id)
                );
            } else {
                assert!(commands.is_empty());
            }
            assert!(!crate::ui::dock_drag::active(&ctx));
        }
    }
}
