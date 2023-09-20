use std::{
    collections::{hash_map::{Entry, OccupiedEntry}, HashMap},
    time,
};

use frankenstein::{
    AllowedUpdate, AsyncApi, AsyncTelegramApi, GetUpdatesParams, KeyboardButton, ParseMode,
    ReplyKeyboardMarkup, ReplyKeyboardRemove, ReplyMarkup, SendMessageParams, UpdateContent, User,
};

use crate::{Pcb, Database};

macro_rules! send_message {
    ($api:expr, $chat:expr, $reply_keyboard_markup:expr, $($arg:tt)*) => {
        send_message($api, $chat, $reply_keyboard_markup, format!($($arg)*)).await
    }
}

enum UserDialogueState {
    SentGreetingWaitForLocation,
    GotLocationWaitForAdditionalInformationYesNo((f64, f64)),
    WaitForAdditionalInformation((f64, f64)),
}

pub async fn send_message(
    api: &AsyncApi,
    chat: i64,
    reply_keyboard_markup: Option<ReplyKeyboardMarkup>,
    message: String,
) {
    let params = SendMessageParams::builder()
        .chat_id(chat)
        .text(&message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(if let Some(reply_keyboard_markup) = reply_keyboard_markup {
            ReplyMarkup::ReplyKeyboardMarkup(reply_keyboard_markup)
        } else {
            ReplyMarkup::ReplyKeyboardRemove(
                ReplyKeyboardRemove::builder().remove_keyboard(true).build(),
            )
        })
        .build();
    if let Err(error) = api.send_message(&params).await {
        eprintln!("Failed sending message: {error:?}");
    }
}

async fn received_pcb_data(api: &AsyncApi, chat_id: i64, user: &User, entry: OccupiedEntry<'_, u64, (time::Instant, UserDialogueState)>, location: (f64, f64), additional_information: Option<String>) {
    if let Some(username) = user.username.clone() {
        let pcb = Pcb {
            user_id: user.id,
            username,
            location,
            additional_information,
        };
        //TODO
        send_message!(api, chat_id, None, "Great\\! We are done here\\. To edit your entry on the map, send me /start again\\.");
        entry.remove();
    } else {
        send_message!(api, chat_id, None, "I'm sorry, you seem to have removed your username during our conversation\\. To allow users to contact you via the map, a Telegram username is necessary\\. Please set a username in the Telegram settings\\. When you are done, you can send me the command /start to restart our conversation\\.");

    }
}

pub fn markdown_escape(string: String) -> String {
    string
        .chars()
        .map(|c| {
            let code_point: u32 = c.into();
            match c {
                // "Any character with code between 1 and 126 inclusively can be escaped anywhere with a preceding '\' character"
                // https://core.telegram.org/bots/api#markdownv2-style
                c if 1u32 <= code_point && code_point <= 126u32 => format!("\\{c}"),
                _ => format!("{c}"),
            }
        })
        .collect()
}

fn vertical_keyboard_layout(buttons: &[&'static str]) -> ReplyKeyboardMarkup {
    ReplyKeyboardMarkup::builder().keyboard(buttons.into_iter().map(|button| vec![KeyboardButton::builder().text(button.to_string()).build()]).collect()).resize_keyboard(true).one_time_keyboard(true).build()
}

pub async fn start_telegram_bot(_: Database) {
    let mut user_states: HashMap<u64, (time::Instant, UserDialogueState)> = HashMap::new();
    
    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");

    let api = AsyncApi::new(&token);
    let mut update_params = GetUpdatesParams::builder()
        .allowed_updates(vec![AllowedUpdate::Message])
        .build();
    loop {
        match api.get_updates(&update_params).await {
            Ok(response) => {
                for update in response.result {
                    if let UpdateContent::Message(message) = update.content {
                        let user = if let Some(user) = message.from {
                            user
                        } else {
                            eprintln!("message.from not set: {message:?}");
                            continue;
                        };
                        match user_states.entry(user.id) {
                            Entry::Occupied(mut user_state) => {
                                match user_state.get_mut().1 {
                                    UserDialogueState::SentGreetingWaitForLocation => {
                                        if let Some(location) = message.location {
                                            send_message!(&api, message.chat.id, Some(vertical_keyboard_layout(&["Yes", "No"])), "Thank you\\. Do you want to add any additional information to the map, apart from a link to your telegram account? For example, you might want to introduce yourself or to add a e\\-mail address\\.");
                                            *user_state.get_mut() = (time::Instant::now(), UserDialogueState::GotLocationWaitForAdditionalInformationYesNo((location.latitude, location.longitude)));
                                        }
                                    },
                                    UserDialogueState::GotLocationWaitForAdditionalInformationYesNo(coords) => match message.text.as_deref() {
                                        Some("Yes") => {
                                            send_message!(&api, message.chat.id, None, "Alright, then you may now enter this additional information\\. Please don't enter too much, I will truncate your input at 250 characters\\.");
                                            *user_state.get_mut() = (time::Instant::now(), UserDialogueState::WaitForAdditionalInformation(coords));
                                        },
                                        Some("No") => received_pcb_data(&api, message.chat.id, &user, user_state, coords, None).await,
                                        _ => send_message!(&api, message.chat.id, Some(vertical_keyboard_layout(&["Yes", "No"])), "Sorry, I don't understand. Please reply either \"Yes\" or \"No\"\\.")

                                    },
                                    UserDialogueState::WaitForAdditionalInformation(coords) => if let Some(text) = message.text {
                                        received_pcb_data(&api, message.chat.id, &user, user_state, coords, Some(text)).await;
                                    } else {
                                        send_message!(&api, message.chat.id, None, "Sorry, this additional information must be text.");
                                    }
                                }
                            }
                            Entry::Vacant(user_state) => {
                                let escaped_first_name = markdown_escape(user.first_name);
                                // new user
                                if message.text.as_deref() == Some("/start") {
                                    if user.username.is_none() {
                                        send_message!(&api, message.chat.id, None, "Hello {escaped_first_name}\\! Looks like you want to add a Dreame adapter to the map\\. However, you do not have a Telegram username\\. To allow users to contact you via the map, a Telegram username is necessary\\. Please set a username in the Telegram settings\\. When you are done, you can send me the command /start to restart the process\\.");
                                    } else {
                                        send_message!(&api, message.chat.id, None, "Hello {escaped_first_name}\\! Looks like you want to add a Dreame adapter to the map\\. To do so, please send me the location that should be shown on the map\\.\n\n*__IMPORTANT__*: The location you send me will be used as\\-is and displayed on the map\\. If you don't want the world to know where exactly you live, you might want to use a location that is a bit away\\.");
                                        user_state.insert((
                                            time::Instant::now(),
                                            UserDialogueState::SentGreetingWaitForLocation,
                                        ));
                                    }
                                } else {
                                    send_message!(&api, message.chat.id, None, "Hi\\! This is the bot managing the Dreame adapter map\\. To add a new adapter or to manage an existing adapter linked to your telegram account, please send me the command /start\\.");
                                }
                            }
                        }
                    }
                    update_params.offset = Some((update.update_id + 1).into());
                }
            }
            Err(error) => {
                eprintln!("Failed to get updates: {error} ({error:?})");
            }
        }
    }
}
