use crate::*;
use super::logistic_regression::*;

use calamine::Xlsx;
use eframe::{egui::{self, pos2, vec2, Button, Color32, Context, FontData, FontDefinitions, FontFamily, Id, Margin, Modal, OpenUrl, Rect, Response, RichText, Shadow, Shape, Style, Theme, Ui, Vec2}, CreationContext};
use font_loader::system_fonts;
use std::{fs::File, io::{BufReader, Read, Write}, sync::Arc};

#[derive(Default, PartialEq)]
pub enum ElisaTab {
    #[default]
    Edit,
    Result,
}

#[derive(Clone)]
pub enum SerdeError {
    FileNotFound,
    CantReadFile,
    CantWriteFile,
    CantDeserialize,
}

fn setup_fonts(context: &Context) {
    let mut fonts = FontDefinitions::default();

    // Since Times New Roman is under copyright, try to load it from the system
    // If we can't find it, embed Computer Modern, a similar font
    let property = system_fonts::FontPropertyBuilder::new().family("Times New Roman").build();
    let default_font = font_loader::system_fonts::get(&property)
        .map(|(data, _)| data)
    .unwrap_or_else(|| include_bytes!("../resources/Computer Modern.ttf").to_vec());

    fonts.font_data.insert(
        "Times New Roman".to_owned(),
        Arc::new(FontData::from_owned(default_font))
    );
    fonts.families.entry(FontFamily::Proportional)
    .or_default()
    .insert(0, "Times New Roman".to_owned());

    context.set_fonts(fonts);
}

fn setup_style(style: &mut Style) {
    let white = Color32::from_hex("#FBFBFE").unwrap();
    let light_blue = Color32::from_hex("#F4F7FE").unwrap();    
    let gray = Color32::from_hex("#B2B6C0").unwrap();
    let dark_gray = Color32::from_hex("#585C65").unwrap();
    
    let spacing = &mut style.spacing;
    spacing.item_spacing = Vec2::splat(0.0);
    spacing.window_margin = Margin::same(0);
    spacing.button_padding = vec2(8.0, 4.0);

    style.interaction.selectable_labels = false;

    style.text_styles.entry(egui::TextStyle::Body).or_default().size = 13.0;
    style.text_styles.entry(egui::TextStyle::Heading).or_default().size = 18.0;
    style.text_styles.entry(egui::TextStyle::Button).or_default().size = 13.0;
    style.text_styles.entry(egui::TextStyle::Monospace).or_default().size = 10.0;

    style.visuals.faint_bg_color = light_blue;
    style.visuals.menu_corner_radius = 0.into();
    style.visuals.override_text_color = Some(Color32::BLACK);
    style.visuals.popup_shadow = Shadow::NONE;
    style.visuals.selection.stroke = (0.0, Color32::BLACK).into();
    style.visuals.window_fill = white;

    let widgets = &mut style.visuals.widgets;
    widgets.active.bg_stroke = (0.0, dark_gray).into();
    widgets.active.corner_radius = 0.into();
    widgets.active.expansion = 0.0;
    widgets.active.fg_stroke = (1.25, dark_gray).into();
    widgets.active.weak_bg_fill = white;

    widgets.hovered.bg_stroke = (0.0, gray).into();
    widgets.hovered.corner_radius = 0.into();
    widgets.hovered.expansion = 0.0;
    widgets.hovered.fg_stroke = (1.0, dark_gray).into();
    widgets.hovered.weak_bg_fill = white;

    widgets.inactive.bg_stroke = (0.0, gray).into();
    widgets.inactive.bg_fill = white;
    widgets.inactive.corner_radius = 0.into();
    widgets.inactive.fg_stroke = (1.0, gray).into();
    widgets.inactive.weak_bg_fill = white;

    widgets.noninteractive.bg_stroke = (1.0, gray).into();
}

