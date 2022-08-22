use std::mem;
use std::io::{Read, Bytes};
use std::iter::Peekable;

// ANCHOR: token
#[derive(Debug, PartialEq)]
pub enum Token {
    // keywords
    And,    Break,  Do,     Else,   Elseif, End,
    False,  For,    Function, Goto, If,     In,
    Local,  Nil,    Not,    Or,     Repeat, Return,
    Then,   True,   Until,  While,

 // +       -       *       /       %       ^       #
    Add,    Sub,    Mul,    Div,    Mod,    Pow,    Len,
 // &       ~       |       <<      >>      //
    BitAnd, BitXor, BitOr,  ShiftL, ShiftR, Idiv,
 // ==       ~=     <=      >=      <       >        =
    Equal,  NotEq,  LesEq,  GreEq,  Less,   Greater, Assign,
 // (       )       {       }       [       ]       ::
    ParL,   ParR,   CurlyL, CurlyR, SqurL,  SqurR,  DoubColon,
 // ;               :       ,       .       ..      ...
    SemiColon,      Colon,  Comma,  Dot,    Concat, Dots,

    // constant values
    Integer(i64),
    Float(f64),
    String(Vec<u8>),

    // name of variables or table keys
    Name(String),

    // end
    Eos,
}
// ANCHOR_END: token

#[derive(Debug)]
// ANCHOR: lex
pub struct Lex<R: Read> {
    input: Peekable::<Bytes::<R>>,
    ahead: Token,
}
// ANCHOR_END: lex

impl<R: Read> Lex<R> {
    pub fn new(input: R) -> Self {
        Lex {
            input: input.bytes().peekable(),
            ahead: Token::Eos,
        }
    }

// ANCHOR: peek_next
    pub fn next(&mut self) -> Token {
        if self.ahead == Token::Eos {
            self.do_next()
        } else {
            mem::replace(&mut self.ahead, Token::Eos)
        }
    }

    pub fn peek(&mut self) -> &Token {
        if self.ahead == Token::Eos {
            self.ahead = self.do_next();
        }
        &self.ahead
    }
// ANCHOR_END: peek_next

    fn do_next(&mut self) -> Token {
        let byt = self.next_byte();
        match byt {
            b'\n' | b'\r' | b'\t' | b' ' => self.do_next(),
            b'+' => Token::Add,
            b'*' => Token::Mul,
            b'%' => Token::Mod,
            b'^' => Token::Pow,
            b'#' => Token::Len,
            b'&' => Token::BitAnd,
            b'|' => Token::BitOr,
            b'(' => Token::ParL,
            b')' => Token::ParR,
            b'{' => Token::CurlyL,
            b'}' => Token::CurlyR,
            b'[' => Token::SqurL,
            b']' => Token::SqurR,
            b';' => Token::SemiColon,
            b',' => Token::Comma,
            b'/' => self.check_ahead(b'/', Token::Idiv, Token::Div),
            b'=' => self.check_ahead(b'=', Token::Equal, Token::Assign),
            b'~' => self.check_ahead(b'=', Token::NotEq, Token::BitXor),
            b':' => self.check_ahead(b':', Token::DoubColon, Token::Colon),
            b'<' => self.check_ahead2(b'=', Token::LesEq, b'<', Token::ShiftL, Token::Less),
            b'>' => self.check_ahead2(b'=', Token::GreEq, b'>', Token::ShiftR, Token::Greater),
            b'\'' | b'"' => self.read_string(byt),
            b'.' => match self.peek_byte() {
                b'.' => {
                    self.next_byte();
                    if self.peek_byte() == b'.' {
                        self.next_byte();
                        Token::Dots
                    } else {
                        Token::Concat
                    }
                },
                b'0'..=b'9' => {
                    self.read_number_fraction(0)
                },
                _ => {
                    Token::Dot
                },
            },
            b'-' => {
                if self.peek_byte() == b'-' {
                    self.next_byte();
                    self.read_comment();
                    self.do_next()
                } else {
                    Token::Sub
                }
            },
            b'0'..=b'9' => self.read_number(byt),
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => self.read_name(byt),
            b'\0' => Token::Eos, // TODO
            _ => panic!("invalid char {byt}"),
        }
    }

