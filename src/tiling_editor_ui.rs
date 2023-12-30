use std::ops::Mul;
use std::sync::{Arc, Mutex};

use crate::tiling::*;
use egui::{emath, Id, Rect};
use kurbo::{Affine, BezPath, Point, Shape};
use whiskers::prelude::egui::emath::RectTransform;
use whiskers::prelude::egui::epaint::PathShape;
use whiskers::prelude::egui::{epaint, Color32, Painter, Pos2, Response, Sense, Stroke, Vec2};
use whiskers::widgets::Widget;
use whiskers::{prelude::*, register_widget_ui};

#[derive(Default)]
pub struct TilingEditorWidget {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Move,
}

#[derive(Clone)]
enum Selection {
    None,
    Points { shape: usize, corners: Vec<usize> },
    Shapes { shape: Vec<usize> },
}

struct WindowState {
    open: bool,
    current_tile: usize,
    draw_transform: RectTransform,
    tool: Tool,
    selection: Selection,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            open: Default::default(),
            current_tile: Default::default(),
            draw_transform: RectTransform::identity(egui::Rect::ZERO),
            tool: Tool::Select,
            selection: Selection::None,
        }
    }
}

/*fn as_points(tile: &Tile, tr: &RectTransform) -> Vec<Pos2> {
    tile.corners
        .iter()
        .map(|p| tr * (p.x as f32, p.y as f32).into())
        .collect()
}*/

fn to_point(p: Pos2) -> kurbo::Point {
    return Point::new(p.x.into(), p.y.into());
}

fn to_pos(p: kurbo::Point) -> Pos2 {
    return Pos2::new(p.x as f32, p.y as f32);
}

fn as_points(tile: &Tile, placement: &Affine, tr: &RectTransform) -> Vec<Pos2> {
    tile.corners
        .iter()
        .map(|p| {
            let p = *placement * *p;
            tr * (p.x as f32, p.y as f32).into()
        })
        .collect()
}

fn rough_bounds(path: &BezPath, transform: &RectTransform) -> Rect {
    let bbox = path.bounding_box();
    let mut res = Rect::NOTHING;
    res.extend_with(transform.transform_pos(to_pos(bbox.origin())));
    let p2 = Point::new(bbox.x1, bbox.y1);
    res.extend_with(transform.transform_pos(to_pos(p2)));
    return res;
}

impl WindowState {
    fn display_shapes(
        &mut self,
        ui: &mut egui::Ui,
        value: &mut TilingStep,
        (response, painter): &(Response, Painter),
    ) {
        if response.clicked() {
            self.selection = Selection::None;
        }

        let current_rule = &value.rules[self.current_tile];
        let mouse_pos = ui
            .input(|inp| inp.pointer.hover_pos())
            .unwrap_or(Pos2::new(0.0, 0.0));
        let draw_mouse_pos = to_point(self.draw_transform.inverse().transform_pos(mouse_pos));

      

        let current_rule = &value.rules[self.current_tile];
        for (j, shape) in current_rule.result.iter().enumerate() {
            let tile = &value.rules[shape.tile_id].tile;

            let points = as_points(tile, &shape.transform, &self.draw_transform);
            for (i, p) in points.iter().enumerate() {
                let point_rect = Rect::from_center_size(*p, Vec2::new(8.0, 8.0));
                let point_resp = ui.interact(
                    point_rect,
                    response.id.with("point").with(j).with(i),
                    Sense::drag(),
                );
                if point_resp.hovered() {
                    

                    painter.circle(
                        *p,
                        7.0,
                        Color32::TRANSPARENT,
                        Stroke::new(1.0, Color32::GREEN),
                    );
                }
                if point_resp.clicked() {
                    println!("corner click");
                    let shift = ui.input(|x| x.modifiers.shift);
                    if !shift {
                        self.selection = Selection::Points {
                            shape: j,
                            corners: vec![i],
                        };
                    } else {
                        let selection_copy = self.selection.clone();
                        self.selection = match &selection_copy {
                            Selection::Points { shape, corners } if *shape == j => {
                                if corners.contains(&i) {
                                    let indexes =
                                        corners.iter().copied().filter(|x| *x == i).collect();
                                    Selection::Points {
                                        shape: j,
                                        corners: indexes,
                                    }
                                } else {
                                    let mut indexes = corners.clone();
                                    indexes.push(i);
                                    Selection::Points {
                                        shape: j,
                                        corners: indexes,
                                    }
                                }
                            }
                            _ => Selection::Points {
                                shape: j,
                                corners: vec![i],
                            },
                        }
                    }
                }

                match &self.selection {
                    Selection::Points { shape, corners } if *shape == j && corners.contains(&i) => {
                        painter.circle(
                            *p,
                            8.0,
                            Color32::TRANSPARENT,
                            Stroke::new(1.0, Color32::DARK_BLUE),
                        );
                    }
                    _ => {}
                }
            }

            let mut stroke = Stroke::new(1.0, Color32::BLACK);
            match &self.selection {
                Selection::Shapes { shape } if shape.contains(&j) => {
                    stroke.color = Color32::GREEN;
                }
                _ => {}
            }

            let shape = egui::Shape::closed_line(points, stroke);
            ui.painter().add(shape);
        }
    
        for (j, shape) in current_rule.result.iter().enumerate() {
            let tile = &value.rules[shape.tile_id].tile;
            let id = response.id.with("subtile").with(j);
            let positioned_tile = shape.transform * value.rules[shape.tile_id].tile.to_path();
            let hovered = positioned_tile.contains(draw_mouse_pos);
            let resp = ui.interact_with_hovered(
                rough_bounds(&positioned_tile, &self.draw_transform),
                hovered,
                id,
                Sense::drag(),
            );

            if resp.clicked() {
                self.selection = Selection::Shapes { shape: vec![j] };
                println!("shape click");
            }

        }
    
    }

