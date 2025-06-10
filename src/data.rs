use asr::{
    game_engine::unity::il2cpp::{Class, Image, Module, UnityPointer},
    Address, Address64, Process,
};
use bytemuck::{AnyBitPattern, CheckedBitPattern};
use core::{fmt, marker::PhantomData, mem::size_of};
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum Location {
    WorldMap = 1,
    CastleCornelia = 2,
    CorneliaThrone = 3,
    Cornelia = 4,
    CorneliaItemShop = 6,
    MatoyaCave = 12,
    Pravoka = 13,
    Elfenheim = 22,
    ElfenheimItemShop = 24,
    _ElfenheimBMShop = 31,
    ElvenCastle = 32,
    WesternKeep = 33,
    Melmond = 34,
    _MelmondArmorShop = 37,
    MelmondBMShop = 39,
    SageCave = 40,
    CrescentLake = 41,
    CLItemShop = 43,
    _CrescentLakeBMShop = 48,
    Onrac = 52,
    OnracItemShop = 54,
    OasisShop = 59,
    Gaia = 60,
    GaiaItemShop = 62,
    _GaiaBMShop = 67,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
pub enum Item {
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

impl super::EnumSetMember for Item {
    fn ordinal(&self) -> Option<u8> {
        u8::try_from(u32::from(*self)).ok()
    }
}

pub struct Data<'a> {
    new_game: NewGame,
    battles: BattleData,
    items: ItemsData,
    user: UserData,
    process: &'a Process,
    module: &'a Module,
    image: &'a Image,
}

impl<'a> Data<'a> {
    pub async fn new(process: &'a Process, module: &'a Module, image: &'a Image) -> Self {
        Self {
            new_game: NewGame::new(),
            battles: BattleData::new(),
            items: ItemsData::new(process, module, image).await,
            user: UserData::new(),
            process,
            module,
            image,
        }
    }
}

impl Data<'_> {
    const ENCOUNTER_ID_INDEX: usize = 0;

    pub fn battle_active(&self) -> bool {
        self.battles
            .active
            .deref(self.process, self.module, self.image)
            .unwrap_or_default()
    }

    pub fn encounter(&self) -> Option<Monster> {
        self.battles
            .monster_party
            .deref::<Pointer<Array<_>>>(self.process, self.module, self.image)
            .ok()?
            .get(self.process, Self::ENCOUNTER_ID_INDEX)
            .and_then(|id| Monster::try_from_primitive(id).ok())
    }

    pub fn battle_result(&self) -> BattleResult {
        self.battles
            .end_result
            .deref::<u32>(self.process, self.module, self.image)
            .map_or(BattleResult::Unknown, BattleResult::from)
    }

    pub fn battle_time(&self) -> f32 {
        self.battles
            .elapsed_time
            .deref(self.process, self.module, self.image)
            .unwrap_or_default()
    }

    pub fn key_item_ids(&self) -> impl Iterator<Item = Item> + '_ {
        self.items
            .key_items
            .deref::<Pointer<Map<u32, Pointer<()>>>>(self.process, self.module, self.image)
            .into_iter()
            .filter_map(|key_items| key_items.iter(self.process))
            .flatten()
            .map(|(item_id_plus_1, _)| item_id_plus_1 - 1)
            .filter_map(|item_id| Item::try_from_primitive(item_id).ok())
    }

    pub fn vehicle_ids(&self) -> impl Iterator<Item = Item> + '_ {
        self.items
            .vehicles
            .deref::<Pointer<List<Pointer<OwnedTransportationData>>>>(
                self.process,
                self.module,
                self.image,
            )
            .into_iter()
            .filter_map(|vehicles| vehicles.iter(self.process))
            .flatten()
            .filter_map(|vehicle| {
                let vehicle = self
                    .items
                    .transport_data
                    .read(self.process, vehicle.addr())
                    .ok()?;
                let vehicle = self
                    .items
                    .save_transport
                    .read(self.process, vehicle.data.addr())
                    .ok()?;

                let item = Item::try_from(vehicle.id).ok()?;
                let _ = u32::try_from(vehicle.map_id).ok()?;

                Some(item)
            })
    }

    pub fn location(&self) -> Option<Location> {
        self.user
            .map_id
            .deref(self.process, self.module, self.image)
            .ok()
            .and_then(|id| Location::try_from_primitive(id).ok())
    }

    pub fn has_fade_out(&self) -> bool {
        self.new_game
            .has_fade_out(self.process, self.module, self.image)
            .unwrap_or(false)
    }
}

