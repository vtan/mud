use std::ops::Add;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    event_writer::EventWriter,
    game_state::{player_ids_in_room_except, GameState, Player},
    line::{span, Color, Line},
};

static ILLEGAL_CHAT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\p{Extended_Pictographic}").unwrap());

pub enum ChatCommand {
    Say,
    Emote,
}

pub fn chat(
    player: &Player,
    words: Vec<&str>,
    kind: ChatCommand,
    writer: &mut EventWriter,
    state: &GameState,
) {
    let mut words_joined = words.join(" ");
    if words_joined.len() > 128 {
        writer.tell(player.id, Line::str("That message is too long."));
    } else if ILLEGAL_CHAT_REGEX.is_match(&words_joined) {
        writer.tell(
            player.id,
            Line::str("That message contains illegal characters."),
        );
    } else {
        if let ChatCommand::Say = kind {
            let mut chars = words_joined.chars();
            if let Some(first_char) = chars.next() {
                words_joined = first_char.to_uppercase().collect::<String>() + chars.as_str()
            }
        }

        let last_char = words_joined.chars().last().unwrap_or(' ');
        if !last_char.is_ascii_punctuation() {
            words_joined = words_joined.add(".");
        }

        let to_self = span(&match kind {
            ChatCommand::Say => format!("You say, \"{}\"", &words_joined),
            ChatCommand::Emote => format!("{} {}", &player.name, &words_joined),
        })
        .color(Color::Yellow)
        .line();
        writer.tell(player.id, to_self);

        let to_others = span(&match kind {
            ChatCommand::Say => format!("{} says, \"{}\"", &player.name, &words_joined),
            ChatCommand::Emote => format!("{} {}", &player.name, &words_joined),
        })
        .color(Color::Yellow)
        .line();
        writer.tell_many(
            player_ids_in_room_except(&state.players, player.room_id, player.id),
            to_others,
        );
    }
}
