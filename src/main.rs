use dotenv::dotenv;
use rand::Rng;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::{env, fs};
use ureq::json;

use serde::{Deserialize, Serialize};
use serenity::all::ResolvedValue::{self, Integer};
use serenity::all::{CommandOptionType, GuildId, Interaction};
use serenity::async_trait;
use serenity::builder::{
    CreateAttachment, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse,
};
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::Client;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Quote {
    quote: String,
    author: String,
}

fn prepare_commands() -> Vec<CreateCommand> {
    let mut cmds: Vec<CreateCommand> = Vec::new();
    // Quote command
    let quote_cmd = CreateCommand::new("quote").description("Sends a Dr. Who quote");

    // Doctor command
    let doctor_cmd_option =
        CreateCommandOption::new(CommandOptionType::Integer, "n", "the number of the doctor")
            .required(true)
            .min_int_value(1)
            .max_int_value(14);
    let doctor_cmd = CreateCommand::new("doctor")
        .description("Sends a pictore of the n-th doctor")
        .add_option(doctor_cmd_option);

    // Episode command
    let episode_cmd_option =
        CreateCommandOption::new(CommandOptionType::String, "name", "title of episode")
            .required(true);
    let episode_cmd = CreateCommand::new("episode")
        .description("Search for a specific Doctor Who episode")
        .add_option(episode_cmd_option);
    cmds.push(quote_cmd);
    cmds.push(doctor_cmd);
    cmds.push(episode_cmd);
    cmds
}

fn quote_cmd_response() -> CreateInteractionResponse {
    let dir = fs::read_to_string("quotes.json");
    let mut quote: Quote;
    // Default quote in case the file quotes.json does not exist
    quote = Quote {
        quote: String::from("quote_not_found"),
        author: String::from("author_not_found"),
    };
    let quotes: Vec<Quote>;
    match dir {
        Ok(body) => {
            quotes = serde_json::from_str(&body).expect("Failed to parse json");
            let mut rng = rand::thread_rng();
            let n1 = rng.gen_range(0..quotes.len());
            quote = quotes[n1].clone();
        }
        Err(_) => {
            println!("Quotes file not found");
        }
    }
    // Creare embed quote
    let rsp_embed = CreateEmbed::new()
        .title(quote.author)
        .description(quote.quote);
    let rsp = CreateInteractionResponseMessage::new().add_embed(rsp_embed);
    CreateInteractionResponse::Message(rsp)
}

async fn doctor_cmd_response(option: u8, ctx: &Context) -> EditInteractionResponse {
    // API key
    let api_key = env::var("RAPID_API").expect("Expected a token in the environment");

    // Preparing search string
    let mut to_search = String::from("doctor who ");
    let numb_as_str: [String; 14] = [
        String::from("first"),
        String::from("second"),
        String::from("third"),
        String::from("fourth"),
        String::from("fifth"),
        String::from("sixth"),
        String::from("seventh"),
        String::from("eighth"),
        String::from("ninth"),
        String::from("tenth"),
        String::from("eleventh"),
        String::from("twelfth"),
        String::from("thirteenth"),
        String::from("fourteenth"),
    ];
    to_search.push_str(&numb_as_str[(option - 1) as usize]);
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
        Ok(body) => Some(body.into_string().unwrap()),
        Err(_) => None,
    };

    let url: &str;
    let resp_json: Value;
    let mut msg_content = format!(
        "Here's a picture of the {} doctor",
        numb_as_str[(option - 1) as usize]
    );

    if let Some(body) = api_response {
        resp_json = serde_json::from_str(&body).expect("Failed to parse json");
        let ok: Option<&str> = resp_json["result"][0]["image"].as_str();
        url = match ok {
            Some(body) => body,
            None => {
                // In caz ca API-ul merge, dar nu gasim nimic
                msg_content = format!(
                    "No image found for the {} doctor",
                    numb_as_str[(option - 1) as usize]
                );
                "https://upload.wikimedia.org/wikipedia/commons/thumb/d/d1/Image_not_available.png/640px-Image_not_available.png"
            }
        };
    } else {
        // In caz ca nu merge API-ul
        msg_content = format!(
            "No image found for the {} doctor",
            numb_as_str[(option - 1) as usize]
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
            println!("Failed to get image");
        }
    }
    rsp
}

