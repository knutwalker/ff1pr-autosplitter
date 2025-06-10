#![no_std]

use asr::{
    future::next_tick,
    game_engine::unity::il2cpp::Module,
    settings::{gui::Title as Heading, Gui},
    timer::{self, TimerState},
    watcher::{Pair, Watcher},
    Process,
};
use core::{marker::PhantomData, ops::ControlFlow};
use num_enum::IntoPrimitive;

use crate::data::{BattleResult, Data, Item, Location, Monster};

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
    /// General Settings
    _general: Heading,

    /// Start the timer on party confirmation
    #[default = true]
    start: bool,

    /// Split when defeating Chaos
    #[default = true]
    chaos: bool,

    /// When to split on battles.
    battle_split: BattleSplit,

    /// Splits: Only enable the settings that match your splits.
    _splits_heading1: Heading,

    /// You don't need to all, only what you want to split.
    _splits_heading2: Heading,

    /// Anything else not mentioned here can be split manually.
    _splits_heading3: Heading,

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

    /// Split after shopping in Elfenheim
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
}

async fn main() {
    asr::set_tick_rate(60.0);

    let mut settings = {
        let mut s = Settings::register();
        s.update();
        log!("Loaded settings: {:?}", SettingsDebug(&s));
        s
    };

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, IntoPrimitive)]
#[repr(u8)]
enum SplitOn {
    Garland,
    Lute,
    Pirates,
    Ship,
    MarshShop,
    MarshCave,
    Piscodemons,
    Crown,
    Astos,
    CrystalEye,
    Tonic,
    MysticKey,
    Nitro,
    Firaga,
    Vampire,
    StarRuby,
    EarthRod,
    Lich,
    Canoe,
    EvilEye,
    LeviStone,
    IceCave,
    AirShip,
    WarpCube,
    WaterfallCave,
    BottledFaerie,
    Oxyale,
    RosettaStone,
    Kraken,
    Chime,
    BlueDragon,
    FlyingFortress,
    Tiamat,
    Marilith,
    DeathEye,
    ChaosShrine,
    Lich2,
    Marilith2,
    Kraken2,
    Tiamat2,
    Chaos,
}

impl SplitOn {
    fn from_watcher(watcher: &Pair<Location>) -> Option<Self> {
        match (watcher.old, watcher.current) {
            (Location::ElfenheimItemShop, Location::Elfenheim) => Some(Self::MarshShop),
            (Location::WorldMap, Location::MarshCave1) => Some(Self::MarshCave),
            (Location::MelmondBMShop, Location::Melmond) => Some(Self::Firaga),
            (Location::IceCave1, Location::WorldMap) => Some(Self::IceCave),
            (Location::WaterfallCave, Location::WorldMap) => Some(Self::WaterfallCave),
            (Location::MirageTower3, Location::FlyingFortress) => Some(Self::FlyingFortress),
            (Location::ChaosShrine3, Location::ChaosShrine2) => Some(Self::ChaosShrine),
            _ => None,
        }
    }
}

impl Settings {
    fn filter(&self, split: SplitOn) -> bool {
        let Settings {
            _general,
            start: _,
            battle_split: _,
            _splits_heading1,
            _splits_heading2,
            _splits_heading3,
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
        } = self;
        return match split {
            SplitOn::Garland => *garland,
            SplitOn::Lute => *lute,
            SplitOn::Pirates => *pirates,
            SplitOn::Ship => *ship,
            SplitOn::MarshShop => *elfen_shop,
            SplitOn::MarshCave => *marsh_cave,
            SplitOn::Piscodemons => *piscodemons,
            SplitOn::Crown => *crown,
            SplitOn::Astos => *astos,
            SplitOn::CrystalEye => *crystal_eye,
            SplitOn::Tonic => *tonic,
            SplitOn::MysticKey => *mystic_key,
            SplitOn::Nitro => *nitro,
            SplitOn::Firaga => *firaga,
            SplitOn::Vampire => *vampire,
            SplitOn::StarRuby => *star_ruby,
            SplitOn::EarthRod => *earth_rod,
            SplitOn::Lich => *lich,
            SplitOn::Canoe => *canoe,
            SplitOn::EvilEye => *evil_eye,
            SplitOn::LeviStone => *levi_stone,
            SplitOn::IceCave => *ice_cave,
            SplitOn::AirShip => *air_ship,
            SplitOn::WarpCube => *warp_cube,
            SplitOn::WaterfallCave => *waterfall_cave,
            SplitOn::BottledFaerie => *bottled_faerie,
            SplitOn::Oxyale => *oxyale,
            SplitOn::RosettaStone => *rosetta_stone,
            SplitOn::Kraken => *kraken,
            SplitOn::Chime => *chime,
            SplitOn::BlueDragon => *blue_dragon,
            SplitOn::FlyingFortress => *flying_fortress,
            SplitOn::Tiamat => *tiamat,
            SplitOn::Marilith => *marilith,
            SplitOn::DeathEye => *death_eye,
            SplitOn::ChaosShrine => *chaos_shrine,
            SplitOn::Lich2 => *lich2,
            SplitOn::Marilith2 => *marilith2,
            SplitOn::Kraken2 => *kraken2,
            SplitOn::Tiamat2 => *tiamat2,
            SplitOn::Chaos => *chaos,
        };
    }
}

