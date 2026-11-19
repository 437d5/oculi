//! Independent time cursors with stable identities and one active selection.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CursorId(pub(crate) u64);

impl CursorId {
    pub fn name(self) -> String {
        let mut index = self.0;
        let mut letters = Vec::new();
        loop {
            letters.push(char::from(b'A' + (index % 26) as u8));
            if index < 26 {
                break;
            }
            index = index / 26 - 1;
        }
        letters.into_iter().rev().collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub id: CursorId,
    pub time: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cursors {
    items: Vec<Cursor>,
    active: Option<CursorId>,
    next_id: u64,
}

impl Cursors {
    pub fn all(&self) -> &[Cursor] {
        &self.items
    }

    pub fn active(&self) -> Option<&Cursor> {
        self.items
            .iter()
            .find(|cursor| Some(cursor.id) == self.active)
    }

    pub fn add(&mut self, time: u64) -> CursorId {
        let id = CursorId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("cursor identifiers exhausted");
        self.items.push(Cursor { id, time });
        self.active = Some(id);
        id
    }

    pub fn place(&mut self, time: u64) -> CursorId {
        if let Some(cursor) = self
            .items
            .iter_mut()
            .find(|cursor| Some(cursor.id) == self.active)
        {
            cursor.time = time;
            cursor.id
        } else {
            self.add(time)
        }
    }

    pub fn select(&mut self, id: CursorId) -> bool {
        if self.items.iter().any(|cursor| cursor.id == id) {
            self.active = Some(id);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: CursorId) -> bool {
        let Some(index) = self.items.iter().position(|cursor| cursor.id == id) else {
            return false;
        };
        self.items.remove(index);
        if self.active == Some(id) {
            self.active = self
                .items
                .get(index.saturating_sub(1))
                .map(|cursor| cursor.id);
        }
        true
    }
}
