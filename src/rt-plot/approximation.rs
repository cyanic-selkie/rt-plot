use ndarray::*;
use ndarray_linalg::*;
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum Type {
    Constant,
    Linear,
    Quadratic,
}

fn polyfit(
    x_values: &Vec<f32>,
    y_values: &Vec<f32>,
    polynomial_degree: usize,
) -> (Vec<f32>, Vec<f32>) {
    let number_of_columns = polynomial_degree + 1;
    let number_of_rows = x_values.len();
    let mut x = Array2::zeros((number_of_rows, number_of_columns));

    for (row, &x_value) in x_values.iter().enumerate() {
        x[(row, 0)] = 1.0;

        for col in 1..number_of_columns {
            x[(row, col)] = x_value.powf(col as f32);
        }
    }

    let y = Array::from(y_values.to_owned());

    let svd = x.svd(true, true).unwrap();

    let u = svd.0.unwrap();
    let singular_values = &svd.1;
    let vt = svd.2.unwrap();

    let sigma_inv = Array2::from_diag(singular_values).inv_into().unwrap();

    let mut sigma_pinv = Array2::zeros((number_of_columns, number_of_rows));
    for (i, &value) in singular_values.iter().enumerate() {
        sigma_pinv[(i, i)] = 1.0 / value;
    }

    let coefficients = vt.t().dot(&sigma_pinv.dot(&u.t().dot(&y)));
    let variances = vt.t().dot(&sigma_inv.dot(&sigma_inv.dot(&vt)));

    (
        coefficients.to_vec(),
        variances.diag().iter().map(|x| x.clone()).collect(),
    )
}

fn mse(ground: &Vec<f32>, predicted: &Vec<f32>, dof: usize) -> f32 {
    ground
        .iter()
        .zip(predicted)
        .fold(0.0, |sum, (g, p)| sum + (p - g).powf(2.0))
        / (ground.len() - dof) as f32
}

pub fn fit(
    data: &BTreeMap<OrderedFloat<f32>, Vec<f32>>,
    range: &std::ops::Range<OrderedFloat<f32>>,
    approximation_type: &Type,
    channel: usize,
) -> (Vec<f32>, Vec<f32>) {
    let mut x = vec![];
    let mut y = vec![];

    for (time, data) in data.range(range.clone()) {
        x.push(time);
        y.push(OrderedFloat(data[channel]));
    }

    // Calculate everything necessary for data normalization.
    // That means we center the data and then normalize it between -1 and 1 on both axis.
    let mean_x: f32 = x.iter().map(|x| x.into_inner()).sum::<f32>() / x.len() as f32;
    let mean_y: f32 = y.iter().map(|y| y.into_inner()).sum::<f32>() / y.len() as f32;

    let max_x = x.iter().max().unwrap().into_inner();
    let min_x = x.iter().min().unwrap().into_inner();

    let max_y = y.iter().max().unwrap().into_inner();
    let min_y = y.iter().min().unwrap().into_inner();

    let mm_x = max_x - min_x;
    let mm_y = max_y - min_y;

    let x: Vec<f32> = x.iter().map(|x| (x.into_inner() - mean_x) / mm_x).collect();
    let y: Vec<f32> = y.iter().map(|y| (y.into_inner() - mean_y) / mm_y).collect();

    let degree = match approximation_type {
        Type::Constant => 0,
        Type::Linear => 1,
        Type::Quadratic => 2,
    };

    let (coefficients, variances) = polyfit(&x, &y, degree);

    let predicted: Vec<f32> = x
        .iter()
        .map(|x| {
            coefficients
                .iter()
                .enumerate()
                .fold(0.0, |sum, (i, coefficient)| {
                    sum + x.powf(i as f32) * coefficient
                })
        })
        .collect();

    // Don't forget to account for scaling of the data.
    let mse = mse(&y, &predicted, y.len() - degree - 1) * mm_y.powf(2.0);

    // Once we get the coefficients, calculate the coefficients for the original, non normalized
    // data.
    let (coefficients, variances) = match approximation_type {
        Type::Constant => {
            let a = coefficients[0];

            let sigma_a = variances[0];

            let a_prime = a * mm_y + mean_y;

            (vec![a_prime], vec![mm_y.powf(2.0) * sigma_a])
        }
        Type::Linear => {
            let a = coefficients[1];
            let b = coefficients[0];

            let sigma_a = variances[1];
            let sigma_b = variances[0];

            let mm_yx = mm_y / mm_x;

            let a_prime = a * mm_yx;
            let b_prime = -a * mm_yx * mean_x + mm_y * b + mean_y;

            (
                vec![b_prime, a_prime],
                vec![mm_y.powf(2.0) * sigma_b, mm_yx.powf(2.0) * sigma_a],
            )
        }
        Type::Quadratic => {
            let a = coefficients[2];
            let b = coefficients[1];
            let c = coefficients[0];

            let sigma_a = variances[2];
            let sigma_b = variances[1];
            let sigma_c = variances[0];

            let mm_yx = mm_y / mm_x;

            let a_prime = a * mm_yx / mm_x;
            let b_prime = b * mm_yx - 2.0 * a * mean_x * mm_yx / mm_x;
            let c_prime =
                a * mm_yx / mm_x * mean_x * mean_x - b * mm_yx * mean_x + c * mm_y + mean_y;

            (
                vec![c_prime, b_prime, a_prime],
                vec![
                    mm_y.powf(2.0) * sigma_c,
                    mm_yx.powf(2.0) * sigma_b,
                    (mm_yx / mm_x).powf(2.0) * sigma_a,
                ],
            )
        }
    };

    let standard_errors = variances.iter().map(|&v| v.sqrt() * mse).collect();

    (coefficients, standard_errors)
}

pub fn transform_coefficients(
    coefficients: &Vec<f32>,
    errors: &Vec<f32>,
    approximation_type: &Type,
) -> (Vec<f32>, Vec<f32>) {
    match approximation_type {
        Type::Constant => (coefficients.clone(), errors.clone()),
        Type::Linear => {
            let b = coefficients[0];
            let a = coefficients[1];

            let sigma_b = errors[0];
            let sigma_a = errors[1];

            (vec![-b / a, a], vec![(sigma_b / a).abs(), sigma_a.abs()])
        }
        Type::Quadratic => {
            let c = coefficients[0];
            let b = coefficients[1];
            let a = coefficients[2];

            let sigma_c = errors[0];
            let sigma_b = errors[1];
            let sigma_a = errors[2];

            (
                vec![c - (b * b) / (4.0 * a), b / (2.0 * a), 2.0 * a],
                vec![
                    ((b / a).powf(4.0) / 16.0 * sigma_a.powf(2.0)
                        + (b / a).powf(2.0) / 4.0 * sigma_b.powf(2.0)
                        + sigma_c.powf(2.0))
                    .sqrt(),
                    ((b / a).powf(2.0) / (4.0 * a.powf(2.0)) * sigma_a.powf(2.0)
                        + sigma_b.powf(2.0) / (4.0 * a.powf(2.0)))
                    .sqrt(),
                    2.0 * sigma_a,
                ],
            )
        }
    }
}

pub fn measurement_to_string(value: f32, error: f32) -> String {
    let lambda = error.log10().round() as i64;
    let nd = {
        if lambda < 0 {
            (-lambda + 1) as usize
        } else {
            1
        }
    };

    format!("{:.nd$} ± {:.nd$}", value, error, nd = nd)
}
