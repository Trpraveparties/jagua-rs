use crate::collision_detection::CDEConfig;
use crate::entities::{Container, InferiorQualityZone};
use crate::geometry::primitives::{Point, Rect, SPolygon};
use crate::geometry::shape_modification::{ShapeModifyConfig, ShapeModifyMode};
use crate::geometry::{DTransformation, OriginalShape};
use anyhow::{Result, ensure};

#[derive(Clone, Debug, Copy, PartialEq)]
/// Represents a rectangular container with fixed height and variable width.
///
/// Patched for Quick Nesting (see vendor/PATCHES.md in nest-engine): a strip
/// may instead carry a fixed `outline`, a simple polygon the container takes
/// as its shape whatever the width says (an offcut sheet of any shape). The
/// width still moves for the algorithms that steer by it, but the container
/// stays the outline.
pub struct Strip {
    pub fixed_height: f32,
    pub cde_config: CDEConfig,
    pub shape_modify_config: ShapeModifyConfig,
    pub width: f32,
    /// Fixed outline of the sheet, if it is not a plain rectangle. `'static`
    /// so the strip stays `Copy`; callers intern their outlines (leak once).
    pub outline: Option<&'static [Point]>,
    /// Holes in the sheet (parts already cut out of an offcut), each a simple
    /// polygon inside the outline. Interned like `outline`.
    pub holes: Option<&'static [&'static [Point]]>,
}

impl Strip {
    pub fn new(
        fixed_height: f32,
        cde_config: CDEConfig,
        shape_modify_config: ShapeModifyConfig,
        width: f32,
    ) -> Result<Self> {
        ensure!(fixed_height > 0.0, "strip height must be positive");
        ensure!(width > 0.0, "strip width must be positive");
        Ok(Strip {
            fixed_height,
            cde_config,
            shape_modify_config,
            width,
            outline: None,
            holes: None,
        })
    }

    /// A strip whose container is always `outline` (a simple polygon inside
    /// `[0, width] x [0, fixed_height]`).
    pub fn with_outline(mut self, outline: &'static [Point]) -> Result<Self> {
        ensure!(outline.len() >= 3, "sheet outline needs at least 3 points");
        SPolygon::new(outline.to_vec())?;
        self.outline = Some(outline);
        Ok(self)
    }

    /// The sheet's holes: no part may lie over them.
    pub fn with_holes(mut self, holes: &'static [&'static [Point]]) -> Result<Self> {
        for h in holes {
            ensure!(h.len() >= 3, "a sheet hole needs at least 3 points");
            SPolygon::new(h.to_vec())?;
        }
        self.holes = if holes.is_empty() { None } else { Some(holes) };
        Ok(self)
    }

    pub fn set_width(&mut self, width: f32) {
        assert!(width > 0.0, "strip width must be positive");
        self.width = width;
    }
}

impl From<Strip> for Container {
    fn from(s: Strip) -> Container {
        let shape = match s.outline {
            Some(points) => SPolygon::new(points.to_vec()).unwrap(),
            None => SPolygon::from(Rect::try_new(0.0, 0.0, s.width, s.fixed_height).unwrap()),
        };
        let quality_zones = match s.holes {
            Some(holes) => vec![
                InferiorQualityZone::new(
                    0,
                    holes
                        .iter()
                        .map(|h| OriginalShape {
                            shape: SPolygon::new(h.to_vec()).unwrap(),
                            pre_transform: DTransformation::empty(),
                            // a hole may only grow when simplified: parts stay off it
                            modify_mode: ShapeModifyMode::Inflate,
                            modify_config: s.shape_modify_config,
                        })
                        .collect(),
                )
                .unwrap(),
            ],
            None => vec![],
        };
        Container::new(
            0,
            OriginalShape {
                shape,
                pre_transform: DTransformation::empty(),
                modify_mode: ShapeModifyMode::Deflate,
                modify_config: s.shape_modify_config,
            },
            quality_zones,
            s.cde_config,
        )
        .unwrap()
    }
}
