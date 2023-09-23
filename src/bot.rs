use std::{
    collections::{
        hash_map::{Entry, OccupiedEntry},
        HashMap,
    },
    time::{self, Duration}, sync::Arc,
};

use frankenstein::{
    AllowedUpdate, AsyncApi, AsyncTelegramApi, GetUpdatesParams, KeyboardButton, ParseMode,
    ReplyKeyboardMarkup, ReplyKeyboardRemove, ReplyMarkup, SendMessageParams, UpdateContent, User,
};
use tokio::sync::Mutex;

use crate::{Database, Pcb};

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

async fn clean_up_stale_user_states(api: Arc<Mutex<AsyncApi>>, user_states: Arc<Mutex<HashMap<u64, (time::Instant, (i64, UserDialogueState))>>>) {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;

        let api = api.lock().await;
        let mut user_states = user_states.lock().await;
        let session_length = if user_states.len() > 10_000 {
            eprintln!("{} concurrent chatbot sessions ongoing – possible DDoS detected! Setting session timeout to 15 minutes", user_states.len());
            900
        } else {
            14400
        };

        //TODO use HashMap::retain instead?
        let mut to_remove = vec![];
        for (user_id, (last_accessed, _)) in user_states.iter() {
            if last_accessed.elapsed() > std::time::Duration::from_secs(session_length) {
                to_remove.push(*user_id);
            }
        }
        for to_remove in to_remove {
            let old_state = user_states.remove(&to_remove).expect("State was gone when I tried to remove it, but it was there before");
            send_message!(&api, old_state.1.0, None, "Sorry, you took too long, I had to cancel our session\\. If you still want to add yourself to the map, send me /start again\\.");
        }
    }
}

async fn received_pcb_data(
    database: &Database,
    api: &AsyncApi,
    chat_id: i64,
    user: &User,
    entry: OccupiedEntry<'_, u64, (time::Instant, (i64, UserDialogueState))>,
    location: (f64, f64),
    additional_information: Option<String>,
) {
    if let Some(username) = user.username.clone() {
        let pcb = Pcb {
            user_id: user.id,
            username,
            location,
            additional_information,
        };
        match database.insert_entry(&pcb).await {
            Ok(()) => send_message!(api, chat_id, None, "Great\\! We are done here\\. To edit your entry on the map, send me /start again\\."),
            Err(error) => {
                eprintln!("Inserting {pcb:?} for user {user:?} went wrong: {error}/{error:?}");
                send_message!(api, chat_id, None, "Sorry, that didn't work\\. Please send me the command /start to try again\\. If the problem persists, please open an issue on GitHub\\.");
            }
        }
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
    ReplyKeyboardMarkup::builder()
        .keyboard(
            buttons
                .into_iter()
                .map(|button| vec![KeyboardButton::builder().text(button.to_string()).build()])
                .collect(),
        )
        .resize_keyboard(true)
        .one_time_keyboard(true)
        .build()
}

pub async fn start_telegram_bot(database: Database) {
    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let api = Arc::new(Mutex::new(AsyncApi::new(&token)));
    let mut update_params = GetUpdatesParams::builder()
        .allowed_updates(vec![AllowedUpdate::Message])
        .build();

    let user_states: Arc<Mutex<HashMap<u64, (time::Instant, (i64,UserDialogueState))>>> = Arc::new(Mutex::new(HashMap::new()));
    tokio::spawn(clean_up_stale_user_states(api.clone(), user_states.clone()));

    loop {
        let api = api.lock().await;
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
                        let mut user_states = user_states.lock().await;
                        match user_states.entry(user.id) {
                            Entry::Occupied(mut user_state) => {
                                match user_state.get_mut().1.1 {
                                    UserDialogueState::SentGreetingWaitForLocation => {
                                        if let Some(location) = message.location {
                                            send_message!(&api, message.chat.id, Some(vertical_keyboard_layout(&["Yes", "No"])), "Thank you\\. Do you want to add any additional information to the map, apart from a link to your telegram account? For example, you might want to introduce yourself or to add a e\\-mail address\\.");
                                            *user_state.get_mut() = (time::Instant::now(), (message.chat.id, UserDialogueState::GotLocationWaitForAdditionalInformationYesNo((location.latitude, location.longitude))));
                                        }
                                    },
                                    UserDialogueState::GotLocationWaitForAdditionalInformationYesNo(coords) => match message.text.as_deref() {
                                        Some("Yes") => {
                                            send_message!(&api, message.chat.id, None, "Alright, then you may now enter this additional information\\. Please don't enter too much, I will truncate your input at 250 characters\\.");
                                            *user_state.get_mut() = (time::Instant::now(), (message.chat.id,UserDialogueState::WaitForAdditionalInformation(coords)));
                                        },
                                        Some("No") => received_pcb_data(&database, &api, message.chat.id, &user, user_state, coords, None).await,
                                        _ => send_message!(&api, message.chat.id, Some(vertical_keyboard_layout(&["Yes", "No"])), "Sorry, I don't understand. Please reply either \"Yes\" or \"No\"\\.")

                                    },
                                    UserDialogueState::WaitForAdditionalInformation(coords) => if let Some(text) = message.text {
                                        received_pcb_data(&database, &api, message.chat.id, &user, user_state, coords, Some(text)).await;
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
                                            (message.chat.id,UserDialogueState::SentGreetingWaitForLocation),
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
