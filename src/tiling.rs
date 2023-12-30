use kurbo::{Affine, BezPath, Point, Rect, Vec2};
pub struct Tile {
    pub corners: Vec<Point>,
}

impl Tile {
    pub fn rhombus(l: f64, angle: f64) -> Tile {
        let angle = angle.to_radians() * 0.5;
        let dx = angle.sin() * l;
        let dy = angle.cos() * l;
        Tile {
            corners: vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: -dx, y: dy },
                Point {
                    x: 0.0,
                    y: 2.0 * dy,
                },
                Point { x: dx, y: dy },
            ],
        }
    }

    pub fn add_to_path(&self, out: &mut BezPath) {
        if self.corners.is_empty() {
            return;
        }
        out.move_to(self.corners[0]);
        for p in self.corners.iter().skip(1) {
            out.line_to(*p);
        }
        out.close_path();
    }

    pub fn add_to_path_t(&self, out: &mut BezPath, transform: &Affine) {
        if self.corners.is_empty() {
            return;
        }
        out.move_to(*transform * self.corners[0]);
        for p in self.corners.iter().skip(1) {
            out.line_to(*transform * *p);
        }
        out.close_path();
    }

    pub fn to_path(&self) -> BezPath {
        let mut result = BezPath::new();
        self.add_to_path(&mut result);
        return result;
    }
}

#[derive(Clone)]
pub struct TilePlacement {
    pub tile_id: usize,
    pub transform: Affine,
}
pub struct TilingRule {
    pub tile: Tile,
    pub result: Vec<TilePlacement>,
}
pub struct TilingStep {
    pub rules: Vec<TilingRule>,
    pub expansion_factor: f64,
}

const DEFAULT_POLYGON_LIMIT: usize = 1000000;

impl TilingStep {
    pub fn expand_tile(&self, placed_tile: &TilePlacement, output: &mut Vec<TilePlacement>) {
        let rule = &self.rules[placed_tile.tile_id];
        for item in &rule.result {
            let mut new_tile = item.clone();
            new_tile.transform = placed_tile.transform * new_tile.transform;
            output.push(new_tile);
        }
    }

    pub fn expand_levels(
        &self,
        input: &Vec<TilePlacement>,
        levels: usize,
        output: &mut Vec<TilePlacement>,
        max_tiles: Option<usize>,
    ) {
        let mut a = input.clone();
        let mut b = Vec::new();
        for _i in 0..levels {
            for tile in &a {
                self.expand_tile(&tile, &mut b);
                if let Some(x) = max_tiles {
                    if x < b.len() {
                        break;
                    }
                }
            }
            std::mem::swap(&mut a, &mut b);
            b.clear();
        }
        output.append(&mut a);
    }

    fn estimate_bounds(&self, placed_tile: &TilePlacement) -> Rect {
        let tile = &self.rules[placed_tile.tile_id];
        let mut result = Rect::from_origin_size(
            placed_tile.transform.translation().to_point(),
            (0.0_f64, 0.0_f64),
        );
        for p in &tile.tile.corners {
            let p2 = placed_tile.transform * *p;
            result = result.union_pt(p2);
        }
        let max_size = f64::max(result.width(), result.height());
        return result.inflate(max_size, max_size);
    }

    pub fn expand_bound(
        &self,
        input: &Vec<TilePlacement>,
        levels: usize,
        bounds: kurbo::Rect,
        output: &mut Vec<TilePlacement>,
        max_tiles: Option<usize>,
    ) {
        let mut a = input.clone();
        let mut b = Vec::new();
        for _i in 0..levels {
            for tile in &a {
                let tile_bounds = self.estimate_bounds(tile);
                if tile_bounds.intersect(bounds).is_empty() {
                    continue;
                }
                self.expand_tile(&tile, &mut b);
                if let Some(x) = max_tiles {
                    if x < b.len() {
                        break;
                    }
                }
            }
            std::mem::swap(&mut a, &mut b);
            b.clear();
        }
        output.append(&mut a);
    }

    pub fn expand_0_levels(
        &self,
        levels: usize,
        initial_scale: f64,
        bounds: Option<Rect>,
        output: &mut Vec<TilePlacement>,
    ) {
        let input = vec![TilePlacement {
            tile_id: 1,
            transform: Affine::scale(initial_scale),
        }];
        if let Some(bounds) = bounds {
            self.expand_bound(&input, levels, bounds, output, Some(DEFAULT_POLYGON_LIMIT));
        } else {
            self.expand_levels(&input, levels, output, Some(DEFAULT_POLYGON_LIMIT));
        }
    }

    pub fn to_bez_path(&self, tiles: &Vec<TilePlacement>) -> BezPath {
        let mut result = BezPath::new();
        for tile in tiles {
            let info = &self.rules[tile.tile_id];
            info.tile.add_to_path_t(&mut result, &tile.transform);
        }
        return result;
    }

    pub fn new() -> TilingStep {
        TilingStep {
            rules: Vec::new(),
            expansion_factor: 1.0,
        }
    }
}
