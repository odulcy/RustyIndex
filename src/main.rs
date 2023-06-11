use error_chain::error_chain;
use serde_json::Value;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

fn main() -> Result<()> {
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

    let json: Value = serde_json::from_str(json_text).unwrap();

    let entities = &json["PriceUpdate"]["entities"];

    for entity in entities.as_array().unwrap() {
        let financial_entity = &entity["financial_entity"]["common_entity_data"];
        if financial_entity["name"] == "CAC 40" {
            let last_value = &financial_entity["last_value_dbl"];
            let value_change = &financial_entity["value_change"].as_str().unwrap();
            let percent_change = &financial_entity["percent_change"].as_str().unwrap();

            println!("{last_value:} ({value_change}) {percent_change}");

            break;
        }
    }

    Ok(())
}
