use super::*;

#[derive(Debug)]
struct File;
impl egui::DroppedFile for File {
    fn path(&self) -> &std::path::Path {
        std::path::Path::new("日本語 file.txt")
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        panic!("routing must not read file contents")
    }
}

#[test]
fn file_drop_on_fixed_toolbar_never_routes_to_scrolled_application_cards() {
    for edge in [
        DockEdge::Top,
        DockEdge::Bottom,
        DockEdge::Left,
        DockEdge::Right,
    ] {
        for toolbar in [false, true] {
            let ctx = egui::Context::default();
            let icons = crate::ui::icons::Icons::new(ctx.clone());
            let mut config = Config::defaults("home".into());
            config.dock.edge = edge;
            let (w, h) = dock_size(&config);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
            let mut rects = [egui::Rect::NOTHING; 2];
            let mut commands = vec![];
            for step in 0..3 {
                let point = if toolbar {
                    rects[1].center()
                } else {
                    egui::pos2(40.0, 40.0)
                };
                let mut input = egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                };
                if step == 2 {
                    input.dropped_files.push(std::sync::Arc::new(File));
                }
                let mut output = ctx.run_ui(input, |ui| {
                    rects = render(ui, &config, &icons, false, Some(point), &mut commands);
                });
                output.textures_delta.clear();
            }
            assert_eq!(commands.len(), 1);
            if toolbar {
                assert!(matches!(commands[0], Command::RejectDrop));
            } else {
                assert!(
                    matches!(commands[0],Command::OpenDropped{id,..} if id==config.items[0].id)
                );
            }
        }
    }
}

#[test]
fn controls_stay_fixed_across_modes_counts_and_scrolling_and_remain_clickable() {
    for edge in [
        DockEdge::Top,
        DockEdge::Bottom,
        DockEdge::Left,
        DockEdge::Right,
    ] {
        let ctx = egui::Context::default();
        let icons = crate::ui::icons::Icons::new(ctx.clone());
        let mut config = Config::defaults("home".into());
        config.dock.edge = edge;
        let (width, height) = dock_size(&config);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
        let mut expected = None;
        for workspaces in [false, true] {
            for count in [0, 1, 16, 64] {
                config.workspaces = (0..count).map(|_| Workspace::new("環境")).collect();
                let snapshot = config.clone();
                let mut commands = vec![];
                let mut rects = [egui::Rect::NOTHING; 2];
                for step in 0..6 {
                    let events = match step {
                        2 => vec![
                            egui::Event::PointerMoved(egui::pos2(40.0, 40.0)),
                            egui::Event::MouseWheel {
                                phase: egui::TouchPhase::Move,
                                unit: egui::MouseWheelUnit::Point,
                                delta: if edge.is_horizontal() {
                                    egui::vec2(-500.0, 0.0)
                                } else {
                                    egui::vec2(0.0, -500.0)
                                },
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                        4 | 5 => {
                            let pos = rects[1].center();
                            vec![
                                egui::Event::PointerMoved(pos),
                                egui::Event::PointerButton {
                                    pos,
                                    button: egui::PointerButton::Primary,
                                    pressed: step == 4,
                                    modifiers: egui::Modifiers::NONE,
                                },
                            ]
                        }
                        _ => vec![],
                    };
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            events,
                            screen_rect: Some(screen),
                            ..Default::default()
                        },
                        |ui| {
                            rects = render(ui, &config, &icons, workspaces, None, &mut commands);
                        },
                    );
                    output.textures_delta.clear();
                    let baseline = expected.get_or_insert(rects);
                    assert_eq!(
                        &rects, baseline,
                        "edge={edge:?}, mode={workspaces}, count={count}, step={step}"
                    );
                    assert!(rects.iter().all(|r| screen.contains_rect(*r)));
                }
                assert_eq!(config, snapshot);
                assert_eq!(commands.len(), 1);
                assert!(matches!(commands[0], Command::OpenSettings));
            }
        }
    }
}
