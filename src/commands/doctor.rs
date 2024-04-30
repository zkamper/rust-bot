use serde_json::Value;
use serenity::builder::{CreateAttachment, EditInteractionResponse};
use serenity::prelude::*;
use std::env;
use ureq::json;

const NUMB_AS_STR: [&str; 14] = [
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eigth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
];

pub async fn doctor_cmd_response(option: u8, ctx: &Context) -> EditInteractionResponse {
    // API key
    let api_key: String = match env::var("RAPID_API") {
        Ok(body) => body,
        Err(_) => return EditInteractionResponse::new().content("Failed to get API key"),
    };

    // Preparing search string
    let mut to_search = String::from("doctor who ");

    to_search.push_str(NUMB_AS_STR[(option - 1) as usize]);
    to_search.push_str(" doctor");

    //println!("{}", to_search);

    // Payload for the http POST request
    let payload = json!({
        "text": &to_search,
        "safesearch": "off",
        "region": "wt-wt",
        "color": "",
        "size": "",
        "type_image": "",
        "layout": "",
        "max_results": 1
    });

    // Preparing http POST request
    let http_req = ureq::post("https://google-api31.p.rapidapi.com/imagesearch")
        .set("content-type", "application/json")
        .set("X-RapidAPI-Key", &api_key)
        .set("X-RapidAPI-Host", "google-api31.p.rapidapi.com")
        .send_json(payload);

    let api_response = match http_req {
        Ok(body) => Some(body.into_string().unwrap_or(String::from(""))),
        Err(_) => None,
    };

    let url: &str;
    let resp_json: Value;
    let mut msg_content = format!(
        "Here's a picture of the {} doctor",
        NUMB_AS_STR[(option - 1) as usize]
    );

    if let Some(body) = api_response {
        let ok_parse: Result<Value, serde_json::Error> = serde_json::from_str(&body);
        resp_json = match ok_parse {
            Ok(body) => body,
            Err(_) => {
                msg_content = format!(
                    "No image found for the {} doctor",
                    NUMB_AS_STR[(option - 1) as usize]
                );
                return EditInteractionResponse::new().content(msg_content);
            }
        };
        let ok: Option<&str> = resp_json["result"][0]["image"].as_str();
        url = match ok {
            Some(body) => body,
            None => {
                // In caz ca API-ul merge, dar nu gasim nimic
                msg_content = format!(
                    "No image found for the {} doctor",
                    NUMB_AS_STR[(option - 1) as usize]
                );
                "https://upload.wikimedia.org/wikipedia/commons/thumb/d/d1/Image_not_available.png/640px-Image_not_available.png"
            }
        };
    } else {
        // In caz ca nu merge API-ul
        msg_content = format!(
            "No image found for the {} doctor",
            NUMB_AS_STR[(option - 1) as usize]
        );
        url = "https://upload.wikimedia.org/wikipedia/commons/thumb/d/d1/Image_not_available.png/640px-Image_not_available.png"
    }

    //println!("{}", url);
    let img_req = CreateAttachment::url(&ctx.http, url).await;

    let mut rsp = EditInteractionResponse::new().content(msg_content);
    match img_req {
        Ok(body) => {
            rsp = rsp.new_attachment(body);
        }
        Err(_) => {
            msg_content = format!(
                "No image found for the {} doctor",
                NUMB_AS_STR[(option - 1) as usize]
            );
            rsp = rsp.content(msg_content);
        }
    }
    rsp
}
