use super::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug)]
struct File(PathBuf);
impl egui::DroppedFile for File {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        panic!("routing must not read file contents")
    }
}

#[test]
fn external_drop_routes_once_to_app_card_and_never_registers() {
    for edge in [
        crate::core::DockEdge::Top,
        crate::core::DockEdge::Bottom,
        crate::core::DockEdge::Left,
        crate::core::DockEdge::Right,
    ] {
        for destination in ["exe", "lnk", "url", "folder", "blank", "clipped", "unknown"] {
            let ctx = egui::Context::default();
            let mut config = Config::defaults("home".into());
            config.dock.edge = edge;
            let kind = if destination == "url" {
                TargetKind::Url
            } else {
                TargetKind::Path
            };
            let target = match destination {
                "lnk" => "app.lnk",
                "url" => "https://example.com",
                "folder" => "folder",
                _ => "app.EXE",
            };
            let item = crate::core::DockItem::new("target", target, kind);
            let id = item.id;
            config.items = vec![item];
            let snapshot = config.clone();
            let rect = egui::Rect::from_min_size(egui::pos2(30.0, 30.0), egui::vec2(88.0, 94.0));
            let point = match destination {
                "blank" => Some(egui::pos2(150.0, 50.0)),
                "unknown" => None,
                _ => Some(rect.center()),
            };
            let mut commands = Vec::new();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 400.0),
                )),
                dropped_files: vec![
                    Arc::new(File("日本語 & first.txt".into())),
                    Arc::new(File("second.txt".into())),
                ],
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                if destination == "clipped" {
                    ui.set_clip_rect(egui::Rect::from_min_max(
                        egui::Pos2::ZERO,
                        egui::pos2(20.0, 20.0),
                    ));
                }
                finish(ui, &config, &[(id, rect)], point, &mut commands);
                finish(ui, &config, &[(id, rect)], point, &mut commands);
            });
            output.textures_delta.clear();
            assert_eq!(commands.len(), 1);
            if matches!(destination, "exe" | "lnk") {
                assert!(
                    matches!(&commands[0], Command::OpenDropped { id: received, paths } if *received == id && paths.len() == 2 && paths[0] == Path::new("日本語 & first.txt"))
                );
            } else {
                assert!(matches!(commands[0], Command::RejectDrop));
            }
            assert_eq!(config, snapshot);
        }
    }
}
