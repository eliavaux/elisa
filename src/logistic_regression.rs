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
    BlankTooBig,
    ControlTooBig,
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
    pub r_sq: f64
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

        let blank = if blank.1 != 0 { blank.0 / blank.1 as f64 } else { 0.0 };
        let control = if control.1 != 0 { control.0 / control.1 as f64 } else { 0.0 };

        let unknowns = unknowns.iter().enumerate().filter_map(|(i, &(sum, count))| {
            if count == 0 { return None }
            let measurement = sum / count as f64;
            let label = microplate.unknown_groups[i].label.clone();
            Some((0.0, measurement, label))
        }).collect();

        let mut concentrations = vec![0.0; standards_len];
        for (i, group) in concentrations.iter_mut().enumerate() {
            let Some(concentration) = microplate.standard_groups[i].concentration else {
                return Err(UnassignedConcentration)
            };
            if !concentration.is_finite() { return Err(InvalidConcentration) }
            *group = concentration;
        }

        let mut standards: Vec<_> = standards.iter().enumerate().filter_map(|(i, &(sum, count))| {
            if count == 0 { return None }
            let concentration = concentrations[i];
            let measurement = sum / count as f64;
            Some((concentration, measurement))
        }).collect();

        // We need at least 4 standards, preferably 8
        if standards.len() < 4 { return Err(NotEnoughStandards) }

        // Sort standards by concentration
        standards.sort_by(|(a_x, _a_y), (b_x, _b_y)| a_x.total_cmp(b_x));

        // Find minimum measurement, this is not necessarily standards.first()
        let standard_min = standards.iter().min_by(|(_a_x, a_y), (_b_x, b_y)| a_y.total_cmp(b_y)).unwrap().1;

        if control > standard_min { return Err(ControlTooBig) }
        if blank > standard_min { return Err(BlankTooBig) }

        
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
        self.mean_squared_error().sqrt()
    }

    #[inline(always)]
    pub fn sy_x(&self) -> f64 {
        let length = self.standards.len() as f64;
        let sum_of_squares = self.sum_of_squares();
        (sum_of_squares / (length - 4.0)).sqrt()
    }

    #[inline(always)]
    pub fn r_squared(&self) -> f64 {
        let n = self.standards.len() as f64;
        let mean = self.standards.iter().map(|&(_x, y)| y).sum::<f64>() / n;

        let total_sum_of_squares: f64 = self.standards.iter().map(|&(_x, y)| {
            let y_hat = y - mean;
            y_hat * y_hat
        }).sum();


        let r = 1.0 - self.sum_of_squares() / total_sum_of_squares;
        r * r
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
        self.r_sq = self.r_squared();
    }
    
    pub fn four_pl_curve_fit(&mut self) {
        let Self { blank, unknowns, standards, control, .. } = self;
        let n = standards.len() as f64;

        // subtract blank
        unknowns.iter_mut().for_each(|(_, v, _)| *v -= *blank);
        standards.iter_mut().for_each(|(_, v)| *v -= *blank);
        *control -= *blank;

        // convert standards x to x hat
        let standards: Vec<_> = standards.iter().map(|&(x, y)| (x.ln(), y)).collect();

        // find the minimum and maximum measurement, this is not necessarily standards.first()
        let min = standards.iter().min_by(|(_a_x, a_y), (_b_x, b_y)| a_y.total_cmp(b_y)).unwrap();
        let max = standards.iter().max_by(|(_a_x, a_y), (_b_x, b_y)| a_y.total_cmp(b_y)).unwrap();


        // guess initial values
        let mut a = *control; // 0-dose asymptote
        let mut b = 1.0;      // slope at IC50
        let mut d = max.1;    // inf-dose asymptote

        // We assume the point of inflection, c, is close to the interpolation between two standards with the greatest slope
        let mut c_incline = 0.0;
        let mut c = 0.0;
        for window in standards.windows(2) {
            let a = window[0];
            let b = window[1];

            let incline = (b.1 - a.1) / (b.0 - a.0);

            if c_incline < incline {
                c_incline = incline;
                c = (a.0 + b.0) / 2.0;
            }
        }

        dbg!(a, b, c, d, blank, *control, min);


        let learn_rate = (0.1, 1.0, 1.0, 0.1);

        // I should really fix this
        for i in 0..100_000 {
            let mut sum_a = 0.0;
            let mut sum_b = 0.0;
            let mut sum_c = 0.0;
            let mut sum_d = 0.0;

            for (x, y) in standards.iter() {
                let ebxc = (b * (x - c)).exp();
                let sigmoid = 1.0 / (1.0 + ebxc);

                let diff = y - d - (a - d) * sigmoid;
                let duda = sigmoid;
                let dudb = (x - c) * ebxc * sigmoid * sigmoid;
                let dudc = ebxc * sigmoid * sigmoid;
                let dudd = sigmoid;

                sum_a += diff * duda;
                sum_b += diff * dudb;
                sum_c += diff * dudc;
                sum_d += diff * dudd;
            }

            let da = -2.0 / n * sum_a;
            let db = 2.0 * (a - d) / n * sum_b;
            let dc = -2.0 * b * (a - d) / n * sum_c;
            let dd = -2.0 / n * sum_d;
            
            a -= learn_rate.0 * da;
            b -= learn_rate.1 * db;
            c -= learn_rate.2 * dc;
            d -= learn_rate.3 * dd;

            // We can make the reasonable assumption that the asymptotic lower bound must be between the control and the first standard
            a = a.clamp(*control, min.1);

            if i % 1000 == 0 { println!("a: {}, b: {}, c: {}, d: {}", a, b, c, d) };
        }


        let c = c.exp();

        self.abcd = (a, b, c, d);
    }
}
