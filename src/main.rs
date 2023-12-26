use std::{vec, time::Instant};

use kurbo::{Affine, BezPath, Point, Vec2};
use whiskers::{prelude::*, widgets::Widget};

#[derive(Sketch)]
struct GridSketch {
    #[param(slider, min = 20.0, max = 400.0)]
    width: f64,
    #[param(slider, min = 20.0, max = 400.0)]
    height: f64,
    #[param(slider, min = 0.001, max = 20.0)]
    line_thickness: f64,
    initial_scale: f64,
    levels: usize,
    tiling: TilingStep,
}

struct Tile {
    corners: Vec<Point>,
}

impl Tile {
    fn rhombus(l: f64, angle: f64) -> Tile {
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
}

#[derive(Clone)]
struct TilePlacement {
    tile_id: usize,
    transform: Affine,
}
struct TilingRule {
    tile: Tile,
    result: Vec<TilePlacement>,
}

#[derive(Default)]
struct DummyWidget {}

impl Widget<TilingStep> for DummyWidget {
    fn ui(&self, ui: &mut egui::Ui, label: &str, value: &mut TilingStep) -> bool {
        return false;
    }
}
register_widget_ui!(TilingStep, DummyWidget);
struct TilingStep {
    rules: Vec<TilingRule>,
}

const DEFAULT_POLYGON_LIMIT:usize = 1000000;

impl TilingStep {
    fn expand_tile(&self, placed_tile: &TilePlacement, output: &mut Vec<TilePlacement>) {
        let rule = &self.rules[placed_tile.tile_id];
        for item in &rule.result {
            let mut new_tile = item.clone();
            new_tile.transform = placed_tile.transform * new_tile.transform;
            output.push(new_tile);
        }
    }

