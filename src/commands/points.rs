use anyhow::Result;
use rand::Rng;
use rusqlite::{Connection, Statement};
use serde::{Deserialize, Serialize};
use serenity::all::{CommandInteraction, GuildChannel};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::prelude::*;
use std::fs;

struct Points {
    id: String,
    score: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Question {
    pub question: String,
    pub answer: String,
}

pub fn points_cmd_response(cmd: &CommandInteraction) -> Result<CreateInteractionResponse> {
    let conn = Connection::open("points.db")?;
    let guild_id = match cmd.guild_id {
        Some(body) => body.to_string(),
        None => {
            return Ok(CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Command was not run in guild"),
            ))
        }
    };
    let search_cmd = format!(
        "select id, score from points where guild = {} order by score desc",
        guild_id
    );
    let mut stmt: Statement<'_> = conn.prepare(&search_cmd)?;
    let points_iter = stmt.query_map([], |row| {
        Ok(Points {
            id: row.get(0).unwrap(),
            score: row.get(1).unwrap(),
        })
    })?;
    let mut msg_content = String::from("## Leaderboard");
    for points in points_iter {
        match points {
            Ok(points) => {
                let line = format!("\n{}: {}", points.id, points.score);
                msg_content.push_str(&line);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    let rsp_msg = CreateInteractionResponseMessage::new().content(msg_content);
    Ok(CreateInteractionResponse::Message(rsp_msg))
}

fn get_rand_number(from: usize, to: usize) -> usize {
    let mut rng = rand::thread_rng();
    rng.gen_range(from..to)
}

// const BLACKLISTED_GUILD: u64 = 1003764837182087188;

pub async fn send_trivia(channel: &GuildChannel, ctx: &Context) -> Result<()> {
    let str = fs::read_to_string("questions.json")?;
    let questions: Vec<Question> = serde_json::from_str(&str)?;
    let question = &questions[get_rand_number(0, questions.len())];
    // if channel.guild_id == BLACKLISTED_GUILD {
    //     return Ok(());
    // }
    channel.say(&ctx.http, &question.question).await?;
    Ok(())
}

pub fn update_user_points(user: &String, guild: String) -> Result<()> {
    let conn = Connection::open("points.db")?;
    let search_cmd = format!(
        "select score from points where id = '{}' and guild = {}",
        user, guild
    );
    let point: Result<i32, rusqlite::Error> = conn.query_row(&search_cmd, [], |row| row.get(0));
    match point {
        Ok(body) => {
            let update_cmd = format!(
                "update points set score = {} where id = '{}' and guild = {}",
                body + 1,
                user,
                guild
            );
            conn.execute(&update_cmd, [])?;
            Ok(())
        }
        Err(_) => {
            let insert_cmd = format!(
                "insert into points (id, guild, score) values (\'{}\', {}, 1)",
                user, guild
            );
            conn.execute(&insert_cmd, [])?;
            Ok(())
        }
    }
}