async fn episode_cmd_response(name: String) -> EditInteractionResponse {
    // API key
    let api_key = env::var("RAPID_API").expect("Expected a token in the environment");

    let conn = Connection::open("episodes.db").expect("cannot connect to db");
    let pattern = name.replace("\n", "").replace("\r", "");
    let search_cmd = String::from("select distinct * from episodes where title like '%")
        + &pattern
        + &String::from("%';");
    let mut stmt = conn.prepare(&search_cmd).expect("could not run statement");
    let episodes_iter = stmt
        .query_map([], |row| {
            Ok(Episode {
                id: row.get(0).unwrap(),
                title: row.get(1).unwrap(),
                season: row.get(2).unwrap(),
                episode: row.get(3).unwrap(),
            })
        })
        .expect("cannot query");
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
                    Ok(body) => Some(body.into_string().unwrap()),
                    Err(_) => None,
                };
                let title = format!("__{}__: ", episode.title);
                msg_content.push_str(&title);
                match api_response {
                    Some(body) => {
                        let resp_json: Value =
                            serde_json::from_str(&body).expect("Failed to parse json");
                        let year: i64 = match resp_json["results"]["releaseDate"]["year"].as_i64() {
                            Some(body) => body,
                            None => 0,
                        };
                        let month: i64 = match resp_json["results"]["releaseDate"]["month"].as_i64()
                        {
                            Some(body) => body,
                            None => 0,
                        };
                        let day: i64 = match resp_json["results"]["releaseDate"]["day"].as_i64() {
                            Some(body) => body,
                            None => 0,
                        };
                        let details = format!(
                            " {}-{}-{}    **({}x{})**\n",
                            year, month, day, episode.season, episode.episode
                        );
                        msg_content.push_str(&details);
                    }
                    None => {
                        msg_content.push_str(" [no data found]");
                        msg_content.push_str("\n");
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    // Preparing http POST request
    // let http_req = ureq::get("https://google-api31.p.rapidapi.com/imagesearch")
    //     .set("X-RapidAPI-Key", &api_key)
    //     .set("X-RapidAPI-Host", "moviesdatabase.p.rapidapi.com");

    // let mut mst_content = String::new();
    EditInteractionResponse::new().content(msg_content)
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // async fn message(&self, _ctx: Context, msg: Message) {
    //     println!("{}: {}", msg.author.name, msg.content);
    // }
    async fn ready(&self, ctx: Context, ready: Ready) {
        let cmds: Vec<CreateCommand> = prepare_commands();
        let mut test_guild: GuildId;
        // Caut serverul de test din cele unde se afla botul
        let guild_token =
            std::env::var("GUILD_ID").expect("Expected a guild id in the environment");
        for guild in ready.guilds {
            // Comenzile specifice pt guild sunt incarcate instant spre deosebire de cele globale
            // Folosesc propriul server de test
            if guild.id.to_string() == guild_token {
                test_guild = guild.id;
                test_guild
                    .set_commands(&ctx.http, cmds.clone())
                    .await
                    .expect("Failed to deploy commands");
            }
        }
        println!("{} is connected!", ready.user.name);
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            // TO-DO: change to match
            if cmd.data.name == "quote" {
                let response = quote_cmd_response();
                cmd.create_response(&ctx, response)
                    .await
                    .expect("Failed to respond to slash command");
            } else if cmd.data.name == "doctor" {
                let num = cmd.data.options()[0].value.clone();
                let mut dr_nr = 0u8;
                match num {
                    Integer(n) => {
                        dr_nr = n as u8;
                    }
                    _ => {
                        println!("Failed to parse doctor command");
                    }
                }
                cmd.defer(&ctx.http).await.expect("Failed to defer");
                let response = doctor_cmd_response(dr_nr, &ctx).await;
                cmd.edit_response(&ctx, response)
                    .await
                    .expect("Failed to respond to slash command");
            } else if cmd.data.name == "episode" {
                let name = cmd.data.options()[0].value.clone();
                let mut episode_name = String::from("");
                match name {
                    ResolvedValue::String(s) => {
                        episode_name = s.to_string();
                    }
                    _ => {
                        println!("Failed to parse episode command");
                    }
                }
                cmd.defer(&ctx.http).await.expect("Failed to defer");
                let response = episode_cmd_response(episode_name).await;
                cmd.edit_response(&ctx, response)
                    .await
                    .expect("Failed to respond to slash command");
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Episode {
    id: String,
    title: String,
    season: i32,
    episode: i32,
}

fn populate_database(conn: &Connection) {
    let read_dir = fs::read_to_string("episodes.json");
    match read_dir {
        Ok(body) => {
            let episodes: Vec<Episode> = serde_json::from_str(&body).expect("Failed to parse json");
            for episode in episodes {
                conn.execute(
                    "insert into episodes (id, title,season,episode) values (?1, ?2, ?3, ?4)",
                    params![episode.id, episode.title, episode.season, episode.episode],
                )
                .expect("cannot insert data");
            }
        }
        Err(e) => {
            println!("Couldn't find episodes.json: {}", e);
        }
    }
    let mut stmt = conn
        .prepare("select * from episodes")
        .expect("cannot prepare statement");
    let episodes_iter = stmt
        .query_map([], |row| {
            Ok(Episode {
                id: row.get(0).unwrap(),
                title: row.get(1).unwrap(),
                season: row.get(2).unwrap(),
                episode: row.get(3).unwrap(),
            })
        })
        .expect("cannot query");
    for episode in episodes_iter {
        match episode {
            Ok(_episode) => {
                //println!("{} {}", _episode.id, _episode.title);
            }
            Err(e) => {
                println!("Couldn't find episodes.json: {}", e);
            }
        }
    }
}
#[tokio::main]
async fn main() {
    dotenv().ok();

    // Prepare Database
    let conn = Connection::open("episodes.db").expect("cannot connect to db");
    conn.execute("drop table if exists episodes", [])
        .expect("cannot reset table");
    let create = r"
    create table if not exists episodes(
        id text not null,
        title text not null,
        season integer not null,
        episode integer not null
    );
    ";
    conn.execute(create, ()).expect("cannot create table");
    populate_database(&conn);

    // Token to connecting to Discord API
    let token: String = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Gateway intents
    let intents: GatewayIntents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client: Client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Deploy chat commands to the guild

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
