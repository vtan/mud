use std::ops::Add;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    event_writer::EventWriter,
    game_state::{GameState, Player},
    line::{span, Line},
};

lazy_static! {
    static ref HELP_LINES: Vec<Line> = vec![
        span("Commands:").bold().line(),
        span("look").color("white").line().push(span(" – Look around or at something")),
        span("north").color("white").line().push(span(", etc. – Move to another room")),
        span("say").color("white").line().push(span(" – Say something to the others in the room")),
        span("emote").color("white").line().push(span(" – Act out something")),
        span("who").color("white").line().push(span(" – See who is online")),
        span("help").color("white").line().push(span(" – You're looking at it")),
        span("There are also special commands for interacting with specific rooms, or objects in there.").line(),
    ];

    static ref ILLEGAL_CHAT_REGEX: Regex = Regex::new(r"\p{Extended_Pictographic}").unwrap();
}

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
        writer.tell(player.id, span("That message is too long.").line());
    } else if ILLEGAL_CHAT_REGEX.is_match(&words_joined) {
        writer.tell(
            player.id,
            span("That message contains illegal characters.").line(),
        );
    } else {
        match kind {
            ChatCommand::Say => {
                let mut chars = words_joined.chars();
                if let Some(first_char) = chars.next() {
                    words_joined = first_char.to_uppercase().collect::<String>() + chars.as_str()
                }
            }
            _ => (),
        }

        let last_char = words_joined.chars().last().unwrap_or(' ');
        if !last_char.is_ascii_punctuation() {
            words_joined = words_joined.add(".");
        }

        static COLOR: &str = "yellow";
        let to_self = span(&match kind {
            ChatCommand::Say => format!("You say, \"{}\"", &words_joined),
            ChatCommand::Emote => format!("{} {}", &player.name, &words_joined),
        })
        .color(COLOR)
        .line();
        writer.tell(player.id, to_self);

        let to_others = span(&match kind {
            ChatCommand::Say => format!("{} says, \"{}\"", &player.name, &words_joined),
            ChatCommand::Emote => format!("{} {}", &player.name, &words_joined),
        })
        .color(COLOR)
        .line();
        writer.tell_room_except(to_others, player.room_id, player.id, state);
    }
}
