use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f_,\[\]()/\\.]+")] // Ignore whitespace and common separators
pub enum Token {
    #[regex(r"(?i)v(ol(ume)?)?")]
    Volume,

    #[regex(r"(?i)c(h(apter)?)?")]
    Chapter,

    #[regex(r"(?i)p(age)?")]
    Page,

    #[regex(r"(?i)cover")]
    Cover,

    #[token("-")]
    Dash,

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f32>().ok())]
    Number(f32),
}
