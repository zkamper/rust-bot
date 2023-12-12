use dotenv::dotenv;
use rand::Rng;
use serenity::all::Embed;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateEmbedAuthor;
use std::fs;
use serenity::all::GuildId;
use serenity::all::Interaction;
use serenity::builder::CreateCommand;
use serenity::builder::CreateInteractionResponse;
use serenity::builder::CreateInteractionResponseMessage;
use std::env;
use serde::{Deserialize, Serialize};
use serenity::Client;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::async_trait;

struct Handler;

#[derive(Serialize, Deserialize,Debug,Clone)]
struct Quote{
    quote: String,
    author: String,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        println!("{}: {}", msg.author.name, msg.content);
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let mut cmds: Vec<CreateCommand>= Vec::new();
        let quote_cmd = CreateCommand::new("quote")
            .description("Sends a Dr. Who quote");
        cmds.push(quote_cmd);

        let mut test_guild: GuildId;
        // Caut serverul de test din cele unde se afla botul
        let guild_token = std::env::var("GUILD_ID").expect("Expected a guild id in the environment");
        for guild in ready.guilds {
            // Comenzile specifice pt guild sunt incarcate instant spre deosebire de cele globale
            // Folosesc propriul server de test
            if guild.id.to_string() == guild_token {
                test_guild = guild.id;
                test_guild.set_commands(&ctx.http,cmds.clone()).await.expect("Failed to deploy commands");
            }
        }
        println!("{} is connected!", ready.user.name);
    }
    async fn interaction_create(&self, ctx: Context, interaction :Interaction){
        if let Interaction::Command(cmd) = interaction {
            // TO-DO: change to match
            if cmd.data.name == "quote" {
                let dir = fs::read_to_string("quotes.json");
                let mut quote: Quote;
                quote = Quote{
                    quote: String::from("quote_not_found"),
                    author: String::from("author_not_found")
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
                let rsp_embed = CreateEmbed::new()
                    .title(quote.author)
                    .description(quote.quote);
                let rsp = CreateInteractionResponseMessage::new()
                    .add_embed(rsp_embed);
                let response = CreateInteractionResponse::Message(rsp);
                cmd.create_response(&ctx,response).await.expect("Failed to respond to slash command");
            }
        }
    }
}


#[tokio::main]
async fn main(){
    dotenv().ok();
    
    // Token to connecting to Discord API
    let token: String = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    
    // Gateway intents
    let intents: GatewayIntents = GatewayIntents::GUILDS | 
                    GatewayIntents::GUILD_MESSAGES | 
                    GatewayIntents::DIRECT_MESSAGES |
                    GatewayIntents::GUILD_MEMBERS |
                    GatewayIntents::MESSAGE_CONTENT;



    let mut client: Client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Deploy chat commands to the guild

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}