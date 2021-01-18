
#[derive(Debug)]
pub struct SourceCodeUnit {
	name: String,
	content: String,
	line_offsets: Vec<usize>,
}

impl SourceCodeUnit {
	pub fn from_filename(filename: &str) -> SourceCodeUnit {
		let src = std::fs::read_to_string(filename)
			.expect(&format!("source file `{}` couldn't be read", filename));
		SourceCodeUnit::from_str(&src, filename.to_string())
	}

	pub fn from_str(s: &str, name: String) -> SourceCodeUnit {
		let line_offsets_iter = s.bytes()
			.enumerate()
			.filter_map(|(i, ch)|
				if ch as char == '\n' {
					Some(i+1)
				} else {
					None 
				});
		let mut line_offsets: Vec<usize> = Some(0usize).into_iter()
			.chain(line_offsets_iter)
			.collect();
		let mut content = s.to_string();
		if *line_offsets.last().unwrap() != content.len() {
			content += "\n";
			line_offsets.push(content.len());
			// If the content didn't end by a `\n`, then now it does.
		}
		SourceCodeUnit {
			name: name,
			content: content,
			line_offsets: line_offsets,
		}
	}
}


#[derive(Debug)]
pub enum ParsingError {
	EofInComment {loc: Loc},
	UnexpectedCharacter {ch: char, loc: Loc},
}

impl std::fmt::Display for ParsingError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ParsingError::EofInComment {loc} =>
				write!(f, "end-of-file in comment started at line {}",
					loc.line_start),
			ParsingError::UnexpectedCharacter {ch, loc} =>
				write!(f, "unexpected character `{}` at line {}",
					ch, loc.line_start),
		}
	}
}


use std::rc::Rc;

#[derive(Debug)]
pub struct ReadingHead {
	scu: Rc<SourceCodeUnit>,
	raw_index: usize,
	line: usize,
}

#[derive(Debug, Clone)]
pub struct Loc {
	scu: Rc<SourceCodeUnit>,
	line_start: usize,
	raw_index_start: usize,
	raw_length: usize,
}

impl ReadingHead {
	pub fn from_scu(scu: Rc<SourceCodeUnit>) -> ReadingHead {
		ReadingHead {
			scu: scu,
			raw_index: 0,
			line: 1,
		}
	}

	fn peek_cur_char(&self) -> Option<char> {
		self.scu.content[self.raw_index..].chars().next()
	}

	fn goto_next_char(&mut self) {
		if let Some(ch) = self.peek_cur_char() {
			self.raw_index += ch.len_utf8();
			match ch {
				'\n' => self.line += 1,
				_ => (),
			}
		}
	}

	fn cur_char_loc(&self) -> Loc {
		Loc {
			scu: Rc::clone(&self.scu),
			line_start: self.line,
			raw_index_start: self.raw_index,
			raw_length: match self.peek_cur_char() {
				Some(ch) => ch.len_utf8(),
				None => 0,
			},
		}
	}

	fn skip_ws(&mut self) -> Result<(), ParsingError> {
		let mut comment: Option<Loc> = None;
		loop {
			match (self.peek_cur_char(), &comment) {
				(Some('#'), None) =>
					comment = Some(self.cur_char_loc()),
				(Some(ch), None) if !ch.is_ascii_whitespace() =>
					break,
				(Some('#'), Some(_)) =>
					comment = None,
				(None, Some(comment_loc)) =>
					return Err(ParsingError::EofInComment {
						loc: comment_loc.clone()
					}),
				(None, None) =>
					break,
				_ => (),
			}
			self.goto_next_char();
		}
		Ok(())
	}
}

#[derive(Debug)]
pub enum Tok {
	Word(String),
	Integer(String),
	BinOp(String),
	Left(String),
	Right(String),
	Void,
}

impl Tok {
	pub fn is_void(&self) -> bool {
		match self {
			Tok::Void => true,
			_ => false,
		}
	}
}

impl ReadingHead {
	pub fn read_cur_tok(&mut self) -> Result<(Tok, Loc), ParsingError> {
		self.skip_ws()?;
		match self.peek_cur_char() {
			Some(ch) if ch.is_ascii_alphabetic() => {
				let (word, loc) = self.read_cur_word();
				Ok((Tok::Word(word), loc))
			},
			Some(ch) if ch.is_ascii_digit() => {
				let (integer, loc) = self.read_cur_integer();
				Ok((Tok::Integer(integer), loc))
			},
			Some(ch) if ch == '+' || ch == '-' || ch == '*' || ch == '/' => {
				self.goto_next_char();
				Ok((Tok::BinOp(ch.to_string()), self.cur_char_loc()))
			},
			Some(ch) if ch == '(' || ch == '[' || ch == '{' => {
				self.goto_next_char();
				Ok((Tok::Left(ch.to_string()), self.cur_char_loc()))
			},
			Some(ch) if ch == ')' || ch == ']' || ch == '}' => {
				self.goto_next_char();
				Ok((Tok::Right(ch.to_string()), self.cur_char_loc()))
			},
			Some(ch) => Err(ParsingError::UnexpectedCharacter {
				ch, loc: self.cur_char_loc(),
			}),
			None => Ok((Tok::Void, self.cur_char_loc())),
		}
	}

	fn read_cur_word(&mut self) -> (String, Loc) {
		let mut word_string = String::new();
		let mut loc = self.cur_char_loc();
		while let Some(ch) = self.peek_cur_char() {
			if !ch.is_ascii_alphabetic() {
				break;
			}
			word_string.push(ch);
			self.goto_next_char();
		}
		std::assert!(word_string.len() >= 1);
		loc.raw_length = word_string.bytes().len();
		(word_string, loc)
	}

	fn read_cur_integer(&mut self) -> (String, Loc) {
		let mut integer_string = String::new();
		let mut loc = self.cur_char_loc();
		while let Some(ch) = self.peek_cur_char() {
			if !ch.is_ascii_digit() {
				break;
			}
			integer_string.push(ch);
			self.goto_next_char();
		}
		std::assert!(integer_string.len() >= 1);
		loc.raw_length = integer_string.bytes().len();
		(integer_string, loc)
	}
}
