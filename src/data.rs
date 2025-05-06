use asr::{
    arrayvec::ArrayVec,
    game_engine::unity::il2cpp::{Class2, Game},
    Pointer,
};

use csharp_mem::{Array, List, Map};
use num_enum::FromPrimitive;

pub struct Data {
    battles: Battles,
    items: Items,
}

impl Data {
    pub fn battle_active(&mut self, game: &Game) -> bool {
        self.battles.active(game)
    }

    pub fn battle_info(&mut self, game: &Game) -> Option<BattleInfo> {
        self.battles.info(game)
    }

    pub fn inventory(&mut self, game: &Game) -> Option<Inventory> {
        self.items.inventory(game)
    }
}

#[derive(Debug)]
pub struct BattleInfo {
    pub playing: bool,
    pub result: BattleResult,
    pub encounter_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub key_items: ArrayVec<u32, 16>,
    pub vehicles: ArrayVec<u32, 4>,
}

impl Data {
    pub fn new() -> Self {
        Self {
            battles: Battles::new(),
            items: Items::new(),
        }
    }
}

#[derive(Class2, Debug)]
struct BattlePlugManager {
    #[singleton]
    #[rename = "instance"]
    _instance: Pointer<Self>,
    #[rename = "<InstantiateManager>k__BackingField"]
    instantiate_manager: Pointer<InstantiateManager>,
    #[rename = "<BattleEndJugment>k__BackingField"]
    judgement: Pointer<BattleEndJugment>,
    #[rename = "<EventCommand>k__BackingField"]
    event: Pointer<EventCommand>,
    #[rename = "isBattle"]
    active: bool,
}

#[derive(Class2, Debug)]
struct InstantiateManager {
    #[rename = "<battleEnemyInstanceData>k__BackingField"]
    enemy_data: Pointer<BattleEnemyInstanceData>,
}

#[derive(Class2, Debug)]
struct BattleEnemyInstanceData {
    #[rename = "<monsterParty>k__BackingField"]
    monster_party: Pointer<MonsterParty>,
}

#[derive(Class2, Debug)]
struct MonsterParty {
    #[rename = "valueIntList"]
    values: Pointer<Array<u32>>,
}

impl MonsterParty {
    const ID_INDEX: usize = 0;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum BattleResult {
    None = 0,
    Win = 1,
    Lose = 2,
    Escape = 3,
    Forced = 4,
    Restart = 5,
    #[num_enum(default)]
    Unknown = u32::MAX,
}

#[derive(Class2, Debug)]
struct BattleEndJugment {
    #[rename = "resultType"]
    result: u32,
}

#[derive(Class2, Debug)]
struct EventCommand {
    #[rename = "<isBattlePlay>k__BackingField"]
    battle_play: bool,
}

#[derive(Class2, Debug)]
struct UserDataManager {
    #[singleton]
    #[rename = "instance"]
    _instance: Pointer<Self>,

    #[rename = "importantOwendItems"]
    key_items: Pointer<Map<u32, Pointer<OwnedItemData>>>,

    #[rename = "<OwnedTransportationList>k__BackingField"]
    vehicles: Pointer<List<Pointer<OwnedTransportationData>>>,
}

#[derive(Class2, Debug)]
struct OwnedItemData {
    #[rename = "<ItemId>k__BackingField"]
    item_id: u32,
}

#[derive(Class2, Debug)]
struct OwnedTransportationData {
    #[rename = "saveData"]
    data: Pointer<SaveTransportationData>,
}

#[derive(Class2, Debug)]
struct SaveTransportationData {
    id: u32,
    #[rename = "mapId"]
    map_id: i32,
}

struct Battles {
    manager: BattlePlugManagerBinding,
    instantiate: InstantiateManagerBinding,
    judgement: BattleEndJugmentBinding,
    enemy_instance: BattleEnemyInstanceDataBinding,
    event_command: EventCommandBinding,
    monster_party: MonsterPartyBinding,
}

impl Battles {
    fn new() -> Self {
        Self {
            manager: BattlePlugManager::bind(),
            instantiate: InstantiateManager::bind(),
            judgement: BattleEndJugment::bind(),
            enemy_instance: BattleEnemyInstanceData::bind(),
            event_command: EventCommand::bind(),
            monster_party: MonsterParty::bind(),
        }
    }

    fn active(&mut self, game: &Game<'_>) -> bool {
        self.manager
            .read(game)
            .map(|m| m.active)
            .unwrap_or_default()
    }

    fn info(&mut self, game: &Game<'_>) -> Option<BattleInfo> {
        let manager = self.manager.read(game)?;

        let instantiate = self
            .instantiate
            .read_pointer(game, manager.instantiate_manager)?;

        let event_command = self.event_command.read_pointer(game, manager.event)?;

        let playing = event_command.battle_play;

        let judgement = self.judgement.read_pointer(game, manager.judgement)?;
        let result = BattleResult::from(judgement.result);

        let enemy_data = self
            .enemy_instance
            .read_pointer(game, instantiate.enemy_data)?;

        let monster_party = self
            .monster_party
            .read_pointer(game, enemy_data.monster_party)?;
        let monster_party = monster_party.values.resolve(game)?;

        let encounter_id = monster_party.get(game, MonsterParty::ID_INDEX)?;

        let result = BattleInfo {
            playing,
            result,
            encounter_id,
        };

        Some(result)
    }
}

struct Items {
    user_data: UserDataManagerBinding,
    item_data: OwnedItemDataBinding,
    transport_data: OwnedTransportationDataBinding,
    save_transport: SaveTransportationDataBinding,
}

impl Items {
    fn new() -> Self {
        Self {
            user_data: UserDataManager::bind(),
            item_data: OwnedItemData::bind(),
            transport_data: OwnedTransportationData::bind(),
            save_transport: SaveTransportationData::bind(),
        }
    }

    fn inventory(&mut self, game: &Game<'_>) -> Option<Inventory> {
        let manager = self.user_data.read(game)?;

        let key_items = manager.key_items.resolve(game)?;

        let key_items = key_items
            .iter(game)
            .filter_map(|(_, item)| self.item_data.read_pointer(game, item).map(|i| i.item_id))
            .collect();

        let vehicles = manager.vehicles.resolve(game)?;

        let vehicles = vehicles
            .iter(game)
            .filter_map(|vehicle| {
                let vehicle = self.transport_data.read_pointer(game, vehicle)?;
                let vehicle = self.save_transport.read_pointer(game, vehicle.data)?;
                let _ = u32::try_from(vehicle.map_id).ok()?;

                Some(vehicle.id)
            })
            .collect();

        Some(Inventory {
            key_items,
            vehicles,
        })
    }
}
