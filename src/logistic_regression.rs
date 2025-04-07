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
    pub group: usize,        // index to group in microplate
    pub value: Option<f64>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Group {
    pub concentration: Option<f64>,
    pub label: String,
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
            standard_groups: vec![default()],
            unknown_groups: vec![default()],
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

#[derive(Clone, Default)]
pub struct Regression {
    pub abcd: (f64, f64, f64, f64),
    pub blank: f64,
    pub control: f64,
    pub unknowns: Vec<(f64, f64, String)>,
    pub standards: Vec<(f64, f64)>,
    pub sse: f64,
    pub mse: f64,
    pub rmse: f64,
    pub sy_x: f64,
}

impl Regression {
    pub fn new(microplate: &Microplate) -> Result<Self, ValueError> {
        use ValueError::*;

        let unknowns_len = microplate.unknown_groups.len();
        let standards_len = microplate.standard_groups.len();

        // (sum, count) pairs
        let mut blank = (0.0, 0);
        let mut control = (0.0, 0);
        let mut unknowns = vec![(0.0, 0); unknowns_len];
        let mut standards = vec![(0.0, 0); standards_len];

        // add up values
        for Sample { typ, group, value } in &microplate.samples {
            if *typ == Unused { continue }
            let Some(value) = value else { return Err(UnassignedValue) };
            if !value.is_finite() { return Err(InvalidValue) }

            match typ {
                Blank => {
                    blank.0 += value;
                    blank.1 += 1;
                },
                Control => {
                    control.0 += value;
                    control.1 += 1;
                },
                Standard => {
                    standards[*group].0 += value;
                    standards[*group].1 += 1;
                },
                Unknown => {
                    unknowns[*group].0 += value;
                    unknowns[*group].1 += 1;
                }
                Unused => ()
            }
        }
        if standards.len() < 4 { return Err(NotEnoughStandards) }

        let blank = if blank.1 != 0 { blank.0 / blank.1 as f64 } else { 0.0 };
        let control = if control.1 != 0 { control.0 / control.1 as f64 } else { 0.0 };

        let unknowns = unknowns.iter().enumerate().map(|(i, &(sum, count))| {
            let measurement = sum / count as f64;
            let label = microplate.unknown_groups[i].label.clone();
            (0.0, measurement, label)
        }).collect();

        let mut concentrations = vec![0.0; standards_len];
        for (i, group) in concentrations.iter_mut().enumerate() {
            let Some(concentration) = microplate.standard_groups[i].concentration else {
                return Err(UnassignedConcentration)
            };
            if !concentration.is_finite() { return Err(InvalidConcentration) }
            *group = concentration;
        }

        let standards = standards.iter().enumerate().map(|(i, &(sum, count))| {
            let concentration = concentrations[i];
            let measurement = sum / count as f64;
            (concentration, measurement)
        }).collect();

        let mut regression = Self {
            blank,
            control,
            unknowns,
            standards,
            ..default()
        };
        
        regression.four_pl_curve_fit();
        regression.calculate_unknowns();
        regression.calculate_parameters();

        Ok(regression)
    }

    #[inline(always)]
    pub fn four_pl(&self, x: f64) -> f64 {
        let (a, b, c, d) = self.abcd;
        d + ((a - d) / (1.0 + (x/c).powf(b)))
    }

    #[inline(always)]
    pub fn inverse_four_pl(&self, y: f64) -> f64 {
        let (a, b, c, d) = self.abcd;
        c * ((a - d) / (y - d) - 1.0).powf(1.0 / b)
    }

    #[inline(always)]
    pub fn sum_of_squares(&self) -> f64 {
        self.standards.iter().map(|&(x, y)| {
            let diff = y - self.four_pl(x);
            diff * diff
        }).sum()
    }
    
    #[inline(always)]
    pub fn mean_squared_error(&self) -> f64 {
        let length = self.standards.len() as f64;
        let sum_of_squares = self.sum_of_squares();
        sum_of_squares / length
    }

    #[inline(always)]
    pub fn root_mean_squared_error(&self) -> f64 {
        let length = self.standards.len() as f64;
        let sum_of_squares = self.sum_of_squares();
        (sum_of_squares / (length - 1.0)).sqrt()
    }

    #[inline(always)]
    pub fn sy_x(&self) -> f64 {
        let length = self.standards.len() as f64;
        let sum_of_squares = self.sum_of_squares();
        (sum_of_squares / (length - 4.0)).sqrt()
    }

    #[inline(always)]
    pub fn calculate_unknowns(&mut self) {
        let (a, b, c, d) = self.abcd;
        for (x, y, _) in &mut self.unknowns {
            *x = c * ((a - d) / (*y - d) - 1.0).powf(1.0 / b)
        }
    }
    
    pub fn calculate_parameters(&mut self) {
        self.sse = self.sum_of_squares();
        self.mse = self.mean_squared_error();
        self.rmse = self.root_mean_squared_error();
        self.sy_x = self.sy_x();
    }
    
    pub fn four_pl_curve_fit(&mut self) {
        let Self { blank, unknowns, standards, control, .. } = self;

        // subtract blank
        unknowns.iter_mut().for_each(|(_, v, _)| *v -= *blank);
        standards.iter_mut().for_each(|(_, v)| *v -= *blank);
        *control -= *blank;

        let n = standards.len() as f64;

        let min = standards.iter().min_by(|&a, &b| a.0.partial_cmp(&b.0).unwrap()).unwrap();
        let max = standards.iter().max_by(|&a, &b| a.0.partial_cmp(&b.0).unwrap()).unwrap();

        // guess initial values
        let mut a = *control;  // 0-dose asymptote
        let mut d = max.1;    // inf-dose asymptote
        let mut c = (max.1 - min.1) / (max.0 - min.0).log10();  // IC50 interpolation (log-scale)
        let mut b = 2.0;      // slope at IC50

        dbg!(a, b, c, d);

        let learn_rate = (0.1, 1.5, 5_000_000.0, 0.5); // These values seem to work well, idk why c's learning rate is so high

        for i in 0..100_000 {
            let mut sum_a = 0.0;
            let mut sum_b = 0.0;
            let mut sum_c = 0.0;
            let mut sum_d = 0.0;

            for (x, y) in standards.iter() {
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
    }
}
