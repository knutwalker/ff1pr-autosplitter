use asr::{
    game_engine::unity::il2cpp::{Class, Image, Module, UnityPointer},
    Address, Address64, Process,
};
use bytemuck::{AnyBitPattern, CheckedBitPattern};
use core::{fmt, marker::PhantomData, mem::size_of};
use num_enum::{FromPrimitive, IntoPrimitive};

#[derive(Copy, Clone, Debug, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Location {
    area_type: u32,
    area_id: u32,
    map_id: u32,
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
    pub fn battles(&self) -> Battles<'_> {
        Battles {
            data: &self.battles,
            process: self.process,
            module: self.module,
            image: self.image,
        }
    }

    pub fn items(&self) -> Items<'_> {
        Items {
            data: &self.items,
            process: self.process,
            module: self.module,
            image: self.image,
        }
    }

    pub fn user(&self) -> User<'_> {
        User {
            data: &self.user,
            process: self.process,
            module: self.module,
            image: self.image,
        }
    }

    pub fn has_fade_out(&self) -> bool {
        self.new_game
            .has_fade_out(self.process, self.module, self.image)
            .unwrap_or(false)
    }
}

pub struct Battles<'a> {
    data: &'a BattleData,
    process: &'a Process,
    module: &'a Module,
    image: &'a Image,
}

impl Battles<'_> {
    const ENCOUNTER_ID_INDEX: usize = 0;

    pub fn active(&self) -> bool {
        self.data
            .active
            .deref(self.process, self.module, self.image)
            .unwrap_or_default()
    }

    pub fn encounter_id(&self) -> Option<u32> {
        self.data
            .monster_party
            .deref::<Pointer<Array<u32>>>(self.process, self.module, self.image)
            .ok()?
            .get(self.process, Self::ENCOUNTER_ID_INDEX)
    }

    pub fn playing(&self) -> bool {
        self.data
            .is_playing
            .deref(self.process, self.module, self.image)
            .unwrap_or_default()
    }

    pub fn result(&self) -> BattleResult {
        let result = self
            .data
            .end_result
            .deref::<u32>(self.process, self.module, self.image)
            .unwrap_or(BattleResult::Unknown.into());

        BattleResult::from(result)
    }

    pub fn elapsed_time(&self) -> f32 {
        self.data
            .elapsed_time
            .deref(self.process, self.module, self.image)
            .unwrap_or_default()
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
    is_playing: UnityPointer<3>,
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
        let is_playing = ptr_path(
            "BattlePlugManager",
            [
                "instance",
                "<EventCommand>k__BackingField",
                "<isBattlePlay>k__BackingField",
            ],
        );
        let elapsed_time = ptr_path("BattlePlugManager", ["instance", "elapsedTime"]);

        Self {
            active,
            monster_party,
            end_result,
            is_playing,
            elapsed_time,
        }
    }
}

#[derive(Class, Debug)]
struct OwnedItemData {
    #[rename = "<ItemId>k__BackingField"]
    item_id: u32,
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
    item_data: OwnedItemDataBinding,
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

        let item_data = OwnedItemData::bind(process, module, image).await;
        let transport_data = OwnedTransportationData::bind(process, module, image).await;
        let save_transport = SaveTransportationData::bind(process, module, image).await;

        Self {
            key_items,
            vehicles,
            item_data,
            transport_data,
            save_transport,
        }
    }
}

pub struct Items<'a> {
    data: &'a ItemsData,
    process: &'a Process,
    module: &'a Module,
    image: &'a Image,
}

impl Items<'_> {
    pub fn key_items_count(&self) -> u32 {
        let Ok(key_items) = self
            .data
            .key_items
            .deref::<Pointer<Map<u32, Pointer<OwnedItemData>>>>(
                self.process,
                self.module,
                self.image,
            )
        else {
            return 0;
        };

        key_items.size(self.process).unwrap_or_default()
    }
}

impl<'a> Items<'a> {
    pub fn key_item_ids(&self) -> impl Iterator<Item = u32> + 'a {
        self.data
            .key_items
            .deref::<Pointer<Map<u32, Pointer<OwnedItemData>>>>(
                self.process,
                self.module,
                self.image,
            )
            .into_iter()
            .filter_map(|key_items| key_items.iter(self.process))
            .flatten()
            .filter_map(|(_, item)| {
                self.data
                    .item_data
                    .read(self.process, item.addr())
                    .ok()
                    .map(|i| i.item_id)
            })
    }

    pub fn vehicle_ids(&self) -> impl Iterator<Item = u32> + 'a {
        self.data
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
                    .data
                    .transport_data
                    .read(self.process, vehicle.addr())
                    .ok()?;
                let vehicle = self
                    .data
                    .save_transport
                    .read(self.process, vehicle.data.addr())
                    .ok()?;
                let _ = u32::try_from(vehicle.map_id).ok()?;

                Some(vehicle.id)
            })
    }
}

struct UserData {
    area_type: UnityPointer<2>,
    area_id: UnityPointer<2>,
    map_id: UnityPointer<2>,
    igt: UnityPointer<3>,
}

impl UserData {
    fn new() -> Self {
        let area_type = ptr_path(
            "UserDataManager",
            ["instance", "<CurrentAreaType>k__BackingField"],
        );
        let area_id = ptr_path(
            "UserDataManager",
            ["instance", "<CurrentAreaId>k__BackingField"],
        );
        let map_id = ptr_path(
            "UserDataManager",
            ["instance", "<CurrentMapId>k__BackingField"],
        );
        let igt = ptr_path("UserDataManager", ["instance", "saveData", "playTime"]);

        Self {
            area_type,
            area_id,
            map_id,
            igt,
        }
    }
}

pub struct User<'a> {
    data: &'a UserData,
    process: &'a Process,
    module: &'a Module,
    image: &'a Image,
}

impl<'a> User<'a> {
    pub fn igt(&self) -> f64 {
        self.data
            .igt
            .deref(self.process, self.module, self.image)
            .unwrap_or_default()
    }

    pub fn location(&self) -> Option<Location> {
        let area_type = self
            .data
            .area_type
            .deref(self.process, self.module, self.image)
            .ok()?;
        let area_id = self
            .data
            .area_id
            .deref(self.process, self.module, self.image)
            .ok()?;
        let map_id = self
            .data
            .map_id
            .deref(self.process, self.module, self.image)
            .ok()?;

        Some(Location {
            area_type,
            area_id,
            map_id,
        })
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

impl<K: 'static, V: 'static> Pointer<Map<K, V>> {
    fn size<R: MemReader>(self, reader: &R) -> Option<u32> {
        let map = self.read(reader)?;
        Some(map.size)
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
