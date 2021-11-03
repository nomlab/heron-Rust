use chrono::prelude::*;
use chrono::{Date, Duration, Utc, Weekday};
use jpholiday::jpholiday::JPHoliday;
use nalgebra::{DMatrix, DVector, RowDVector};
use polars::prelude::*;
use smartcore::linear::linear_regression::*;

fn weekdays(date: &Date<Utc>) -> String {
    let jpholiday = JPHoliday::new();
    if jpholiday.is_holiday(&date.naive_utc()) {
        return "祝日".to_string();
    }
    match date.weekday() {
        Weekday::Mon => return "月曜日".to_string(),
        Weekday::Tue => return "火曜日".to_string(),
        Weekday::Wed => return "水曜日".to_string(),
        Weekday::Thu => return "木曜日".to_string(),
        Weekday::Fri => return "金曜日".to_string(),
        Weekday::Sat => return "土曜日".to_string(),
        Weekday::Sun => return "日曜日".to_string(),
    }
}

fn weekdays_considering_nholiday(dates: &Vec<Date<Utc>>) -> Vec<String> {
    dates.iter().map(|date| weekdays(&date)).collect()
}

fn monthweek(date: &Date<Utc>) -> String {
    let month = date.month().to_string() + "月";
    let week = ((date.day() - 1) / 7 + 1).to_string() + "w";
    month + &week
}

fn monthweeks(dates: &Vec<Date<Utc>>) -> Vec<String> {
    dates.iter().map(|date| monthweek(&date)).collect()
}

fn month(date: &Date<Utc>) -> String {
    date.month().to_string() + "月"
}

fn months(dates: &Vec<Date<Utc>>) -> Vec<String> {
    dates.iter().map(|date| month(&date)).collect()
}

///////////////////////////////////////////
// Generate parameters list
//-----------------------------------------
// dates  : date of vector
// return : list of parameters
///////////////////////////////////////////
// Example
//-----------------------------------------
// dates  : vec!['2013/4/2', '2013/4/3', ...]
// return :
//           date  wday   week month monthday
//    ('2013/4/2',  Wed, 4月1w,  4月,       2
//     '2013/4/3',  Thu, 4月1w,  4月,       3
//      ...
///////////////////////////////////////////
fn get_params_list(dates: &Vec<Date<Utc>>) -> DataFrame {
    let wdays = weekdays_considering_nholiday(dates);
    let weeks = monthweeks(dates);
    let months = months(dates);
    // let holidays = holidays(&dates);
    //
    let plist = df!("wday" => &wdays,
		    "weeks" => &weeks,
		    "months" => &months)
    .unwrap();

    plist
}

fn dates_to_occurreds(dates: &Vec<Date<Utc>>, range: &Vec<Date<Utc>>) -> Vec<f64> {
    let len = (range[1] - range[0]).num_days() + 1;
    let mut occurreds = vec![0.0; len as usize];

    let seq_dates: Vec<Date<Utc>> = (0..len).map(|x| range[0] + Duration::days(x)).collect();

    for date in dates.iter() {
        for (j, seq_date) in seq_dates.iter().enumerate() {
            if date == seq_date {
                occurreds[j] = 1.0;
            }
        }
    }
    occurreds
}

fn mean(v: &Vec<f64>) -> f64 {
    let mut sum: f64 = 0.0;
    for i in v {
        sum += *i;
    }
    sum / v.len() as f64
}

fn get_ac(f: &Vec<f64>, range: &Vec<Date<Utc>>) -> Vec<f64> {
    // let start = 0;
    let end = (range[1] - range[0]).num_days() + 1;

    let mut ac: Vec<f64> = vec![0.0; (end + 1) as usize];

    for lag in 0..end {
        let mut p = Vec::new();
        // f[lag] * f[0], f[lag + 1] * f[1], ..., f[f.len()] * f[f.len()]
        // f[lag] * f[0], f[lag + 1] * f[1], ..., f[f.len()] * f[f.len() - lag]
        for i in 0..(f.len() as i64 - lag) {
            p.push(f[(lag + i) as usize] * f[i as usize]);
        }
        if lag != 0 {
            ac[(lag - 1) as usize] = mean(&p);
        }
    }

    ac
}

fn get_big_wave_cycle(dates: &Vec<Date<Utc>>, range: &Vec<Date<Utc>>) -> usize {
    let series = dates_to_occurreds(dates, range);
    let mut ac = get_ac(&series, range);
    let mut max = 0.0;
    let mut max_index = 0;

    // 要修正
    // 長過ぎる周期をカット
    if ac.len() > 400 {
        ac = ac[..400].to_vec();
    }
    for (i, &val) in ac.iter().enumerate() {
        if max < val {
            max = val;
            max_index = i + 1;
        }
    }
    max_index
}

fn closest_event_index(events: &Vec<Date<Utc>>, date: Date<Utc>) -> usize {
    let last = events.len() - 1;
    let mut index: usize = 0;
    for i in 0..=last {
        if events[i] <= date && date <= events[i + 1] {
            if (date - events[i]).num_days() < (events[i + 1] - date).num_days() {
                index = i;
            } else {
                index = i + 1;
            };
        }
    }

    if events[last] < date {
        index = last
    }

    index
}

fn get_candidates(events: &Vec<Date<Utc>>, range: &Vec<i64>, period: usize) -> Vec<Date<Utc>> {
    let latest = events.last().unwrap();
    let criterion = *latest - Duration::days(period as i64);
    let i = closest_event_index(events, criterion);
    let mut d = (events[i + 1] - events[i]).num_days();
    if d > 365 {
        d = 365;
    }
    let pivot = *latest + Duration::days(d);
    let candidates: Vec<Date<Utc>> = range.iter().map(|x| pivot + Duration::days(*x)).collect();

    candidates
}

