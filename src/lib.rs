use crate::{
    data::Data,
    game::{Monster, SplitOn},
    splits::{Action, Progress},
};
use asr::{
    future::next_tick,
    timer::{self, TimerState},
    Process,
};

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
mod game;
mod splits;

asr::async_main!(stable);

enum State<'a> {
    NotRunning,
    Running(Data<'a>, Progress),
}

async fn main() {
    asr::set_tick_rate(60.0);
    let settings = Settings::register();
    log!("Loaded settings: {settings:?}");

    loop {
        let process = Process::wait_attach("FINAL FANTASY.exe").await;
        log!("attached to process");
        process
            .until_closes(async {
                let mut state = State::NotRunning;

                'outer: loop {
                    match state {
                        State::NotRunning => {
                            if timer::state() == TimerState::Running {
                                state = State::Running(Data::new(&process).await, Progress::new());
                                continue 'outer;
                            }
                        }
                        State::Running(ref mut data, ref mut progress) => {
                            match timer::state() {
                                TimerState::NotRunning => {
                                    state = State::NotRunning;
                                    continue 'outer;
                                }
                                TimerState::Running => progress.running(data),
                                _ => {}
                            }

                            for action in progress.actions() {
                                act(action, &settings);
                            }
                        }
                    }

                    next_tick().await;
                }
            })
            .await;
    }
}

#[derive(Debug, asr::user_settings::Settings)]
pub struct Settings {
    /// Split battles on death animation (checked) or after spoils screens (unchecked).
    #[default = false]
    split_on_death_animation: bool,

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

impl Settings {
    fn filter(&self, action: &Action) -> bool {
        let Action::Split(s) = action else {
            return false;
        };
        return match s {
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
            SplitOn::Pickup(game::Pickup::Lute) => self.lute,
            SplitOn::Pickup(game::Pickup::Ship) => self.ship,
            SplitOn::Pickup(game::Pickup::Crown) => self.crown,
            SplitOn::Pickup(game::Pickup::CrystalEye) => self.crystal_eye,
            SplitOn::Pickup(game::Pickup::Tonic) => self.tonic,
            SplitOn::Pickup(game::Pickup::MysticKey) => self.mystic_key,
            SplitOn::Pickup(game::Pickup::Nitro) => self.nitro,
            SplitOn::Pickup(game::Pickup::StarRuby) => self.star_ruby,
            SplitOn::Pickup(game::Pickup::EarthRod) => self.earth_rod,
            SplitOn::Pickup(game::Pickup::Canoe) => self.canoe,
            SplitOn::Pickup(game::Pickup::LeviStone) => self.levi_stone,
            SplitOn::Pickup(game::Pickup::AirShip) => self.air_ship,
            SplitOn::Pickup(game::Pickup::WarpCube) => self.warp_cube,
            SplitOn::Pickup(game::Pickup::BottledFaerie) => self.bottled_faerie,
            SplitOn::Pickup(game::Pickup::Oxyale) => self.oxyale,
            SplitOn::Pickup(game::Pickup::RosettaStone) => self.rosetta_stone,
            SplitOn::Pickup(game::Pickup::Chime) => self.chime,
        };
    }
}

fn act(action: Action, settings: &Settings) {
    if settings.filter(&action) {
        log!("Decided on an action: {action:?}");
        match action {
            Action::Start => {
                log!("Starting timer");
                timer::start();
            }
            Action::Split(split) => {
                log!("Splitting: {split:?}");
                timer::split();
            }
            Action::Pause => {
                log!("Pause game time");
                timer::pause_game_time();
            }
            Action::Resume => {
                log!("Resume game time");
                timer::resume_game_time();
            }
            _ => {}
        }
    }
}
