#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod logistic_regression;
mod ui;

use crate::app::*;
use eframe::egui::{self, IconData};

fn main() -> eframe::Result {
    env_logger::init();

    #[cfg(target_os = "macos")]
    let icon = include_bytes!("../resources/Icon_MacOS.png");
    #[cfg(not(target_os = "macos"))]
    let icon = include_bytes!("../resources/Icon.png");
    
    let icon = image::load_from_memory(icon).unwrap();
    let icon_data = IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_bytes(),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_inner_size([890.0, 720.0])
            .with_icon(icon_data)
            // .with_min_inner_size(vec2(890.0, 690.0))
            .with_drag_and_drop(true),

        ..default()
    };
    eframe::run_native("Elisa", options, Box::new(|cc|
        Ok(Box::from(Elisa::new(cc)))
    ))
}

pub fn default<D: Default>() -> D {
    D::default()
}
