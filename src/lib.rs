use std::num::NonZeroU32;

use crate::data::{BattleResult, Data};
use asr::{
    future::next_tick,
    game_engine::unity::il2cpp::{Game, Module},
    settings::{gui::Title, Gui},
    timer::{self, TimerState},
    watcher::Watcher,
    Process,
};
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[cfg(any(debug_assertions, debugger))]
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        let mut buf = ::asr::arrayvec::ArrayString::<8192>::new();
        let _ = ::core::fmt::Write::write_fmt(
            &mut buf,
            ::core::format_args!($($arg)*),
        );
        ::asr::print_message(&buf);
    }};
}

#[cfg(not(any(debug_assertions, debugger)))]
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {};
}

#[macro_export]
macro_rules! dbg {
    // Copy of ::std::dbg! but for no_std with redirection to log!
    () => {
        $crate::log!("[{}:{}]", ::core::file!(), ::core::line!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                $crate::log!("[{}:{}] {} = {:?}",
                    ::core::file!(), ::core::line!(), ::core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}

mod data;

asr::async_main!(stable);

#[allow(clippy::large_enum_variant)]
enum State {
    NotRunning,
    Running(Data, Splits),
}

async fn main() {
    asr::set_tick_rate(60.0);
    let mut settings = Settings::register();
    log!("Loaded settings: {:?}", SettingsDebug(&settings));

    loop {
        let process = Process::wait_attach("FINAL FANTASY.exe").await;
        log!("attached to process");
        process
            .until_closes(async {
                let module = Module::wait_attach_auto_detect(&process).await;
                let image = module.wait_get_default_image(&process).await;
                log!("Attached to the game");
                let game = Game::new(&process, module, image);

                let mut state = State::NotRunning;

                'outer: loop {
                    match state {
                        State::NotRunning => {
                            if timer::state() == TimerState::Running {
                                state = State::Running(Data::new(), Splits::new());
                                continue 'outer;
                            }
                        }
                        State::Running(ref mut data, ref mut splits) => match timer::state() {
                            TimerState::NotRunning => {
                                state = State::NotRunning;
                                continue 'outer;
                            }
                            TimerState::Running => {
                                settings.update();
                                let early = settings.battle_split == BattleSplit::DeathAnimation;

                                if let Some(split) = splits
                                    .check(data, &game, early)
                                    .filter(|s| settings.filter(*s))
                                {
                                    log!("Splitting: {split:?}");
                                    timer::split();
                                }
                            }
                            _ => {}
                        },
                    }

                    next_tick().await;
                }
            })
            .await;
    }
}

#[derive(Gui, Debug, Copy, Clone, PartialEq, Eq)]
pub enum BattleSplit {
    /// Split battles as soon as the death animation starts.
    DeathAnimation,

    /// Split battles after all spoils are collected and the battle fades out.
    #[default]
    BattleEnd,
}

#[derive(Gui)]
pub struct Settings {
    /// Boss related splits.
    ///
    /// The default is to split when the battle fades into the field/wm screen.
    /// This can be controlled with the first settings.
    _bosses: Title,

    /// When to split on battles.
    battle_split: BattleSplit,

    /// Split when defeating Garland
    #[default = true]
    garland: bool,

    /// Split when defating the Pirates
    #[default = false]
    pirates: bool,

    /// Split when defeating Piscodemons
    #[default = false]
    piscodemons: bool,

    /// Split when defeating Astos
    #[default = true]
    astos: bool,

    /// Split when defeating Vampire
    #[default = true]
    vampire: bool,

    /// Split when defeating Lich
    #[default = true]
    lich: bool,

    /// Split when defeating Evil Eye
    #[default = false]
    evil_eye: bool,

    /// Split when defeating Kraken
    #[default = true]
    kraken: bool,

    /// Split when defeating Blue Dragon
    #[default = false]
    blue_dragon: bool,

    /// Split when defeating Tiamat
    #[default = true]
    tiamat: bool,

    /// Split when defeating Marilith
    #[default = true]
    marilith: bool,

    /// Split when defeating Death Eye
    #[default = false]
    death_eye: bool,

    /// Split when defeating Lich 2
    #[default = true]
    lich2: bool,

    /// Split when defeating Marilith 2
    #[default = true]
    marilith2: bool,

    /// Split when defeating Kraken 2
    #[default = true]
    kraken2: bool,

    /// Split when defeating Tiamat 2
    #[default = true]
    tiamat2: bool,

    /// Split when defeating Chaos
    #[default = true]
    chaos: bool,

    /// Item related splits.
    ///
    /// Splits happen when the "recieved" dialog box vanishes.
    _items: Title,

    /// Split when obtaining the lute
    #[default = false]
    lute: bool,

    /// Split when obtaining the ship
    #[default = true]
    ship: bool,

    /// Split when obtaining the crown
    #[default = true]
    crown: bool,

    /// Split when obtaining the Crystal Eye
    #[default = false]
    crystal_eye: bool,

    /// Split when obtaining the Tonic
    #[default = true]
    tonic: bool,

    /// Split when obtaining the Mystic Key
    #[default = true]
    mystic_key: bool,

    /// Split when obtaining the Nitro
    #[default = true]
    nitro: bool,

    /// Split when obtaining the Star Ruby
    #[default = false]
    star_ruby: bool,

    /// Split when obtaining the Earth Rod
    #[default = true]
    earth_rod: bool,

    /// Split when obtaining the Canoe
    #[default = true]
    canoe: bool,

    /// Split when obtaining the Levi Stone
    #[default = true]
    levi_stone: bool,

    /// Split when obtaining the Air Ship
    #[default = false]
    air_ship: bool,

    /// Split when obtaining the Warp Cube
    #[default = true]
    warp_cube: bool,

    /// Split when obtaining the Bottled Faerie
    #[default = false]
    bottled_faerie: bool,

    /// Split when obtaining the Oxyale
    #[default = true]
    oxyale: bool,

    /// Split when obtaining the Rosetta Stone
    #[default = true]
    rosetta_stone: bool,

    /// Split when obtaining the Chime
    #[default = true]
    chime: bool,
}

struct SettingsDebug<'a>(&'a Settings);

impl std::fmt::Debug for SettingsDebug<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Settings {
            _bosses: _,
            battle_split,
            garland,
            pirates,
            piscodemons,
            astos,
            vampire,
            lich,
            evil_eye,
            kraken,
            blue_dragon,
            tiamat,
            marilith,
            death_eye,
            lich2,
            marilith2,
            kraken2,
            tiamat2,
            chaos,
            _items: _,
            lute,
            ship,
            crown,
            crystal_eye,
            tonic,
            mystic_key,
            nitro,
            star_ruby,
            earth_rod,
            canoe,
            levi_stone,
            air_ship,
            warp_cube,
            bottled_faerie,
            oxyale,
            rosetta_stone,
            chime,
        } = self.0;

        f.debug_struct("Settings")
            .field("battle_split", battle_split)
            .field("garland", garland)
            .field("pirates", pirates)
            .field("piscodemons", piscodemons)
            .field("astos", astos)
            .field("vampire", vampire)
            .field("lich", lich)
            .field("evil_eye", evil_eye)
            .field("kraken", kraken)
            .field("blue_dragon", blue_dragon)
            .field("tiamat", tiamat)
            .field("marilith", marilith)
            .field("death_eye", death_eye)
            .field("lich2", lich2)
            .field("marilith2", marilith2)
            .field("kraken2", kraken2)
            .field("tiamat2", tiamat2)
            .field("chaos", chaos)
            .field("lute", lute)
            .field("ship", ship)
            .field("crown", crown)
            .field("crystal_eye", crystal_eye)
            .field("tonic", tonic)
            .field("mystic_key", mystic_key)
            .field("nitro", nitro)
            .field("star_ruby", star_ruby)
            .field("earth_rod", earth_rod)
            .field("canoe", canoe)
            .field("levi_stone", levi_stone)
            .field("air_ship", air_ship)
            .field("warp_cube", warp_cube)
            .field("bottled_faerie", bottled_faerie)
            .field("oxyale", oxyale)
            .field("rosetta_stone", rosetta_stone)
            .field("chime", chime)
            .finish()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SplitOn {
    Monster(Monster),
    Pickup(Pickup),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
enum Monster {
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
enum Pickup {
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

impl Settings {
    fn filter(&self, split: SplitOn) -> bool {
        return match split {
            SplitOn::Monster(Monster::Garland) => self.garland,
            SplitOn::Monster(Monster::Pirates) => self.pirates,
            SplitOn::Monster(Monster::Piscodemons) => self.piscodemons,
            SplitOn::Monster(Monster::Astos) => self.astos,
            SplitOn::Monster(Monster::Vampire) => self.vampire,
            SplitOn::Monster(Monster::Lich) => self.lich,
            SplitOn::Monster(Monster::EvilEye) => self.evil_eye,
            SplitOn::Monster(Monster::Kraken) => self.kraken,
            SplitOn::Monster(Monster::BlueDragon) => self.blue_dragon,
            SplitOn::Monster(Monster::Tiamat) => self.tiamat,
            SplitOn::Monster(Monster::Marilith) => self.marilith,
            SplitOn::Monster(Monster::DeathEye) => self.death_eye,
            SplitOn::Monster(Monster::Lich2) => self.lich2,
            SplitOn::Monster(Monster::Marilith2) => self.marilith2,
            SplitOn::Monster(Monster::Kraken2) => self.kraken2,
            SplitOn::Monster(Monster::Tiamat2) => self.tiamat2,
            SplitOn::Monster(Monster::Chaos) => self.chaos,
            SplitOn::Pickup(Pickup::Lute) => self.lute,
            SplitOn::Pickup(Pickup::Ship) => self.ship,
            SplitOn::Pickup(Pickup::Crown) => self.crown,
            SplitOn::Pickup(Pickup::CrystalEye) => self.crystal_eye,
            SplitOn::Pickup(Pickup::Tonic) => self.tonic,
            SplitOn::Pickup(Pickup::MysticKey) => self.mystic_key,
            SplitOn::Pickup(Pickup::Nitro) => self.nitro,
            SplitOn::Pickup(Pickup::StarRuby) => self.star_ruby,
            SplitOn::Pickup(Pickup::EarthRod) => self.earth_rod,
            SplitOn::Pickup(Pickup::Canoe) => self.canoe,
            SplitOn::Pickup(Pickup::LeviStone) => self.levi_stone,
            SplitOn::Pickup(Pickup::AirShip) => self.air_ship,
            SplitOn::Pickup(Pickup::WarpCube) => self.warp_cube,
            SplitOn::Pickup(Pickup::BottledFaerie) => self.bottled_faerie,
            SplitOn::Pickup(Pickup::Oxyale) => self.oxyale,
            SplitOn::Pickup(Pickup::RosettaStone) => self.rosetta_stone,
            SplitOn::Pickup(Pickup::Chime) => self.chime,
        };
    }
}

#[derive(Debug, Clone, Default)]
struct TickTimer {
    deadline: Option<NonZeroU32>,
}

impl TickTimer {
    fn set(&mut self, deadline: u32) {
        self.deadline = NonZeroU32::new(deadline);
    }

    fn tick(&mut self) -> bool {
        let Some(deadline) = self.deadline else {
            return false;
        };
        let deadline = NonZeroU32::new(deadline.get() - 1);
        self.deadline = deadline;
        return self.deadline.is_none();
    }
}

struct Splits {
    in_battle: Watcher<bool>,
    battle_playing: Watcher<bool>,
    items: ahash::AHashSet<Pickup>,
    chaos: TickTimer,
}

impl Splits {
    fn new() -> Self {
        Self {
            in_battle: Watcher::new(),
            battle_playing: Watcher::new(),
            items: ahash::AHashSet::new(),
            chaos: TickTimer::default(),
        }
    }

    fn check(&mut self, data: &mut Data, game: &Game, early: bool) -> Option<SplitOn> {
        if self.chaos.tick() {
            return Some(SplitOn::Monster(Monster::Chaos));
        }

        if let Some(mon) = self.battle_check(data, game, early) {
            if mon == Monster::Chaos {
                self.chaos.set(60);
                return None;
            }
            return Some(SplitOn::Monster(mon));
        }

        if let Some(item) = self.inventory_check(data, game) {
            return Some(SplitOn::Pickup(item));
        }

        return None;
    }

    // TODO
    // [ ] check active only, load info only in battle
    // [ ] move to class iinstead of class2
    // [ ] move to default asr, latest

    fn battle_check(&mut self, data: &mut Data, game: &Game, early: bool) -> Option<Monster> {
        let in_battle = self.in_battle.update_infallible(data.battle_active(game));
        if in_battle.current == false && in_battle.unchanged() {
            return None;
        }

        let info = data.battle_info(game)?;

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

    fn inventory_check(&mut self, data: &mut Data, game: &Game) -> Option<Pickup> {
        if let Some(inventory) = data.inventory(game) {
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
