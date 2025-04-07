use std::{cmp::Ordering::*, fmt::Display, fs::File, io::BufReader, num::ParseFloatError, path::PathBuf};

use calamine::{open_workbook, DataType, Reader, ReaderRef, Xlsx, XlsxError};
use eframe::{egui::{self, text::LayoutJob, vec2, Align2, Color32, DragValue, FontFamily, FontId, Grid, Layout, Margin, Response, RichText, ScrollArea, Sense, Shape, Stroke, TextEdit, Ui, Vec2, Widget}, epaint};
use egui_extras::{Column, TableBuilder};

use crate::{*, logistic_regression::*};

const ALPHABET: [char; 26] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
    'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'
];

struct SampleButton<'a> {
    sample: Sample,
    radius: f32,
    current_value: &'a mut Option<usize>,
    alternative: usize,
}

impl<'a> SampleButton<'a> {
    fn new(sample: Sample, radius: f32, current_value: &'a mut Option<usize>, alternative: usize) -> Self {
        Self {
            sample,
            radius,
            current_value,
            alternative,
        }
    }
}

impl Widget for SampleButton<'_>{
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            sample,
            radius,
            current_value,
            alternative,
        } = self;

        let min_size = 2.0 * Vec2::splat(radius);
        let (response, painter) = ui.allocate_painter(min_size + Vec2::splat(4.0), Sense::click());
        let visuals = &ui.visuals().widgets;

        let stroke = if Some(alternative) == *current_value {
            visuals.active.fg_stroke
        } else if response.hovered() {
            visuals.hovered.fg_stroke
        } else {
            visuals.inactive.fg_stroke
        };
        painter.circle(
            response.rect.center(),
            radius,
            sample.typ.color(),
            stroke
        );
        let text = match sample.typ {
            SampleType::Unknown | SampleType::Standard => true,
            SampleType::Blank | SampleType::Unused | SampleType::Control => false,
        };
        
        if text {
            painter.text(
                response.rect.center(),
                Align2::CENTER_CENTER,
                format!("{}", sample.group + 1),
                FontId::default(),
                ui.visuals().text_color()
            );
        }

        response
    }
}

impl Elisa {
    pub fn measurements(&mut self, ui: &mut Ui) {
        let microplate = &mut self.microplate;
        let textfield = &mut self.data_textfield;
        let data_sheets = &mut self.sheet_names;
        let selected_sheet = &mut self.selected_sheet;
        let excel = &mut self.excel;

        let width = 293.0;
        let space = 10.0;
        let stroke = ui.visuals().noninteractive().bg_stroke;
        let fill = ui.visuals().faint_bg_color;

        ui.vertical(|ui| {
            egui::Frame::new().show(ui, |ui| {
                ui.set_width(width);
                ui.vertical_centered(|ui| { ui.heading("Measurements"); });
                ui.add_space(space);
                egui::Frame::new()
                    .fill(fill).stroke(stroke)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_width(width - 20.0);
                        ui.set_height(ui.available_height());
                        ui.horizontal(|ui| {
                            egui::Frame::new().show(ui, |ui| {
                                let button = ui.button(RichText::new("Open"));
                                Self::dashed_outline(ui, &button);
                                if button.clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Excel Spreadsheet", &["xlsx"])
                                        .pick_file() {
                                        match open_workbook::<Xlsx<_>, PathBuf>(path) {
                                            Ok(mut xlsx) => {
                                                *data_sheets = xlsx.sheet_names();
                                                if data_sheets.is_empty() {
                                                    todo!();
                                                }
                                                match Elisa::parse_xlsx_sheet(&mut xlsx, *selected_sheet) {
                                                    Ok(data) => {
                                                        let string = Elisa::data_to_string(data); 
                                                        *textfield = string;
                                                    },
                                                    Err(error) => eprintln!("error parsing excel sheet: {}", error)
                                                }
                                                *excel = Some(xlsx);  
                                            }
                                            Err(err) => eprintln!("Could not load excel spreadsheet: {err}"),
                                        }
                                    }
                                }
                            });

                            ui.add_space(space);
                            ui.label(RichText::new("or edit manually:").size(15.0));
                        });
                        ui.add_space(space);
                        if let Some(excel) = excel {
                            match data_sheets.len().cmp(&1) {
                                Greater => {
                                    ScrollArea::horizontal().max_height(20.0).id_salt("Sheets").show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            for (i, sheet) in data_sheets.iter().enumerate() {
                                                if ui.radio_value(selected_sheet, i, sheet).clicked() {
                                                    match Elisa::parse_xlsx_sheet(excel, *selected_sheet) {
                                                        Ok(data) => {
                                                           let string = Elisa::data_to_string(data);
                                                           *textfield = string;
                                                        },
                                                        Err(error) => eprintln!("Error parsing excel sheet: {}", error)
                                                    }
                                                }
                                                ui.add_space(space);
                                            }
                                        });
                                        ui.add_space(space);
                                    });
                                },
                                Equal => {
                                    match Elisa::parse_xlsx_sheet(excel, *selected_sheet) {
                                        Ok(data) => {
                                               let string = Elisa::data_to_string(data); 
                                               *textfield = string;
                                        },
                                        Err(error) => eprintln!("error parsing excel sheet: {}", error)
                                    }
                                },
                                Less => ()
                
                            }
                        }

