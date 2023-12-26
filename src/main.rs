//! This example demonstrates the use of the [`Grid`] helper.

use kurbo::{Affine, Point, Vec2};
use whiskers::prelude::*;

#[derive(Sketch)]
struct GridSketch {
    #[param(slider, min = 20.0, max = 400.0)]
    width: f64,
    #[param(slider, min = 20.0, max = 400.0)]
    height: f64,
}

struct Tile {
    corners: Vec<Point>,
}

struct TilePlacement {
    tile_id: usize,
    offset: Point,
    scale: f64,
    rotation: f64,
}
struct TilingRule {
    tile: Tile,
    result: Vec<TilePlacement>,
}

struct TilingStep {
    rules: Vec<TilingRule>,
}

fn expand_tile(placedTile: &TilePlacement, rules: &TilingStep, output: &mut Vec<TilePlacement>) {
    let rule = &rules.rules[placedTile.tile_id];
    for item in &rule.result {}
}

impl Default for GridSketch {
    fn default() -> Self {
        Self {
            width: 100.0,
            height: 100.0,
        }
    }
}

impl App for GridSketch {
    fn update(&mut self, sketch: &mut Sketch, _ctx: &mut Context) -> anyhow::Result<()> {
        sketch.scale(Unit::Mm);
        sketch.stroke_width(0.3);

        sketch.rect(0f64, 0f64, self.height, self.width);
        Ok(())
    }
}

fn main() -> Result {
    Runner::new(GridSketch::default())
        .with_page_size_options(PageSize::A5H)
        .with_layout_options(LayoutOptions::Center)
        .run()
}
