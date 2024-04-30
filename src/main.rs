use crate::commands::points::send_trivia;
use clap::{Parser, Subcommand};
use commands::doctor::doctor_cmd_response;
use commands::episode::{episode_cmd_response, Episode};
use commands::points::{points_cmd_response, update_user_points, Question};
use commands::quote::quote_cmd_response;
use dotenv::dotenv;
use rusqlite::{params, Connection, Statement};
use serenity::all::ResolvedValue::{self, Integer};
use serenity::all::{ChannelType, CommandOptionType, Interaction, Message};
use serenity::async_trait;
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, EditMessage,
};
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::Client;
use std::time::Duration;
use std::{env, fs};
use tokio::time::sleep;
mod commands;

// Verify integrity function
fn verify_integrity() -> u8 {
    let mut verify_integrity: u8 = 0;
    match env::var("DISCORD_TOKEN") {
        Ok(_) => {
            println!("✅Token found in .env");
        }
        Err(_) => {
            println!("Token not found in .env");
            verify_integrity += 1;
        }
    }
    match env::var("RAPID_API") {
        Ok(_) => {
            println!("✅API key found in .env");
        }
        Err(_) => {
            println!("API key not found in .env");
            verify_integrity += 1;
        }
    }
    match env::var("CLIENT_ID") {
        Ok(_) => {
            println!("✅Client ID found in .env");
        }
        Err(_) => {
            println!("❌Client ID not found in .env");
            verify_integrity += 1;
        }
    }
    match fs::read_to_string("questions.json") {
        Ok(_) => {
            println!("✅questions.json found");
        }
        Err(_) => {
            println!("❌questions.json not found");
            verify_integrity += 1;
        }
    }
    match fs::read_to_string("episodes.json") {
        Ok(_) => {
            println!("✅episodes.json found");
        }
        Err(_) => {
            println!("❌episodes.json not found");
            verify_integrity += 1;
        }
    }
    match fs::read_to_string("quotes.json") {
        Ok(_) => {
            println!("✅quotes.json found");
        }
        Err(_) => {
            println!("❌quotes.json not found, Bot can't send quotes");
            verify_integrity += 1;
        }
    }
    verify_integrity
}

// Handler for gateway events
struct Handler;

// Parser for cmd line arguments
#[derive(Parser, Debug)]
#[command(version, about = "Discord Bot for Dr. Who Trivia")]
struct Args {
    #[clap(subcommand)]
    command: Option<Subcmd>,
}

#[derive(Subcommand, Debug)]
enum Subcmd {
    HelpBot,
    Verify,
}
// Prepares commands to be deployed to the Discord API
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

    // Points command
    let points = CreateCommand::new("points")
        .description("Shows the number of points user have on this guild");
    cmds.push(quote_cmd);
    cmds.push(doctor_cmd);
    cmds.push(episode_cmd);
    cmds.push(points);
    cmds
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let client_token = env::var("CLIENT_ID").unwrap();
        if let Some(mut msg_reply) = msg.referenced_message {
            if msg_reply.author.id.to_string() == client_token && msg_reply.content.contains("**Q")
            {
                let str = fs::read_to_string("questions.json").unwrap();
                let questions: Vec<Question> = match serde_json::from_str(&str) {
                    Ok(body) => body,
                    Err(e) => {
                        println!("Failed to parse questions.json: {}", e);
                        return;
                    }
                };
                for question in &questions {
                    if question.question == msg_reply.content
                        && question.answer == msg.content.to_ascii_lowercase()
                    {
                        let new_msg = msg_reply.content.replace("**", "__");
                        // Update user points
                        let guild_id = match msg.guild_id {
                            Some(body) => body.to_string(),
                            None => String::from(""),
                        };
                        match update_user_points(&msg.author.name, guild_id) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to update user points: {}", e);
                            }
                        }
                        msg_reply
                            .edit(&ctx.http, EditMessage::new().content(new_msg))
                            .await
                            .unwrap_or(());
                        // Sends trivia question
                        sleep(Duration::from_secs(3)).await;
                        if let Some(guild_channel_id) =
                            msg.channel_id.to_channel(&ctx).await.unwrap().guild()
                        {
                            match send_trivia(&guild_channel_id, &ctx).await {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Failed to send trivia: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    async fn ready(&self, ctx: Context, ready: Ready) {
        let cmds: Vec<CreateCommand> = prepare_commands();
        let res = ctx.http.create_global_commands(&cmds).await;
        match res {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to deploy commands: {}", e);
            }
        }
        let guilds = ctx.cache.guilds();
        for guild in guilds {
            let channels = match guild.channels(&ctx.http).await {
                Ok(body) => body,
                Err(e) => {
                    println!("Failed to get channels: {}", e);
                    continue;
                }
            };
            for (_id, channel) in channels {
                if channel.kind == ChannelType::Text && channel.name == "general" {
                    // Sends trivia question
                    match send_trivia(&channel, &ctx).await {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Failed to send trivia: {}", e);
                        }
                    }
                }
            }
        }

        println!("{} is connected!", ready.user.name);
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            match cmd.data.name.as_str() {
                "quote" => {
                    let response = quote_cmd_response();
                    cmd.create_response(&ctx, response).await.unwrap_or(());
                }
                "doctor" => {
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
                        .unwrap_or(serenity::all::Message::default());
                }
                "episode" => {
                    match cmd.defer(&ctx.http).await {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Failed to defer command: {}", e);
                        }
                    }
                    let name = cmd.data.options()[0].value.clone();
                    let mut episode_name = String::from("");
                    match name {
                        ResolvedValue::String(s) => {
                            episode_name = s.to_string();
                        }
                        _ => {
                            cmd.edit_response(
                                &ctx,
                                EditInteractionResponse::new()
                                    .content("Failed to parse episode command"),
                            )
                            .await
                            .unwrap_or(serenity::all::Message::default());
                        }
                    }
                    let ok = episode_cmd_response(episode_name).await;
                    let response = match ok {
                        Ok(body) => body,
                        Err(_) => EditInteractionResponse::new()
                            .content("Failed to get episode, please try again later"),
                    };
                    cmd.edit_response(&ctx, response)
                        .await
                        .unwrap_or(serenity::all::Message::default());
                }
                "points" => {
                    let response = match points_cmd_response(&cmd) {
                        Ok(body) => body,
                        Err(e) => {
                            println!("Error: {}", e);
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("Failed to get points, please try again later"),
                            )
                        }
                    };
                    cmd.create_response(&ctx, response).await.unwrap_or(());
                }
                _ => {}
            }
        }
    }
}

