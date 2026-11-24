//! A list longer than the screen shows: which row is selected and which row
//! is at the top of the view. The folder browser and the repair view both
//! use it, so a list scrolls the same way everywhere: the arrow keys move
//! the selection and scroll only when it would leave the view; the wheel and
//! the scroll bar move the view and leave the selection alone.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollList {
    /// How many rows the screen shows at once.
    visible_rows: usize,
    len: usize,
    selected: usize,
    /// The row at the top of the view.
    top: usize,
}

impl ScrollList {
    #[must_use]
    pub fn new(visible_rows: usize) -> ScrollList {
        ScrollList {
            visible_rows,
            len: 0,
            selected: 0,
            top: 0,
        }
    }

    #[must_use]
    pub fn visible_rows(&self) -> usize {
        self.visible_rows
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn selected(&self) -> usize {
        self.selected
    }

    #[must_use]
    pub fn top(&self) -> usize {
        self.top
    }

    /// A new list of `len` rows: the selection and the view go to the top.
    pub fn reset(&mut self, len: usize) {
        self.len = len;
        self.selected = 0;
        self.top = 0;
    }

    /// The rows changed (one was removed, say) but it is the same list: the
    /// selection stays on its row number, moved up only if the list is now
    /// shorter, and the view follows it.
    pub fn resize(&mut self, len: usize) {
        self.len = len;
        self.top = self.top.min(self.len.saturating_sub(self.visible_rows));
        self.select(self.selected);
    }

    pub fn up(&mut self) {
        self.select(self.selected.saturating_sub(1));
    }

    pub fn down(&mut self) {
        self.select(self.selected + 1);
    }

    pub fn page_up(&mut self) {
        self.select(self.selected.saturating_sub(self.visible_rows));
    }

    pub fn page_down(&mut self) {
        self.select(self.selected + self.visible_rows);
    }

    pub fn first(&mut self) {
        self.select(0);
    }

    pub fn last(&mut self) {
        self.select(self.len.saturating_sub(1));
    }

    /// Selects a row (the last one if `index` is past the end) and scrolls
    /// just far enough to show it.
    pub fn select(&mut self, index: usize) {
        if self.len == 0 {
            self.selected = 0;
            self.top = 0;
            return;
        }
        self.selected = index.min(self.len - 1);
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + self.visible_rows {
            self.top = self.selected + 1 - self.visible_rows;
        }
    }

    /// Scrolls the view by `rows` (negative: toward the top) and leaves the
    /// selection where it is.
    pub fn scroll_by(&mut self, rows: isize) {
        let last_top: usize = self.len.saturating_sub(self.visible_rows);
        self.top = self.top.saturating_add_signed(rows).min(last_top);
    }

    /// One page of the view, up or down, as a click on the scroll bar asks.
    pub fn scroll_page(&mut self, up: bool) {
        let page: isize = isize::try_from(self.visible_rows).unwrap_or(isize::MAX);
        self.scroll_by(if up { -page } else { page });
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollList;

    fn list(len: usize) -> ScrollList {
        let mut list: ScrollList = ScrollList::new(5);
        list.reset(len);
        list
    }

    #[test]
    fn the_view_scrolls_only_when_the_selection_leaves_it() {
        let mut list: ScrollList = list(20);
        for _ in 0..4 {
            list.down();
        }
        assert_eq!((list.selected(), list.top()), (4, 0));
        list.down();
        assert_eq!((list.selected(), list.top()), (5, 1));
        list.up();
        list.up();
        assert_eq!(list.top(), 1, "going back up does not move the view");
    }

    #[test]
    fn the_wheel_moves_the_view_not_the_selection() {
        let mut list: ScrollList = list(20);
        list.scroll_by(7);
        assert_eq!((list.selected(), list.top()), (0, 7));
        list.scroll_by(100);
        assert_eq!(list.top(), 15);
        list.scroll_page(true);
        assert_eq!(list.top(), 10);
    }

    #[test]
    fn the_selection_stays_inside_a_short_list_and_an_empty_one_is_safe() {
        let mut short: ScrollList = list(3);
        short.page_down();
        assert_eq!(short.selected(), 2);
        short.first();
        short.up();
        assert_eq!(short.selected(), 0);
        let mut empty: ScrollList = list(0);
        empty.down();
        empty.last();
        empty.scroll_by(5);
        assert_eq!((empty.selected(), empty.top()), (0, 0));
    }

    #[test]
    fn resizing_keeps_the_selection_near_where_it_was() {
        let mut list: ScrollList = list(20);
        list.select(12);
        list.resize(13);
        assert_eq!(list.selected(), 12);
        list.resize(10);
        assert_eq!(list.selected(), 9);
        assert!(list.top() <= list.selected());
    }
}
