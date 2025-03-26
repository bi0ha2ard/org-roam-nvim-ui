#[derive(Default)]
pub struct History<T: Copy = usize> {
    current: Option<T>,
    history: Vec<T>,
    redo: Vec<T>,
}

impl<T: Copy> History<T> {
    pub fn push(&mut self, next: Option<T>) {
        if let Some(curr) = self.current {
            self.history.push(curr);
        }
        self.current = next;
        self.redo.clear();
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.history.is_empty() {
            return self.current;
        }
        if let Some(curr) = self.current {
            self.redo.push(curr);
        }
        self.current = self.history.pop();
        self.current
    }

    pub fn unpop(&mut self) -> Option<T> {
        if self.redo.is_empty() {
            return self.current;
        }
        if let Some(curr) = self.current {
            self.history.push(curr);
        }
        self.current = self.redo.pop();
        self.current
    }
}

#[cfg(test)]
mod tests {
    use crate::History;

    #[test]
    fn pushing() {
        let mut h = History::default();
        h.push(Some(1));
        h.push(Some(2));
        assert!(matches!(h.current, Some(2)));
        assert_eq!(h.history, [1]);
        assert!(h.redo.is_empty());

        let res = h.pop();
        assert!(matches!(res, Some(1)));
        assert!(matches!(h.current, Some(1)));
        assert!(h.history.is_empty());
        assert_eq!(h.redo, [2]);

        let res = h.unpop();
        assert!(matches!(res, Some(2)));
        assert!(matches!(h.current, Some(2)));
        assert_eq!(h.history, [1]);
        assert!(h.redo.is_empty());
    }
}
