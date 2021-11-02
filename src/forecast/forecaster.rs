use chrono::prelude::*;
use chrono::{Date, Duration, Utc, Weekday};
use jpholiday::jpholiday::JPHoliday;
use ndarray::prelude::*;
use ndarray_glm::{utility::standardize, Linear, ModelBuilder};
use polars::prelude::*;

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
    ((date.day() - 1) / 7 + 1).to_string() + "w"
}

fn monthweeks(dates: &Vec<Date<Utc>>) -> Vec<String> {
    dates.iter().map(|date| monthweek(&date)).collect()
}

fn months(dates: &Vec<Date<Utc>>) -> Vec<u32> {
    dates.iter().map(|date| date.month()).collect()
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
//           date  wday week month monthday
//    ('2013/4/2',  Wed,   1,    4,       2
//     '2013/4/3',  Thu,   1,    4,       3
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

fn gen_lm(cdv: &Series) -> Vec<Series> {
    let nrow = cdv.len();
    let col_uniq = if cdv.name() == "wday" {
        Series::new("wdays", &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])
    } else {
        cdv.unique().unwrap().sort(false)
    };
    let ncol = col_uniq.len();

    // let mut m: Array2<f32> = Array::zeros((nrow, ncol));

    let mut vec = vec![vec![0.0; nrow]; ncol];
    // for row in 0..nrow {
    for row in 0..nrow {
        for col in 0..ncol {
            if cdv.get(row) == col_uniq.get(col) {
                // m[[row, col]] = 1.0;
                vec[col][row] += 1.0;
                break;
            }
        }
    }

    let mut param = Vec::new();

    for (i, v) in vec.iter().enumerate() {
        param.push(Series::new(&col_uniq.get(i).to_string(), v));
    }
    // let mut m = DataFrame::new(param).unwrap();
    param
}

fn get_lm_all(first: Date<Utc>, last: Date<Utc>) -> DataFrame {
    let len = (last - first).num_days();
    let dates: Vec<Date<Utc>> = (0..=len).map(|x| first + Duration::days(x)).collect();
    let plist_alldate = get_params_list(&dates);
    let cols = plist_alldate.get_columns();

    let mut param = gen_lm(&cols[0]);
    if cols.len() > 1 {
        for col in &cols[1..cols.len()] {
            param.append(&mut gen_lm(&col));
        }
    }
    let lm = DataFrame::new(param).unwrap();
    lm
}

// fn annual_lm(dates: &Vec<Date<Utc>>, first: Date<Utc>, last: Date<Utc>) {
//     let lms = vec![0];
//     monthdays_uni =
// }

fn get_ts(recurrence: &Vec<Date<Utc>>, first: Date<Utc>, last: Date<Utc>) -> Array1<f32> {
    let len = (last - first).num_days();
    let dates: Vec<Date<Utc>> = (0..=len).map(|x| first + Duration::days(x)).collect();
    let mut ts = Array::zeros(dates.len());
    for i in 0..dates.len() {
        for r in recurrence {
            if &dates[i] == r {
                ts[i] = 1.0
            }
        }
    }
    ts
}

fn get_w(ts: Array1<f32>, df: &DataFrame) -> Array1<f32> {
    let lm = df.to_ndarray::<Float32Type>().unwrap();
    let lm = standardize(lm);
    let model = ModelBuilder::<Linear>::data(&ts.view(), &lm.view())
        .build()
        .unwrap();
    let fit = model.fit_options().l2_reg(1e-5).fit().unwrap();
    fit.result
}

fn get_f(candidates_plist: &DataFrame, lm: DataFrame, w: Array1<f32>) -> Array1<f32> {
    let colname_lm = lm.get_column_names();
    let mut m: Array2<f32> = Array::zeros((candidates_plist.height(), colname_lm.len()));

    let cols = candidates_plist.get_columns();

    for col in cols.iter() {
        for i in 0..m.shape()[0] {
            for j in 0..colname_lm.len() {
                if col.get(i).to_string() == colname_lm[j] {
                    m[[i, j]] = 1.0;
                }
            }
        }
    }

    let f = m.dot(&w.slice(s![1..])) + w.slice(s![0]);

    f
}

fn max_index(array: Array1<f32>) -> usize {
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
    let recurrence_plist = get_params_list(&recurrence);

    // 次の予定の候補日
    let mut period = get_big_wave_cycle(&recurrence, &range_recurrence);
    if period == 0 {
        period = 365;
    }

    let candidates = get_candidates(&recurrence, &range_candidate, period);
    let candidates_plist = get_params_list(&candidates);

    let lm = get_lm_all(first, last);
    // let annu_lm = annual_lm(&recurrence, first, last);
    // println!("lm: {:?}", lm);

    let ts = get_ts(&recurrence, first, last);
    let w = get_w(ts, &lm);

    let f = get_f(&candidates_plist, lm, w);

    let index = max_index(f);

    let forecasted = candidates[index];

    forecasted
}
