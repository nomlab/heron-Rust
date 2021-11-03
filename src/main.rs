#[macro_use]
extern crate polars;

#[macro_use]
extern crate clap;

mod forecast;
mod google;

use self::forecast::forecaster;
use self::google::google_auth;

use chrono::prelude::*;
use chrono::{NaiveDate, Utc};
use clap::{App, Arg};
use std::env;
use std::io::{self, BufRead};

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

fn main() {
    let app = App::new(crate_name!())
        .version(crate_version!()) // バージョン情報
        .author(crate_authors!()) // 作者情報
        .about(crate_description!()) // このアプリについて
        .arg(
            Arg::with_name("command") // 位置引数を定義
                .help("sample positional argument") // ヘルプメッセージ
                .required(true), // この引数は必須であることを定義
        )
        .arg(
            Arg::with_name("method") // オプションを定義
                .help("Set the forecasting algorithm.") // ヘルプメッセージ
                .short("m") // ショートコマンド
                .long("method") // ロングコマンド
                .takes_value(true), // 値を持つことを定義
        )
        .arg(
            Arg::with_name("input") // オプションを定義
                .help("Get training occurrence list from the FILE.") // ヘルプメッセージ
                .short("i") // ショートコマンド
                .long("input") // ロングコマンド
                .takes_value(true), // 値を持つことを定義
        )
        .arg(
            Arg::with_name("forecast-year") // オプションを定義
                .help("fib") // ヘルプメッセージ
                .short("f") // ショートコマンド
                .long("forecast-year") // ロングコマンド
                .takes_value(true), // 値を持つことを定義
        )
        .arg(
            Arg::with_name("calendar_id") // オプションを定義
                .help("fib") // ヘルプメッセージ
                .short("c") // ショートコマンド
                .long("calendar_id") // ロングコマンド
                .takes_value(true), // 値を持つことを定義
        )
        .arg(
            Arg::with_name("sampling-range") // オプションを定義
                .help("Date range in the form of YYYY/MM/DD-YYYY/MM/DD.") // ヘルプメッセージ
                .short("s") // ショートコマンド
                .long("sampling-range") // ロングコマンド
                .takes_value(true), // 値を持つことを定義
        );
    let matches = app.get_matches();

    if let Some(c) = matches.value_of("command") {
        match c {
            "forecast" => {
                let mut events: Vec<Date<Utc>> = vec![];
                let mut range_recurrence: Vec<Date<Utc>> = vec![];
                let mut range_candidates: Vec<i64> = vec![];

                //////////////////////////////////////////////////////////
                // Option: --input
                //////////////////////////////////////////////////////////
                if let Some(_) = matches.value_of("input") {
                    if let Some(calendar_id) = matches.value_of("calendar_id") {
                        let events_list =
                            google::google_calendar::get_oneday_schedule(calendar_id.to_string());
                        events = events_list
                            .items
                            .iter()
                            .map(|i| match &i.start {
                                Some(dt) => match &dt.date_time {
                                    Some(d) => d.parse::<DateTime<Utc>>().unwrap().date(),
                                    None => Date::from_utc(
                                        NaiveDate::parse_from_str(
                                            &dt.date.as_ref().unwrap(),
                                            "%Y-%m-%d",
                                        )
                                        .unwrap(),
                                        Utc,
                                    ),
                                },
                                None => Utc.ymd(2000, 1, 1),
                            })
                            .collect();
                    } else {
                        println!("Input calendar_id");
                    }
                } else {
                    let stdin = io::stdin();
                    let mut lines = stdin.lock().lines();

                    while let Some(line) = lines.next() {
                        // let length: i32 = line.unwrap().trim().parse().unwrap();
                        if line.as_ref().unwrap() == &"EOF".to_string() {
                            break;
                        }
                        events.push(Date::from_utc(
                            NaiveDate::parse_from_str(&line.unwrap(), "%Y-%m-%d").unwrap(),
                            Utc,
                        ));
                    }
                }

                ///////////////////////////////////////////////////
                // Option: --sampling-range
                ///////////////////////////////////////////////////
                if let Some(o) = matches.value_of("sampling-range") {
                    range_recurrence = o
                        .split("-")
                        .map(|d| {
                            Date::from_utc(NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap(), Utc)
                        })
                        .collect();
                } else {
                    let first = fiscal_year_first_date(events[0]);
                    let last = events.last().unwrap().clone();
                    range_recurrence = vec![first, last];
                }

                ///////////////////////////////////////////////////
                // Option: --method
                ///////////////////////////////////////////////////

                // fib

                ///////////////////////////////////////////////////
                // Option: --candidate-range
                ///////////////////////////////////////////////////
                if let Some(o) = matches.value_of("candidate-range") {
                    let num: i64 = o.parse::<i64>().expect("Please num");
                    if (num > 0) && (num % 2 == 1) {
                        let n = (num - 1) / 2;
                        range_candidates = (-n..=n).collect();
                    }
                } else {
                    range_candidates = (-3..4).collect();
                }

                ////////////////////////////////////////////////////
                // Option: --forecast_year
                ////////////////////////////////////////////////////
                if let Some(o) = matches.value_of("forecast-year") {
                    let forecast_year: i32 = o.parse::<i32>().expect("Please num");
                    let forecast_start = Utc.ymd(forecast_year, 4, 1);
                    let forecast_end = Utc.ymd(forecast_year + 1, 4, 1);
                    loop {
                        let forecasted =
                            forecaster::forecast(&range_recurrence, &range_candidates, &events);
                        events.push(forecasted);
                        if forecasted < forecast_start {
                            continue;
                        }
                        if forecasted > forecast_end {
                            break;
                        }
                        println!("forecast: {:?}", forecasted);
                        range_recurrence[1] = forecasted;
                    }
                } else {
                    let forecasted =
                        forecaster::forecast(&range_recurrence, &range_candidates, &events);
                    println!("forecast: {:?}", forecasted);
                }
            }
            "show" => println!("fib"),
            _ => println!("No matching command"),
        }
    }
}
