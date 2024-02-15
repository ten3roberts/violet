use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;

use crate::text::CursorLocation;

#[derive(Default, Debug)]
pub struct EditorLine {
    text: String,
}

impl EditorLine {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn push(&mut self, c: char) {
        self.text.push(c);
    }

    pub fn insert(&mut self, idx: usize, c: char) {
        self.text.insert(idx, c);
    }

    pub fn remove(&mut self, idx: usize) {
        self.text.remove(idx);
    }

    pub fn drain(&mut self, range: std::ops::Range<usize>) {
        self.text.drain(range);
    }

    pub fn push_str(&mut self, s: &str) {
        self.text.push_str(s);
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }

    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    pub fn graphemes(&self) -> impl DoubleEndedIterator<Item = (usize, &str)> + '_ {
        self.text.grapheme_indices(true)
    }

    pub fn words(&self) -> impl DoubleEndedIterator<Item = (usize, &str)> + '_ {
        self.text.unicode_word_indices()
    }

    pub fn last_grapheme(&self) -> usize {
        self.graphemes().last().map(|(i, _)| i).unwrap_or(0)
    }

    pub fn find_grapheme(&self, start: usize) -> Option<(usize, &str)> {
        self.graphemes().find(|(i, _)| *i == start)
    }

    pub fn text(&self) -> &str {
        self.text.as_ref()
    }
}

/// Text editor driver
pub struct TextEditor {
    text: Vec<EditorLine>,
    /// The current cursor position
    ///
    cursor: CursorLocation,
}

/// Movement action for the cursor
pub enum CursorMove {
    Up,
    Down,
    Left,
    Right,
    ForwardWord,
    BackwardWord,
    SetPosition(CursorLocation),
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
            text: vec![EditorLine::default()],
        }
    }

    pub fn move_cursor(&mut self, direction: CursorMove) {
        match direction {
            CursorMove::Up => {
                self.cursor.row = self.cursor.row.saturating_sub(1);
            }
            CursorMove::Down => {
                self.cursor.row = (self.cursor.row + 1).min(self.text.len() - 1);
            }
            CursorMove::Left => {
                if let Some((i, _)) = self
                    .line()
                    .graphemes()
                    .take_while(|(i, _)| *i < self.cursor.col)
                    .last()
                {
                    self.cursor.col = i;
                } else if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                    self.cursor.col = self.line().len();
                }
            }
            CursorMove::Right => {
                let next_glyph = self.line().graphemes().find(|(i, _)| *i == self.cursor.col);

                if let Some((i, g)) = next_glyph {
                    self.cursor.col = i + g.len();
                } else if self.cursor.row < self.text.len() - 1 {
                    self.cursor.row += 1;
                    self.cursor.col = 0;
                }
            }
            CursorMove::ForwardWord => {
                let word = self
                    .line()
                    .words()
                    .find_or_last(|(i, _)| *i >= self.cursor.col);
                tracing::info!(?word, "current word");
                if let Some((i, word)) = word {
                    self.cursor.col = i + word.len();
                }
            }
            CursorMove::BackwardWord => {
                if self.cursor.col > 0 {
                    let word = self
                        .line()
                        .words()
                        .rev()
                        .find(|(i, _)| *i < self.cursor.col);
                    tracing::info!(?word, "current word");
                    if let Some((i, _)) = word {
                        self.cursor.col = i;
                    }
                } else if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                    self.cursor.col = self.line().len();
                }
            }
            CursorMove::SetPosition(pos) => {
                if (pos.row > self.text.len() - 1) || (pos.col > self.text[pos.row].len()) {
                    tracing::error!(?pos, "invalid cursor position");
                    return;
                }

                self.cursor = pos;
            }
        }
    }

    pub fn edit(&mut self, action: EditAction) {
        if !self.past_eol() {
            assert!(
                self.line().find_grapheme(self.cursor.col).is_some(),
                "expected cursor to be on a grapheme"
            );
        }
        match action {
            EditAction::InsertChar(c) => {
                let col = self.insert_column();
                let line = &mut self.text[self.cursor.row];
                line.insert(col, c);
                self.cursor.col += c.len_utf8();
            }
            EditAction::DeleteBackwardChar => {
                if self.cursor.col > 0 {
                    let col = self.cursor.col;
                    let current_grapheme =
                        find_before(self.line().graphemes(), col).map(|(i, v)| (i, v.len()));

                    if let Some((i, l)) = current_grapheme {
                        let line = &mut self.text[self.cursor.row];
                        tracing::info!("deleting grapheme at {}..{}", i, i + l);
                        line.drain(i..(i + l));
                        self.cursor.col -= l;
                    }
                } else if self.cursor.row > 0 {
                    let line = self.text.remove(self.cursor.row);
                    self.cursor.row -= 1;
                    self.cursor.col = self.text[self.cursor.row].len();
                    self.text[self.cursor.row].push_str(&line.text);
                }
            }
            EditAction::DeleteBackwardWord => {
                let line = &mut self.text[self.cursor.row];
                if self.cursor.col > 0 {
                    let graphemes = line.graphemes().peekable();
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

                    prev[prev.len() - 1].push_str(&cur[0].text);
                    self.text.remove(self.cursor.row);
                    self.cursor.row -= 1;
                    self.cursor.col = last_word_end;
                }
            }
            EditAction::InsertLine => {
                let col = self.insert_column();
                let line = &mut self.text[self.cursor.row];
                let new_line = line.text.split_off(col);

                self.text
                    .insert(self.cursor.row + 1, EditorLine::new(new_line));

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

    fn line(&self) -> &EditorLine {
        &self.text[self.cursor.row.min(self.text.len() - 1)]
    }

    pub fn lines(&self) -> &[EditorLine] {
        self.text.as_ref()
    }

    pub fn set_text(&mut self, text: impl IntoIterator<Item = String>) {
        self.text.clear();
        self.text.extend(text.into_iter().map(EditorLine::new));

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

    pub fn cursor(&self) -> CursorLocation {
        self.cursor
    }

    pub fn past_eol(&self) -> bool {
        self.cursor.col >= self.text[self.cursor.row].len()
    }

    fn insert_column(&self) -> usize {
        self.cursor.col.min(self.line().len())
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

fn find_before<T>(
    iter: impl DoubleEndedIterator<Item = (usize, T)>,
    col: usize,
) -> Option<(usize, T)> {
    iter.rev().find(|(i, _)| *i < col)
}
