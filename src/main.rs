#[macro_use]
extern crate serde_derive;

extern crate serde_json;
extern crate tg_botapi;
extern crate rusqlite;
extern crate crypto;

use crypto::md5::Md5;
use crypto::digest::Digest;

use rusqlite::Connection;

use std::path::Path;

use tg_botapi::args;
use tg_botapi::types;
use tg_botapi::types::InputMessageContent;
use tg_botapi::types::InlineQueryResult;

use tg_botapi::BotApi;

use std::collections::HashMap;
use std::sync::Arc;
// use std::thread;
// use std::env;

use serde_json::Value;
use serde_json::Number;

#[derive(Debug, Serialize, Deserialize)]
struct Paste {
    text: String,
    uses: i64,
}

fn main() {
    let token = ""; // I'm lazy go away
    let bot = Arc::new(BotApi::new_debug(token));

    let me_irl = bot.get_me().expect("Could not establish a connection :\\");
    let about = "Hey, I'm Paste Bot!\n\n\

                I'm an inline bot that to make it easier for you to shitpost by \
                letting you add custom endings to your message. Or if you want \
                to send a copypasta but hate searching it up eerytime, I can do \
                that too. Each paste is configurable. Your pastes are sorted inline \
                by how often you use them (although this is also configurable).\n\n\

                Use /newpaste to get started.\n\n\

                Made by @JuanPotato, \
                <a href=\"https://github.com/JuanPotato/PasteBot\">Source</a>";

    let db_path = Path::new("./database.db");
    let exists = db_path.exists();

    let conn = Connection::open(db_path).unwrap();

    if !exists {
        conn.execute("
            CREATE TABLE users (
                id              INTEGER PRIMARY KEY,
                pastes          STRING DEFAULT '[]',
                state           INTEGER NOT NULL DEFAULT 0
            )", &[]).unwrap();
        // States
        // 0 Nothing going on
        // 1 User initiated /newpaste waiting for paste contents
    }

    let mut update_args = args::GetUpdates::new().timeout(600).offset(0);

    'update_loop: loop {
        let updates = bot.get_updates(&update_args).unwrap();

        for update in updates {
            update_args.offset = Some(update.update_id + 1);

            if let Some(message) = update.message {
                if let Some(from) = message.from { // This is getting nasty

                    let message_text = message.text.unwrap_or(String::new());
                    let mut split_text = message_text.split_whitespace();

                    if let Some(cmd) = split_text.next() {
                        match cmd {
                            "/start" | "/help" => {
                                conn.execute("
                                    INSERT OR IGNORE INTO users (id)
                                    VALUES (?1)",
                                    &[&from.id]).unwrap();

                                let _ = bot.send_message(&args::SendMessage::new(about)
                                    .chat_id(message.chat.id).parse_mode("HTML"));
                            }
                            "/listpastes" => {
                                let res_pastes: Result<String, _> = conn.query_row(
                                    "SELECT pastes FROM users WHERE id=?1",
                                    &[&from.id],
                                    |row| {
                                        row.get(0)
                                    }
                                );

                                match res_pastes {
                                    Ok(str_pastes) => {
                                        let pastes: Vec<Paste> = serde_json::from_str(&str_pastes).unwrap();

                                    let _ = bot.send_message(&args::SendMessage
                                        ::new(&serde_json::to_string(&pastes).unwrap())
                                        .chat_id(message.chat.id));
                                    }

                                    Err(e) => {
                                        // Oh shit waddup
                                    }
                                }
                            }
                            "/newpaste" => {
                                conn.execute("
                                    UPDATE users
                                    SET state = 1
                                    WHERE id=?1",
                                    &[&from.id]).unwrap();

                                let _ = bot.send_message(&args::SendMessage
                                    ::new("Ok, send me the text you want that paste to be.")
                                    .chat_id(message.chat.id));
                            }
                            _ => {
                                let cur_state: Result<i64, _> = conn.query_row(
                                    "SELECT state FROM users WHERE id=?1",
                                    &[&from.id], |row| {
                                        row.get(0)
                                    }
                                );

                                match cur_state {
                                    Ok(num) => {
                                        match num {
                                            1 => {
                                                conn.execute("
                                                    UPDATE users
                                                    SET state = 0
                                                    WHERE id=?1",
                                                    &[&from.id]).unwrap();

                                                let res_pastes: Result<String, _> = conn.query_row(
                                                    "SELECT pastes FROM users WHERE id=?1",
                                                    &[&from.id],
                                                    |row| {
                                                        row.get(0)
                                                    }
                                                );

                                                match res_pastes {
                                                    Ok(str_pastes) => {
                                                        let mut pastes: Vec<Paste> = serde_json::from_str(&str_pastes).unwrap();
                                                        pastes.push(Paste { text: message_text.to_string(), uses: 0 });
                                                        conn.execute("
                                                            UPDATE users
                                                            SET pastes = ?2
                                                            WHERE id=?1",
                                                            &[&from.id, &serde_json::to_string(&pastes).unwrap()]).unwrap();

                                                        let _ = bot.send_message(&args::SendMessage
                                                            ::new("Added.")
                                                            .chat_id(message.chat.id));
                                                    }

                                                    Err(e) => {
                                                        // Oh shit waddup
                                                    }
                                                }
                                            },

                                            _ => { }
                                        }
                                    }

                                    Err(e) => {
                                        // Oh shit waddup
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(inline_query) = update.inline_query {
                let res_pastes: Result<String, _> = conn.query_row(
                    "SELECT pastes FROM users WHERE id=?1",
                    &[&inline_query.from.id], |row| row.get(0));

                match res_pastes {
                    Ok(str_pastes) => {
                        let mut pastes: Vec<Paste> = serde_json::from_str(&str_pastes).unwrap();
                        pastes.sort_by_key(|p| p.uses);
                        let mut results = Vec::new();

                        let mut sh = Md5::new();
                        for paste in pastes {
                            let content = InputMessageContent::new_text(&paste.text);
                            sh.input_str(&paste.text);

                            let hash = sh.result_str();
                            let res = InlineQueryResult::new_article(&hash, &paste.text, &content);

                            results.push(res);
                            sh.reset();
                        }
                        let _ = bot.answer_inline_query(&args::AnswerInlineQuery::new(&inline_query.id, &results).cache_time(0));
                    }

                    Err(e) => {
                        // Oh shit waddup
                    }
                }
            }
        }
    }
    update_args.limit = Some(0);
    update_args.timeout = Some(0);
    let _ = bot.get_updates(&update_args);
}
