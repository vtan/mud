use once_cell::sync::Lazy;

use crate::{event_writer::EventWriter, game_state::Player, id::Id, line::Line};

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

static ALIAS_LINES: Lazy<Vec<Line>> = Lazy::new(|| {
    ALIASES
        .iter()
        .map(|(alias, resolution)| Line::str(&format!("{} → {}", alias, resolution)))
        .collect::<Vec<_>>()
});

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
    writer.tell_lines(player_id, &ALIAS_LINES);
}