                        let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                            let font_id = FontId::monospace(12.0);
                            let layout_job = LayoutJob::simple(string.to_owned(), font_id, Color32::BLACK, f32::INFINITY);
                            ui.fonts(|f| f.layout_job(layout_job))
                        };
                        
                        let text_edit_height = ui.available_height() - 40.0;

                        let scroll_area = ScrollArea::both()
                            .max_height(text_edit_height)
                            .id_salt("Measurements")
                            .show(ui, |ui| {
                                ui.add(egui::TextEdit::multiline(textfield)
                                    .layouter(&mut layouter)
                                    .desired_rows(microplate.height)
                                    .desired_width(f32::INFINITY)
                                )
                        });
                        let mut text_edit = scroll_area.inner;
                        text_edit.rect = scroll_area.inner_rect;
                        text_edit.rect.max.y = ui.cursor().min.y; // If you don't do this, the rect will grow past the cursor, for some reason
                        Self::dashed_outline(ui, &text_edit);
                        ui.add_space(space);
                        match Elisa::string_to_data(textfield, microplate.width, microplate.height) {
                            Ok(data) => {
                                let button = ui.button("Assign values");
                                Self::dashed_outline(ui, &button);
                                if button.clicked() {
                                    for (y, line) in data.into_iter().enumerate() {
                                        for (x, cell) in line.into_iter().enumerate() {
                                            microplate.samples[microplate.height * x + y].value = cell;
                                        }
                                    }
                                }
                            },
                            Err(error) => {
                                eprintln!("Error parsing string to data: {}", error);
                                ui.label("Could not parse data");
                            }
                        }
                    });
            });
        });
    }
    
    pub fn microplate_view(&mut self, ui: &mut Ui) {
        let microplate = &mut self.microplate;
        let radius = 30.0 / 2.0;
        let spacing = 10.0 - 4.0;
        let cell_size = 2.0 * Vec2::splat(radius);
        let response_color = ui.visuals().text_color();

        let where_to_put_background = ui.painter().add(Shape::Noop);
        
        let frame_response = egui::Frame::new().inner_margin(Margin { right: 17, bottom: 17, ..default()}).show(ui, |ui| {
            Grid::new("Microplate")
                .spacing(Vec2::splat(spacing))
                .min_col_width(radius + spacing / 2.0)
                .max_col_width(radius + spacing / 2.0)
                .min_row_height(radius + spacing / 2.0)
                .show(ui, |ui| {
                    ui.allocate_exact_size(cell_size, Sense::hover());
                    for i in 1..=microplate.width {
                        let (response, painter) = ui.allocate_painter(cell_size, Sense::hover());
                        painter.text(
                            response.rect.center(),
                            Align2::CENTER_TOP,
                            format!("{i}"),
                            FontId::new(radius, FontFamily::default()),
                            response_color
                        );
                    }
                    ui.end_row();
                    for i in 0..microplate.height {
                        let (response, painter) = ui.allocate_painter(cell_size, Sense::hover());
                        painter.text(
                            response.rect.center(),
                            Align2::LEFT_CENTER,
                            ALPHABET[i%26],
                            FontId::new(radius, FontFamily::default()),
                            response_color
                        );
                        for ii in 0..microplate.width {
                            let index = ii * microplate.height + i;
                            let sample = microplate.samples[index].clone();
                            let response = ui.add(SampleButton::new(sample, radius, &mut self.selected_sample, index));
                            if response.clicked() {
                                if self.selected_sample == Some(index) {
                                    self.selected_sample = None;
                                } else {
                                    self.selected_sample = Some(index);
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
        });

        let fill = ui.visuals().faint_bg_color;
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;

        let mut rect = frame_response.response.rect;
        rect.set_width(rect.width());
        rect.set_height(rect.height());
        let bevel_point_1 = rect.left_top() + vec2(0.0, 30.0);
        let bevel_point_2 = rect.left_top() + vec2(30.0, 0.0);
        let points = [bevel_point_1, bevel_point_2, rect.right_top(), rect.right_bottom(), rect.left_bottom()];
        let mut shape = epaint::PathShape::closed_line(points.to_vec(), stroke);
        shape.fill = fill;

        ui.painter().set(where_to_put_background, shape);
    }
    
    pub fn sample_menu(&mut self, ui: &mut Ui) {
        let radius = 15.0;
        let samples = &mut self.microplate.samples;
        let stroke = ui.visuals().noninteractive().bg_stroke;
        let fill = ui.visuals().faint_bg_color;

        ui.vertical(|ui| {
            egui::Frame::new().show(ui, |ui| {
                let width = ui.available_width();
                ui.set_width(width);
                ui.vertical_centered(|ui| { ui.heading("Sample Menu"); });
                ui.add_space(10.0);
                egui::Frame::new()
                    .fill(fill).stroke(stroke)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_width(width - 20.0);
                        ui.set_min_height(195.0);
                        if let Some(index) = self.selected_sample {
                            use SampleType::*;
                            
                            ui.horizontal(|ui| {
                                ui.label(format!("Selected sample {}", index + 1));

                                let (response, painter) = ui.allocate_painter(vec2(ui.available_width(), 2.0 * radius), Sense::hover());
                                painter.circle(response.rect.right_center() - vec2(2.0 * radius - 10.0, 0.0), radius, samples[index].typ.color(), Stroke::NONE);
                            });
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            let row_height = 30.0;
                            let mut list = vec!["Sample Type", "Measurement"];
                            match samples[index].typ {
                                Standard => {
                                    list.push("Group")
                                },
                                Unknown => {
                                    list.push("Group");
                                    list.push("Label");
                                }
                                _ => ()
                            }

                            // Building two tables with different alignment is suboptimal
                            ui.horizontal_top(|ui| {
                                TableBuilder::new(ui).id_salt("Names")
                                    .column(Column::auto()).body(|body| {
                                        body.rows(row_height, list.len(), |mut rows| {
                                            let index = rows.index();
                                            rows.col(|ui| {
                                                ui.horizontal_centered(|ui| {
                                                    ui.label(list[index]);
                                                });
                                            });
                                        });
                                });
                                TableBuilder::new(ui).id_salt("Ui objects").column(Column::remainder())
                                    .cell_layout(Layout::default().with_cross_align(egui::Align::Max))
                                    .body(|mut body| {
                                        body.row(row_height, |mut row| {
                                            row.col(|ui| {
                                                ui.horizontal_centered(|ui| {
                                                    let menu_button = ui.menu_button(format!("{:?}", samples[index].typ), |ui| {
                                                        if ui.button("Unused").clicked() { samples[index].typ = Unused }
                                                        if ui.button("Standard").clicked() { samples[index].typ = Standard }
                                                        if ui.button("Control").clicked() { samples[index].typ = Control }
                                                        if ui.button("Unknown").clicked() { samples[index].typ = Unknown }
                                                        if ui.button("Blank").clicked() { samples[index].typ = Blank }
                                                    });
                                                    Self::dashed_outline(ui, &menu_button.response);
                                                });
                                            });
                                        });
                                        body.row(row_height, |mut row| {
                                            row.col(|ui| {
                                                ui.horizontal_centered(|ui| {
                                                    let measurement = samples[index].value.map(|f| format!("{:.5}", f)).unwrap_or("N/A".to_string());
                                                    ui.label(measurement);
                                                });
                                            });                                        
                                        });

                                        if samples[index].typ == Unknown || samples[index].typ == Standard {
                                            body.row(row_height, |mut row| {
                                                row.col(|ui| {
                                                    ui.horizontal_centered(|ui| {
                                                        self.selected_sample_group = samples[index].group + 1;
                                                        let drag_value = DragValue::new(&mut self.selected_sample_group).speed(0.03).range(1..=100);
                                                        let mut drag_value_resp = ui.add(drag_value);
                                                        samples[index].group = self.selected_sample_group - 1;
                                                
                                                        let id = drag_value_resp.id;
                                                        // stolen from egui source code
                                                        let interactive = ui.memory_mut(|mem| {
                                                            mem.interested_in_focus(id, ui.layer_id());
                                                            mem.has_focus(id)
                                                        });

                                                        if interactive {
                                                            drag_value_resp.rect = drag_value_resp.rect.expand2(vec2(9.0, 3.0));
                                                        }

                                                        Self::dashed_outline(ui, &drag_value_resp);
                                                    });

                                                    let max_standard_group = samples.iter()
                                                        .filter(|sample| sample.typ == SampleType::Standard)
                                                        .map(|sample| sample.group)
                                                        .max().unwrap_or_default();
                                                    self.microplate.standard_groups.resize_with(max_standard_group + 1, default);

                                                    let max_unknown_group = samples.iter()
                                                        .filter(|sample| sample.typ == SampleType::Unknown)
                                                        .map(|sample| sample.group)
                                                        .max().unwrap_or_default();
                                                    self.microplate.unknown_groups.resize_with(max_unknown_group + 1, default);
                                                });
                                            });
                                        }

                                        if samples[index].typ == Unknown {
                                            body.row(row_height, |mut row| {
                                                row.col(|ui| {
                                                    ui.horizontal_centered(|ui| {
                                                        let label = &mut self.microplate.unknown_groups[samples[index].group].label;
                                                        let mut text_edit = ui.add(TextEdit::singleline(label).desired_width(100.0));
                                                        text_edit.rect = text_edit.rect.expand2(vec2(4.0, 2.0));
                                                        Self::dashed_outline(ui, &text_edit);
                                                    });
                                                });
                                            });
                                        }
                                    });
                            });
                        } else {
                            ui.label("Please select a sample from the microplate.");
                        }
                });
            });
        });            
    }
    
    pub fn standards_concentrations(&mut self, ui: &mut Ui) {
        let groups = &mut self.microplate.standard_groups;
        
        let stroke = ui.visuals().noninteractive().bg_stroke;
        let fill = ui.visuals().faint_bg_color;

        ui.vertical(|ui| {
            egui::Frame::new().show(ui, |ui| {
                let width = ui.available_width();
                ui.set_width(width);
                ui.vertical_centered_justified(|ui| { ui.heading("Standards Concentrations") });
                ui.add_space(10.0);
                egui::Frame::new()
                    .fill(fill).stroke(stroke)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_width(width - 20.0);
                        let height = ui.available_height();
                        ui.set_min_height(height);
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                TableBuilder::new(ui)
                                    .id_salt("Standards Concentrations")
                                    .min_scrolled_height(height - 20.0)
                                    .max_scroll_height(height - 20.0)
                                    .columns(Column::exact(90.0), 2)
                                    .header(20.0, |mut header| {
                                        header.col(|ui| { ui.label("Group"); });
                                        header.col(|ui| { ui.label("Concentrations"); });
                                    })
                                    .body(|body| {
                                        body.rows(25.0, groups.len(), |mut row| {
                                            let index = row.index();
                                            let text_edit = &mut self.standards_textfield[index];
                                            row.col(|ui| { ui.label(format!("Standard {}", index + 1)); });
                                            if let Some(concentration) = groups[index].concentration {
                                                *text_edit = concentration.to_string();
                                            }
                                            row.col(|ui| {
                                                let mut text_edit = ui.text_edit_singleline(text_edit);
                                                text_edit.rect = text_edit.rect.expand2(vec2(3.7, 1.7));
                                                Self::dashed_outline(ui, &text_edit);    
                                            });
                                            groups[index].concentration = text_edit.parse().ok();
                                        });
                                    });
                            });
                            ui.add_space(10.0);
                            

                            let (button, painter) = ui.allocate_painter(Vec2::splat(26.0), Sense::click());

                            let background_fill = ui.visuals().widgets.inactive.weak_bg_fill;
                            let stroke_active = ui.visuals().widgets.active.bg_stroke;
                            let stroke_hovered = ui.visuals().widgets.hovered.bg_stroke;
                            let stroke_inactive = ui.visuals().widgets.inactive.bg_stroke;
                            
                            let stroke = if button.clicked() || button.has_focus() {
                                stroke_active.color
                            } else if button.hovered() {
                                stroke_hovered.color
                            } else {
                                stroke_inactive.color
                            };
                            let font_id = FontId::proportional(10.0);

                            painter.circle_filled(button.rect.center(), 12.0, background_fill);
                            painter.text(button.rect.center(), Align2::CENTER_CENTER, "➗2", font_id, Color32::BLACK);
                            painter.circle_stroke(button.rect.center(), 12.0, (1.15, stroke));
                            if button.clicked() {
                                if let Some(Group { concentration: Some(mut next), .. }) = groups.first() {
                                    for (i, group) in groups.iter_mut().enumerate().skip(1) {
                                        next /= 2.0;
                                        self.standards_textfield[i] = next.to_string();
                                        group.concentration = Some(next);
                                    }
                                }
                            }
                        });
                    });
            });
        });
    }
    
    pub fn run_notes(&mut self, ui: &mut Ui) {
        let microplate = &mut self.microplate;

        let space = 10.0;
        let stroke = ui.visuals().noninteractive().bg_stroke;
        let fill = ui.visuals().faint_bg_color;

        ui.vertical(|ui| {
            egui::Frame::new().show(ui, |ui| {
                ui.set_width(200.0);
                ui.vertical_centered_justified(|ui| { ui.heading("Run Notes") });
                ui.add_space(space);
                egui::Frame::new()
                    .fill(fill).stroke(stroke)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_width(180.0);
                        ui.set_min_height(ui.available_height());

                        ui.horizontal(|ui| {
                            ui.label("Name");
                            ui.add_space(50.0);
                            let mut text_edit = ui.add(TextEdit::singleline(&mut microplate.name));
                            text_edit.rect = text_edit.rect.expand2(vec2(4.0, 2.0)); // Account for margin
                            Self::dashed_outline(ui, &text_edit);
                        });

                        ui.add_space(space);
                        ui.label("Description");
                        ui.add_space(5.0);
                        let scroll_area = egui::ScrollArea::vertical()
                            .max_height(ui.available_height() - 40.0)
                            .show(ui, |ui| {
                                ui.add(TextEdit::multiline(&mut microplate.description).desired_rows(8))
                            });
                        let mut text_edit = scroll_area.inner;
                        text_edit.rect = scroll_area.inner_rect;
                        text_edit.rect.max.y = ui.cursor().min.y; // If you don't do this, the rect will grow past the cursor, for some reason
                        Self::dashed_outline(ui, &text_edit);
                        ui.add_space(space);

                        let button = ui.button("Calculate");
                        Self::dashed_outline(ui, &button);
                        if button.clicked() {
                            match Regression::new(microplate) {
                                Ok(regression) => {
                                    self.regression = Some(regression);
                                    self.current_tab = ElisaTab::Result;
                                },
                                Err(error) => { self.value_error_modal = Some(error) }
                            }
                        }
                    });
            });
        });
    }
    
    fn string_to_data(data: &str, width: usize, height: usize) -> Result<Vec<Vec<Option<f64>>>, StringToDataError> {
        use StringToDataError::*;
        let mut result = Vec::new();
        for line in data.lines() {
            let mut row = Vec::new();
            for value in line.split_whitespace() {
                if value == "_" {
                    row.push(None);
                } else {
                    let value = value.to_string().replace(",", ".");
                    row.push(Some(value.parse::<f64>()?));
                }
            }
            if row.len() > width { return Err(WidthTooLarge) }
            result.push(row);
        }
        if result.len() > height { return Err(HeightTooLarge) }

        Ok(result)
    }

    fn data_to_string(data: Vec<Vec<Option<f64>>>) -> String {
        let mut result = String::new();
        for row in data {
            for value in row {
                if let Some(value) = value {
                    result.push_str(&value.to_string());
                } else {
                    result.push('_');
                }
                result.push(' ');
            }
            result.push('\n');
        }
        println!("{}", result);
        result
    }
    
    fn parse_xlsx_sheet(excel: &mut Xlsx<BufReader<File>>, sheet: usize) -> Result<Vec<Vec<Option<f64>>>, ParseExcelError> {
        use ParseExcelError::*;

        let data = excel.worksheet_range_at_ref(sheet).unwrap()?;
        if data.get_size() < (65, 8) {
            return Err(SheetSize)
        }
        let Some(mut table_dimensions) = data[(25, 4)].as_string() else {
            return Err(NoDimensions)
        };
        table_dimensions.retain(|char| char.is_ascii_uppercase());
        let table_height = table_dimensions.chars().max().unwrap_or('A'); // maybe replace unwrap_or(...) with else { return Err(...)}?
        let table_height = (u32::from(table_height) - u32::from('A') + 1) as usize;
        let result: Vec<Vec<Option<f64>>> = data.rows()
            .skip(37 + 2 * table_height)
            .take(table_height)
            .map(|row| 
                row.iter()
                    .skip(1)
                    .map(|cell| cell.get_float())
                    .collect()
            ).collect();
        Ok(result)
    }
}

// Hmmm... maybe I should use thiserror

#[derive(Debug)]
enum StringToDataError {
    WidthTooLarge,
    HeightTooLarge,
    Parse(ParseFloatError),
}

impl From<ParseFloatError> for StringToDataError {
    fn from(value: ParseFloatError) -> Self {
        Self::Parse(value)
    }
}

impl Display for StringToDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = match self {
            Self::WidthTooLarge => String::from("String has more entries than microplate is wide"),
            Self::HeightTooLarge => String::from("String has more entries than microplate is high"),
            Self::Parse(value) => format!("{}", value),
        };

        write!(f, "{}", error)
    }
}

#[derive(Debug)]
enum ParseExcelError {
    SheetSize,
    NoDimensions,
    XlsxError(XlsxError),
}

impl From<XlsxError> for ParseExcelError {
    fn from(value: XlsxError) -> Self {
        ParseExcelError::XlsxError(value)
    }
}

impl Display for ParseExcelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = match self {
            Self::SheetSize => String::from("Sheet size is too small"),
            Self::NoDimensions => String::from("Could not parse table dimensions"),
            Self::XlsxError(value) => format!("{}", value)
        };
        write!(f, "{}", error)
    }
}
