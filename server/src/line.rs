use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Line {
    pub spans: Vec<LineSpan>,
}

impl Line {
    pub fn spans(spans: Vec<LineSpan>) -> Line {
        Line { spans }
    }

    pub fn str(str: &str) -> Line {
        Line { spans: vec![span(str)] }
    }

    pub fn push(mut self, span: LineSpan) -> Line {
        self.spans.push(span);
        self
    }

    pub fn extend(mut self, spans: Vec<LineSpan>) -> Line {
        self.spans.extend(spans);
        self
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct LineSpan {
    pub text: String,
    pub bold: Option<bool>,
    pub color: Option<&'static str>,
}

impl LineSpan {
    pub fn line(self) -> Line {
        Line { spans: vec![self] }
    }

    pub fn bold(self) -> Self {
        LineSpan { text: self.text, bold: Some(true), color: self.color }
    }

    pub fn color(self, color: Color) -> Self {
        LineSpan {
            text: self.text,
            bold: self.bold,
            color: Some(color.as_str()),
        }
    }
}

pub fn span(str: &str) -> LineSpan {
    LineSpan { text: str.to_string(), bold: None, color: None }
}

pub enum Color {
    White,
    DarkGrey,
    Blue,
    Yellow,
    Orange,
    LightRed,
    Red,
    LightCyan,
    Cyan,
}

impl Color {
    fn as_str(&self) -> &'static str {
        match self {
            Color::White => "white",
            Color::DarkGrey => "dark-grey",
            Color::Blue => "blue",
            Color::Yellow => "yellow",
            Color::Orange => "orange",
            Color::LightRed => "light-red",
            Color::Red => "red",
            Color::LightCyan => "light-cyan",
            Color::Cyan => "cyan",
        }
    }
}