    fn peek_byte(&mut self) -> u8 {
        match self.input.peek() {
            Some(Ok(byt)) => *byt,
            Some(_) => panic!("lex peek error"),
            None => b'\0',
        }
    }
    fn next_byte(&mut self) -> u8 {
        match self.input.next() {
            Some(Ok(byt)) => byt,
            Some(_) => panic!("lex read error"),
            None => b'\0',
        }
    }

    fn check_ahead(&mut self, ahead: u8, long: Token, short: Token) -> Token {
        if self.peek_byte() == ahead {
            self.next_byte();
            long
        } else {
            short
        }
    }
    fn check_ahead2(&mut self, ahead1: u8, long1: Token, ahead2: u8, long2: Token, short: Token) -> Token {
        let byt = self.peek_byte();
        if byt == ahead1 {
            self.next_byte();
            long1
        } else if byt == ahead2 {
            self.next_byte();
            long2
        } else {
            short
        }
    }

    fn read_number(&mut self, first: u8) -> Token {
        // heximal
        if first == b'0' {
            let second = self.peek_byte();
            if second == b'x' || second == b'X' {
                return self.read_heximal();
            }
        }

        // decimal
        let mut n = (first - b'0') as i64;
        loop {
            let byt = self.peek_byte();
            if let Some(d) = char::to_digit(byt as char, 10) {
                self.next_byte();
                n = n * 10 + d as i64;
            } else if byt == b'.' {
                return self.read_number_fraction(n);
            } else if byt == b'e' || byt == b'E' {
                return self.read_number_exp(n as f64);
            } else {
                break;
            }
        }

        // check following
        let fch = self.peek_byte();
        if (fch as char).is_alphabetic() || fch == b'.' {
            panic!("malformat number");
        }

        Token::Integer(n)
    }
    fn read_number_fraction(&mut self, i: i64) -> Token {
        self.next_byte(); // skip '.'

        let mut n: i64 = 0;
        let mut x: f64 = 1.0;
        loop {
            let byt = self.peek_byte();
            if let Some(d) = char::to_digit(byt as char, 10) {
                self.next_byte();
                n = n * 10 + d as i64;
                x *= 10.0;
            } else {
                break;
            }
        }
        Token::Float(i as f64 + n as f64 / x)
    }
    fn read_number_exp(&mut self, _: f64) -> Token {
        self.next_byte(); // skip 'e'
        todo!("lex number exp")
    }
    fn read_heximal(&mut self) -> Token {
        self.next_byte(); // skip 'x'
        todo!("lex heximal")
    }

    fn read_string(&mut self, quote: u8) -> Token {
        let mut s = Vec::new();
        loop {
            match self.next_byte() {
                b'\n' | b'\0' => panic!("unfinished string"),
                b'\\' => todo!("escape"),
                byt if byt == quote => break,
                byt => s.push(byt),
            }
        }
        Token::String(s)
    }

    fn read_name(&mut self, first: u8) -> Token {
        let mut s = String::new();
        s.push(first as char);

        loop {
            let ch = self.peek_byte() as char;
            if ch.is_alphanumeric() || ch == '_' {
                self.next_byte();
                s.push(ch);
            } else {
                break;
            }
        }

        match &s as &str { // TODO optimize by hash
            "and"      => Token::And,
            "break"    => Token::Break,
            "do"       => Token::Do,
            "else"     => Token::Else,
            "elseif"   => Token::Elseif,
            "end"      => Token::End,
            "false"    => Token::False,
            "for"      => Token::For,
            "function" => Token::Function,
            "goto"     => Token::Goto,
            "if"       => Token::If,
            "in"       => Token::In,
            "local"    => Token::Local,
            "nil"      => Token::Nil,
            "not"      => Token::Not,
            "or"       => Token::Or,
            "repeat"   => Token::Repeat,
            "return"   => Token::Return,
            "then"     => Token::Then,
            "true"     => Token::True,
            "until"    => Token::Until,
            "while"    => Token::While,
            _          => Token::Name(s),
        }
    }

    // '--' has been read
    fn read_comment(&mut self) {
        match self.next_byte() {
            b'[' => todo!("long comment"),
            _ => { // line comment
                loop {
                    let byt = self.next_byte();
                    if byt == b'\n' || byt == b'\0' {
                        break;
                    }
                }
            }
        }
    }
}
