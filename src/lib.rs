#![no_std]

use asr::{
    future::next_tick,
    game_engine::unity::il2cpp::Module,
    settings::Gui,
    timer::{self, TimerState},
    watcher::Watcher,
    Process,
};
use core::ops::ControlFlow;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::data::{BattleResult, Data};

mod data;

asr::async_main!(stable);
asr::panic_handler!();

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

#[derive(Gui, Debug, Copy, Clone, PartialEq, Eq)]
pub enum BattleSplit {
    /// Split battles as soon as the death animation starts
    DeathAnimation,

    /// Split battles after all spoils are collected and the battle fades out
    #[default]
    BattleEnd,
}

#[derive(Gui)]
pub struct Settings {
    /// Start the timer on party confirmation
    #[default = true]
    start: bool,

    /// When to split on battles.
    battle_split: BattleSplit,

    /// Split when defeating Garland
    #[default = true]
    garland: bool,

    /// Split when obtaining the lute
    #[default = false]
    lute: bool,

    /// Split when defating the Pirates
    #[default = false]
    pirates: bool,

    /// Split when obtaining the ship
    #[default = true]
    ship: bool,

    /// Split when defeating Piscodemons
    #[default = false]
    piscodemons: bool,

    /// Split when obtaining the crown
    #[default = true]
    crown: bool,

    /// Split when defeating Astos
    #[default = true]
    astos: bool,

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

    /// Split when defeating Vampire
    #[default = true]
    vampire: bool,

    /// Split when obtaining the Star Ruby
    #[default = false]
    star_ruby: bool,

    /// Split when obtaining the Earth Rod
    #[default = true]
    earth_rod: bool,

    /// Split when defeating Lich
    #[default = true]
    lich: bool,

    /// Split when obtaining the Canoe
    #[default = true]
    canoe: bool,

    /// Split when defeating Evil Eye
    #[default = false]
    evil_eye: bool,

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

    /// Split when defeating Kraken
    #[default = true]
    kraken: bool,

    /// Split when obtaining the Chime
    #[default = true]
    chime: bool,

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
}

async fn main() {
    asr::set_tick_rate(60.0);
    let mut settings = Settings::register();
    log!("Loaded settings: {:?}", SettingsDebug(&settings));

    loop {
        let process = Process::wait_attach("FINAL FANTASY.exe").await;
        log!("attached to process");
        process
            .until_closes(game_loop(&process, &mut settings))
            .await;
    }
}

enum State {
    NotRunning(Title),
    Running(Splits),
}

async fn game_loop(process: &Process, settings: &mut Settings) {
    let module = Module::wait_attach_auto_detect(process).await;
    let image = module.wait_get_default_image(process).await;
    log!("Attached to the game");

    let mut data = Data::new(process, &module, &image).await;
    log!("Loaded game data");

    let mut state = State::NotRunning(Title::new());

    'outer: loop {
        settings.update();
        match main_loop(&mut data, &mut state, settings) {
            ControlFlow::Continue(()) => continue 'outer,
            ControlFlow::Break(Action::Start) if settings.start => {
                log!("Starting timer");
                timer::start();
            }
            ControlFlow::Break(Action::Split(split)) if settings.filter(split) => {
                log!("Splitting: {split:?}");
                timer::split();
            }
            ControlFlow::Break(Action::Start) => {
                log!("Ignoring: Start");
            }
            ControlFlow::Break(Action::Split(split)) => {
                log!("Ignoring: {split:?}");
            }
            ControlFlow::Break(Action::None) => {}
        }

        next_tick().await;
    }
}

