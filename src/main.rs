extern crate tg_botapi;

use tg_botapi::args;
use tg_botapi::types;
use tg_botapi::BotApi;

use std::sync::Arc;
// use std::thread;
// use std::env;

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

    let mut update_args = args::GetUpdates::new().timeout(600).offset(0);

    'update_loop: loop {
        let updates = bot.get_updates(&update_args).unwrap();

        for update in updates {
            update_args.offset = Some(update.update_id + 1);

            if let Some(message) = update.message {

                let message_text = message.text.unwrap_or(String::new());
                let mut split_text = message_text.split_whitespace();

                if let Some(cmd) = split_text.next() {
                    match cmd {
                        "/start" | "/help" | "/about" | "/source" => {
                            let _ = bot.send_message(&args::SendMessage::new(about)
                                .chat_id(message.chat.id).parse_mode("HTML"));
                        }
                        "/newpaste" => {

                        }
                        _ => {

                        }
                    }
                }
            }

            if let Some(inline_query) = update.inline_query {
                let lenny_txt = format!("{} {}", inline_query.query, "( ͡° ͜ʖ ͡°)");
                let shrug_txt = format!("{} {}", inline_query.query, "¯\\_(ツ)_/¯");
                let gnu_txt = 
                    "I'd just like to interject for moment. What you're refering \
                     to as Linux, is in fact, GNU/Linux, or as I've recently taken \
                     to calling it, GNU plus Linux. Linux is not an operating system \
                     unto itself, but rather another free component of a fully functioning \
                     GNU system made useful by the GNU corelibs, shell utilities and vital \
                     system components comprising a full OS as defined by POSIX.\n\n\
                     Many computer users run a modified version of the GNU system every day, \
                     without realizing it. Through a peculiar turn of events, the version of \
                     GNU which is widely used today is often called Linux, and many of its \
                     users are not aware that it is basically the GNU system, developed by \
                     the GNU Project.\n\n\
                     There really is a Linux, and these people are using it, but it is just \
                     a part of the system they use. Linux is the kernel: the program in the \
                     system that allocates the machine's resources to the other programs \
                     that you run. The kernel is an essential part of an operating system, \
                     but useless by itself; it can only function in the context of a complete \
                     operating system. Linux is normally used in combination with the GNU \
                     operating system: the whole system is basically GNU with Linux added, \
                     or GNU/Linux. All the so-called Linux distributions are really \
                     distributions of GNU/Linux!";

                let lenny = types::InputMessageContent::new_text(&lenny_txt);
                let shrug = types::InputMessageContent::new_text(&shrug_txt);
                let gnu = types::InputMessageContent::new_text(gnu_txt);
                let results = &[types::InlineQueryResult::new_article("lenny", &lenny_txt, &lenny),
                                types::InlineQueryResult::new_article("shrug", &shrug_txt, &shrug),
                                types::InlineQueryResult::new_article("GNU/Linux", &gnu_txt, &gnu)];

                let _ = bot.answer_inline_query(&args::AnswerInlineQuery::new(&inline_query.id, results).cache_time(0));
            }
        }
    }
    update_args.limit = Some(0);
    update_args.timeout = Some(0);
    let _ = bot.get_updates(&update_args);
}
