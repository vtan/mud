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

impl LineSpan {
    pub fn line(self) -> Line {
        Line { spans: vec![self] }
    }

    pub fn bold(self) -> Self {
        LineSpan {
            text: self.text,
            bold: Some(true),
        }
    }
}

pub fn span(str: &str) -> LineSpan {
    LineSpan {
        text: str.to_string(),
        bold: None,
    }
}
