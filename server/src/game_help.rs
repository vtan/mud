use lazy_static::lazy_static;

use crate::{
    event_writer::EventWriter,
    game_state::Player,
    id::Id,
    line::{span, Color, Line},
};

lazy_static! {
    static ref HELP_LINES: Vec<Line> = vec![
        span("Commands:").bold().line(),
        span("look").color(Color::White).line().push(span(" – Look around or at something")),
        span("north").color(Color::White).line().push(span(", etc. – Move to another room")),
        span("kill").color(Color::White).line().push(span(", etc. – Attack something or someone")),
        span("say").color(Color::White).line().push(span(" – Say something to the others in the room")),
        span("emote").color(Color::White).line().push(span(" – Act out something")),
        span("roll").color(Color::White).line().push(span(" – Roll a die")),
        span("who").color(Color::White).line().push(span(" – See who is online")),
        span("alias").color(Color::White).line().push(span(" – List short aliases for commands")),
        span("help").color(Color::White).line().push(span(" – You're looking at it")),
        Line::str("There are also special commands for interacting with specific rooms, or objects in there."),
    ];
}

pub fn help(player_id: Id<Player>, writer: &mut EventWriter) {
    writer.tell_lines(player_id, &HELP_LINES);
}
