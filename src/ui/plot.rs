use std::path::PathBuf;

use eframe::egui::{self, vec2, Color32, Label, RichText, Ui, UserData};
use egui_extras::{Column, TableBuilder};
use egui_plot::{AxisTransforms, Line, Plot, PlotPoint, PlotPoints, Points, Text};
use image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use pdf_writer::{Content, Finish, Name, Pdf, Ref, Str, TextStr};

use crate::{logistic_regression::*, Elisa};

impl Elisa {
    pub fn plot(&mut self, ui: &mut Ui) {
        let Some(regression) = self.regression.as_ref() else { return };
        let Regression { abcd, unknowns, standards, ..} = regression;
        let Microplate { samples, ..} = &self.microplate;
        let &(a, b, c, d) = abcd;
        
        let stroke = ui.visuals().noninteractive().bg_stroke;

        let color = ui.style().noninteractive().text_color();
        let axis_transforms = AxisTransforms::new(egui_plot::AxisTransform::Logarithmic(10.0), egui_plot::AxisTransform::Linear);

        let four_pl = move |x: f64| {
            d + ((a - d) / (1.0 + (x/c).powf(b)))
        };
        
        ui.add_space(10.0);
        let mut plot = Plot::new("4PL Plot")
            .show_x(false)
            .show_y(false)
            .axis_transforms(axis_transforms)
            .x_axis_label("Dose")
            .y_axis_label("Measurement")
            .show_background(false)
            .height(500.0)
            .width(500.0)
            .show(ui, |ui| {
            // Curve
            let line_points = PlotPoints::from_explicit_callback(four_pl, .., 5000);
            let line = Line::new(line_points)
                .allow_hover(false)
                .color(color)
                .name("4PL");
            ui.line(line);
        
            // Standards points
            for (i, &(_j, dose, value)) in standards.iter().enumerate() {
                let label = &samples[i].label;
                let color = SampleType::Standard.color();
            
                let point = Points::new([dose, value])
                    .name(label)
                    .radius(5.0)
                    .color(color);
                ui.points(point);
            }
        
            // Unknowns points
            let white = Color32::from_hex("#FBFBFE").unwrap();
            for (i, &(j, dose, value)) in unknowns.iter().enumerate() {
                let label = &samples[j].label;
                let color = SampleType::Unknown.color();
                let name = if label.is_empty() {
                    format!("Unknown {}", i + 1)
                } else {
                    label.clone()
                };

                let point = Points::new([dose, value])
                    .name(label)
                    .radius(5.0)
                    .color(color);

                ui.points(point);

                let mut point = ui.screen_from_plot(PlotPoint::new(dose, value));
                point.y -= 15.0;
                let point = ui.plot_from_screen(point);
                ui.text(Text::new(
                    point,
                    RichText::new(name).size(11.0).background_color(white.gamma_multiply(0.5))
                ));
            }
        });
        ui.painter().rect_stroke(plot.response.rect, 0.0, stroke, eframe::egui::StrokeKind::Inside);
        plot.response.rect = plot.response.rect.expand(10.0);
        plot.response.rect.min.x -= 40.0;
        plot.response.rect.max.y += 40.0;
        self.plot_response = Some(plot.response);
    }

    pub fn plot_parameters(&mut self, ui: &mut Ui) -> Option<()> {
        let regression = self.regression.as_ref()?;
        let Regression { abcd, ..} = regression;
        let &(a, b, c, d) = abcd;

        let background = ui.visuals().faint_bg_color;
        let stroke = ui.visuals().noninteractive().bg_stroke;

        let mse = regression.mean_squared_error();
        let sse = regression.sum_of_squares();
        let sy_x = regression.sy_x();
        let rmse = regression.root_mean_squared_error();
        let list = [("a", a), ("b", b), ("c", c), ("d", d), ("MSE", mse), ("SSE", sse), ("Sy.x", sy_x), ("RMSE", rmse)];

        self.plot_parameters = Some(list);

        egui::Frame::new().show(ui, |ui| {
            let width = ui.available_width().max(20.0);
            ui.set_width(width);

            ui.vertical_centered(|ui| ui.heading("Parameters"));
            ui.add_space(10.0);
            egui::Frame::new()
                .fill(background).stroke(stroke)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_width(width - 20.0);
                    ui.spacing_mut().item_spacing = vec2(20.0, 5.0);

                    TableBuilder::new(ui).id_salt("Plot parameters")
                        .max_scroll_height(100.0)
                        .column(Column::auto())
                        .column(Column::remainder())
                        .body(|body| {
                            body.rows(20.0, list.len(), |mut row| {
                                let index = row.index();
                                row.col(|ui| { ui.add(Label::new(list[index].0).selectable(true)); });
                                row.col(|ui| { ui.add(Label::new(format!("{}", list[index].1)).selectable(true)); });
                            });
                        });
                });
        });
        Some(())
    }

    pub fn save_as(&mut self, ui: &mut Ui) {
        let Some(plot_response) = &self.plot_response else { return };
        
        ui.horizontal(|ui| {
            let button = ui.button(RichText::new("Save as PNG"));
            Self::dashed_outline(ui, &button);
            if button.clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(UserData::default()));
            }
            ui.add_space(10.0);

            let button = ui.button(RichText::new("Save as PDF"));
            Self::dashed_outline(ui, &button);
            if button.clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(UserData::default()));
                self.pdf_report = true;
            }

            let image = ui.ctx().input(|i| {
                i.events.iter()
                    .filter_map(|event| {
                        if let egui::Event::Screenshot { image, .. } = event {
                            Some(image.clone())
                        } else {
                            None
                        }
                    }).last()
            });

            if let Some(image) = image {
                let ppp = ui.pixels_per_point();
                let image = image.region(&plot_response.rect, Some(ppp));
                // if we ever need to render the image
                // let texture = ui.ctx().load_texture("screenshot", image.clone(), default());

                let width = image.width();
                let height = image.height();

                // could be done async, but it's fine for now
                let Some(image) = RgbaImage::from_raw(width as u32, height as u32, image.as_raw().to_vec()) else {
                    eprintln!("Image dimensions are wrong, how did we get here...");
                    return
                };

                if self.pdf_report {
                    self.pdf_report = false;

                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("pdf", &["pdf"])
                        .set_file_name(self.microplate.name.clone())
                        .save_file() {
                        create_pdf(path, image, &self.microplate, &self.plot_parameters.unwrap());
                    }
                } else if let Some(path) = rfd::FileDialog::new()
                    .add_filter("png", &["png"])
                    .set_file_name(self.microplate.name.clone())
                    .save_file() {
                    if let Err(error) = image.save(path) {
                        eprintln!("{error}");
                        todo!()
                    }
                }
            }

        });
    }
}

