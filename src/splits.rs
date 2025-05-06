use std::num::NonZeroU32;

use crate::{
    data::Data,
    game::{Game, Monster, SplitOn},
};

#[derive(Copy, Clone, Debug)]
pub enum Action {
    Split(SplitOn),
}

pub struct Progress {
    game: Game,
    chaos: Delayed,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            game: Game::new(),
            chaos: Delayed::default(),
        }
    }

    pub fn running(&mut self, data: &mut Data<'_>, early: bool) -> Option<Action> {
        if let Some(action) = self.chaos.tick() {
            return Some(action);
        }

        if let Some(event) = self.game.running(data, early) {
            if matches!(event, SplitOn::Monster(Monster::Chaos)) == false {
                return Some(Action::Split(event));
            }
            self.chaos.set(60, Action::Split(event));
        }

        return None;
    }
}

#[derive(Debug, Clone, Default)]
struct Delayed {
    delay: Option<NonZeroU32>,
    action: Option<Action>,
}

impl Delayed {
    fn set(&mut self, amount: u32, action: Action) {
        self.delay = NonZeroU32::new(amount);
        self.action = Some(action);
    }

    fn tick(&mut self) -> Option<Action> {
        let n = self.delay?;
        self.delay = NonZeroU32::new(n.get() - 1);
        self.delay
            .is_none()
            .then(|| self.action.take().expect("double tick"))
    }
}
