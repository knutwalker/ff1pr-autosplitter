use crate::data::{BattleResult, Change, CurrentEncounter, Data, GameStart, Inventory};
pub use crate::data::{Enemy, KeyItem, Level};
use asr::{arrayvec::ArrayVec, watcher::Watcher};
use num_enum::{IntoPrimitive, TryFromPrimitive};

pub struct Game {
    in_battle: Watcher<bool>,
    battle_playing: Watcher<bool>,
    judgement: Watcher<bool>,
    // inventory: Watcher<Inventory>,
    items: ahash::AHashSet<Pickup>,
    // loading: Watcher<bool>,
    // cutscene: Watcher<bool>,
    // level: Watcher<Level>,
    // encounter: Option<ArrayVec<Enemy, 6>>,
    // events: ArrayVec<Event, 7>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    GameStarted,
    LoadStart,
    LoadEnd,
    #[allow(dead_code)]
    PauseStart,
    #[allow(dead_code)]
    PauseEnd,
    LevelChange {
        from: Level,
        to: Level,
    },
    CutsceneStart,
    CutsceneEnd,
    EncounterStart(Enemy),
    EncounterEnd(Enemy),
    EncountersStart(ArrayVec<Enemy, 6>),
    EncountersEnd(ArrayVec<Enemy, 6>),
    PickedUpKeyItem(KeyItem),
    LostKeyItem(KeyItem),
}

impl Game {
    pub fn new() -> Self {
        Self {
            in_battle: Watcher::new(),
            battle_playing: Watcher::new(),
            // inventory: Watcher::new(),
            judgement: Watcher::new(),
            items: ahash::AHashSet::new(),
            // loading: Watcher::new(),
            // cutscene: Watcher::new(),
            // level: Watcher::new(),
            // encounter: None,
            // events: ArrayVec::new(),
        }
    }

    // pub fn events(&mut self) -> impl Iterator<Item = Event> + '_ {
    //
    //     // self.events.drain(..)
    // }

    pub fn not_running(&mut self, data: &mut Data<'_>) {

        // self.start(data);
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

    fn battle_check(&mut self, data: &mut Data<'_>, early: bool) -> Option<Monster> {
        let info = data.battle_info()?;

        if self.in_battle.pair.is_none() {
            log!("Current Encounter: {:?}", info);
        }

        // TODO
        // [ ] check active only, load info only in battle
        // [ ] move to class iinstead of class2
        // [ ] move to default asr, latest

        let playing = self.battle_playing.update_infallible(info.playing);
        // let judgement = self.judgement.update_infallible(info.end_enabled);

        let in_battle = self.in_battle.update_infallible(data.battle_active());
        if in_battle.current == false && in_battle.unchanged() {
            return None;
        }

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

                // if judgement.changed_to(&true) {
                //     log!("Did we beat CHAOS just now?: {:?}", info);
                // }
            }
        }

        return None;
    }

    fn inventory_check(&mut self, data: &mut Data<'_>) -> Option<Pickup> {
        if let Some(inventory) = data.inventory() {
            for item in inventory
                .key_items
                .iter()
                .map(|i| i.item_id)
                .chain(inventory.vehicles.iter().map(|v| v.id))
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

    // fn start(&mut self, data: &mut Data<'_>) -> Option<()> {
    //     let level = data.current_progression()?.level?;
    //     if level == Level::TitleScreen {
    //         let start = data.game_start()?;
    //         if start == GameStart::JustStarted {
    //             self.events.push(Event::GameStarted);
    //         }
    //     }
    //
    //     Some(())
    // }
    //
    // fn key_item_changes(&mut self, data: &mut Data<'_>) {
    //     for item in data.key_item_changes() {
    //         let event = match item {
    //             Change::PickedUp(item) => Event::PickedUpKeyItem(item),
    //             Change::Lost(item) => Event::LostKeyItem(item),
    //         };
    //         self.events.push(event);
    //     }
    // }
    //
    // fn level_changes(&mut self, data: &mut Data<'_>) -> Option<()> {
    //     let progression = data.current_progression()?;
    //
    //     let loading = self.loading.update_infallible(progression.is_loading);
    //     if loading.changed_to(&true) {
    //         self.events.push(Event::LoadStart);
    //     } else if loading.changed_to(&false) {
    //         self.events.push(Event::LoadEnd);
    //     }
    //
    //     let cutscene = self.cutscene.update_infallible(progression.is_in_cutscene);
    //     if cutscene.changed_to(&true) {
    //         self.events.push(Event::CutsceneStart);
    //     } else if cutscene.changed_to(&false) {
    //         self.events.push(Event::CutsceneEnd);
    //     }
    //
    //     let level = self
    //         .level
    //         .update(progression.level)
    //         .filter(|o| o.changed())?;
    //
    //     self.events.push(Event::LevelChange {
    //         from: level.old,
    //         to: level.current,
    //     });
    //
    //     Some(())
    // }
    //
    // fn encounter_changes(&mut self, data: &mut Data<'_>) {
    //     let in_encounter = self.encounter.is_some();
    //     if in_encounter {
    //         match data.encounter() {
    //             Some(enc) if enc.done => {
    //                 let Some(start) = self.encounter.take() else {
    //                     unreachable!();
    //                 };
    //
    //                 let event = if start.len() == 1 {
    //                     Event::EncounterEnd(start[0])
    //                 } else {
    //                     Event::EncountersEnd(start)
    //                 };
    //                 self.events.push(event);
    //             }
    //             Some(_) => {}
    //             None => {
    //                 self.encounter = None;
    //             }
    //         }
    //     } else {
    //         match data.encounter() {
    //             Some(enc) if !enc.done => {
    //                 let CurrentEncounter::InEncounter(mut enemies) = data.current_enemies() else {
    //                     log!("encounter without enemies");
    //                     unreachable!();
    //                 };
    //                 enemies.sort_unstable();
    //                 self.encounter = Some(enemies.clone());
    //
    //                 let event = if enemies.len() == 1 {
    //                     Event::EncounterStart(enemies[0])
    //                 } else {
    //                     Event::EncountersStart(enemies)
    //                 };
    //                 self.events.push(event);
    //             }
    //             _ => {}
    //         }
    //     }
    // }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SplitOn {
    Monster(Monster),
    Pickup(Pickup),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum Monster {
    Garland = 350,
    Pirates = 349,
    Piscodemons = 88,
    Astos = 348,
    Vampire = 347,
    Lich = 345,
    EvilEye = 312,
    Kraken = 343,
    BlueDragon = 239,
    Tiamat = 342,
    Marilith = 344,
    DeathEye = 197,
    Lich2 = 338,
    Marilith2 = 339,
    Kraken2 = 340,
    Tiamat2 = 341,
    Chaos = 346,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum Pickup {
    Lute = 44,
    Ship = 4,
    Crown = 45,
    CrystalEye = 46,
    Tonic = 47,
    MysticKey = 48,
    Nitro = 49,
    StarRuby = 52,
    EarthRod = 53,
    Canoe = 60,
    LeviStone = 54,
    AirShip = 3,
    WarpCube = 57,
    BottledFaerie = 58,
    Oxyale = 59,
    RosettaStone = 51,
    Chime = 55,
}
