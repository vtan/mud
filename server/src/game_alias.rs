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

pub fn resolve_aliases(mut words: Vec<&str>) -> Vec<&str> {
    for (alias, resolution) in ALIASES {
        if words[0] == *alias {
            words[0] = resolution;
            return words;
        }
    }
    words
}

pub fn alias(player_id: Id<Player>, writer: &mut EventWriter) {
    writer.tell_many(player_id, &ALIAS_LINES);
}
