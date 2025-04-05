#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod logistic_regression;
mod ui;

use crate::app::*;
use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_inner_size([890.0, 720.0])
            // .with_min_inner_size(vec2(890.0, 690.0))
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Elisa", options, Box::new(|cc|
        Ok(Box::from(Elisa::new(cc)))
    ))
}

pub fn default<D: Default>() -> D {
    D::default()
}
