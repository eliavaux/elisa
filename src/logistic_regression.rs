use crate::*;
use egui::Color32;
use serde::{Deserialize, Serialize};
use SampleType::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum SampleType {
    #[default]
    Unused,   // Unused
    Blank,    // Noise
    Control,  // Concentration of 0%
    Standard, // Standard values for curve
    Unknown,  // Unknowns we want to estimate
}

impl SampleType {
    pub fn cycle(&self) -> SampleType {
        match self {
            Unused => Blank,
            Blank => Control,
            Control => Standard,
            Standard => Unknown,
            Unknown => Unused,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Unused => Color32::from_hex("#D8DCE7").unwrap(),
            Unknown => Color32::from_hex("#8CF490").unwrap(),
            Standard => Color32::from_hex("#F57373").unwrap(),
            Control => Color32::from_hex("#818FEF").unwrap(),
            Blank => Color32::from_hex("#F1E07D").unwrap(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sample {
    pub typ: SampleType,
    pub group: u8,          // index to group in microplate
    pub label: String,
    pub value: Option<f64>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Group {
    pub concentration: Option<f64>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Microplate {
    pub name: String,
    pub description: String,
    pub height: usize,
    pub width: usize,
    pub samples: Vec<Sample>,
    pub standard_groups: Vec<Group>,
    pub unknown_groups: Vec<Group>,
}

impl Microplate {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            height,
            width,
            samples: vec![default(); width * height],
            ..default()
        }
    }
}

#[derive(Clone, Debug)]
pub enum ValueError {
    UnassignedConcentration,
    UnassignedValue,
    InvalidConcentration,
    InvalidValue,
    NotEnoughStandards,
}

pub enum RegressionError {
}

#[derive(Clone)]
pub struct Regression {
    pub abcd: (f64, f64, f64, f64),
    pub blanks: Vec<(usize, f64)>,
    pub controls: Vec<(usize, f64)>,
    pub unknowns: Vec<(usize, f64, f64)>,
    pub standards: Vec<(usize, f64, f64)>,
}

impl Regression {
    pub fn new(microplate: &Microplate) -> Result<Self, ValueError> {
        use ValueError::*;

        let abcd = (0.0, 0.0, 0.0, 0.0);
        let mut blanks = Vec::new();
        let mut unknowns = Vec::new();
        let mut standards = Vec::new();
        let mut controls = Vec::new();

        for (i, Sample { typ, group, value, .. }) in microplate.samples.iter().enumerate() {
            if typ == &Unused { continue }
            let Some(value) = *value else { return Err(UnassignedValue) };
            if !value.is_finite() { return Err(InvalidValue) }

            match typ {
                Unknown => unknowns.push((i, 0.0, value)),
                Standard => {
                    let group = &microplate.standard_groups[*group as usize];
                    if let Some(concentration) = group.concentration {
                        if !concentration.is_finite() { return Err(InvalidConcentration) }
                        standards.push((i, concentration, value))
                    } else {
                        return Err(UnassignedConcentration)
                    }
                },
                Control => controls.push((i, value)),
                Blank => blanks.push((i, value)),
                Unused => (),
            }
        }
        if standards.len() < 4 { return Err(NotEnoughStandards) }

        Ok(Self { abcd, blanks, unknowns, standards, controls })
    }

    pub fn four_pl(&self, x: f64) -> f64 {
        let (a, b, c, d) = self.abcd;
        d + ((a - d) / (1.0 + (x/c).powf(b)))
    }

    pub fn sum_of_squares(&self) -> f64 {
        self.standards.iter().map(|&(_, x, y)| {
            let diff = y - self.four_pl(x);
            diff * diff
        }).sum()
    }
    
    pub fn mean_squared_error(&self) -> f64 {
        let length = self.standards.len() as f64;
        let sum_of_squares = self.sum_of_squares();
        sum_of_squares / length
    }

    pub fn root_mean_squared_error(&self) -> f64 {
        let length = self.standards.len() as f64;
        let sum_of_squares = self.sum_of_squares();
        (sum_of_squares / (length - 1.0)).sqrt()
    }

    pub fn sy_x(&self) -> f64 {
        let length = self.standards.len() as f64;
        let sum_of_squares = self.sum_of_squares();
        (sum_of_squares / (length - 4.0)).sqrt()
    }
    
    pub fn calculate_unknowns(&mut self, abcd: &(f64, f64, f64, f64)) {
        let (a, b, c, d) = abcd;
        for (_i, x, y) in &mut self.unknowns {
            *x = c * ((a - d) / (*y - d) - 1.0).powf(1.0 / b)
        }
    }
    
    pub fn four_pl_curve_fit(&mut self) -> Result<(f64, f64, f64, f64), RegressionError> {
        let Self { blanks, unknowns, standards, controls, .. } = self;

        // subtract blanks
        if !blanks.is_empty() {
            let blanks_mean = blanks.iter().map(|(_, v)| v).sum::<f64>() / blanks.len() as f64;
            unknowns.iter_mut().for_each(|(_, _c, v)| *v -= blanks_mean);
            standards.iter_mut().for_each(|(_, _c, v)| *v -= blanks_mean);
            controls.iter_mut().for_each(|(_, v)| *v -= blanks_mean);
        }

        standards.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap()); // Sort standards by concentration
        let n = standards.len() as f64;

        let min = standards.first().unwrap();
        let max = standards.last().unwrap();

        // guess initial values
        let mut a = if !controls.is_empty() { // 0-dose asymptote
            controls.iter().map(|(_, v)| v).sum::<f64>() / controls.len() as f64
        } else { min.1 };
        let mut d = max.2;                    // inf-dose asymptote
        let mut c = (max.1 - min.1).sqrt();   // IC50 interpolation (log-scale)
        let mut b = 2.0;                      // slope at IC50

        let learn_rate = (0.1, 1.5, 5_000_000.0, 0.5); // These values seem to work well, idk why c's learning rate is so high

        for i in 0..100_000 {
            let mut sum_a = 0.0;
            let mut sum_b = 0.0;
            let mut sum_c = 0.0;
            let mut sum_d = 0.0;

            for (_i, x, y) in standards.iter() {
                let xc = x / c;
                let xcb = xc.powf(b);
                let xcb1 = xcb + 1.0;
                let xcb1sq = xcb1 * xcb1;
                let lxcxcb = xc.log10() * xcb;
                
                let diff = y - d - (a - d) / xcb1;
                let duda = 1.0 / xcb1;
                let dudb = lxcxcb / xcb1sq;
                let dudc = xcb / xcb1sq;
                let dudd = -(1.0 / xcb1) - 1.0;
               
                sum_a += diff * duda;
                sum_b += diff * dudb;
                sum_c += diff * dudc;
                sum_d += diff * dudd;   
            }

            let da = 2.0 / n * sum_a;
            let db = 2.0 * (d - a) / n * sum_b;
            let dc = 2.0 * b * (a - d) / c / n * sum_c;
            let dd = 2.0 / n * sum_d;
            
            a += learn_rate.0 * da;
            b += learn_rate.1 * db;
            c += learn_rate.2 * dc;
            d -= learn_rate.3 * dd;

            if i % 1000 == 0 { println!("a: {}, b: {}, c: {}, d: {}", a, b, c, d) };
        }

        self.abcd = (a, b, c, d);
        Ok((a, b, c, d))
    }
}
