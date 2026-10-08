//! Sprite names and their rectangles in the sprite atlas
//! (`assets/sprites.png` + `assets/sprites.json`, produced by
//! `scripts/art/generate_art.py`).

use std::collections::HashMap;

use enum_map::{Enum, EnumMap};
use macroquad::math::Rect;
use serde::Deserialize;

/// Every sprite in the atlas. Character sprites face north; the engine
/// rotates them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpriteId {
    Hero1,
    Hero2,
    #[serde(rename = "rat_0")]
    Rat0,
    #[serde(rename = "rat_1")]
    Rat1,
    #[serde(rename = "rat_2")]
    Rat2,
    #[serde(rename = "rat_3")]
    Rat3,
    #[serde(rename = "cyborg_0")]
    Cyborg0,
    #[serde(rename = "cyborg_1")]
    Cyborg1,
    #[serde(rename = "cyborg_2")]
    Cyborg2,
    #[serde(rename = "cyborg_3")]
    Cyborg3,
    Plank,
    Web,
    Keg,
    HoleBase,
    HoleSwirl,
    TriggerPlate,
    Note,

    Glow,
    Shadow,
    Puff,
    Spark,
    Star,
    Ring,
    Slash,
    Shard,
    Splinter,
    Strand,
    Tuft,
    Confetti,

    IconUndo,
    IconRestart,
    IconWait,
    IconMap,
    IconMenu,
    IconGear,
    IconMusic,
    IconSound,
    IconMute,
    IconLock,
    IconCheck,
    IconStar,
    IconPlay,
    IconClose,
    IconSkull,
    IconSword,
    IconRat,
    IconSwipe,
    IconDrag,
    IconTap,
    IconPlayers,
    IconExport,
    IconImport,
    IconTrophy,
    IconChevron,
    IconDoor,

    PortraitHero1,
    PortraitHero2,

    NodeLocked,
    NodeOpen,
    NodeDone,

    PropBarrel,
    PropCrate,
    PropKegs,
    PropBookshelf,
    PropTable,
    PropCauldron,
    PropTesla,
    PropPipes,
    PropTree,
    PropBush,
    PropBrazier,
    PropBanner,
    PropCheese,
    PropBones,
    PropCandles,
    PropRug,
    PropStairs,
    PropWell,
    PropFlag,

    TitleRat,
}

impl SpriteId {
    pub(crate) const RAT_FRAMES: [SpriteId; 4] = [
        SpriteId::Rat0,
        SpriteId::Rat1,
        SpriteId::Rat2,
        SpriteId::Rat3,
    ];
    pub(crate) const CYBORG_FRAMES: [SpriteId; 4] = [
        SpriteId::Cyborg0,
        SpriteId::Cyborg1,
        SpriteId::Cyborg2,
        SpriteId::Cyborg3,
    ];
}

#[derive(Deserialize)]
struct AtlasFile {
    width: u32,
    height: u32,
    sprites: HashMap<SpriteId, PixelRect>,
}

#[derive(Deserialize)]
struct PixelRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// Where each sprite lives in the atlas texture.
pub(crate) struct Atlas {
    pub(crate) size: (u32, u32),
    /// Source rectangles in atlas pixels.
    pub(crate) rects: EnumMap<SpriteId, Rect>,
}

impl Atlas {
    pub(crate) fn parse(json: &str) -> Self {
        let file: AtlasFile = serde_json::from_str(json).expect("invalid sprite atlas JSON");
        let rects = EnumMap::from_fn(|id: SpriteId| {
            let r = file
                .sprites
                .get(&id)
                .unwrap_or_else(|| panic!("sprite atlas is missing {id:?}"));
            Rect::new(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
        });
        Self {
            size: (file.width, file.height),
            rects,
        }
    }
}