fn gen_lm(cdv: &Series) -> (Vec<String>, Vec<f64>) {
    let nrow = cdv.len();
    let mut col_uniq: Vec<String> = vec![];
    col_uniq.push(cdv.get(0).to_string());
    for i in 0..cdv.len() {
        let mut flag = true;
        for col in col_uniq.iter() {
            if &cdv.get(i).to_string() == col {
                flag = false;
                break;
            }
        }
        if flag {
            col_uniq.push(cdv.get(i).to_string());
        }
    }
    let ncol = col_uniq.len();

    // let mut m: Array2<f64> = Array::zeros((nrow, ncol));

    let mut vec: Vec<f64> = vec![0.0; nrow * ncol];
    for row in 0..nrow {
        for col in 0..ncol {
            if cdv.get(row).to_string() == col_uniq[col] {
                // m[[row, col]] = 1.0;
                vec[col * nrow + row] = 1.0;
            }
        }
    }
    return (col_uniq, vec);
}

fn get_lm_all(first: Date<Utc>, last: Date<Utc>) -> (DMatrix<f64>, Vec<String>) {
    let len = (last - first).num_days();
    let dates: Vec<Date<Utc>> = (0..=len).map(|x| first + Duration::days(x)).collect();
    let plist_alldate = get_params_list(&dates);
    let cols = plist_alldate.get_columns();

    let (mut col_uniq, mut lms) = gen_lm(&cols[0]);
    if cols.len() > 1 {
        for col in &cols[1..cols.len()] {
            let (mut name, mut lm) = gen_lm(&col);
            col_uniq.append(&mut name);
            lms.append(&mut lm);
        }
    }
    let ncol = len;
    let nrow = lms.len() as i64 / ncol;

    let lms = DMatrix::from_row_slice(nrow as usize, (ncol + 1) as usize, &lms);
    return (lms, col_uniq);
}

// fn annual_lm(dates: &Vec<Date<Utc>>, first: Date<Utc>, last: Date<Utc>) {
//     let lms = vec![0];
//     monthdays_uni =
// }

fn get_ts(recurrence: &Vec<Date<Utc>>, first: Date<Utc>, last: Date<Utc>) -> Vec<f64> {
    let len = (last - first).num_days();
    let dates: Vec<Date<Utc>> = (0..=len).map(|x| first + Duration::days(x)).collect();
    let mut ts = vec![0.0; dates.len()];
    for (i, val) in dates.iter().enumerate() {
        for r in recurrence {
            if val == r {
                ts[i] = 1.0;
            }
        }
    }
    ts
}

fn lm_fit(ts: Vec<f64>, data: &DMatrix<f64>) -> (Vec<f64>, f64) {
    // 重回帰
    // let x = data.columns(1, data.ncols() - 1).into_owned();
    let y = RowDVector::from_row_slice(&ts);

    let lr = LinearRegression::fit(
        data,
        &y,
        LinearRegressionParameters {
            solver: LinearRegressionSolverName::QR,
        },
    )
    .unwrap();

    let coefs = lr.coefficients().as_slice().to_vec();
    let inter = lr.intercept();
    return (coefs, inter);
}

fn get_w(ts: Vec<f64>, lm: &mut DMatrix<f64>) -> (Vec<f64>, f64) {
    let dx = lm.transpose();

    // 重回帰
    let (coefs, inter) = lm_fit(ts, &dx);
    return (coefs, inter);
}

fn get_f(
    candidates_plist: &DataFrame,
    coefs: Vec<f64>,
    inter: f64,
    colname: Vec<String>,
) -> DVector<f64> {
    let mut m = DMatrix::<f64>::zeros(candidates_plist.height(), colname.len());

    let cols = candidates_plist.get_columns();

    for col in cols.iter() {
        for i in 0..m.nrows() {
            for (j, val) in colname.iter().enumerate() {
                if col.get(i).to_string() == *val {
                    m[(i, j)] = 1.0;
                }
            }
        }
    }

    let coefs = DVector::from_vec(coefs);
    let f = (m * coefs).add_scalar(inter);

    f
}

fn max_index(array: DVector<f64>) -> usize {
    let mut index: usize = 0;
    for i in 0..array.len() {
        if array[index] < array[i] {
            index = i;
        }
    }
    index
}

pub fn forecast(
    range_recurrence: &Vec<Date<Utc>>,
    range_candidate: &Vec<i64>,
    events: &Vec<Date<Utc>>,
) -> Date<Utc> {
    // recurrence: 予定発生履歴
    let first = range_recurrence[0];
    let last = range_recurrence[1];
    let recurrence = events;
    // 次の予定の候補日
    let mut period = get_big_wave_cycle(&recurrence, &range_recurrence);
    if period == 0 {
        period = 365;
    }

    let candidates = get_candidates(&recurrence, &range_candidate, period);
    let candidates_plist = get_params_list(&candidates);

    let (mut lm, cols) = get_lm_all(first, last);
    // let annu_lm = annual_lm(&recurrence, first, last);
    // println!("lm: {:?}", lm);

    let ts = get_ts(&recurrence, first, last);
    let (coefs, inter) = get_w(ts, &mut lm);

    let f = get_f(&candidates_plist, coefs, inter, cols);

    let index = max_index(f);

    let forecasted = candidates[index];

    forecasted
}
