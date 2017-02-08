#[macro_use]
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

use std::sync::Arc;
// use std::thread;
// use std::env;

#[derive(Debug)]
struct Paste {
    text: String,
    hash: String,
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
                state           INTEGER NOT NULL DEFAULT 0,
                amount          INTEGER NOT NULL DEFAULT 0
                new             BOOLEAN DEFAULT FALSE
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
                            "/listpastes" => { // Will change to an inline /managepastes later, this is for debugging right now
                                let res_pastes: Result<String, _> = conn.query_row(&format!(
                                    "SELECT Group_Concat(text) FROM pastes{}", from.id),
                                    &[], |row| row.get(0));

                                match res_pastes {
                                    Ok(str_pastes) => {
                                        let _ = bot.send_message(&args::SendMessage
                                            ::new(&str_pastes)
                                            .chat_id(message.chat.id));
                                    }

                                    Err(e) => println!("{:?}", e)
                                }
                            }
                            "/newpaste" => {
                                let amount: Result<bool, _> = conn.query_row(
                                    "SELECT new FROM users WHERE id=?1",
                                    &[&from.id], |row| row.get(0));

                                match amount {
                                    Ok(num) => {
                                        if num {
                                            conn.execute(&format!("
                                                CREATE TABLE pastes{} (
                                                    hash            STRING NOT NULL PRIMARY KEY,
                                                    text            STRING NOT NULL,
                                                    uses            INTEGER NOT NULL DEFAULT 0
                                                )", from.id), &[]).unwrap();
                                        }
                                    },
                                    Err(e) => println!("{:?}", e)
                                }

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
                                    &[&from.id], |row| row.get(0));

                                match cur_state {
                                    Ok(num) => {
                                        match num {
                                            1 => {
                                                conn.execute("
                                                    UPDATE users
                                                    SET state = 0
                                                    WHERE id=?1",
                                                    &[&from.id]).unwrap();

                                                let mut sh = Md5::new();
                                                sh.input_str(&message_text);

                                                conn.execute(&format!("
                                                    INSERT OR IGNORE INTO pastes{} (hash, text)
                                                    VALUES (?1, ?2)", from.id),
                                                    &[&sh.result_str(), &message_text]).unwrap();

                                                let _ = bot.send_message(&args::SendMessage
                                                    ::new("Added.")
                                                    .chat_id(message.chat.id));
                                            },

                                            _ => {}
                                        }
                                    }

                                    Err(e) => println!("{:?}", e)
                                }
                            }
                        }
                    }
                }
            }

            if let Some(inline_query) = update.inline_query {
                handle_inline(&bot, &inline_query, &conn);
            }
        }
    }
    update_args.limit = Some(0);
    update_args.timeout = Some(0);
    let _ = bot.get_updates(&update_args);
}

fn handle_inline(bot: &BotApi, inline_query: &types::InlineQuery, conn: &Connection) {
    let query = format!("SELECT text,hash FROM pastes{} ORDER BY uses DESC", inline_query.from.id);
    let mut stmt = conn.prepare(&query).unwrap();
    let mut res_pastes = stmt.query_map_named(&[], |row| {
        Paste {
            text: row.get(0),
            hash: row.get(1),
        }
    }).unwrap();

    let mut pastes: Vec<Paste> = Vec::new();
    let mut contents: Vec<InputMessageContent> = Vec::new();
    let mut results = Vec::new();

    for res_paste in res_pastes {
        match res_paste {
            Ok(paste) => {
                pastes.push(paste)
            }
            Err(e) => println!("{:?}", e)
        }
    }

    for paste in &pastes {
        contents.push(InputMessageContent::new_text(&paste.text));
    }

    for (i, paste) in pastes.iter().enumerate() {
        results.push(InlineQueryResult::new_article(&paste.hash, &paste.text, &contents[i]));
    }


    let _ = bot.answer_inline_query(&args::AnswerInlineQuery::new(&inline_query.id, &results).cache_time(0));
}
