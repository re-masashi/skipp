use crate::lexer::tokens::TokenType;
use crate::parser::{Class, Function, NodePosition, Parser};
use crate::{unwrap_some, Result};

use std::collections::HashMap;

impl Parser {
    pub fn parse_class(&mut self) -> Result<(Class, NodePosition)> {
        let mut fns: Vec<(Function, NodePosition)> = Vec::new();

        // println!("{:#?}", self.tokens.peek());

        self.advance();
        let nx = unwrap_some!(self.tokens.next()); // Eat class
        let start = NodePosition {
            pos: nx.pos,
            line_no: nx.line_no,
            file: nx.file.to_string(),
        };
        // println!("{:#?}", self.tokens.peek());

        let name = match &unwrap_some!(self.tokens.peek()).type_ {
            TokenType::Identifier(i) => i.clone(),
            _ => return Err("Syntax Error: expected Identifier after keyword 'class'".to_string()),
        };
        self.advance();
        self.tokens.next(); // eat the identifier

        self.advance();
        match unwrap_some!(self.tokens.next()).type_ {
            TokenType::LBrace => {}
            _ => return Err("Expected '{' in class".to_string()),
        }

        while unwrap_some!(self.tokens.peek()).type_ != TokenType::RBrace {
            // println!("{:#?}", self.tokens.peek());
            match unwrap_some!(self.tokens.peek()).type_ {
                TokenType::Def => {}
                _ => return Err(self.parser_error("SyntaxError: expected Function")),
            }
            match self.parse_function() {
                Ok((f, p)) => fns.insert(fns.len(), (f, p)),
                Err(e) => {
                    println!("oops");
                    return Err(e);
                }
            }
        }
        self.advance();
        self.tokens.next(); // eat '}'
        Ok((Class { name, fns }, start))
    }

    pub fn parse_struct(
        &mut self,
    ) -> Result<((String, HashMap<String, (String, i32)>), NodePosition)> {
        let mut members: HashMap<String, (String, i32)> = HashMap::new();

        // println!("{:#?}", self.tokens.peek());

        self.advance();
        let nx = unwrap_some!(self.tokens.next()); // Eat struct
        let start = NodePosition {
            pos: nx.pos,
            line_no: nx.line_no,
            file: nx.file.to_string(),
        };

        let name = match &unwrap_some!(self.tokens.peek()).type_ {
            TokenType::Identifier(i) => i.clone(),
            _ => return Err("Syntax Error: expected Identifier after keyword 'struct'".to_string()),
        };
        self.advance();
        self.tokens.next(); // eat the identifier

        self.advance();
        match unwrap_some!(self.tokens.next()).type_ {
            TokenType::LBrace => {}
            _ => return Err("Expected '{' in struct".to_string()),
        }

        let mut index = 0;

        while unwrap_some!(self.tokens.peek()).type_ != TokenType::RBrace {
            // println!("{:#?}", self.tokens.peek());
            let mut name = "".to_string();
            match &unwrap_some!(self.tokens.peek()).type_ {
                TokenType::Identifier(n) => {
                    name = n.clone();
                    self.advance();
                    self.tokens.next();
                }
                _ => return Err(self.parser_error("SyntaxError: expected Identifier")),
            }
            self.advance();
            if let TokenType::Colon = unwrap_some!(self.tokens.next()).type_ {
            } else {
                return Err(self.parser_error("SyntaxError: expected colon"));
            }

            self.advance();
            if let TokenType::Identifier(type_) = unwrap_some!(self.tokens.next()).type_ {
                members.insert(name.clone(), (type_, index));
            } else {
                return Err(self.parser_error("SyntaxError: expected type"));
            }
            index += 1;
        }
        self.advance();
        self.tokens.next(); // eat '}'
        Ok(((name, members), start))
    }
}
