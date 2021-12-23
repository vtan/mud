use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Line {
    pub spans: Vec<LineSpan>,
}

#[derive(Serialize, Clone, Debug)]
pub struct LineSpan {
    pub text: String,
    pub bold: Option<bool>,
}

impl From<&str> for Line {
    fn from(text: &str) -> Line {
        Line {
            spans: vec![LineSpan { text: text.to_string(), bold: None }],
        }
    }
}

impl From<String> for Line {
    fn from(text: String) -> Line {
        Line {
            spans: vec![LineSpan { text, bold: None }],
        }
    }
}
