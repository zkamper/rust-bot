use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Quote {
    quote: String,
    author: String,
}

pub fn quote_cmd_response() -> CreateInteractionResponse {
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
            let ok = serde_json::from_str(&body);
            quotes = match ok {
                Ok(body) => body,
                Err(_) => {
                    println!("Failed to parse quotes.json");
                    let rsp_embed = CreateEmbed::new()
                        .title(quote.author)
                        .description(quote.quote);
                    let rsp = CreateInteractionResponseMessage::new().add_embed(rsp_embed);
                    return CreateInteractionResponse::Message(rsp);
                }
            };
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
