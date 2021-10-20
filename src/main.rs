#[macro_use]
extern crate polars;

mod forecast;
mod google;

use self::forecast::forecaster;
use self::google::google_auth;

use chrono::prelude::*;
use chrono::{NaiveDate, Utc};
use plotters::prelude::*;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[test]
fn example() -> Result<()> {
    let weekday = Utc.ymd(2020, 4, 1).weekday();
    print!("{}", weekday);
    Ok(())
}

#[test]
fn print_typename<T>(_: T) {
    println!("{}", std::any::type_name::<T>());
}

fn fiscal_year_first_date(date: Date<Utc>) -> Date<Utc> {
    let mut y = date.year();
    let m = date.month();
    if m == 1 || m == 2 || m == 3 {
        y = y - 1;
    }
    return Utc.ymd(y, 4, 1);
}

// fn chart_context(score: Vec<(f32, f32)>) {
//     // 描画先をBackendとして指定。ここでは画像に出力するためBitMapBackend
//     let root = BitMapBackend::new("chart.png", (640, 480)).into_drawing_area();
//     root.fill(&WHITE).unwrap();
//
//     // グラフの軸の設定など
//     let mut chart = ChartBuilder::on(&root)
//         .caption("time", ("sans-serif", 50).into_font())
//         .margin(5)
//         .x_label_area_size(30)
//         .y_label_area_size(30)
//         .build_ranged(-0f32..50f32, 0.0f32..5.0f32)
//         .unwrap();
//
//     chart.configure_mesh().draw().unwrap();
//
//     // データの描画。(x, y)のイテレータとしてデータ点を渡す
//     chart.draw_series(LineSeries::new(score, &RED)).unwrap();
// }

fn main() {
    let args: Vec<String> = env::args().collect();

    let command = &args[1];
    let calendar = &args[2];
    let recurrence = &args[3];
    match command.as_str() {
        "forecast" => forecast(calendar, recurrence),
        "show" => show(calendar, recurrence),
        _ => println!("No matching command"),
    }
}

fn forecast(calendar: &String, recurrence: &String) {
    let events_list =
        google::google_calendar::get_oneday_schedule(calendar.to_string(), recurrence.to_string());
    let mut events: Vec<Date<Utc>> = events_list
        .items
        .iter()
        .map(|i| match &i.start {
            Some(dt) => match &dt.date_time {
                Some(d) => d.parse::<DateTime<Utc>>().unwrap().date(),
                None => Date::from_utc(
                    NaiveDate::parse_from_str(&dt.date.as_ref().unwrap(), "%Y-%m-%d").unwrap(),
                    Utc,
                ),
            },
            None => Utc.ymd(2000, 1, 1),
        })
        .collect();
    let first = fiscal_year_first_date(events[0]);
    let last = events.last().unwrap().clone();
    // let mut score: Vec<(f32, f32)> = Vec::new();
    // let mut count: f32 = 1.0;
    loop {
        //
        let range_candidates: Vec<i64> = (-3..4).collect();
        let range_recurrence = vec![first, last];
        let start = Instant::now();
        let forecasted = forecaster::forecast(range_recurrence, &range_candidates, &events);
        let end = start.elapsed();
        events.push(forecasted); //
        if forecasted < Utc.ymd(2021, 4, 1) {
            continue;
        }
        if forecasted >= Utc.ymd(2022, 4, 1) {
            //
            break; //
        } //
          // count += 1.0;
          // score.push((
          //     count,
          //     (end.as_secs() as u32 + end.subsec_nanos() / 1_000_000) as f32,
          // ));
          // println!(
          //     "{}.{:0.3}秒経過",
          //     end.as_secs(),
          //     end.subsec_nanos() / 1_000_000
          // );
        println!("forecast: {:?}", forecasted);
        let mut file = File::create("foo.txt").unwrap();
        file.write_all(forecasted.to_string().as_bytes()).unwrap();
    } //
      // chart_context(score);
}
fn show(calendar: &String, recurrence: &String) {
    let event_list =
        google::google_calendar::get_oneday_schedule(calendar.to_string(), recurrence.to_string());
    println!("items: {}", event_list.items.len());

    for item in event_list.items {
        println!(
            "summary:{}, start:{}\nExtended property:{}",
            item.summary.unwrap(),
            match item.start {
                Some(dt) => match dt.date_time {
                    Some(d) => d,
                    None => dt.date.unwrap(),
                },
                None => "No time".to_string(),
            },
            item.extended_properties
                .unwrap()
                .shared
                .unwrap()
                .get(&"recurrence_name".to_string())
                .unwrap()
        );
    }
}
