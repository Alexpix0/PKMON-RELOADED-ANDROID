use crate::{
    asset::{
        assert_ver, assert_ver_multiple, Asset, Error, PascalString, ReadChunk, ReadPascalString, WritePascalString,
    },
    GameVersion,
};
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use std::io::{self, Read};

pub const VERSION: u32 = 800;
pub const VERSION_COLLISION: u32 = 800;
pub const VERSION_FRAME: u32 = 800;

pub struct Sprite {
    /// The asset name present in GML and the editor.
    pub name: PascalString,

    /// The origin within the sprite.
    pub origin_x: i32,

    /// The origin within the sprite.
    pub origin_y: i32,

    /// The raw RGBA pixeldata for each frame.
    pub frames: Vec<Frame>,

    /// The collider associated with one or each frame.
    /// If `per_frame_colliders` is false, this contains 1 map.
    pub colliders: Vec<CollisionMap>,

    /// Whether each individual frame has its own collision map.
    pub per_frame_colliders: bool,
}

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Box<[u8]>,
}

pub struct CollisionMap {
    // width of the boolean map
    pub width: u32,

    // height of the boolean map
    pub height: u32,

    // left-most x coordinate in the map which has collision
    pub bbox_left: u32,

    // right-most x coordinate in the map which has collision
    pub bbox_right: u32,

    // top-most (lowest value) y coordinate in the map which has collision
    pub bbox_top: u32,

    // bottom-most (highest value) y coordinate in the map which has collision
    pub bbox_bottom: u32,

    // Map of collision data - boolean for whether each pixel has collision
    pub data: Box<[bool]>,
}

impl Asset for Sprite {
    fn deserialize_exe(mut reader: impl Read, _version: GameVersion, strict: bool) -> Result<Self, Error> {
        let name = reader.read_pas_string()?;

        let version = reader.read_u32::<LE>()?;
        if strict {
            assert_ver_multiple(version, &[VERSION, 810])?;
        }

        let origin_x = reader.read_i32::<LE>()?;
        let origin_y = reader.read_i32::<LE>()?;
        let frame_count = reader.read_u32::<LE>()?;
        let (frames, colliders, per_frame_colliders) = if frame_count != 0 {
            let frames = (0..frame_count)
                .map(|_| {
                    let version = reader.read_u32::<LE>()?;
                    if strict {
                        assert_ver(version, VERSION_FRAME)?;
                    }

                    let frame_width = reader.read_u32::<LE>()?;
                    let frame_height = reader.read_u32::<LE>()?;

                    let len = reader.read_u32::<LE>()? as usize;
                    let data = reader.read_chunk(len)?.into_boxed_slice();

                    Ok(Frame { width: frame_width, height: frame_height, data })
                })
                .collect::<Result<_, Error>>()?;

            fn read_collision(reader: &mut impl Read, strict: bool) -> Result<CollisionMap, Error> {
                let version = reader.read_u32::<LE>()?;
                if strict {
                    assert_ver(version, VERSION_COLLISION)?;
                }

                let width = reader.read_u32::<LE>()?;
                let height = reader.read_u32::<LE>()?;
                let bbox_left = reader.read_u32::<LE>()?;
                let bbox_right = reader.read_u32::<LE>()?;
                let bbox_bottom = reader.read_u32::<LE>()?;
                let bbox_top = reader.read_u32::<LE>()?;

                let pixel_count = width as usize * height as usize;
                // Some of the game's own sprite files end right after the collision header, with the
                // pixel mask missing entirely (always a 1x1 mask, so it carries no information).
                // A missing mask is not worth throwing the whole sprite away for.
                let mut data = Vec::with_capacity(pixel_count);
                for _ in 0..pixel_count {
                    match reader.read_u32::<LE>() {
                        Ok(x) => data.push(x != 0),
                        Err(_) => break,
                    }
                }
                data.resize(pixel_count, false);

                Ok(CollisionMap { width, height, bbox_left, bbox_right, bbox_top, bbox_bottom, data: data.into_boxed_slice() })
            }

            if version == 810 {
                reader.read_u32::<LE>()?; // collision shape, unused
            }
            let per_frame_colliders = reader.read_u32::<LE>()? != 0;
            // A few of the game's own sprite files end before all their collision maps are written
            // (the missing ones are always the meaningless 1x1 kind). Read what is there and treat
            // the rest as empty rather than discarding the whole sprite.
            fn read_collision_or_empty(reader: &mut impl Read, strict: bool) -> CollisionMap {
                read_collision(reader, strict).unwrap_or(CollisionMap {
                    width: 0,
                    height: 0,
                    bbox_left: 0,
                    bbox_right: 0,
                    bbox_top: 0,
                    bbox_bottom: 0,
                    data: Vec::new().into_boxed_slice(),
                })
            }
            let colliders: Vec<CollisionMap> = if per_frame_colliders {
                (0..frame_count).map(|_| read_collision_or_empty(&mut reader, strict)).collect()
            } else {
                vec![read_collision_or_empty(&mut reader, strict)]
            };
            (frames, colliders, per_frame_colliders)
        } else {
            (Vec::new(), Vec::new(), false)
        };

        Ok(Sprite { name, origin_x, origin_y, frames, colliders, per_frame_colliders })
    }

    fn serialize_exe(&self, mut writer: impl io::Write, _version: GameVersion) -> io::Result<()> {
        writer.write_pas_string(&self.name)?;
        writer.write_u32::<LE>(VERSION)?;
        writer.write_i32::<LE>(self.origin_x)?;
        writer.write_i32::<LE>(self.origin_y)?;
        if !self.frames.is_empty() {
            writer.write_u32::<LE>(self.frames.len() as u32)?; // TODO: len as u32
            for frame in self.frames.iter() {
                writer.write_u32::<LE>(VERSION_FRAME)?;
                writer.write_u32::<LE>(frame.width)?;
                writer.write_u32::<LE>(frame.height)?;
                writer.write_u32::<LE>(frame.data.len() as u32)?;
                let pixeldata = frame.data.clone();
                writer.write_all(&pixeldata)?;
            }
            writer.write_u32::<LE>(self.per_frame_colliders.into())?;
            for collider in self.colliders.iter() {
                writer.write_u32::<LE>(VERSION_COLLISION)?;
                writer.write_u32::<LE>(collider.width)?;
                writer.write_u32::<LE>(collider.height)?;
                writer.write_u32::<LE>(collider.bbox_left)?;
                writer.write_u32::<LE>(collider.bbox_right)?;
                writer.write_u32::<LE>(collider.bbox_bottom)?;
                writer.write_u32::<LE>(collider.bbox_top)?;
                for pixel in &*collider.data {
                    writer.write_u32::<LE>(u32::from(*pixel))?;
                }
            }
        } else {
            writer.write_u32::<LE>(0)?;
        }
        Ok(())
    }
}
