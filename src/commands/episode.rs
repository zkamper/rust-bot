use anyhow::Result;
use rusqlite::{Connection, Statement};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::builder::EditInteractionResponse;
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct Episode {
    pub id: String,
    pub title: String,
    pub season: i32,
    pub episode: i32,
}

pub async fn episode_cmd_response(name: String) -> Result<EditInteractionResponse> {
    // API key
    let api_key: String = env::var("RAPID_API")?;
    let conn: Connection = Connection::open("episodes.db")?;
    let pattern = name.replace(['\n', '\r', '\t'], "");
    let search_cmd = String::from("select distinct * from episodes where title like '%")
        + &pattern
        + &String::from("%';");
    let mut stmt: Statement<'_> = conn.prepare(&search_cmd)?;
    let episodes_iter = stmt.query_map([], |row| {
        Ok(Episode {
            id: row.get(0).unwrap(),
            title: row.get(1).unwrap(),
            season: row.get(2).unwrap(),
            episode: row.get(3).unwrap(),
        })
    })?;
    let mut msg_content: String =
        String::from("## Here are the episodes that match your search:\n");
    for episode in episodes_iter {
        match episode {
            Ok(episode) => {
                let url = format!(
                    "https://moviesdatabase.p.rapidapi.com/titles/{}",
                    episode.id
                );
                let http_req = ureq::get(&url)
                    .set("X-RapidAPI-Key", &api_key)
                    .set("X-RapidAPI-Host", "moviesdatabase.p.rapidapi.com");
                let api_response = match http_req.call() {
                    Ok(body) => Some(body.into_string().unwrap_or(String::from(""))),
                    Err(_) => None,
                };
                let title = format!("__{}__: ", episode.title);
                msg_content.push_str(&title);
                match api_response {
                    Some(body) => {
                        let resp_json: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                        let year: i64 = resp_json["results"]["releaseDate"]["year"]
                            .as_i64()
                            .unwrap_or(0);
                        let month: i64 = resp_json["results"]["releaseDate"]["month"]
                            .as_i64()
                            .unwrap_or(0);
                        let day: i64 = resp_json["results"]["releaseDate"]["day"]
                            .as_i64()
                            .unwrap_or(0);
                        let details = format!(
                            " {}-{}-{}    **({}x{})**\n",
                            year, month, day, episode.season, episode.episode
                        );
                        msg_content.push_str(&details);
                    }
                    None => {
                        msg_content.push_str(" [no data found]");
                        msg_content.push('\n');
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    Ok(EditInteractionResponse::new().content(msg_content))
}