fn create_pdf(path: PathBuf, image: ImageBuffer<Rgba<u8>, Vec<u8>>, microplate: &Microplate, parameters: &[(&str, f64); 8]) {
    let mut pdf = Pdf::new();

    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let page_id = Ref::new(3);
    let content_id = Ref::new(4);
    let font_id = Ref::new(5);
    let image_id = Ref::new(6);
    let annotation_id = Ref::new(7);

    let font_name = Name(b"Times-Roman");
    let image_name = Name(b"Plot");

    // Page tree
    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id).kids([page_id]).count(1);
    pdf.type1_font(font_id).base_font(font_name);

    // A4 page
    let mut page = pdf.page(page_id);
    let a4 = pdf_writer::Rect::new(0.0, 0.0, 595.0, 842.0);
    page.media_box(a4);
    page.parent(page_tree_id);
    page.contents(content_id);

    let mut resources = page.resources();
    resources.fonts().pair(font_name, font_id);
    resources.x_objects().pair(image_name, image_id);
    resources.finish();
    page.annotations([annotation_id]);
    page.finish();

    let mut content = Content::new();

    // Title
    content.begin_text();
    content.set_font(font_name, 24.0);
    content.next_line(50.0, 842.0 - 80.0);
    content.show(Str(b"Assay Analysis - 4PL"));

    // Date
    let date_time = chrono::offset::Local::now();
    let date = format!("{}", date_time.format("%d.%m.%Y, %H:%M"));
    content.set_font(font_name, 12.0);
    content.next_line(-10.0, -20.0);
    content.show(Str(date.as_bytes()));
    content.end_text();

    // Image
    let image_rgb: Vec<u8> = image.pixels().flat_map(|p| {
        let p = p.to_rgb().0;
        // A tad hacky, but it works
        match p {
            [251, 251, 254] => [255, 255, 255],
            _ => p
        }
    }).collect();

    let mut image_obj = pdf.image_xobject(image_id, &image_rgb);
    image_obj.width(image.width() as i32);
    image_obj.height(image.height() as i32);
    image_obj.color_space().device_rgb();
    image_obj.bits_per_component(8);
    image_obj.finish();

    content.save_state();
    content.transform([300.0, 0.0, 0.0, 300.0, 20.0, 842.0 - 440.0]);
    content.x_object(image_name);
    content.restore_state();

    // Table
    content.begin_text();
    content.set_font(font_name, 10.0);
    content.next_line(400.0, 842.0 - 175.0);

    for (name, value) in parameters {
        content.show(Str(name.as_bytes()));
        content.next_line(40.0, 0.0);
        content.show(Str(value.to_string().as_bytes()));
        content.next_line(-40.0, -15.0);
    }
    content.end_text();
    
    // Name
    content.begin_text();
    content.set_font(font_name, 12.0);
    content.next_line(40.0, 842.0 - 470.0);
    content.show(Str(format!("Name: {}", microplate.name).as_bytes()));

    // Description
    content.next_line(0.0, -30.0);
    content.show(Str(b"Description"));


    content.next_line(10.0, -20.0);
    for line in microplate.description.split("\n") {
        content.show(Str(line.as_bytes()));
        content.next_line(0.0, -15.0);
    }
    content.end_text();
    
    // Link
    content.begin_text();
    content.set_font(font_name, 10.0);
    content.next_line(595.0 - 80.0, 40.0);
    content.show(Str(b"Eliavaux"));
    content.end_text();
    
    let mut annotation = pdf.annotation(annotation_id);
    annotation.subtype(pdf_writer::types::AnnotationType::Link);
    annotation.rect(pdf_writer::Rect::new(595.0 - 80.0, 40.0 - 3.0, 595.0 - 80.0 + 40.0, 40.0 + 8.0 + 3.0));
    annotation.contents(TextStr("Link to Eliavaux's GitHub"));
    annotation.color_rgb(0.0, 0.0, 1.0);

    annotation.action()
        .action_type(pdf_writer::types::ActionType::Uri)
        .uri(Str(b"https://www.github.com/eliavaux"));
    // annotation.border_style().width(2.0).style(pdf_writer::types::BorderType::Underline);
    annotation.finish();


    pdf.stream(content_id, &content.finish());    
    std::fs::write(path, pdf.finish()).unwrap();
}
