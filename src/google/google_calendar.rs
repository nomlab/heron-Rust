extern crate chrono;
extern crate google_calendar3;
extern crate reqwest;
extern crate serde;

use google_calendar3::Event;
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::google::google_auth;
use crate::google_auth::AccessTokenResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub items: Vec<Event>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventItem {
    pub summary: String,
    // pub originalStartTime: Option<OriginalStartTime>,
    pub start: EventItemPeriod,
    pub end: EventItemPeriod,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventItemPeriod {
    //unused when all day schedule
    pub date_time: Option<String>,
    //unused when not all day schedule
    pub date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OriginalStartTime {
    pub date_time: String,
}

#[test]
fn print_typename<T>(_: T) {
    println!("{}", std::any::type_name::<T>());
}

pub fn get_oneday_schedule(email: String) -> CalendarEvent {
    let token: AccessTokenResponse = google_auth::get_access_token();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("OAuth {}", token.access_token)).unwrap(),
    );

    let response = reqwest::blocking::ClientBuilder::new()
        .default_headers(headers.clone())
        .build()
        .unwrap()
        .get(&format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            email
        ))
        .query(&[
            ("timeZone", "jst"),
            ("maxResults", "2000"),
	    ("orderBy", "starttime"),
	    ("singleEvents", "true")
            // ("timeMin", &oneday.and_hms(0, 0, 0).to_rfc3339()),
            // ("timeMax", &oneday.and_hms(23, 59, 59).to_rfc3339()),
        ])
        .send()
        .unwrap()
        .text()
        .unwrap();

    serde_json::from_str(&response).unwrap()
}

// pub fn get_today_schedule(email: String) -> CalendarEvent {
//     get_oneday_schedule(email, )
// }
