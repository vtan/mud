use crate::{
    event_writer::EventWriter,
    game_state::Player,
    id::Id,
    line::{span, Line},
};
use lazy_static::lazy_static;

static ALIASES: &[(&str, &str)] = &[
    ("l", "look"),
    ("\"", "say"),
    (":", "emote"),
    ("n", "north"),
    ("ne", "northeast"),
    ("e", "east"),
    ("se", "southeast"),
    ("s", "south"),
    ("sw", "southwest"),
    ("w", "west"),
    ("nw", "northwest"),
    ("u", "up"),
    ("d", "down"),
];

lazy_static! {
    static ref ALIAS_LINES: Vec<Line> = ALIASES
        .iter()
        .map(|(alias, resolution)| span(&format!("{} → {}", alias, resolution)).line())
        .collect::<Vec<_>>();
}

pub fn resolve_aliases<'a>(
    mut command: &'a str,
    mut args: Vec<&'a str>,
) -> (&'a str, Vec<&'a str>) {
    if let Some(stripped) = command.strip_prefix("\"") {
        command = "\"";
        if !stripped.is_empty() {
            args.insert(0, stripped);
        }
    } else if let Some(stripped) = command.strip_prefix("*") {
        command = ":";
        if !stripped.is_empty() {
            args.insert(0, stripped);
        }
    }
    for (alias, resolution) in ALIASES {
        if command == *alias {
            command = resolution;
            return (command, args);
        }
    }
    return (command, args);
}

pub fn alias(player_id: Id<Player>, writer: &mut EventWriter) {
    writer.tell_many(player_id, &ALIAS_LINES);
}
