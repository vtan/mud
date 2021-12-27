use lazy_static::lazy_static;

use crate::{
    event_writer::EventWriter,
    game_state::Player,
    id::Id,
    line::{span, Line},
};

lazy_static! {
    static ref HELP_LINES: Vec<Line> = vec![
        span("Commands:").bold().line(),
        span("look").color("white").line().push(span(" – Look around or at something")),
        span("north").color("white").line().push(span(", etc. – Move to another room")),
        span("kill").color("white").line().push(span(", etc. – Attack something or someone")),
        span("say").color("white").line().push(span(" – Say something to the others in the room")),
        span("emote").color("white").line().push(span(" – Act out something")),
        span("roll").color("white").line().push(span(" – Roll a die")),
        span("who").color("white").line().push(span(" – See who is online")),
        span("alias").color("white").line().push(span(" – List short aliases for commands")),
        span("help").color("white").line().push(span(" – You're looking at it")),
        span("There are also special commands for interacting with specific rooms, or objects in there.").line(),
    ];
}

pub fn help(player_id: Id<Player>, writer: &mut EventWriter) {
    writer.tell_many(player_id, &HELP_LINES);
}
