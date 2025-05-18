#![no_std]

use asr::{
    future::next_tick,
    game_engine::unity::il2cpp::Module,
    settings::{gui::Title as Heading, Gui},
    time::Duration,
    timer::{self, TimerState},
    watcher::{Pair, Watcher},
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

    /// Splits Settings
    _splits_title: Heading,

    /// When to split on battles.
    battle_split: BattleSplit,

    /// Split when defeating Garland
    #[default = false]
    garland: bool,

    /// Split when obtaining the Lute
    #[default = false]
    lute: bool,

    /// Split when defating the Pirates
    #[default = false]
    pirates: bool,

    /// Split when obtaining the Ship
    #[default = false]
    ship: bool,

    /// Split when done with shopping in Elfenheim
    #[default = false]
    elfen_shop: bool,

    /// Split when entering the Marsh Cave
    #[default = false]
    marsh_cave: bool,

    /// Split when defeating Piscodemons
    #[default = false]
    piscodemons: bool,

    /// Split when obtaining the Crown
    #[default = false]
    crown: bool,

    /// Split when defeating Astos
    #[default = false]
    astos: bool,

    /// Split when obtaining the Crystal Eye
    #[default = false]
    crystal_eye: bool,

    /// Split when obtaining the Tonic
    #[default = false]
    tonic: bool,

    /// Split when obtaining the Mystic Key
    #[default = false]
    mystic_key: bool,

    /// Split when obtaining the Nitro
    #[default = false]
    nitro: bool,

    /// Split when having bought Firaga
    #[default = false]
    firaga: bool,

    /// Split when defeating Vampire
    #[default = false]
    vampire: bool,

    /// Split when obtaining the Star Ruby
    #[default = false]
    star_ruby: bool,

    /// Split when obtaining the Earth Rod
    #[default = false]
    earth_rod: bool,

    /// Split when defeating Lich
    #[default = false]
    lich: bool,

    /// Split when obtaining the Canoe
    #[default = false]
    canoe: bool,

    /// Split when defeating Evil Eye
    #[default = false]
    evil_eye: bool,

    /// Split when obtaining the Levi Stone
    #[default = false]
    levi_stone: bool,

    /// Split when leaving the Ice Cave
    #[default = false]
    ice_cave: bool,

    /// Split when obtaining the Air Ship
    #[default = false]
    air_ship: bool,

    /// Split when obtaining the Warp Cube
    #[default = false]
    warp_cube: bool,

    /// Split when leaving the Waterfall Cave
    #[default = false]
    waterfall_cave: bool,

    /// Split when obtaining the Bottled Faerie
    #[default = false]
    bottled_faerie: bool,

    /// Split when obtaining the Oxyale
    #[default = false]
    oxyale: bool,

    /// Split when obtaining the Rosetta Stone
    #[default = false]
    rosetta_stone: bool,

    /// Split when defeating Kraken
    #[default = false]
    kraken: bool,

    /// Split when obtaining the Chime
    #[default = false]
    chime: bool,

    /// Split when defeating Blue Dragon
    #[default = false]
    blue_dragon: bool,

    /// Split when entering the Flying Fortress
    #[default = false]
    flying_fortress: bool,

    /// Split when defeating Tiamat
    #[default = false]
    tiamat: bool,

    /// Split when defeating Marilith
    #[default = false]
    marilith: bool,

    /// Split when defeating Death Eye
    #[default = false]
    death_eye: bool,

    /// Split when opening the Chaos Shrine with the Lute
    #[default = false]
    chaos_shrine: bool,

    /// Split when defeating Lich 2
    #[default = false]
    lich2: bool,

    /// Split when defeating Marilith 2
    #[default = false]
    marilith2: bool,

    /// Split when defeating Kraken 2
    #[default = false]
    kraken2: bool,

    /// Split when defeating Tiamat 2
    #[default = false]
    tiamat2: bool,

    /// Split when defeating Chaos
    #[default = true]
    chaos: bool,

    /// IGT Settings
    _igt_title: Heading,

    /// Report the IGT as "Game Time"
    #[default = false]
    igt: bool,
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

    let data = Data::new(process, &module, &image).await;
    log!("Loaded game data");

    let mut state = State::NotRunning(Title::new());

    'outer: loop {
        settings.update();
        if settings.igt {
            let igt = data.user().igt();
            timer::set_game_time(Duration::seconds_f64(igt));
        }
        match main_loop(&data, &mut state, settings.battle_split) {
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

fn main_loop(data: &Data<'_>, state: &mut State, battle_split: BattleSplit) -> ControlFlow<Action> {
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
                if let Some(split) = splits.check(data, battle_split) {
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
    Field(FieldSplit),
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct NoBattle;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, TryFromPrimitive)]
#[repr(u32)]
enum Field {
    WorldMap = 1,
    CastleCornelia = 2,
    CorneliaThrone = 3,
    MatoyaCave = 12,
    Pravoka = 13,
    Elfenheim = 22,
    ElfenheimItemShop = 24,
    ElvenCastle = 32,
    WesternKeep = 33,
    Melmond = 34,
    MelmondBMShop = 39,
    SageCave = 40,
    CresentLake = 41,
    OasisShop = 59,
    Gaia = 60,
    Lufenia = 70,
    MarshCave1 = 73,
    MarshCave3 = 75,
    EarthCave3 = 78,
    IceCave1 = 88,
    IceCave2 = 91,
    Underwater5 = 103,
    WaterfallCave = 104,
    MirageTower3 = 107,
    FlyingFortress = 108,
    ChaosShrine2 = 114,
    ChaosShrine3 = 115,
    AirHangar = 122,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum FieldSplit {
    MarshShop,
    MarshCave,
    Firaga,
    IceCave,
    WaterfallCave,
    FlyingFortress,
    ChaosShrine,
}

impl FieldSplit {
    fn from_watcher(watcher: &Pair<Field>) -> Option<Self> {
        match (watcher.old, watcher.current) {
            (Field::ElfenheimItemShop, Field::Elfenheim) => Some(FieldSplit::MarshShop),
            (Field::WorldMap, Field::MarshCave1) => Some(FieldSplit::MarshCave),
            (Field::MelmondBMShop, Field::Melmond) => Some(FieldSplit::Firaga),
            (Field::IceCave1, Field::WorldMap) => Some(FieldSplit::IceCave),
            (Field::WaterfallCave, Field::WorldMap) => Some(FieldSplit::WaterfallCave),
            (Field::MirageTower3, Field::FlyingFortress) => Some(FieldSplit::FlyingFortress),
            (Field::ChaosShrine3, Field::ChaosShrine2) => Some(FieldSplit::ChaosShrine),
            _ => None,
        }
    }
}

impl Field {
    fn has_key_item(self) -> bool {
        matches!(
            self,
            Field::CorneliaThrone
                | Field::Pravoka
                | Field::MarshCave3
                | Field::WesternKeep
                | Field::MatoyaCave
                | Field::ElvenCastle
                | Field::CastleCornelia
                | Field::EarthCave3
                | Field::SageCave
                | Field::CresentLake
                | Field::IceCave2
                | Field::AirHangar
                | Field::WaterfallCave
                | Field::OasisShop
                | Field::Gaia
                | Field::Underwater5
                | Field::Lufenia
        )
    }
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
            SplitOn::Field(FieldSplit::MarshShop) => self.elfen_shop,
            SplitOn::Field(FieldSplit::MarshCave) => self.marsh_cave,
            SplitOn::Field(FieldSplit::Firaga) => self.firaga,
            SplitOn::Field(FieldSplit::IceCave) => self.ice_cave,
            SplitOn::Field(FieldSplit::WaterfallCave) => self.waterfall_cave,
            SplitOn::Field(FieldSplit::FlyingFortress) => self.flying_fortress,
            SplitOn::Field(FieldSplit::ChaosShrine) => self.chaos_shrine,
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

    fn new_game(&mut self, data: &Data) -> bool {
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
    field: Watcher<Field>,
    items: Inventory,
    chaos_end: f32,
}

impl Splits {
    fn new() -> Self {
        Self {
            in_battle: Watcher::new(),
            battle_playing: Watcher::new(),
            field: Watcher::new(),
            items: Inventory::empty(),
            chaos_end: f32::MAX,
        }
    }

    fn check(&mut self, data: &Data, split: BattleSplit) -> Option<SplitOn> {
        match self.battle_check(data, split)? {
            Ok(split) => return Some(SplitOn::Monster(split)),
            Err(_no_battle) => {}
        }

        let field = match self.field_check(data)? {
            Ok(field) => return Some(SplitOn::Field(field)),
            Err(field) => field,
        };

        if field.has_key_item() {
            if let Some(item) = self.inventory_check(data) {
                return Some(SplitOn::Pickup(item));
            }
        }

        return None;
    }

    fn battle_check(
        &mut self,
        data: &Data,
        split: BattleSplit,
    ) -> Option<Result<Monster, NoBattle>> {
        let battles = data.battles();

        let in_battle = self.in_battle.update_infallible(battles.active());
        if in_battle.current == false && in_battle.unchanged() {
            return Some(Err(NoBattle));
        }

        let monster = battles.encounter_id()?;
        let monster = Monster::try_from(monster).ok()?;

        let playing = self.battle_playing.update_infallible(battles.playing());

        if in_battle.changed_to(&true) {
            log!("Encounter: {monster:?} -- Started");
            return None;
        }

        if in_battle.changed_to(&false) {
            if playing.changed_to(&false) {
                log!("Battle reset detected, no split!");
                return None;
            }

            log!("Encounter: {monster:?} -- Ended");
            if split == BattleSplit::BattleEnd {
                // Chaos is always split on animation
                if monster != Monster::Chaos {
                    return Some(Ok(monster));
                }
            }
        }

        if playing.changed_to(&false) {
            let result = battles.result();
            log!("Encounter: {monster:?} -- {result:?}");
            if result == BattleResult::Win {
                if monster == Monster::Chaos {
                    let elapsed_time = battles.elapsed_time();

                    self.chaos_end = elapsed_time + {
                        const FRAMES: f32 = 113.0;
                        const FPS: f32 = 60.0;
                        const TIME: f32 = FRAMES / FPS;

                        TIME
                    };
                } else if split == BattleSplit::DeathAnimation {
                    return Some(Ok(monster));
                }
            }
        } else if playing.current == false && monster == Monster::Chaos {
            let elapsed_time = battles.elapsed_time();
            if elapsed_time > self.chaos_end {
                self.chaos_end = f32::MAX;
                return Some(Ok(monster));
            }
        }

        return None;
    }

    fn field_check(&mut self, data: &Data) -> Option<Result<FieldSplit, Field>> {
        let field = data.user().map_id()?;
        let field = Field::try_from(field).ok()?;
        let field = self.field.update_infallible(field);
        if let Some(field) = FieldSplit::from_watcher(field) {
            return Some(Ok(field));
        }

        Some(Err(field.current))
    }

    fn inventory_check(&mut self, data: &Data) -> Option<Pickup> {
        let items = data.items();

        if let Some(item) = items
            .key_item_ids()
            .filter_map(|i| Pickup::try_from(i).ok())
            .find(|item| self.items.insert(*item))
        {
            log!("Picked up the {item:?}");
            return Some(item);
        }

        if let Some(vehicle) = items
            .vehicle_ids()
            .filter_map(|i| Pickup::try_from(i).ok())
            .find(|item| self.items.insert(*item))
        {
            log!("Obtained up the {vehicle:?}");
            return Some(vehicle);
        }

        return None;
    }
}

#[allow(dead_code)]
struct SettingsDebug<'a>(&'a Settings);

impl core::fmt::Debug for SettingsDebug<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Settings {
            _splits_title,
            start,
            battle_split,
            garland,
            lute,
            pirates,
            ship,
            elfen_shop,
            marsh_cave,
            piscodemons,
            crown,
            astos,
            crystal_eye,
            tonic,
            mystic_key,
            nitro,
            firaga,
            vampire,
            star_ruby,
            earth_rod,
            lich,
            canoe,
            evil_eye,
            levi_stone,
            ice_cave,
            air_ship,
            warp_cube,
            waterfall_cave,
            bottled_faerie,
            oxyale,
            rosetta_stone,
            kraken,
            chime,
            blue_dragon,
            flying_fortress,
            tiamat,
            marilith,
            death_eye,
            chaos_shrine,
            lich2,
            marilith2,
            kraken2,
            tiamat2,
            chaos,
            _igt_title,
            igt,
        } = self.0;

        f.debug_struct("Settings")
            .field("start", start)
            .field("battle_split", battle_split)
            .field("garland", garland)
            .field("lute", lute)
            .field("pirates", pirates)
            .field("ship", ship)
            .field("elfen_shop", elfen_shop)
            .field("marsh_cave", marsh_cave)
            .field("piscodemons", piscodemons)
            .field("crown", crown)
            .field("astos", astos)
            .field("crystal_eye", crystal_eye)
            .field("tonic", tonic)
            .field("mystic_key", mystic_key)
            .field("nitro", nitro)
            .field("firaga", firaga)
            .field("vampire", vampire)
            .field("star_ruby", star_ruby)
            .field("earth_rod", earth_rod)
            .field("lich", lich)
            .field("canoe", canoe)
            .field("evil_eye", evil_eye)
            .field("levi_stone", levi_stone)
            .field("ice_cave", ice_cave)
            .field("air_ship", air_ship)
            .field("warp_cube", warp_cube)
            .field("waterfall_cave", waterfall_cave)
            .field("bottled_faerie", bottled_faerie)
            .field("oxyale", oxyale)
            .field("rosetta_stone", rosetta_stone)
            .field("kraken", kraken)
            .field("chime", chime)
            .field("blue_dragon", blue_dragon)
            .field("flying_fortress", flying_fortress)
            .field("tiamat", tiamat)
            .field("marilith", marilith)
            .field("death_eye", death_eye)
            .field("chaos_shrine", chaos_shrine)
            .field("lich2", lich2)
            .field("marilith2", marilith2)
            .field("kraken2", kraken2)
            .field("tiamat2", tiamat2)
            .field("chaos", chaos)
            .field("igt", igt)
            .finish()
    }
}
