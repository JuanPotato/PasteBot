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
use tg_botapi::types::InlineKeyboardButton;
use tg_botapi::types::ReplyMarkup;

use tg_botapi::BotApi;

use std::sync::Arc;
// use std::thread;
// use std::env;

#[derive(Debug)]
struct Paste {
    text: String,
    hash: String,
    uses: i64,
}

fn main() {
    let token = ""; // I'm lazy go away
    let bot = Arc::new(BotApi::new_debug(token));

    // let me_irl = bot.get_me().expect("Could not establish a connection :\\");

    let db_path = Path::new("./database.db");
    let exists = db_path.exists();

    let conn = Connection::open(db_path).unwrap();

    if !exists {
        conn.execute("
            CREATE TABLE users (
                id              INTEGER PRIMARY KEY,
                state           INTEGER NOT NULL DEFAULT 0,
                amount          INTEGER NOT NULL DEFAULT 0,
                new             BOOLEAN NOT NULL DEFAULT 1
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
                if let Some(ref from) = message.from { // This is getting nasty
                    conn.execute("INSERT OR IGNORE INTO users (id) VALUES (?1)",
                                 &[&from.id]).unwrap();

                    let message_text = message.text.unwrap_or(String::new());
                    let mut split_text = message_text.split_whitespace();

                    if let Some(cmd) = split_text.next() {
                        match cmd {
                            "/start" | "/help" => {
                                welcome_message(&bot, message.chat.id)
                            }
                            "/listpastes" => { // Will change to an inline /managepastes later, this is for debugging right now
                                handle_list_pastes(&bot, &from, &message.chat, &conn);
                            }
                            "/managepastes" => { // Will change to an inline /managepastes later, this is for debugging right now
                                handle_manage_pastes(&bot, &from, &message.chat, &conn);
                            }
                            "/newpaste" => {
                                handle_new_paste(&bot, &from, &message.chat, &conn);
                            }
                            _ => {
                                let cur_state: Result<i64, _> = conn.query_row(
                                    "SELECT state FROM users WHERE id=?1",
                                    &[&from.id], |row| row.get(0));

                                match cur_state {
                                    Ok(num) => {
                                        match num {
                                            1 => add_new_paste(&bot, &from, &message.chat, &message_text, &conn),

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

            if let Some(ref inline_query) = update.inline_query {
                conn.execute("INSERT OR IGNORE INTO users (id) VALUES (?1)",
                             &[&inline_query.from.id]).unwrap();
                handle_inline(&bot, &inline_query, &conn);
            }

            if let Some(ref callback_query) = update.callback_query {
                handle_button(&bot, callback_query, &conn);
            }

            if let Some(ref chosen_inline_result) = update.chosen_inline_result {
                handle_chosen_paste(&chosen_inline_result, &conn);
            }
        }
    }
}

fn welcome_message(bot: &BotApi, chat_id: i64) {
    let about = "Hey, I'm Paste Bot!\n\n\

                I'm an inline bot that to make it easier for you to shitpost by \
                letting you add custom endings to your message. Or if you want \
                to send a copypasta but hate searching it up eerytime, I can do \
                that too. Each paste is configurable. Your pastes are sorted inline \
                by how often you use them (although this is also configurable).\n\n\

                Use /newpaste to get started.\n\n\

                Made by @JuanPotato, \
                <a href=\"https://github.com/JuanPotato/PasteBot\">Source</a>";
    // Is it bad practise to have a huge string inside a function?

    let _ = bot.send_message(&args::SendMessage
                             ::new(about)
                             .chat_id(chat_id)
                             .parse_mode("HTML"));
}

fn needs_pastes(conn: &Connection, id: i64) -> bool {
    // conn.query_row(
    //     "SELECT new,amount FROM users WHERE id=?1",
    //     &[&id], |row| {
    //         let new: bool = row.get(0);
    //         let amount: i64 = row.get(1);
    //         new && amount > 0
    //     }).unwrap()
    conn.query_row(  // Which one is faster though :\
        "SELECT CASE WHEN amount > 0 AND new != 1 
            THEN 0
            ELSE 1
        END AS thing
        FROM users WHERE id=?1",
        &[&id], |row| row.get(0)).unwrap()
}

fn paste_count(conn: &Connection, id: i64) -> i64 {
    conn.query_row(
        "SELECT amount FROM users WHERE id=?1",
        &[&id], |row| row.get(0)).unwrap()
}

fn handle_button(bot: &BotApi, callback_query: &types::CallbackQuery, conn: &Connection) {
    // match callback_query.data.unwrap().as_str() {
        // "delete" => {
        //     // Development is put on hold until I release a new version of 
        // },
        // "back" => {
        //     // let edit_args = args::EditMessageText::new(&text)
        //     //     .chat_id(msg.chat.id)
        //     //     .message_id(msg.message_id)
        //     //     .parse_mode("Markdown")
        //     //     .reply_markup(&reply_markup);
        //     //     let _ = bot.edit_message_text(&edit_args);

        // },
        // _ => {
            let paste = conn.query_row(
                &format!("SELECT text,hash,uses FROM pastes{} WHERE hash=?1 ORDER BY uses",
                            callback_query.from.id),
                &[&callback_query.data], |row| {
                    Paste {
                        text: row.get(0),
                        hash: row.get(1),
                        uses: row.get(2)
                    }
                }).unwrap();

            if let Some(ref msg) = callback_query.message {
                let text = format!("Text: {}\n\nUses: {}", paste.text, paste.uses);
                
                let reply_markup = ReplyMarkup::new_inline_keyboard(
                    vec![
                        vec![
                            InlineKeyboardButton::new("Delete".into())
                                .callback_data("delete".into()),
                            InlineKeyboardButton::new("Back".into())
                                .callback_data("back".into()),
                        ]
                    ]
                );

                let edit_args = args::EditMessageText::new(&text)
                    .chat_id(msg.chat.id)
                    .message_id(msg.message_id)
                    .parse_mode("Markdown")
                    .reply_markup(reply_markup.into());
                let _ = bot.edit_message_text(&edit_args);
            }
    //     }
    // }
}

fn get_pastes_as_buttons(user_id: i64, conn: &Connection) -> Vec<Vec<InlineKeyboardButton>> {
    let query = format!("SELECT text,hash FROM pastes{} ORDER BY uses DESC LIMIT 6",
                        user_id);
    let mut stmt = conn.prepare(&query).unwrap();
    let mut res_pastes = stmt.query_map_named(&[], |row| {
        let mut text: String = row.get(0);

        Paste {
            text: if text.len() < 10 {
                text
            } else {
                text.truncate(7);
                format!("{}...", text)
            },
            hash: row.get(1),
            uses: 0
        }

    }).unwrap();

    let mut pastes: Vec<Paste> = Vec::new();

    for res_paste in res_pastes {
        match res_paste {
            Ok(paste) => {
                pastes.push(paste);
            }
            Err(e) => println!("{:?}", e) // Why would this happen
        }
    }

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    let mut i = 0;
    let len = pastes.len();
    while i < len {
        let p1 = pastes.get(i).unwrap();
        if i + 1 < len {
            let p2 = pastes.get(i+1).unwrap();
            buttons.push(vec![
                InlineKeyboardButton::new(p1.text.clone())
                    .callback_data(p1.hash.clone()),
                InlineKeyboardButton::new(p2.text.clone())
                    .callback_data(p2.hash.clone())
                ]);
        } else {
            buttons.push(vec![
                InlineKeyboardButton::new(p1.text.clone())
                    .callback_data(p1.hash.clone())
            ]);
        }
    }

    buttons
}

fn handle_manage_pastes(bot: &BotApi, from: &types::User, chat: &types::Chat, conn: &Connection) {
    if needs_pastes(conn, from.id) {
        let _ = bot.send_message(&args::SendMessage
            ::new("It doesn't seem like you have any pastes. :(\n\
                   Use /newpaste to make one.")
            .chat_id(chat.id));
    } else {
        let keyboard = get_pastes_as_buttons(from.id, conn);

        let _ = bot.send_message(
            &args::SendMessage
                ::new("Select a paste")
                .chat_id(chat.id)
                .reply_markup(ReplyMarkup::new_inline_keyboard(keyboard).into())
                );
    }
}

fn handle_list_pastes(bot: &BotApi, from: &types::User, chat: &types::Chat, conn: &Connection) {
    if needs_pastes(conn, from.id) {
        let _ = bot.send_message(&args::SendMessage
            ::new("It doesn't seem like you have any pastes. :(\n\
                   Use /newpaste to make one.")
            .chat_id(chat.id));
    } else {
        let res_pastes: Result<String, _> = conn.query_row(&format!(
            "SELECT Group_Concat(text) FROM pastes{}", from.id),
            &[], |row| row.get(0));

        match res_pastes {
            Ok(str_pastes) => {
                let _ = bot.send_message(&args::SendMessage
                    ::new(&str_pastes)
                    .chat_id(chat.id));
            }

            Err(e) => println!("{:?}", e)
        }
    }
}

fn add_new_paste(bot: &BotApi, from: &types::User, chat: &types::Chat, message_text: &str, conn: &Connection) {
    conn.execute("UPDATE users
                  SET state=0
                  WHERE id=?1",
                 &[&from.id]).unwrap();

    let mut sh = Md5::new();
    sh.input_str(message_text);

    conn.execute(&format!("
        INSERT OR IGNORE INTO pastes{} (hash, text)
        VALUES (?1, ?2)", from.id),
        &[&sh.result_str(), &message_text]).unwrap();

    conn.execute("UPDATE users
                  SET amount=amount+1
                  WHERE id=?1",
                 &[&from.id]).unwrap();

    let _ = bot.send_message(&args::SendMessage
        ::new("Added.")
        .chat_id(chat.id));
}

fn handle_new_paste(bot: &BotApi, from: &types::User, chat: &types::Chat, conn: &Connection) {
    let is_new = conn.query_row(
        "SELECT new FROM users WHERE id=?1",
        &[&from.id], |row| row.get(0)).unwrap();

    if is_new {
        conn.execute(&format!("
            CREATE TABLE pastes{} (
                hash            STRING NOT NULL PRIMARY KEY,
                text            STRING NOT NULL,
                uses            INTEGER NOT NULL DEFAULT 0
            )", from.id), &[]).unwrap();

        conn.execute("
            UPDATE users
            SET new=0
            WHERE id=?1",
            &[&from.id]).unwrap();
    }

    conn.execute("
        UPDATE users
        SET state=1
        WHERE id=?1",
        &[&from.id]).unwrap();

    let _ = bot.send_message(&args::SendMessage
        ::new("Ok, send me the text you want that paste to be.")
        .chat_id(chat.id));
}

fn handle_inline(bot: &BotApi, inline_query: &types::InlineQuery, conn: &Connection) {
    if needs_pastes(conn, inline_query.from.id) {
        let _ = bot.answer_inline_query(
            &args::AnswerInlineQuery::new(
                &inline_query.id, vec![]
            ).switch_pm_text("You don't have any pastes, tap me to start.")
             .is_personal(true).cache_time(0));
    } else {
        let query = format!("SELECT text,hash FROM pastes{} ORDER BY uses DESC",
                            inline_query.from.id);
        let mut stmt = conn.prepare(&query).unwrap();
        let res_pastes = stmt.query_map_named(&[], |row| {
            Paste {
                text: row.get(0),
                hash: row.get(1),
                uses: 0
            }
        }).unwrap();

        let mut pastes: Vec<Paste> = Vec::new();
        let mut results = Vec::new();

        for res_paste in res_pastes {
            match res_paste {
                Ok(paste) => {
                    pastes.push(paste)
                }
                Err(e) => println!("{:?}", e)
            }
        }


        for paste in pastes {
            results.push(InlineQueryResult::new_article(paste.hash.clone(), paste.text.clone(), InputMessageContent::new_text(paste.text.clone())));
        }

        let _ = bot.answer_inline_query(&args::AnswerInlineQuery::new(&inline_query.id, results).cache_time(0));
    }
}

fn handle_chosen_paste(chosen_inline_result: &types::ChosenInlineResult, conn: &Connection) {
    conn.execute(&format!(
        "UPDATE pastes{} SET uses = uses+1 WHERE hash=?1",
        chosen_inline_result.from.id),
    &[&chosen_inline_result.result_id]).unwrap();
}
