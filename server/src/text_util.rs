use crate::line::{span, LineSpan};

pub fn and_list(words: &[String]) -> String {
    match words.len() {
        0 => "".to_string(),
        1 => words[0].clone(),
        2 => format!("{} and {}", words[0], words[1]),
        len => format!("{} and {}", words[0..len - 1].join(", "), words[len - 1]),
    }
}

pub fn and_list_span(mut words: Vec<LineSpan>) -> Vec<LineSpan> {
    match words.len() {
        0 => words,
        1 => words,
        2 => {
            words.insert(1, span(" and "));
            words
        }
        len => {
            words.insert(len - 1, span(" and "));
            for i in (1..len - 1).rev() {
                words.insert(i, span(", "));
            }
            words
        }
    }
}

pub fn are(len: usize) -> &'static str {
    if len > 1 {
        "are"
    } else {
        "is"
    }
}

pub fn plural(len: usize, str: &str) -> String {
    if len > 1 {
        format!("{}s", str)
    } else {
        str.to_string()
    }
}