#[derive(Copy, Clone, Debug)]
struct NoBattle;

impl Location {
    fn has_key_item(self) -> bool {
        matches!(
            self,
            Location::CorneliaThrone
                | Location::Pravoka
                | Location::MarshCave3
                | Location::WesternKeep
                | Location::MatoyaCave
                | Location::ElvenCastle
                | Location::CastleCornelia
                | Location::EarthCave3
                | Location::SageCave
                | Location::CrescentLake
                | Location::IceCave2
                | Location::AirHangar
                | Location::WaterfallCave
                | Location::OasisShop
                | Location::Gaia
                | Location::Underwater5
                | Location::Lufenia
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct EnumSet<T>(u64, PhantomData<T>);

trait EnumSetMember {
    fn ordinal(&self) -> Option<u8>;
}

impl<T: EnumSetMember> EnumSet<T> {
    const fn empty() -> Self {
        Self(0, PhantomData)
    }

    fn insert(&mut self, item: &T) -> bool {
        let Some(ord) = item.ordinal() else {
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

impl EnumSetMember for SplitOn {
    fn ordinal(&self) -> Option<u8> {
        Some(u8::from(*self))
    }
}

type Inventory = EnumSet<Item>;
type SeenSplits = EnumSet<SplitOn>;

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
    battle_result: Watcher<BattleResult>,
    location: Watcher<Location>,
    items: Inventory,
    seen: SeenSplits,
    chaos_end: f32,
}

impl Splits {
    fn new() -> Self {
        Self {
            in_battle: Watcher::new(),
            battle_result: Watcher::new(),
            location: Watcher::new(),
            items: Inventory::empty(),
            seen: SeenSplits::empty(),
            chaos_end: f32::MAX,
        }
    }

    fn check(&mut self, data: &Data, split: BattleSplit) -> Option<SplitOn> {
        let split = self.split_check(data, split)?;
        self.seen.insert(&split).then_some(split)
    }

    fn split_check(&mut self, data: &Data, split: BattleSplit) -> Option<SplitOn> {
        match self.battle_check(data, split)? {
            Ok(monster) => {
                return Some(match monster {
                    Monster::Garland => SplitOn::Garland,
                    Monster::Pirates => SplitOn::Pirates,
                    Monster::Piscodemons => SplitOn::Piscodemons,
                    Monster::Astos => SplitOn::Astos,
                    Monster::Vampire => SplitOn::Vampire,
                    Monster::Lich => SplitOn::Lich,
                    Monster::EvilEye => SplitOn::EvilEye,
                    Monster::Kraken => SplitOn::Kraken,
                    Monster::BlueDragon => SplitOn::BlueDragon,
                    Monster::Tiamat => SplitOn::Tiamat,
                    Monster::Marilith => SplitOn::Marilith,
                    Monster::DeathEye => SplitOn::DeathEye,
                    Monster::Lich2 => SplitOn::Lich2,
                    Monster::Marilith2 => SplitOn::Marilith2,
                    Monster::Kraken2 => SplitOn::Kraken2,
                    Monster::Tiamat2 => SplitOn::Tiamat2,
                    Monster::Chaos => SplitOn::Chaos,
                })
            }
            Err(_no_battle) => {}
        }

        let field = match self.field_check(data)? {
            Ok(split) => return Some(split),
            Err(field) => field,
        };

        if field.has_key_item() {
            if let Some(item) = self.inventory_check(data) {
                return Some(match item {
                    Item::Lute => SplitOn::Lute,
                    Item::Ship => SplitOn::Ship,
                    Item::Crown => SplitOn::Crown,
                    Item::CrystalEye => SplitOn::CrystalEye,
                    Item::Tonic => SplitOn::Tonic,
                    Item::MysticKey => SplitOn::MysticKey,
                    Item::Nitro => SplitOn::Nitro,
                    Item::StarRuby => SplitOn::StarRuby,
                    Item::EarthRod => SplitOn::EarthRod,
                    Item::Canoe => SplitOn::Canoe,
                    Item::LeviStone => SplitOn::LeviStone,
                    Item::AirShip => SplitOn::AirShip,
                    Item::WarpCube => SplitOn::WarpCube,
                    Item::BottledFaerie => SplitOn::BottledFaerie,
                    Item::Oxyale => SplitOn::Oxyale,
                    Item::RosettaStone => SplitOn::RosettaStone,
                    Item::Chime => SplitOn::Chime,
                });
            }
        }

        return None;
    }

    fn battle_check(
        &mut self,
        data: &Data,
        split: BattleSplit,
    ) -> Option<Result<Monster, NoBattle>> {
        let in_battle = self.in_battle.update_infallible(data.battle_active());
        if in_battle.current == false && in_battle.unchanged() {
            return Some(Err(NoBattle));
        }

        let monster = data.encounter()?;

        let result = data.battle_result();
        let result = self.battle_result.update_infallible(result);

        if in_battle.changed_to(&true) {
            log!("Encounter: {monster:?} -- Started");
            return None;
        }

        if in_battle.changed_to(&false) {
            if result.changed_from(&BattleResult::Win) {
                log!("Encounter: {monster:?} -- Ended");
                if split == BattleSplit::BattleEnd {
                    // Chaos is always split on animation
                    if monster != Monster::Chaos {
                        return Some(Ok(monster));
                    }
                }
            }

            if result.unchanged() && result.current == BattleResult::None {
                log!("Encounter: {monster:?} -- Reset");
            }

            return None;
        }

        if result.changed() {
            let result = result.current;
            log!("Encounter: {monster:?} -- {result:?}");
        }

        if monster == Monster::Chaos {
            if result.changed_to(&BattleResult::Win) {
                log!("Chaos defeated, GG!");
                let elapsed_time = data.battle_time();

                self.chaos_end = elapsed_time + {
                    const FRAMES: f32 = 120.0;
                    const FPS: f32 = 60.0;
                    // 2 seconds of "battle igt"
                    FRAMES / FPS
                };
            }

            if result.unchanged() && result.current == BattleResult::Win {
                let elapsed_time = data.battle_time();
                if elapsed_time > self.chaos_end {
                    self.chaos_end = f32::MAX;
                    return Some(Ok(monster));
                }
            }
        }

        if result.changed_to(&BattleResult::Win) {
            if split == BattleSplit::DeathAnimation {
                return Some(Ok(monster));
            }
        }

        return None;
    }

    fn field_check(&mut self, data: &Data) -> Option<Result<SplitOn, Location>> {
        let location = data.location()?;
        let location = self.location.update_infallible(location);
        if let Some(field) = SplitOn::from_watcher(location) {
            return Some(Ok(field));
        }

        Some(Err(location.current))
    }

    fn inventory_check(&mut self, data: &Data) -> Option<Item> {
        if let Some(item) = data.key_item_ids().find(|item| self.items.insert(item)) {
            log!("Picked up the {item:?}");
            return Some(item);
        }

        if let Some(vehicle) = data.vehicle_ids().find(|item| self.items.insert(item)) {
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
            _general,
            start,
            battle_split,
            _splits_heading1,
            _splits_heading2,
            _splits_heading3,
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
            .finish()
    }
}
