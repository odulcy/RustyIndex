use chrono::{DateTime, Datelike, Local, Timelike, Weekday};
use error_chain::error_chain;
use serde_json::Value;
use std::env;
use std::fs;
use std::fs::metadata;
use std::fs::File;
use std::io::Write;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

fn write_to_file(content: &str, file_path: &str) -> std::io::Result<()> {
    // Open the file in write mode, creating it if it doesn't exist
    let mut file = File::create(file_path)?;

    // Write the content to the file
    file.write_all(content.as_bytes())?;

    Ok(())
}

/// Returns filepath to cached.
fn get_path_to_cached_file() -> String {
    let home = env::var("HOME").unwrap();

    format!("{home}/.config/polybar/.indexes.txt")
}

/// Fetch index from Google API (at the moment, only CAC40 is available).
///
/// # Returns
///
/// JSON parsed as Serde Value.
fn _fetch_index() -> Result<Value> {
    let url = reqwest::Url::parse_with_params(
        "https://www.google.com/async/finance_wholepage_price_updates",
        &[
            ("ei", "si0PZIezLcmakdUPzLWewAg"),
            ("yv", "3"),
            ("cs", "0"),
            (
                "async",
                "mids:/m/016j14|/m/02q4tvl|/g/11f3jy5sx4|/g/1q52g5hz_,currencies:",
            ),
        ],
    )
    .unwrap();
    let raw_data = reqwest::blocking::get(url)?.text()?;

    // There is a weird artifact, remove it
    let json_text = &raw_data[5..];

    let file_path = get_path_to_cached_file();
    if let Err(err) = write_to_file(json_text, &file_path) {
        eprintln!("Failed to write to file: {}", err);
    }

    let json: Value = serde_json::from_str(json_text).unwrap();

    Ok(json)
}

/// Returns true if it's a working hour, i.e. it's a work day
/// between 8 a.m. and 9 p.m.
fn is_working_hours() -> bool {
    let now = Local::now();
    let current_day = now.weekday();
    let current_time = now.time();
    let working_hours = current_day != Weekday::Sat
        && current_day != Weekday::Sun
        && current_time.hour() >= 8
        && current_time.hour() < 19;

    working_hours
}

/// Fetch index from Google API
///
/// If time is outside market hours, read local cache
///
/// # Returns
///
/// Json parsed as Serde Value.
fn fetch_index() -> Result<Value> {
    let file_path = get_path_to_cached_file();

    let file_metadata = metadata(&file_path).ok();

    let should_fetch;

    if file_metadata.is_some() {
        let modified_time: DateTime<Local> = file_metadata.unwrap().modified().unwrap().into();
        let cache_up_to_date = modified_time.hour() >= 8 && modified_time.hour() < 19;

        // Execute code for workday between 8 a.m. and 7 p.m.
        // Also execute it if cache is not up-to-date
        should_fetch = is_working_hours() || !cache_up_to_date
    } else {
        should_fetch = true;
    }

    if should_fetch {
        _fetch_index()
    } else {
        // Read the text file for non-working hours
        let file_content = fs::read_to_string(file_path).unwrap();

        Ok(serde_json::from_str(&file_content).unwrap())
    }
}

fn main() -> Result<()> {
    let json = fetch_index().unwrap();

    let entities = &json["PriceUpdate"]["entities"];

    for entity in entities.as_array().unwrap() {
        let financial_entity = &entity["financial_entity"]["common_entity_data"];
        if financial_entity["name"] == "CAC 40" {
            let last_value = &financial_entity["last_value_dbl"];
            let value_change = &financial_entity["value_change"].as_str().unwrap();
            let percent_change = &financial_entity["percent_change"].as_str().unwrap();

            let mut msg = String::new();
            if !is_working_hours() {
                // Market is closed !
                msg.push_str("🔒 ")
            }
            msg.push_str(&format!("{last_value} ({value_change}) {percent_change}"));
            println!("{}", msg);

            break;
        }
    }

    Ok(())
}
