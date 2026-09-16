use buraaq_source::Span;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Note,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle,
}

impl Label {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Help {
    pub message: String,
    pub suggestion: Option<Suggestion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub replacement: String,
    pub span: Option<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: Level,
    pub code: Option<String>,
    pub message: String,
    pub reason: Option<String>,
    pub labels: Vec<Label>,
    pub help: Option<Help>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            code: None,
            message: message.into(),
            reason: None,
            labels: Vec::new(),
            help: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_help(mut self, help: Help) -> Self {
        self.help = Some(help);
        self
    }

    pub fn with_suggestion(mut self, message: impl Into<String>, replacement: impl Into<String>) -> Self {
        self.help = Some(Help {
            message: message.into(),
            suggestion: Some(Suggestion {
                replacement: replacement.into(),
                span: None,
            }),
        });
        self
    }
}