fn ptr_path<const N: usize>(cls: &'static str, path: [&'static str; N]) -> UnityPointer<N> {
    UnityPointer::new(cls, 0, &path)
}

struct NewGame {
    fade_out_finish: UnityPointer<2>,
}

impl NewGame {
    fn new() -> Self {
        let fade_out_finish =
            UnityPointer::new("FadeManager", 1, &["instance", "fadeOutFinishedCallback"]);
        Self { fade_out_finish }
    }

    fn has_fade_out(&self, process: &Process, module: &Module, image: &Image) -> Option<bool> {
        let ptr = self
            .fade_out_finish
            .deref::<Address64>(process, module, image)
            .ok()?;
        Some(ptr.is_null() == false)
    }
}

struct BattleData {
    active: UnityPointer<2>,
    monster_party: UnityPointer<5>,
    end_result: UnityPointer<3>,
    elapsed_time: UnityPointer<2>,
}

impl BattleData {
    fn new() -> Self {
        let active = ptr_path("BattlePlugManager", ["instance", "isBattle"]);
        let monster_party = ptr_path(
            "BattlePlugManager",
            [
                "instance",
                "<InstantiateManager>k__BackingField",
                "<battleEnemyInstanceData>k__BackingField",
                "<monsterParty>k__BackingField",
                "valueIntList",
            ],
        );
        let end_result = ptr_path(
            "BattlePlugManager",
            [
                "instance",
                "<BattleEndJugment>k__BackingField",
                "resultType",
            ],
        );
        let elapsed_time = ptr_path("BattlePlugManager", ["instance", "elapsedTime"]);

        Self {
            active,
            monster_party,
            end_result,
            elapsed_time,
        }
    }
}

#[derive(Class, Debug)]
struct OwnedTransportationData {
    #[rename = "saveData"]
    data: Pointer<SaveTransportationData>,
}

#[derive(Class, Debug)]
struct SaveTransportationData {
    id: u32,
    #[rename = "mapId"]
    map_id: i32,
}

struct ItemsData {
    key_items: UnityPointer<2>,
    vehicles: UnityPointer<2>,
    transport_data: OwnedTransportationDataBinding,
    save_transport: SaveTransportationDataBinding,
}

impl ItemsData {
    async fn new(process: &Process, module: &Module, image: &Image) -> Self {
        let key_items = ptr_path("UserDataManager", ["instance", "importantOwendItems"]);
        let vehicles = ptr_path(
            "UserDataManager",
            ["instance", "<OwnedTransportationList>k__BackingField"],
        );

        let transport_data = OwnedTransportationData::bind(process, module, image).await;
        let save_transport = SaveTransportationData::bind(process, module, image).await;

        Self {
            key_items,
            vehicles,
            transport_data,
            save_transport,
        }
    }
}

struct UserData {
    map_id: UnityPointer<2>,
}

impl UserData {
    fn new() -> Self {
        let map_id = ptr_path(
            "UserDataManager",
            ["instance", "<CurrentMapId>k__BackingField"],
        );

        Self { map_id }
    }
}

/// Trait for things that can read data from memory.
trait MemReader: Sized {
    /// Reads a value from memory.
    fn read<T: CheckedBitPattern, A: Into<Address>>(&self, addr: A) -> Option<T>;
}

impl MemReader for Process {
    fn read<T: CheckedBitPattern, A: Into<Address>>(&self, addr: A) -> Option<T> {
        self.read(addr).ok()
    }
}

/// A pointer to a value in memory.
/// This type has the same memory layout as an [`Address64`] and
/// can be used in place of it, typically in classes derived when
/// the `derive` feature is enabled and used.
/// Using this type instead of [`Address64`] can give a bit more
/// type safety.
#[repr(C)]
struct Pointer<T> {
    address: Address64,
    _t: PhantomData<T>,
}

