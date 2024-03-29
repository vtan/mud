use crate::line::{span, LineSpan};

pub fn and_spans(mut words: Vec<LineSpan>) -> Vec<LineSpan> {
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

pub fn and_span_vecs(mut words: Vec<Vec<LineSpan>>) -> Vec<LineSpan> {
    match words.len() {
        0 => vec![],
        1 => words[0].to_owned(),
        2 => {
            words.insert(1, vec![span(" and ")]);
            words.into_iter().flatten().collect()
        }
        len => {
            words.insert(len - 1, vec![span(" and ")]);
            for i in (1..len - 1).rev() {
                words.insert(i, vec![span(", ")]);
            }
            words.into_iter().flatten().collect()
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