    fn expand_levels(
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

    fn expand_0_levels(&self, levels: usize, initial_scale: f64, output: &mut Vec<TilePlacement>) {
        let input = vec![TilePlacement {
            tile_id: 1,
            transform: Affine::scale(initial_scale),
        }];
        self.expand_levels(&input, levels, output, Some(DEFAULT_POLYGON_LIMIT));
    }

    fn to_bez_path(&self, tiles: &Vec<TilePlacement>) -> BezPath {
        let mut result = BezPath::new();
        for tile in tiles {
            let info = &self.rules[tile.tile_id];
            if info.tile.corners.is_empty() {
                continue;
            }
            let corners = &info.tile.corners;
            result.move_to(tile.transform * corners[0]);
            for corner in corners {
                result.line_to(tile.transform * *corner);
            }
            result.line_to(tile.transform * corners[0]);
        }
        return result;
    }

    fn new() -> TilingStep {
        TilingStep { rules: Vec::new() }
    }
}

impl Default for GridSketch {
    fn default() -> Self {
        Self {
            width: 100.0,
            height: 100.0,
            line_thickness: 0.5,
            initial_scale: 1.0,
            tiling: TilingStep::new(),
            levels: 5,
        }
    }
}

impl App for GridSketch {
    fn update(&mut self, sketch: &mut Sketch, _ctx: &mut Context) -> anyhow::Result<()> {
        sketch.scale(Unit::Mm);
        sketch.stroke_width(self.line_thickness);

        let mut shapes: Vec<TilePlacement> = Vec::new();
        let before = Instant::now();
        self.tiling
            .expand_0_levels(self.levels, self.initial_scale, &mut shapes);
        println!("Generate time: {:.2?}", before.elapsed());
        let before = Instant::now();
        let path = self.tiling.to_bez_path(&shapes);
        println!("Convert to path time: {:.2?}", before.elapsed());
        let before = Instant::now();
        sketch.add_path(path);
        println!("Sketch time: {:.2?}", before.elapsed());
        

        sketch.rect(0f64, 0f64, self.height, self.width);
        Ok(())
    }
}

fn main() -> Result {
    let SQUARE_GRID: TilingStep = TilingStep {
        rules: vec![TilingRule {
            tile: Tile {
                corners: vec![
                    Point { x: 0f64, y: 0f64 },
                    Point { x: 0f64, y: 1f64 },
                    Point { x: 1f64, y: 1f64 },
                    Point { x: 1f64, y: 0f64 },
                ],
            },
            result: vec![
                TilePlacement {
                    tile_id: 0,
                    transform: (Affine::scale(0.5)).pre_translate(Vec2::new(0.0, 0.0)),
                },
                TilePlacement {
                    tile_id: 0,
                    transform: (Affine::scale(0.5)).then_translate(Vec2::new(0.5, 0.0)),
                },
                TilePlacement {
                    tile_id: 0,
                    transform: (Affine::scale(0.5)).then_translate(Vec2::new(0.0, 0.5)),
                },
                TilePlacement {
                    tile_id: 0,
                    transform: (Affine::scale(0.5)).then_translate(Vec2::new(0.5, 0.5)),
                },
            ],
        }],
    };

    let soc_scale = 1.0 / (1.0 + 2.0 * 36_f64.to_radians().cos());
    let socd1 = 36_f64.to_radians().sin_cos();
    let soc_corn1 = Vec2::new(-socd1.0, socd1.1);
    let soc_corn2 = Vec2::new(0.0, 2.0 * socd1.1);
    let soc_corn3 = Vec2::from(socd1);
    let soc_p4 = (soc_corn2 + Vec2::new(0.0, 1.0)) * soc_scale;
    let socd2 = 18_f64.to_radians().sin_cos();
    let soc_cornb1 = Vec2::new(-socd2.0, socd2.1);
    let soc_cornb2 = Vec2::new(0.0, 2.0 * socd2.1);
    let soc_cornb3 = Vec2::from(socd2);
    let socolar_5 = TilingStep {
        rules: vec![
            TilingRule {
                tile: Tile::rhombus(1.0, 72.0),
                result: vec![
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale)),
                    },
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(-144_f64.to_radians())
                            .then_translate(soc_corn1),
                    },
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(-72_f64.to_radians())
                            .then_translate(soc_corn1),
                    },
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(72_f64.to_radians())
                            .then_translate(soc_corn3),
                    },
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(144_f64.to_radians())
                            .then_translate(soc_corn2),
                    },
                    TilePlacement {
                        tile_id: 1,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(162_f64.to_radians())
                            .then_translate(soc_p4),
                    },
                    TilePlacement {
                        tile_id: 1,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(18_f64.to_radians())
                            .then_translate(soc_corn3 * soc_scale),
                    },
                    TilePlacement {
                        tile_id: 1,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(90_f64.to_radians())
                            .then_translate(soc_corn3 + 1.0 * (soc_corn1 * soc_scale)),
                    },
                ],
            },
            TilingRule {
                tile: Tile::rhombus(1.0, 36.0),
                result: vec![
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(-18_f64.to_radians())
                            .then_translate(Vec2::default()),
                    },
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(-162_f64.to_radians())
                            .then_translate(soc_cornb1),
                    },
                    TilePlacement {
                        tile_id: 0,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(-90_f64.to_radians())
                            .then_translate(soc_cornb1),
                    },
                    TilePlacement {
                        tile_id: 1,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(144_f64.to_radians())
                            .then_translate(soc_cornb3),
                    },
                    TilePlacement {
                        tile_id: 1,
                        transform: (Affine::scale(soc_scale))
                            .then_rotate(-144_f64.to_radians())
                            .then_translate(soc_cornb2 - soc_cornb3 * soc_scale),
                    },
                ],
            },
        ],
    };

    let mut data = GridSketch::default();
    data.tiling = socolar_5;
    Runner::new(data)
        .with_page_size_options(PageSize::A5H)
        .with_layout_options(LayoutOptions::Center)
        .run()
}