impl<T: CheckedBitPattern> Pointer<T> {
    /// Read a value from memory by following this pointer.
    fn read<R: MemReader>(self, reader: &R) -> Option<T> {
        if self.address.is_null() {
            None
        } else {
            reader.read(self.address)
        }
    }
}

impl<T> Pointer<T> {
    /// Return the address of this pointer.
    const fn address(self) -> Address64 {
        self.address
    }

    /// Return the address of this pointer as generic `Address`.
    fn addr(self) -> Address {
        self.address.into()
    }
}

impl<T: CheckedBitPattern + 'static> Pointer<Array<T>> {
    fn iter<R: MemReader>(self, reader: &R) -> Option<ArrayIter<'_, T, R>> {
        let array = self.read(reader)?;
        let start = self.address() + Array::<T>::DATA;
        let end = start + (size_of::<T>() * array.size as usize) as u64;

        Some(ArrayIter {
            pos: start,
            end,
            reader,
            _t: PhantomData,
        })
    }

    fn get<R: MemReader>(self, reader: &R, index: usize) -> Option<T> {
        let array = self.read(reader)?;
        if index >= array.size as usize {
            return None;
        }
        let offset = self.address() + Array::<T>::DATA + (index * size_of::<T>()) as u64;
        reader.read(offset)
    }
}

impl<T: CheckedBitPattern + 'static> Pointer<List<T>> {
    fn iter<R: MemReader>(self, reader: &R) -> Option<impl Iterator<Item = T> + '_> {
        let list = self.read(reader)?;
        Some(list.items.iter(reader)?.take(list.size as _))
    }
}

impl<K: AnyBitPattern + 'static, V: AnyBitPattern + 'static> Pointer<Map<K, V>> {
    fn iter<R: MemReader>(self, reader: &R) -> Option<impl Iterator<Item = (K, V)> + '_> {
        let map = self.read(reader)?;
        Some(
            map.entries
                .iter(reader)?
                .filter(|o| o._hash != 0 || o._next != 0)
                .take(map.size as _)
                .map(|o| (o.key, o.value)),
        )
    }
}

impl<T> From<Pointer<T>> for Address {
    fn from(ptr: Pointer<T>) -> Self {
        ptr.address.into()
    }
}

impl<T> From<Pointer<T>> for Address64 {
    fn from(ptr: Pointer<T>) -> Self {
        ptr.address
    }
}

impl<T> fmt::Debug for Pointer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pointer")
            .field("address", &self.address)
            .field("type", &core::any::type_name::<T>())
            .finish()
    }
}

// This is a manual implementation and not derived because the derive
// implementation would add a `T: Copy` bound, which is not required.
impl<T> ::core::marker::Copy for Pointer<T> {}

// This is a manual implementation and not derived because the derive
// implementation would add a `T: Clone` bound, which is not required.
impl<T> ::core::clone::Clone for Pointer<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// This is a manual implementation and not derived because the derive
// macro would add a `T: AnyBitPattern` bound, which is not required.
//
// SAFETY:
// Similar to raw pointers, a pointer is valid for any bit pattern
// Dereferencing the pointer is not, though.
unsafe impl<T: 'static> ::bytemuck::AnyBitPattern for Pointer<T> {}

// This is a manual implementation and not derived because the derive
// macro would add a `T: Zeroable` bound, which is not required.
//
// SAFETY:
// A zeroed pointer is the null pointer, and it is a valid pointer.
// It must not be derreferenced, though.
unsafe impl<T: 'static> ::bytemuck::Zeroable for Pointer<T> {}

#[repr(C)]
struct Array<T> {
    _type_id: u64,
    _header: u64,
    _header2: u64,
    size: u32,
    _t: PhantomData<T>,
}

impl<T> Array<T> {
    const DATA: u64 = 0x20;
}

const _: () = {
    assert!(size_of::<Array<()>>() == Array::<()>::DATA as usize);
};

