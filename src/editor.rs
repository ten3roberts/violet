use unicode_segmentation::UnicodeSegmentation;

pub struct CursorLocation {
    pub row: usize,
    pub col: usize,
}

pub struct TextEditor {
    text: Vec<String>,
    // The current cursor position
    cursor: CursorLocation,
}

/// Movement action for the cursor
pub enum CursorMove {
    Up,
    Down,
    Left,
    Right,
}

pub enum EditAction {
    InsertChar(char),
    DeleteBackwardChar,
    DeleteBackwardWord,
    InsertLine,
    DeleteLine,
}

pub enum EditorAction {
    CursorMove(CursorMove),
    Edit(EditAction),
    SetText(Vec<String>),
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            cursor: CursorLocation { row: 0, col: 0 },
            text: vec![String::new()],
        }
    }

    pub fn move_cursor(&mut self, direction: CursorMove) {
        let line = self.line();

        match direction {
            CursorMove::Up => self.cursor.row = self.cursor.row.saturating_sub(1),
            CursorMove::Down => self.cursor.row = (self.cursor.row + 1).min(self.text.len() - 1),
            CursorMove::Left => {
                if self.cursor.col > 0 {
                    for (i, g) in line.graphemes(true).enumerate() {
                        if i == self.cursor.col {
                            self.cursor.col -= g.len();
                            break;
                        }
                    }
                } else {
                    self.cursor.row = self.cursor.row.saturating_sub(1);
                    self.cursor.col = self.text[self.cursor.row].len();
                }
            }
            CursorMove::Right => {
                if self.cursor.col < line.len() {
                    for (i, g) in line.graphemes(true).enumerate() {
                        if i == self.cursor.col {
                            self.cursor.col += g.len();
                            break;
                        }
                    }
                } else {
                    self.cursor.row = (self.cursor.row + 1).min(self.text.len() - 1);
                }
            }
            CursorMove::Right => todo!(),
        }
    }

    pub fn edit(&mut self, action: EditAction) {
        match action {
            EditAction::InsertChar(c) => {
                let line = &mut self.text[self.cursor.row];
                line.insert(self.cursor.col, c);
                self.cursor.col += 1;
            }
            EditAction::DeleteBackwardChar => {
                if self.cursor.col > 0 {
                    let line = &mut self.text[self.cursor.row];
                    if self.cursor.col == line.len() {
                        line.pop();
                        self.cursor.col = self.cursor.col.saturating_sub(1);
                    } else {
                        line.remove(self.cursor.col);
                    }
                } else if self.cursor.row > 0 {
                    self.text.remove(self.cursor.row);
                    self.cursor.row -= 1;
                    self.cursor.col = self.text[self.cursor.row].len();
                }
            }
            EditAction::DeleteBackwardWord => {
                let line = &mut self.text[self.cursor.row];
                if self.cursor.col > 0 {
                    let graphemes = line.grapheme_indices(true).peekable();
                    let mut word_begin = 0;
                    let mut in_word = false;
                    for (i, g) in graphemes {
                        if i >= self.cursor.col {
                            break;
                        }
                        if !g.chars().all(char::is_whitespace) {
                            if !in_word {
                                word_begin = i;
                            }
                            in_word = true;
                        } else {
                            in_word = false;
                        }
                    }
                    line.drain(word_begin..self.cursor.col);
                    self.cursor.col = word_begin;
                } else if self.cursor.row > 0 {
                    let last_word_end = self.text[self.cursor.row - 1].len();

                    let (prev, cur) = self.text.split_at_mut(self.cursor.row);

                    prev[prev.len() - 1].push_str(&cur[0]);
                    self.text.remove(self.cursor.row);
                    self.cursor.row -= 1;
                    self.cursor.col = last_word_end;
                }
            }
            EditAction::InsertLine => {
                self.text.insert(self.cursor.row + 1, String::new());
                self.cursor.row += 1;
                self.cursor.col = 0;
            }
            EditAction::DeleteLine => {
                if self.cursor.row == 0 && self.text.len() == 1 {
                    self.text[0].clear();
                    self.cursor.col = 0;
                } else {
                    self.text.remove(self.cursor.row);
                    self.cursor.col = self.cursor.col.min(self.text[self.cursor.row].len())
                }
            }
        }
    }

    pub fn apply_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::CursorMove(m) => self.move_cursor(m),
            EditorAction::Edit(e) => self.edit(e),
            EditorAction::SetText(v) => self.set_text(v),
        }
    }

    fn line(&self) -> &str {
        &self.text[self.cursor.row.min(self.text.len() - 1)]
    }

    pub fn text(&self) -> &[String] {
        self.text.as_ref()
    }

    pub fn set_text(&mut self, text: impl IntoIterator<Item = String>) {
        self.text.clear();
        self.text.extend(text);

        self.cursor.row = self.cursor.row.min(self.text.len() - 1);
        self.cursor.col = self.cursor.col.min(self.text[self.cursor.row].len());
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.text.len() - 1);
        self.cursor.col = col.min(self.text[self.cursor.row].len());
    }

    pub fn set_cursor_at_end(&mut self) {
        self.cursor.row = self.text.len() - 1;
        self.cursor.col = self.text[self.cursor.row].len();
    }
}