#[derive(Default)]
pub struct Elisa {
    pub current_tab: ElisaTab,
    pub microplate: Microplate,
    pub data_textfield: String,
    pub excel: Option<Xlsx<BufReader<File>>>,
    pub max_standard_group: usize,
    pub max_unknown_group: usize,
    pub pdf_report: bool,
    pub plot_response: Option<Response>,
    pub plot_parameters: Option<[(&'static str, f64); 8]>,
    pub sheet_names: Vec<String>,
    pub regression: Option<Regression>,
    pub selected_sheet: usize,
    pub selected_sample: Option<usize>,
    pub selected_sample_group: usize,
    pub standards_textfield: Vec<String>,
    pub serde_error_modal: Option<SerdeError>,
    pub value_error_modal: Option<ValueError>,
}

impl Elisa {
    pub fn new(creation_context: &CreationContext) -> Self {
        let ctx = &creation_context.egui_ctx;
        setup_fonts(ctx);

        ctx.set_theme(Theme::Light);
        ctx.style_mut_of(Theme::Light, setup_style);

        let width = 12;
        let height = 8;
        let max_groups = 100;
        Self {
            microplate: Microplate::new(width, height),
            standards_textfield: vec![String::new(); max_groups],
            ..default()
        }
    }
}

impl eframe::App for Elisa {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.current_tab {
            ElisaTab::Edit => self.assay_edit(ctx),
            ElisaTab::Result => self.assay_result(ctx),
        }
    }
}

impl Elisa {
    fn assay_edit(&mut self, ctx: &egui::Context) {
        let white = Color32::from_hex("#FBFBFE").unwrap();
        egui::CentralPanel::default().frame(egui::Frame::default().inner_margin(0.0).fill(white)).show(ctx, |ui| {
            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
            self.show_modals(ui);

            ui.painter().hline(0.0..=ui.max_rect().width(), 30.0, stroke);
            ui.painter().vline(30.0, 0.0..=ui.max_rect().height(), stroke);

            egui::Frame::new()
                .inner_margin(Margin { left: 60, right: 30, top: 60, bottom: 30})
                .show(ui, |ui| {
                let available_height = ui.available_height();
                ui.horizontal(|ui| {
                    ui.set_height(available_height);
                    ui.vertical(|ui| {
                        self.microplate_view(ui);
                        ui.add_space(30.0);
                        let remaining_height = ui.available_height();
                        ui.horizontal(|ui| {
                            ui.set_height(remaining_height);
                            self.run_notes(ui); 
                            ui.add_space(30.0);
                            self.measurements(ui);
                        });
                    });
                    ui.add_space(30.0);
                    ui.vertical(|ui| {
                        self.sample_menu(ui);
                        ui.add_space(30.0);
                        self.standards_concentrations(ui);
                    })
                });
            });

            self.save_load_buttons(ui);
            let mut rect = ctx.input(|i| i.screen_rect());
            rect.min = rect.max - vec2(120.0, 30.0);
            let link = ui.put(rect, Button::new("∞ Eliavaux"));
            let url = "https://github.com/eliavaux";

            if link.clicked() {
                let modifiers = ui.ctx().input(|i| i.modifiers);
                ui.ctx().open_url(OpenUrl {
                    url: url.to_string(),
                    new_tab: modifiers.any(),
                });
            }
            if link.middle_clicked() {
                ui.ctx().open_url(OpenUrl {
                    url: url.to_string(),
                    new_tab: true,
                });
            }
        });
    }
    