fn populate_database(conn: &Connection) {
    let read_dir = fs::read_to_string("episodes.json");
    match read_dir {
        Ok(body) => {
            let episodes: Vec<Episode> = serde_json::from_str(&body).unwrap_or(Vec::new());
            for episode in episodes {
                conn.execute(
                    "insert into episodes (id, title,season,episode) values (?1, ?2, ?3, ?4)",
                    params![episode.id, episode.title, episode.season, episode.episode],
                )
                .unwrap_or(0);
            }
        }
        Err(e) => {
            println!("Couldn't find episodes.json: {}", e);
        }
    }
    let mut stmt: Statement<'_>;
    match conn.prepare("select * from episodes") {
        Ok(body) => {
            stmt = body;
        }
        Err(e) => {
            println!("couldn't populate database: {}", e);
            return;
        }
    };
    let try_episodes_iter = stmt.query_map([], |row| {
        Ok(Episode {
            id: row.get(0).unwrap(),
            title: row.get(1).unwrap(),
            season: row.get(2).unwrap(),
            episode: row.get(3).unwrap(),
        })
    });
    match try_episodes_iter {
        Ok(_) => {}
        Err(_) => {
            return;
        }
    };
    let episodes_iter = try_episodes_iter.unwrap();
    for episode in episodes_iter {
        match episode {
            Ok(_) => {
                //println!("{} {}", _episode.id, _episode.title);
            }
            Err(e) => {
                println!("Couldn't find episodes.json: {}", e);
            }
        }
    }
}

fn prepare_points_database() {
    let conn = match Connection::open("points.db") {
        Ok(body) => body,
        Err(e) => {
            println!("Couldn't connect to points database {}", e);
            return;
        }
    };
    let create = r"
    create table if not exists points(
        id text not null,
        guild text not null,
        score integer not null
    );
    ";
    match conn.execute(create, ()) {
        Ok(_) => {}
        Err(e) => {
            println!("Couldn't create table: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Check command line arguments
    let args = Args::parse();
    if let Some(subcmd) = args.command {
        match subcmd {
            Subcmd::HelpBot => {
                println!("K9 is the Discord Bot I've made for the Rust course project. The commands it implements are as follows:
                - /quote: sends a random quote from the Doctor Who series
                - /doctor n: sends a picture of the n-th doctor
                - /episode name: searches for a specific episode
                - /points: shows the number of points user have on this guild
Users can also answer trivia questions by replying to the bot's messages with the correct answer. The bot will then update the user's points and send another trivia question.");
            }
            Subcmd::Verify => {
                let integrity = verify_integrity();
                if integrity == 0 {
                    println!("✅Everything is in order");
                } else {
                    println!("❌{} problems found. Please fix them before starting the app", integrity);
                    return;
                }
            }
        }
    }

    // Prepare Episodes Database
    let conn = match Connection::open("episodes.db") {
        Ok(body) => body,
        Err(e) => {
            println!("Couldn't connect to episodes database {}", e);
            return;
        }
    };
    match conn.execute("drop table if exists episodes", []) {
        Ok(_) => {}
        Err(e) => {
            println!("Couldn't drop table: {}", e);
            return;
        }
    }
    let create = r"
    create table if not exists episodes(
        id text not null,
        title text not null,
        season integer not null,
        episode integer not null
    );
    ";
    match conn.execute(create, ()) {
        Ok(_) => {}
        Err(e) => {
            println!("Couldn't create table: {}", e);
        }
    }
    populate_database(&conn);

    //Prepare Points Database
    prepare_points_database();

    // Token to connecting to Discord API
    let token: String =
        env::var("DISCORD_TOKEN").expect("discord bot token should be in .env, can't start bot");

    // Gateway intents
    let intents: GatewayIntents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client: Client = match Client::builder(&token, intents)
        .event_handler(Handler)
        .await
    {
        Ok(body) => body,
        Err(e) => {
            println!("Failed to create client: {}", e);
            return;
        }
    };

    //Start the client
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
