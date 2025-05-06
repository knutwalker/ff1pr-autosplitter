use crate::data::{BattleResult, Data};
use asr::watcher::Watcher;
use num_enum::{IntoPrimitive, TryFromPrimitive};

pub struct Game {
    in_battle: Watcher<bool>,
    battle_playing: Watcher<bool>,
    items: ahash::AHashSet<Pickup>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            in_battle: Watcher::new(),
            battle_playing: Watcher::new(),
            items: ahash::AHashSet::new(),
        }
    }

    pub fn running(&mut self, data: &mut Data<'_>, early: bool) -> Option<SplitOn> {
        if let Some(mon) = self.battle_check(data, early) {
            return Some(SplitOn::Monster(mon));
        }

        if let Some(item) = self.inventory_check(data) {
            return Some(SplitOn::Pickup(item));
        }

        return None;
    }

    // TODO
    // [ ] check active only, load info only in battle
    // [ ] move to class iinstead of class2
    // [ ] move to default asr, latest

    fn battle_check(&mut self, data: &mut Data<'_>, early: bool) -> Option<Monster> {
        let in_battle = self.in_battle.update_infallible(data.battle_active());
        if in_battle.current == false && in_battle.unchanged() {
            return None;
        }

        let info = data.battle_info()?;

        let playing = self.battle_playing.update_infallible(info.playing);

        if let Ok(mon) = Monster::try_from(info.encounter_id) {
            if in_battle.changed_to(&true) {
                log!("Battle started against {mon:?}, encounter: {info:?}");
            } else if in_battle.changed_to(&false) {
                if playing.changed_to(&false) {
                    log!("Battle reset detected, no split!");
                    return None;
                }

                log!("Battle ended against {mon:?}, encounter: {info:?}");
                if !early {
                    if mon != Monster::Chaos {
                        return Some(mon);
                    }
                }
            } else if in_battle.current {
                if playing.changed_to(&false) {
                    log!("Battle done: {:?}", info.result);
                    if info.result == BattleResult::Win {
                        if mon == Monster::Chaos {
                            return Some(mon);
                        }

                        if early {
                            return Some(mon);
                        }
                    }
                }
            }
        }

        return None;
    }

    fn inventory_check(&mut self, data: &mut Data<'_>) -> Option<Pickup> {
        if let Some(inventory) = data.inventory() {
            for item in inventory
                .key_items
                .iter()
                .chain(inventory.vehicles.iter())
                .copied()
                .filter_map(|i| Pickup::try_from(i).ok())
            {
                if self.items.insert(item) {
                    log!("Picked up the {item:?}");
                    return Some(item);
                }
            }
        }

        return None;
    }
}