struct ArrayIter<'a, T, R> {
    pos: Address64,
    end: Address64,
    reader: &'a R,
    _t: PhantomData<T>,
}

impl<T: CheckedBitPattern, R: MemReader> Iterator for ArrayIter<'_, T, R> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.end {
            return None;
        }

        let item: T = self.reader.read(self.pos)?;

        self.pos = self.pos + size_of::<T>() as u64;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end.value().saturating_sub(self.pos.value()) as usize;
        (remaining, Some(remaining))
    }
}

impl<T> fmt::Debug for Array<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Array")
            .field("size", &self.size)
            .field("type", &core::any::type_name::<T>())
            .finish()
    }
}

// This is a manual implementation and not derived because the derive
// implementation would add a `T: Copy` bound, which is not required.
impl<T> ::core::marker::Copy for Array<T> {}

// This is a manual implementation and not derived because the derive
// implementation would add a `T: Clone` bound, which is not required.
impl<T> ::core::clone::Clone for Array<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// This is a manual implementation and not derived because the derive
// macro would add a `T: AnyBitPattern` bound, which is not required.
//
// SAFETY:
// While technically not any bit pattern is allowed, we are ignoring
// the C# object header internals, so for the purpose of this type
// they can indeed be anything.
unsafe impl<T: 'static> ::bytemuck::AnyBitPattern for Array<T> {}

// This is a manual implementation and not derived because the derive
// macro would add a `T: Zeroable` bound, which is not required.
//
// SAFETY:
// Similar to the logic for AnyBitPattern, we accept zeroed values
// because we only care about the size field and that one is ok
// to be zero.
unsafe impl<T: 'static> ::bytemuck::Zeroable for Array<T> {}

#[repr(C)]
struct List<T> {
    _type_id: u64,
    _header: u64,
    items: Pointer<Array<T>>,
    size: u32,
}

impl<T> fmt::Debug for List<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("List")
            .field("items", &self.items)
            .field("size", &self.size)
            .finish()
    }
}

// This is a manual implementation and not derived because the derive
// implementation would add a `T: Copy` bound, which is not required.
impl<T> ::core::marker::Copy for List<T> {}

// This is a manual implementation and not derived because the derive
// implementation would add a `T: Clone` bound, which is not required.
impl<T> ::core::clone::Clone for List<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// This is a manual implementation and not derived because the derive
// macro would add a `T: AnyBitPattern` bound, which is not required.
unsafe impl<T: 'static> ::bytemuck::AnyBitPattern for List<T> {}

// This is a manual implementation and not derived because the derive
// macro would add a `T: Zeroable` bound, which is not required.
unsafe impl<T: 'static> ::bytemuck::Zeroable for List<T> {}

#[repr(C)]
struct Map<K, V> {
    _type_id: u64,
    _header: u64,
    _header_2: u64,
    entries: Pointer<Array<Entry<K, V>>>,
    size: u32,
}

#[derive(Copy, Clone, Debug, AnyBitPattern)]
#[repr(C)]
struct Entry<K, V> {
    _hash: u32,
    _next: u32,
    key: K,
    value: V,
}

impl<K, V> fmt::Debug for Map<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map")
            .field("entries", &self.entries)
            .field("size", &self.size)
            .finish()
    }
}

// This is a manual implementation and not derived because the derive
// implementation would add `K: Copy` and `V: Copy` bounds, which is
// not required.
impl<K, V> ::core::marker::Copy for Map<K, V> {}

// This is a manual implementation and not derived because the derive
// implementation would add `K: Clone` and `V: Clone` bounds, which is
// not required.
impl<K, V> ::core::clone::Clone for Map<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

// This is a manual implementation and not derived because the derive
// macro would add `K: AnyBitPattern` and `V: AnyBitPattern` bounds,
// which is not required.
unsafe impl<K: 'static, V: 'static> ::bytemuck::AnyBitPattern for Map<K, V> {}

// This is a manual implementation and not derived because the derive
// macro would add `K: Zeroable` and `V: Zeroable` bounds, which is
// not required.
unsafe impl<K: 'static, V: 'static> ::bytemuck::Zeroable for Map<K, V> {}