    fn assay_result(&mut self, ctx: &egui::Context) {
        let white = Color32::from_hex("#FBFBFE").unwrap();

        egui::CentralPanel::default().frame(egui::Frame::default().inner_margin(0.0).fill(white)).show(ctx, |ui| {
            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;

            ui.painter().hline(0.0..=ui.max_rect().width(), 30.0, stroke);
            ui.painter().vline(30.0, 0.0..=ui.max_rect().height(), stroke);

            egui::Frame::new()
                .inner_margin(Margin { left: 60, right: 30, top: 60, bottom: 30})
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            self.plot(ui);
                            ui.add_space(30.0);
                            ui.vertical(|ui| {
                                self.plot_parameters(ui);
                            });
                        });
                        ui.add_space(30.0);
                        self.save_as(ui);
                    });
                    ui.spacing_mut().button_padding = vec2(4.0, 2.0);
                    let rect = Rect::from_min_size(pos2(45.0, 5.0), vec2(50.0, 20.0));
                    let button = ui.put(rect, Button::new(RichText::new("Back").size(13.5)));
                    Self::dashed_outline(ui, &button);
                    if button.clicked() {
                        self.current_tab = ElisaTab::Edit;
                    }
            });
            let mut rect = ctx.input(|i| i.screen_rect());
            rect.min = rect.max - vec2(120.0, 30.0);
            let link = ui.put(rect, Button::new("∞ Eliavaux"));
            let url = "https://github.com/eliavaux";

            if link.clicked() {
                let modifiers = ui.ctx().input(|i| i.modifiers);
                ui.ctx().open_url(OpenUrl {
                    url: url.to_string(),
                    new_tab: modifiers.any(),
                });
            }
            if link.middle_clicked() {
                ui.ctx().open_url(OpenUrl {
                    url: url.to_string(),
                    new_tab: true,
                });
            }
        });
    }
    
    fn save_load_buttons(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            use SerdeError::*;
            
            ui.spacing_mut().button_padding = vec2(4.0, 2.0);
            let rect = Rect::from_min_size(pos2(45.0, 5.0), vec2(50.0, 20.0));
            let button = ui.put(rect, Button::new(RichText::new("Save").size(13.5)));
            Self::dashed_outline(ui, &button);
            if button.clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text", &["json"])
                    .set_file_name("Assay")
                    .save_file() {
                    if let Ok(mut file) = File::create(path) {
                        let serialized = serde_json::to_string(&self.microplate).unwrap();
                        if file.write_all(serialized.as_bytes()).is_err() {
                            self.serde_error_modal = Some(CantWriteFile);
                        }
                    } else {
                        self.serde_error_modal = Some(FileNotFound);
                    }
                }
            }

            let rect = Rect::from_min_size(pos2(45.0 + 50.0 + 10.0, 5.0), vec2(50.0, 20.0));
            let button = ui.put(rect, Button::new(RichText::new("Load").size(13.5)));
            Self::dashed_outline(ui, &button);
            if button.clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text", &["json"])
                    .pick_file() {
                    if let Ok(mut file) = File::open(path) {
                        let mut buf = Vec::new();
                        if file.read_to_end(&mut buf).is_err() {
                            self.serde_error_modal = Some(CantReadFile);                                
                        }
                        if let Ok(microplate) = serde_json::from_slice::<Microplate>(&buf) {
                            self.microplate = microplate;
                        } else {
                            self.serde_error_modal = Some(CantDeserialize);
                        }
                    } else {
                        self.serde_error_modal = Some(FileNotFound);
                    }
                }
            }
        });
    }
    
    fn show_modals(&mut self, ui: &mut Ui) {
        use SerdeError::*;

        if let Some(serde_error) = self.serde_error_modal.clone() {
            Modal::new(Id::new("Load Assay Error")).show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    let label = match serde_error {
                        FileNotFound => "Could not find file",
                        CantReadFile => "Could not read contents of the file",
                        CantWriteFile => "Could not write contents to the file",
                        CantDeserialize => "Could not load microplate from contents",
                    };
                    ui.label(format!("{}\nPlease try a different file.", label));
                    ui.add_space(10.0);
                    ui.separator();
                    if ui.button("Ok").clicked() {
                        self.serde_error_modal = None;
                    } 
                });
            });
        }
        
        if let Some(value_error) = self.value_error_modal.clone() {
            Modal::new(Id::new("Value Error")).show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    
                    use ValueError::*;
                    let text = match value_error {
                        UnassignedConcentration => "Microplate has a standard sample without a concentration.",
                        UnassignedValue => "Microplate has a sample without a value.",
                        InvalidConcentration => "Microplate has a standard sample with an invalid concentration.",
                        InvalidValue => "Microplate has a sample an invalid value.",
                        NotEnoughStandards => "Microplate does not have enough standards for four parameter analysis.",
                    };
                    ui.label(text);
                    ui.add_space(10.0);
                    ui.separator();
                    if ui.button("Ok").clicked() {
                        self.value_error_modal = None;
                    } 
                });
            });
        }
    }

    pub fn dashed_outline(ui: &mut Ui, response: &Response) {
        let rect = response.rect;

        let stroke_active = ui.visuals().widgets.active.bg_stroke;
        let stroke_hovered = ui.visuals().widgets.hovered.bg_stroke;
        let stroke_inactive = ui.visuals().widgets.inactive.bg_stroke;

        let stroke = if response.clicked() || response.has_focus() {
            stroke_active.color
        } else if response.hovered() {
            stroke_hovered.color
        } else {
            stroke_inactive.color
        };

        let points = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];

        let mut shapes = vec![];
        Shape::dashed_line_many(&points, (1.15, stroke), 2.25, 2.25, &mut shapes);
        let painter = ui.painter();
        for shape in shapes {
           painter.add(shape);
        }
    }
}
