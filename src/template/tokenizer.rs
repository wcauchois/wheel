#![allow(dead_code)]

use std::iter::Peekable;

#[derive(Debug)]
pub enum Keyword {
    For,
    If,
    Then,
    Else,
    ElseIf,
    EndFor,
    EndIf,
}

impl Keyword {
    fn from_string(s: &str) -> Option<Keyword> {
        match s.as_ref() {
            "for" => Some(Keyword::For),
            "if" => Some(Keyword::If),
            "then" => Some(Keyword::Then),
            "else" => Some(Keyword::Else),
            "elseif" => Some(Keyword::ElseIf),
            "endfor" => Some(Keyword::EndFor),
            "endif" => Some(Keyword::EndIf),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Token {
    Keyword(Keyword),
    BeginDirective,
    EndDirective,
    BeginSubstitution,
    EndSubstitution,
    VariableName(String),
    Operator(String),
    StringLiteral(String),
    NumberLiteral(i32),
    BooleanLiteral(bool),
    TextContent(String),
    LeftBracket,
    RightBracket,
    Dot
}

#[derive(PartialEq, Eq)]
enum TokenizerState {
    Neutral,
    InsideDirective,
    InsideSubstitution,
}

enum ConsumptionType {
    Single,
    Double { consumed_second: bool },
    Name,
    Numeric,
    String { saw_closing_quote: bool },
    None,
}

impl ConsumptionType {
    fn satisfies(&mut self, c: char) -> bool {
        match *self {
            ConsumptionType::Single | ConsumptionType::None => false,
            ConsumptionType::Double { ref mut consumed_second } => {
                let ret = !(*consumed_second);
                *consumed_second = true;
                ret
            },
            ConsumptionType::Name => c.is_alphabetic() || c.is_numeric(),
            ConsumptionType::Numeric => c.is_numeric(),
            ConsumptionType::String { ref mut saw_closing_quote } => {
                let is_quote = c == '"';
                let ret = !*saw_closing_quote;
                if is_quote {
                    *saw_closing_quote = true;
                }
                ret
            },
        }
    }

    fn determine(c: char) -> ConsumptionType {
        if c.is_alphabetic() {
            ConsumptionType::Name
        } else if c.is_numeric() {
            ConsumptionType::Numeric
        } else if c == '%' || c == '}' {
            ConsumptionType::Double { consumed_second: false }
        } else if c == '"' {
            ConsumptionType::String { saw_closing_quote: false }
        } else {
            ConsumptionType::Single // Operator
        }
    }
}

pub struct TokenIter<I> where I: Iterator<Item=char> {
    chars: Peekable<I>,
    state: TokenizerState,
}

impl<I> Iterator for TokenIter<I> where I: Iterator<Item=char> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        match self.state {
            TokenizerState::Neutral => {
                let mut buffer = String::new();
                let mut first = true;
                let mut have_any_content = false;
                loop {
                    let is_brace = match self.chars.peek() {
                        Some(p) => {
                            have_any_content = true;
                            *p == '{'
                        },
                        None => false,
                    };
                    if is_brace {
                        if first {
                            self.chars.next(); // Skip over brace we just peeked.
                            let next_char = self.chars.next().unwrap();
                            if next_char == '{' {
                                self.state = TokenizerState::InsideSubstitution;
                                return Some(Token::BeginSubstitution);
                            } else if next_char == '%' {
                                self.state = TokenizerState::InsideDirective;
                                return Some(Token::BeginDirective);
                            } else {
                                panic!("Expected { or %");
                            }
                        } else {
                            // Leave brace for next call to this iterator's next().
                            break;
                        }
                    }
                    match self.chars.next() {
                        Some(c) => buffer.push(c),
                        None => break,
                    }
                    first = false;
                }
                if have_any_content {
                    Some(Token::TextContent(buffer))
                } else {
                    None
                }
            }
            TokenizerState::InsideDirective | TokenizerState::InsideSubstitution => {
                // Consume leading whitespace
                loop {
                    let is_whitespace = match self.chars.peek() {
                        Some(p) => (*p).is_whitespace(),
                        None => false,
                    };
                    if !is_whitespace || self.chars.next().is_none() {
                        break;
                    }
                }
                let mut consumption_type = match self.chars.peek() {
                    Some(p) => ConsumptionType::determine(*p),
                    None => ConsumptionType::None,
                };
                if match consumption_type { ConsumptionType::None => true, _ => false} {
                    None
                } else {
                    let mut buffer = String::new();
                    let mut first = true;
                    loop {
                        let is_satisfied = match self.chars.peek() {
                            Some(p) => first || consumption_type.satisfies(*p),
                            None => false,
                        };
                        if is_satisfied {
                            buffer.push(self.chars.next().unwrap());
                        } else {
                            break;
                        }
                        first = false;
                    }
                    match consumption_type {
                        ConsumptionType::Name => {
                            Some(
                                Keyword::from_string(&buffer)
                                    .map(|k| Token::Keyword(k))
                                    .unwrap_or_else(|| Token::VariableName(buffer))
                            )
                        },
                        ConsumptionType::Double { .. } => {
                            if buffer == "%}" {
                                if self.state != TokenizerState::InsideDirective {
                                    panic!("Encountered directive close while not inside directive");
                                }
                                self.state = TokenizerState::Neutral;
                                Some(Token::EndDirective)
                            } else if buffer == "}}" {
                                if self.state != TokenizerState::InsideSubstitution {
                                    panic!("Encountered substitution close while not inside substitution");
                                }
                                self.state = TokenizerState::Neutral;
                                Some(Token::EndSubstitution)
                            } else {
                                panic!("Uknown operator: {}", buffer);
                            }
                        },
                        ConsumptionType::Numeric => {
                            Some(
                                Token::NumberLiteral(buffer.parse::<i32>().unwrap())
                            )
                        },
                        ConsumptionType::String { .. } => {
                            let trimmed_string = (&buffer[1..buffer.len() - 1]).to_string(); 
                            Some(Token::StringLiteral(trimmed_string))
                        },
                        ConsumptionType::Single => {
                            None // TODO
                        },
                        ConsumptionType::None => None,
                    }
                }
            }
        }
    }
}

pub fn tokenize<I>(chars: I) -> TokenIter<I>
    where I: Iterator<Item=char>
{
    TokenIter {
        chars: chars.peekable(),
        state: TokenizerState::Neutral,
    }
}

