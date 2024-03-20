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

/// The core text editor buffer
pub struct TextEditor {
    text: Vec<EditorLine>,
    /// The current cursor position
    ///
    cursor: CursorLocation,
    selection: Option<CursorLocation>,
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

pub enum EditAction<S = String> {
    InsertText(S),
    DeleteBackwardChar,
    DeleteBackwardWord,
    InsertLine,
    DeleteLine,
}

pub enum EditorAction<S = String> {
    CursorMove(CursorMove),
    SelectionMove(CursorMove),
    SelectionClear,
    Edit(EditAction<S>),
    SetText(Vec<S>),
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            cursor: CursorLocation { row: 0, col: 0 },
            text: vec![EditorLine::default()],
            selection: None,
        }
    }

    pub fn move_cursor(&mut self, m: CursorMove) {
        self.cursor = self.get_new_cursor(m, self.cursor)
    }

    pub fn move_selection(&mut self, m: CursorMove) {
        self.selection = Some(self.get_new_cursor(m, self.selection.unwrap_or(self.cursor)))
    }

    fn get_new_cursor(&self, m: CursorMove, cursor: CursorLocation) -> CursorLocation {
        match m {
            CursorMove::Up => CursorLocation {
                row: cursor.row.saturating_sub(1),
                col: cursor.col,
            },
            CursorMove::Down => CursorLocation {
                row: (cursor.row + 1).min(self.text.len() - 1),
                col: cursor.col,
            },
            CursorMove::Left => {
                if let Some((i, _)) = self
                    .line()
                    .graphemes()
                    .take_while(|(i, _)| *i < self.cursor.col)
                    .last()
                {
                    CursorLocation {
                        row: cursor.row,
                        col: i,
                    }
                } else if self.cursor.row > 0 {
                    CursorLocation {
                        row: cursor.row - 1,
                        col: self.line().len(),
                    }
                } else {
                    cursor
                }
            }
            CursorMove::Right => {
                let next_glyph = self.line().graphemes().find(|(i, _)| *i == self.cursor.col);

                if let Some((i, g)) = next_glyph {
                    CursorLocation {
                        row: cursor.row,
                        col: i + g.len(),
                    }
                } else if self.cursor.row < self.text.len() - 1 {
                    CursorLocation {
                        row: cursor.row + 1,
                        col: 0,
                    }
                } else {
                    cursor
                }
            }
            CursorMove::ForwardWord => {
                let word = self
                    .line()
                    .words()
                    .find_or_last(|(i, _)| *i >= self.cursor.col);
                tracing::info!(?word, "current word");
                if let Some((i, word)) = word {
                    CursorLocation {
                        row: cursor.row,
                        col: i + word.len(),
                    }
                } else {
                    cursor
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
                        CursorLocation {
                            row: cursor.row,
                            col: i,
                        }
                    } else {
                        cursor
                    }
                } else if self.cursor.row > 0 {
                    CursorLocation {
                        row: cursor.row - 1,
                        col: self.line().len(),
                    }
                } else {
                    cursor
                }
            }
            CursorMove::SetPosition(pos) => {
                if (pos.row > self.text.len() - 1) || (pos.col > self.text[pos.row].len()) {
                    tracing::error!(?pos, "invalid cursor position");
                    cursor
                } else {
                    pos
                }
            }
        }
    }

    pub fn edit<S: AsRef<str>>(&mut self, action: EditAction<S>) {
        if !self.past_eol() {
            assert!(
                self.line().find_grapheme(self.cursor.col).is_some(),
                "expected cursor to be on a grapheme"
            );
        }

        match action {
            EditAction::InsertText(text) => {
                self.delete_selected_text();
                let mut insert_lines = text.as_ref().lines();

                if let Some(text) = insert_lines.next() {
                    let col = self.insert_column();
                    let line = &mut self.text[self.cursor.row];
                    line.text.insert_str(col, text);
                    self.cursor.col += text.graphemes(true).count();
                }

                for text in insert_lines {
                    let current_line = &mut self.text[self.cursor.row];
                    let mut next_line = current_line.text.split_off(self.cursor.col);
                    next_line.insert_str(0, text);
                    self.text
                        .insert(self.cursor.row + 1, EditorLine::new(next_line));
                    self.cursor.row += 1;
                    self.cursor.col = text.graphemes(true).count();
                }
            }
            EditAction::DeleteBackwardChar => {
                if self.delete_selected_text() {
                    return;
                }
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
                if self.delete_selected_text() {
                    return;
                }
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
                self.delete_selected_text();
                let col = self.insert_column();
                let line = &mut self.text[self.cursor.row];
                let new_line = line.text.split_off(col);

                self.text
                    .insert(self.cursor.row + 1, EditorLine::new(new_line));

                self.cursor.row += 1;
                self.cursor.col = 0;
            }
            EditAction::DeleteLine => {
                if self.delete_selected_text() {
                    return;
                }
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

    pub fn apply_action<S: AsRef<str>>(&mut self, action: EditorAction<S>) {
        match action {
            EditorAction::CursorMove(m) => self.move_cursor(m),
            EditorAction::Edit(e) => self.edit(e),
            EditorAction::SetText(v) => self.set_text(v.iter().map(|v| v.as_ref())),
            EditorAction::SelectionMove(m) => self.move_selection(m),
            EditorAction::SelectionClear => self.clear_selection(),
        }
    }

    fn line(&self) -> &EditorLine {
        &self.text[self.cursor.row.min(self.text.len() - 1)]
    }

    pub fn lines(&self) -> &[EditorLine] {
        self.text.as_ref()
    }

    pub fn lines_str(&self) -> impl Iterator<Item = &str> {
        self.text.iter().map(|l| l.text.as_str())
    }

    pub fn set_text<'a>(&mut self, text: impl IntoIterator<Item = &'a str>) {
        let at_end_col = self.cursor.col >= self.text[self.cursor.row].len();
        let at_end_row = self.cursor.row >= self.text.len() - 1;

        self.text.clear();
        self.text.extend(text.into_iter().map(EditorLine::new));

        self.cursor.row = self.cursor.row.min(self.text.len() - 1);
        self.cursor.col = self.cursor.col.min(self.text[self.cursor.row].len());

        if at_end_row {
            self.cursor.row = self.text.len() - 1;
        }

        if at_end_col {
            self.cursor.col = self.text[self.cursor.row].len();
        }
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

    pub fn selection_bounds(&self) -> Option<(CursorLocation, CursorLocation)> {
        let sel = self.selection?;
        if sel < self.cursor {
            Some((sel, self.cursor))
        } else {
            Some((self.cursor, sel))
        }
    }

    pub fn selected_text(&self) -> Option<Vec<&str>> {
        let (start, end) = self.selection_bounds()?;

        let mut text = Vec::new();
        for (i, line) in self.text[start.row..=end.row].iter().enumerate() {
            let row = start.row + i;

            if row == start.row && row == end.row {
                text.push(&line.text[start.col..end.col]);
            } else if row == start.row {
                text.push(&line.text[start.col..]);
            } else if row == end.row {
                text.push(&line.text[..end.col]);
            } else {
                text.push(&line.text);
            }
        }

        Some(text)
    }

    pub fn delete_selected_text(&mut self) -> bool {
        let Some((start, end)) = self.selection_bounds() else {
            return false;
        };

        if start.row == end.row {
            self.text[start.row].text.drain(start.col..end.col);
        } else {
            self.text[start.row].text.truncate(start.col);
            self.text[end.row].text.drain(0..end.col);
            self.text.drain(start.row + 1..end.row);
        }

        self.cursor = start;
        self.clear_selection();

        true
    }

    pub fn set_selection(&mut self, sel: Option<CursorLocation>) {
        self.selection = sel;
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn selection(&self) -> Option<CursorLocation> {
        self.selection
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor() {
        let mut editor = TextEditor::new();
        editor.edit(EditAction::InsertText("This is some text"));
        assert_eq!(editor.lines_str().collect_vec(), &["This is some text"]);

        editor.move_cursor(CursorMove::BackwardWord);
        assert_eq!(editor.cursor().col, "This is some ".len());

        editor.edit(EditAction::InsertText("other "));
        assert_eq!(
            editor.lines_str().collect_vec(),
            &["This is some other text"]
        );

        editor.edit(EditAction::<String>::DeleteBackwardWord);

        assert_eq!(editor.lines_str().collect_vec(), &["This is some text"]);

        editor.edit(EditAction::InsertText(
            "other text,\nand a new line for the previous ",
        ));

        assert_eq!(
            editor.lines_str().collect_vec(),
            &[
                "This is some other text,",
                "and a new line for the previous text"
            ]
        );
    }
}