    fn tiling_editor_window(&mut self, ui: &mut egui::Ui, value: &mut TilingStep, window_id: Id) {
        let ctx = ui.ctx();

        let mut open = self.open;
        egui::Window::new("My Window")
            .id(window_id)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Hello World!");

                let selected_tile = self.current_tile;
                egui::SidePanel::left("tileedit_left")
                    .resizable(true)
                    .default_width(150.0)
                    .width_range(80.0..=200.0)
                    .show_inside(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Left Panel");
                        });
                        let rule_selection = egui::ComboBox::from_label("Tile")
                            .selected_text(format!("{selected_tile}"))
                            .show_ui(ui, |ui| {
                                //ui.style_mut().wrap = Some(false);
                                //ui.set_min_width(60.0);
                                for (i, rule) in value.rules.iter().enumerate() {
                                    ui.selectable_value(&mut self.current_tile, i, format!("{i}"));
                                }
                            });

                        if rule_selection.response.changed() {
                            self.selection = Selection::None;
                        }

                        ui.radio_value(&mut self.tool, Tool::Select, "Select");
                        ui.radio_value(&mut self.tool, Tool::Move, "Move");
                        /*egui::ScrollArea::vertical().show(ui, |ui| {

                        });*/
                    });

                egui::SidePanel::right("tileedit_right")
                    .resizable(true)
                    .default_width(150.0)
                    .width_range(80.0..=200.0)
                    .show_inside(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Right Panel");
                        });
                        egui::ScrollArea::vertical().show(ui, |ui| {});
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Central Panel");
                    });
                    egui::ScrollArea::both().show(ui, |ui| {
                        let available_space = ui.available_size();
                        //let space = egui::Vec2::new(available_space.min_elem(), available_space.min_elem());
                        //let (_id, rect) = ui.allocate_space(available_space);
                        let (response, painter) =
                            ui.allocate_painter(available_space, Sense::click());

                        let target_rect = if available_space.x > available_space.y {
                            let xs = 0.5 * 4.0 * available_space.x / available_space.y;
                            Rect::from_x_y_ranges(-xs..=xs, 2.0..=-2.0)
                        } else {
                            let ys = 0.5 * 4.0 * available_space.y / available_space.x;
                            Rect::from_x_y_ranges(-2.0..=2.0, ys..=-ys)
                        };

                        let to_screen = emath::RectTransform::from_to(target_rect, response.rect);
                        self.draw_transform = to_screen.clone();

                        ui.painter().arrow(
                            to_screen * Pos2::new(-2.0, 0.0),
                            egui::Vec2::new(4.0, 0.0).mul(to_screen.scale()),
                            egui::Stroke::new(1.0, Color32::GRAY),
                        );
                        ui.painter().arrow(
                            to_screen * Pos2::new(0.0, -2.0),
                            egui::Vec2::new(0.0, 4.0).mul(to_screen.scale()),
                            egui::Stroke::new(1.0, Color32::GRAY),
                        );

                        if !(0..=value.rules.len()).contains(&(self.current_tile)) {
                            return;
                        }
                        let rule = &value.rules[self.current_tile];
                        let points = as_points(&rule.tile, &Affine::IDENTITY, &to_screen);

                        painter.add(egui::Shape::closed_line(
                            points,
                            Stroke::new(4.0, Color32::LIGHT_BLUE),
                        ));

                        self.display_shapes(ui, value, &(response, painter));
                    });
                });
            });
        self.open = open;
    }
}

impl Widget<TilingStep> for TilingEditorWidget {
    fn ui(&self, ui: &mut egui::Ui, label: &str, value: &mut TilingStep) -> bool {
        let window_id = Id::new("My window");
        let ctx = ui.ctx();
        let window_data: Arc<Mutex<WindowState>> =
            ctx.memory(|mem| mem.data.get_temp(window_id).unwrap_or_default());
        {
            let mut window_state = window_data.lock().unwrap();
            window_state.tiling_editor_window(ui, value, window_id);
            if ui.button("Edit tiling").clicked() {
                window_state.open = true;
            }
        }
        ui.ctx().memory_mut(|mem| {
            mem.data.insert_temp(window_id, window_data);
        });

        false
    }
}

register_widget_ui!(TilingStep, TilingEditorWidget);