fn main_loop(
    data: &mut Data<'_>,
    state: &mut State,
    settings: &mut Settings,
) -> ControlFlow<Action> {
    match state {
        State::NotRunning(title) => match timer::state() {
            TimerState::Running => {
                *state = State::Running(Splits::new());
                return ControlFlow::Continue(());
            }
            TimerState::NotRunning => {
                if title.new_game(data) {
                    *state = State::Running(Splits::new());
                    return ControlFlow::Break(Action::Start);
                }
            }
            _ => {}
        },
        State::Running(splits) => match timer::state() {
            TimerState::NotRunning => {
                *state = State::NotRunning(Title::new());
                return ControlFlow::Continue(());
            }
            TimerState::Running => {
                let early = settings.battle_split == BattleSplit::DeathAnimation;
                if let Some(split) = splits.check(data, early) {
                    return ControlFlow::Break(Action::Split(split));
                }
            }
            _ => {}
        },
    };
    ControlFlow::Break(Action::None)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Action {
    None,
    Start,
    Split(SplitOn),
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

#[derive(Debug, Clone, Copy)]
struct Inventory(u64);

impl Inventory {
    const fn empty() -> Self {
        Self(0)
    }

    fn insert(&mut self, item: Pickup) -> bool {
        let Ok(ord) = u8::try_from(u32::from(item)) else {
            return false;
        };
        if ord >= 64 {
            return false;
        }

        let mask = 1_u64 << ord;
        let previous = self.0 & mask;
        self.0 |= mask;
        return previous == 0;
    }
}

struct Title {
    fade_out: Watcher<bool>,
}

impl Title {
    fn new() -> Self {
        Self {
            fade_out: Watcher::new(),
        }
    }

    fn new_game(&mut self, data: &mut Data) -> bool {
        let fade_out = self.fade_out.update_infallible(data.has_fade_out());
        if fade_out.changed_to(&true) {
            log!("Fade out detected");
            return true;
        }
        return false;
    }
}

struct Splits {
    in_battle: Watcher<bool>,
    battle_playing: Watcher<bool>,
    items: Inventory,
    chaos_end: f32,
}

impl Splits {
    fn new() -> Self {
        Self {
            in_battle: Watcher::new(),
            battle_playing: Watcher::new(),
            items: Inventory::empty(),
            chaos_end: f32::MAX,
        }
    }

    fn check(&mut self, data: &mut Data, early: bool) -> Option<SplitOn> {
        if let Some(mon) = self.battle_check(data, early) {
            return Some(SplitOn::Monster(mon));
        }

        if let Some(item) = self.inventory_check(data) {
            return Some(SplitOn::Pickup(item));
        }

        return None;
    }

    fn battle_check(&mut self, data: &mut Data, early: bool) -> Option<Monster> {
        let in_battle = self.in_battle.update_infallible(data.battle_active());
        if in_battle.current == false && in_battle.unchanged() {
            return None;
        }

        let info = data.battle_info()?;

        let playing = self.battle_playing.update_infallible(info.playing);

        if let Ok(mon) = Monster::try_from(info.encounter_id) {
            if in_battle.changed_to(&true) {
                log!("Encounter: {mon:?} -- Started");
            } else if in_battle.changed_to(&false) {
                if playing.changed_to(&false) {
                    log!("Battle reset detected, no split!");
                    return None;
                }

                log!("Encounter: {mon:?} -- Ended");
                if !early {
                    if mon != Monster::Chaos {
                        return Some(mon);
                    }
                }
            } else if in_battle.current {
                if playing.changed_to(&false) {
                    log!("Encounter: {mon:?} -- {:?}", info.result);
                    if info.result == BattleResult::Win {
                        if mon == Monster::Chaos {
                            self.chaos_end = info.elapsed_time + {
                                const FRAMES: f32 = 113.0;
                                const FPS: f32 = 60.0;
                                const TIME: f32 = FRAMES / FPS;

                                TIME
                            };

                            return None;
                        }

                        if early {
                            return Some(mon);
                        }
                    }
                } else if playing.current == false && mon == Monster::Chaos {
                    if info.elapsed_time > self.chaos_end {
                        self.chaos_end = f32::MAX;
                        return Some(mon);
                    }
                }
            }
        }

        return None;
    }

    fn inventory_check(&mut self, data: &mut Data) -> Option<Pickup> {
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

#[allow(dead_code)]
struct SettingsDebug<'a>(&'a Settings);

impl core::fmt::Debug for SettingsDebug<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Settings {
            start,
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
            .field("start", start)
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
